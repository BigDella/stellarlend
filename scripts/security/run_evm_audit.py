#!/usr/bin/env python3
"""Run Slither and Mythril, normalize findings, and gate on severity."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


SEVERITY_ORDER = {
    "informational": 0,
    "info": 0,
    "optimization": 0,
    "low": 1,
    "medium": 2,
    "moderate": 2,
    "high": 3,
    "critical": 4,
}


@dataclass
class Finding:
    tool: str
    check: str
    severity: str
    source: str
    description: str
    confidence: str = ""
    id: str = ""
    suppressed: bool = False
    suppression_reason: str = ""
    baseline: bool = False
    raw: dict[str, Any] = field(default_factory=dict)

    def finalize(self) -> "Finding":
        self.severity = normalize_severity(self.severity)
        fingerprint = "|".join(
            [
                self.tool,
                self.check,
                self.severity,
                self.source,
                compact(self.description),
            ]
        )
        digest = hashlib.sha256(fingerprint.encode("utf-8")).hexdigest()[:16]
        self.id = f"{self.tool}:{self.check}:{digest}"
        return self

    def to_json(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "tool": self.tool,
            "check": self.check,
            "severity": self.severity,
            "confidence": self.confidence,
            "source": self.source,
            "description": self.description,
            "suppressed": self.suppressed,
            "suppression_reason": self.suppression_reason,
            "baseline": self.baseline,
        }


def compact(value: str) -> str:
    return " ".join(str(value or "").split())


def normalize_severity(value: str) -> str:
    severity = compact(value).lower()
    if severity in {"optimization", "informational", "info"}:
        return "informational"
    if severity in {"medium", "moderate"}:
        return "medium"
    if severity in {"low", "high", "critical"}:
        return severity
    return "low"


def severity_value(value: str) -> int:
    return SEVERITY_ORDER.get(normalize_severity(value), 1)


def load_json(path: Path, default: dict[str, Any]) -> dict[str, Any]:
    if not path.exists():
        return default
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, sort_keys=True)
        handle.write("\n")


def run_command(command: list[str], log_path: Path, timeout: int) -> int:
    log_path.parent.mkdir(parents=True, exist_ok=True)
    with log_path.open("w", encoding="utf-8") as log:
        log.write(f"$ {' '.join(command)}\n\n")
        try:
            completed = subprocess.run(
                command,
                stdout=log,
                stderr=subprocess.STDOUT,
                text=True,
                timeout=timeout,
                check=False,
            )
            return completed.returncode
        except subprocess.TimeoutExpired:
            log.write(f"\nCommand timed out after {timeout} seconds.\n")
            return 124


def run_json_command(command: list[str], json_path: Path, log_path: Path, timeout: int) -> int:
    json_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.parent.mkdir(parents=True, exist_ok=True)
    with log_path.open("w", encoding="utf-8") as log:
        log.write(f"$ {' '.join(command)}\n\n")
        try:
            completed = subprocess.run(
                command,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=timeout,
                check=False,
            )
        except subprocess.TimeoutExpired:
            log.write(f"\nCommand timed out after {timeout} seconds.\n")
            return 124

        if completed.stdout.strip():
            json_path.write_text(completed.stdout, encoding="utf-8")
        if completed.stderr:
            log.write(completed.stderr)
        if completed.stdout and not completed.stdout.lstrip().startswith(("{", "[")):
            log.write("\nNon-JSON stdout:\n")
            log.write(completed.stdout)
        return completed.returncode


def source_from_slither(detector: dict[str, Any]) -> str:
    elements = detector.get("elements") or []
    for element in elements:
        source = element.get("source_mapping") or {}
        filename = source.get("filename_relative") or source.get("filename")
        lines = source.get("lines") or []
        if filename:
            if lines:
                return f"{filename}:{lines[0]}"
            return filename
    return "unknown"


def parse_slither(path: Path) -> list[Finding]:
    payload = load_json(path, {"results": {"detectors": []}})
    findings: list[Finding] = []
    for detector in payload.get("results", {}).get("detectors", []) or []:
        findings.append(
            Finding(
                tool="slither",
                check=str(detector.get("check") or detector.get("impact") or "detector"),
                severity=str(detector.get("impact") or "low"),
                confidence=str(detector.get("confidence") or ""),
                source=source_from_slither(detector),
                description=compact(detector.get("description") or detector.get("markdown") or ""),
                raw=detector,
            ).finalize()
        )
    return findings


def source_from_mythril(issue: dict[str, Any], contract_path: Path) -> str:
    locations = issue.get("locations") or []
    if locations:
        source_map = locations[0].get("sourceMap") or {}
        filename = source_map.get("filename") or str(contract_path)
        line = source_map.get("lineno")
        if line:
            return f"{filename}:{line}"
        return filename
    return str(contract_path)


def parse_mythril(path: Path, contract_path: Path) -> list[Finding]:
    payload = load_json(path, {"issues": []})
    findings: list[Finding] = []
    for issue in payload.get("issues", []) or []:
        check = str(issue.get("swc-id") or issue.get("title") or "issue")
        findings.append(
            Finding(
                tool="mythril",
                check=check,
                severity=str(issue.get("severity") or "low"),
                source=source_from_mythril(issue, contract_path),
                description=compact(issue.get("description") or issue.get("title") or ""),
                raw=issue,
            ).finalize()
        )
    return findings


def load_baseline(path: Path) -> set[str]:
    payload = load_json(path, {"findings": []})
    ids = set()
    for item in payload.get("findings", []) or []:
        if isinstance(item, str):
            ids.add(item)
        elif isinstance(item, dict) and item.get("id"):
            ids.add(str(item["id"]))
    return ids


def suppression_matches(finding: Finding, suppression: dict[str, Any]) -> bool:
    if suppression.get("id") and suppression["id"] != finding.id:
        return False
    for key in ("tool", "check", "severity", "source"):
        expected = suppression.get(key)
        if expected and str(expected).lower() not in str(getattr(finding, key)).lower():
            return False
    return any(suppression.get(key) for key in ("id", "tool", "check", "severity", "source"))


def apply_controls(findings: list[Finding], baseline_path: Path, suppressions_path: Path) -> None:
    baseline_ids = load_baseline(baseline_path)
    suppression_payload = load_json(suppressions_path, {"suppressions": []})
    suppressions = suppression_payload.get("suppressions", []) or []

    for finding in findings:
        finding.baseline = finding.id in baseline_ids
        for suppression in suppressions:
            if suppression_matches(finding, suppression):
                finding.suppressed = True
                finding.suppression_reason = str(suppression.get("reason") or "suppressed")
                break


def write_summary(
    path: Path,
    contracts: list[Path],
    findings: list[Finding],
    blocking: list[Finding],
    tool_errors: list[str],
    threshold: str,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    counts: dict[str, int] = {severity: 0 for severity in ("critical", "high", "medium", "low", "informational")}
    for finding in findings:
        counts[finding.severity] = counts.get(finding.severity, 0) + 1

    with path.open("w", encoding="utf-8") as handle:
        handle.write("# EVM Security Audit Summary\n\n")
        handle.write(f"- Solidity files scanned: {len(contracts)}\n")
        handle.write(f"- Total findings: {len(findings)}\n")
        handle.write(f"- Gate threshold: {threshold}\n")
        handle.write(f"- Blocking findings: {len(blocking)}\n\n")
        handle.write("| Severity | Count |\n| --- | ---: |\n")
        for severity in ("critical", "high", "medium", "low", "informational"):
            handle.write(f"| {severity} | {counts.get(severity, 0)} |\n")
        if tool_errors:
            handle.write("\n## Tool Errors\n\n")
            for error in tool_errors:
                handle.write(f"- {error}\n")
        if blocking:
            handle.write("\n## Blocking Findings\n\n")
            for finding in blocking:
                handle.write(f"- `{finding.id}` {finding.severity}: {finding.description} ({finding.source})\n")


def ensure_tools_available(tool_names: list[str]) -> list[str]:
    missing = [tool for tool in tool_names if shutil.which(tool) is None]
    return [f"{tool} is not installed or not on PATH" for tool in missing]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--contracts-dir", type=Path, default=Path("contracts/evm"))
    parser.add_argument("--out-dir", type=Path, default=Path("security-reports/evm"))
    parser.add_argument("--baseline", type=Path, default=Path("contracts/evm/security-baseline.json"))
    parser.add_argument("--suppressions", type=Path, default=Path("contracts/evm/security-suppressions.json"))
    parser.add_argument("--severity-threshold", default="medium")
    parser.add_argument("--timeout", type=int, default=900)
    args = parser.parse_args()

    args.out_dir.mkdir(parents=True, exist_ok=True)
    contracts = sorted(args.contracts_dir.rglob("*.sol")) if args.contracts_dir.exists() else []
    threshold = normalize_severity(args.severity_threshold)

    if not contracts:
        write_json(args.out_dir / "findings.json", [])
        write_summary(args.out_dir / "summary.md", contracts, [], [], [], threshold)
        print(f"No Solidity contracts found under {args.contracts_dir}; wrote empty audit report.")
        return 0

    tool_errors = ensure_tools_available(["slither", "myth"])
    if tool_errors:
        write_json(args.out_dir / "findings.json", [])
        write_summary(args.out_dir / "summary.md", contracts, [], [], tool_errors, threshold)
        for error in tool_errors:
            print(error, file=sys.stderr)
        return 2

    findings: list[Finding] = []

    slither_json = args.out_dir / "slither.json"
    slither_cmd = [
        "slither",
        str(args.contracts_dir),
        "--json",
        str(slither_json),
        "--config-file",
        str(args.contracts_dir / "slither.config.json"),
    ]
    slither_rc = run_command(slither_cmd, args.out_dir / "slither.log", args.timeout)
    if slither_json.exists():
        findings.extend(parse_slither(slither_json))
    if slither_rc not in (0, 255):
        tool_errors.append(f"Slither exited with status {slither_rc}; see slither.log")

    mythril_dir = args.out_dir / "mythril"
    mythril_dir.mkdir(parents=True, exist_ok=True)
    for contract in contracts:
        safe_name = hashlib.sha256(str(contract).encode("utf-8")).hexdigest()[:12]
        mythril_json = mythril_dir / f"{safe_name}.json"
        mythril_log = mythril_dir / f"{safe_name}.log"
        mythril_cmd = [
            "myth",
            "analyze",
            str(contract),
            "--execution-timeout",
            str(args.timeout),
            "--output",
            "json",
        ]
        mythril_rc = run_json_command(mythril_cmd, mythril_json, mythril_log, args.timeout + 30)
        if mythril_json.exists():
            findings.extend(parse_mythril(mythril_json, contract))
        if mythril_rc not in (0, 1):
            tool_errors.append(f"Mythril exited with status {mythril_rc} for {contract}; see {mythril_log.name}")

    apply_controls(findings, args.baseline, args.suppressions)
    normalized = [finding.to_json() for finding in sorted(findings, key=lambda item: item.id)]
    write_json(args.out_dir / "findings.json", normalized)

    blocking = [
        finding
        for finding in findings
        if not finding.suppressed
        and not finding.baseline
        and severity_value(finding.severity) >= severity_value(threshold)
    ]
    write_summary(args.out_dir / "summary.md", contracts, findings, blocking, tool_errors, threshold)

    if tool_errors:
        print("Security audit completed with tool errors.", file=sys.stderr)
        return 2
    if blocking:
        print(f"Security audit failed with {len(blocking)} blocking finding(s).", file=sys.stderr)
        return 1

    print(f"Security audit passed with {len(findings)} finding(s).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
