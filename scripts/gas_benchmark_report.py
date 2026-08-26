#!/usr/bin/env python3
"""Validate gas benchmark results and generate a trend dashboard."""

from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def load_json(path: Path, default: Any) -> Any:
    if not path or not path.exists():
        return default
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def normalize_operation(raw: dict[str, Any], default_contract: str = "") -> dict[str, Any]:
    operation = str(raw.get("operation") or raw.get("name") or "")
    contract = str(raw.get("contract") or default_contract or "")
    scenario = str(raw.get("scenario") or raw.get("description") or "")

    if "::" not in operation and contract:
        contract_key = contract.replace("-", "_")
        operation = f"{contract_key}::{operation}"

    instructions = raw.get("instructions", raw.get("cpu_insns", raw.get("cpu", 0)))
    memory = raw.get("memory_bytes", raw.get("mem_bytes", raw.get("memory", 0)))
    budget = int(raw.get("budget") or 0)

    return {
        "operation": operation,
        "contract": contract or operation.split("::", 1)[0],
        "scenario": scenario,
        "description": str(raw.get("description") or scenario or operation),
        "instructions": int(instructions or 0),
        "memory_bytes": int(memory or 0),
        "storage_reads": int(raw.get("storage_reads") or raw.get("disk_read_entries") or 0),
        "storage_writes": int(raw.get("storage_writes") or raw.get("write_entries") or 0),
        "cold_storage": bool(raw.get("cold_storage", "cold" in scenario.lower())),
        "budget": budget,
        "within_budget": bool(raw.get("within_budget", budget == 0 or int(instructions or 0) <= budget)),
        "tags": [str(tag) for tag in raw.get("tags", [])],
    }


def load_results(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    payload = load_json(path, {})
    if "results" in payload:
        return payload, [normalize_operation(item) for item in payload.get("results", [])]
    if "benchmarks" in payload:
        contract = str(payload.get("contract") or "")
        return payload, [normalize_operation(item, contract) for item in payload.get("benchmarks", [])]
    return payload, []


def load_baseline(path: Path) -> dict[str, dict[str, Any]]:
    if not path:
        return {}
    _, baseline_results = load_results(path)
    return {item["operation"]: item for item in baseline_results}


def coverage_failures(results: list[dict[str, Any]], coverage_path: Path | None) -> list[str]:
    if not coverage_path:
        return []
    coverage = load_json(coverage_path, {"required_operations": []})
    required = [str(item) for item in coverage.get("required_operations", [])]
    measured = {item["operation"] for item in results}
    return [operation for operation in required if operation not in measured]


def budget_failures(results: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [item for item in results if item["budget"] > 0 and item["instructions"] > item["budget"]]


def regression_failures(
    results: list[dict[str, Any]],
    baseline: dict[str, dict[str, Any]],
    max_regression_pct: float,
) -> list[dict[str, Any]]:
    failures = []
    for item in results:
        prior = baseline.get(item["operation"])
        if not prior:
            continue
        old = int(prior.get("instructions") or 0)
        new = int(item.get("instructions") or 0)
        if old <= 0:
            continue
        pct = ((new - old) / old) * 100
        if pct > max_regression_pct:
            failures.append({**item, "baseline": old, "increase_pct": pct})
    return failures


def storage_summary(results: list[dict[str, Any]]) -> dict[str, dict[str, int]]:
    summary: dict[str, dict[str, int]] = defaultdict(lambda: defaultdict(int))
    for item in results:
        bucket = summary[item["contract"]]
        bucket["operations"] += 1
        bucket["reads"] += item["storage_reads"]
        bucket["writes"] += item["storage_writes"]
        bucket["cold"] += 1 if item["cold_storage"] else 0
        bucket["max_instructions"] = max(bucket["max_instructions"], item["instructions"])
    return summary


def cross_contract_results(results: list[dict[str, Any]]) -> list[dict[str, Any]]:
    markers = ("cross", "bridge", "amm", "flash_loan", "callback", "auto_swap")
    selected = []
    for item in results:
        haystack = " ".join([item["operation"], item["description"], *item["tags"]]).lower()
        if any(marker in haystack for marker in markers):
            selected.append(item)
    return selected


def history_entry(payload: dict[str, Any], results: list[dict[str, Any]]) -> dict[str, Any]:
    instructions = [item["instructions"] for item in results]
    return {
        "timestamp": payload.get("timestamp") or datetime.now(timezone.utc).isoformat(),
        "source": "gas-benchmark-report",
        "total_benchmarks": len(results),
        "passed": len(results) - len(budget_failures(results)),
        "failed": len(budget_failures(results)),
        "max_instructions": max(instructions) if instructions else 0,
        "avg_instructions": int(sum(instructions) / len(instructions)) if instructions else 0,
    }


def read_history(path: Path | None) -> list[dict[str, Any]]:
    if not path or not path.exists():
        return []
    entries = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        try:
            entries.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return entries


def write_history(path: Path, entries: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [json.dumps(entry, sort_keys=True, separators=(",", ":")) for entry in entries]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def render_dashboard(
    payload: dict[str, Any],
    results: list[dict[str, Any]],
    missing: list[str],
    over_budget: list[dict[str, Any]],
    regressions: list[dict[str, Any]],
    history: list[dict[str, Any]],
) -> str:
    total = len(results)
    passed = total - len(over_budget)
    failed = len(over_budget)
    lines = [
        "# Gas Benchmark Dashboard",
        "",
        f"- Timestamp: {payload.get('timestamp') or datetime.now(timezone.utc).isoformat()}",
        f"- Total benchmarks: {total}",
        f"- Passed budgets: {passed}",
        f"- Failed budgets: {failed}",
        f"- Missing required operations: {len(missing)}",
        f"- Regression findings: {len(regressions)}",
        "",
        "## Contract Summary",
        "",
        "| Contract | Operations | Max instructions | Storage reads | Storage writes | Cold cases |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]

    for contract, item in sorted(storage_summary(results).items()):
        lines.append(
            f"| {contract} | {item['operations']} | {item['max_instructions']} | "
            f"{item['reads']} | {item['writes']} | {item['cold']} |"
        )

    lines.extend(["", "## Cross-Contract And Integration Calls", ""])
    cross_items = cross_contract_results(results)
    if cross_items:
        lines.extend(["| Operation | Instructions | Memory bytes |", "| --- | ---: | ---: |"])
        for item in sorted(cross_items, key=lambda row: row["operation"]):
            lines.append(f"| {item['operation']} | {item['instructions']} | {item['memory_bytes']} |")
    else:
        lines.append("No cross-contract tagged operations were found in this run.")

    if missing:
        lines.extend(["", "## Missing Coverage", ""])
        lines.extend(f"- `{operation}`" for operation in missing)

    if over_budget:
        lines.extend(["", "## Over Budget", "", "| Operation | Actual | Budget |", "| --- | ---: | ---: |"])
        for item in over_budget:
            lines.append(f"| {item['operation']} | {item['instructions']} | {item['budget']} |")

    if regressions:
        lines.extend(
            [
                "",
                "## Regressions Above Threshold",
                "",
                "| Operation | Current | Baseline | Increase |",
                "| --- | ---: | ---: | ---: |",
            ]
        )
        for item in regressions:
            lines.append(
                f"| {item['operation']} | {item['instructions']} | {item['baseline']} | "
                f"{item['increase_pct']:.2f}% |"
            )

    lines.extend(["", "## Historical Trend", ""])
    if history:
        lines.extend(["| Timestamp | Total | Passed | Failed | Max instructions | Avg instructions |", "| --- | ---: | ---: | ---: | ---: | ---: |"])
        for item in history[-10:]:
            lines.append(
                f"| {item.get('timestamp', '')} | {item.get('total_benchmarks', 0)} | "
                f"{item.get('passed', 0)} | {item.get('failed', 0)} | "
                f"{item.get('max_instructions', 0)} | {item.get('avg_instructions', 0)} |"
            )
    else:
        lines.append("No historical entries are available yet.")

    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--results", type=Path, required=True)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--coverage", type=Path)
    parser.add_argument("--dashboard", type=Path, required=True)
    parser.add_argument("--history", type=Path)
    parser.add_argument("--history-out", type=Path)
    parser.add_argument("--max-regression-pct", type=float, default=10.0)
    parser.add_argument("--fail-on-regression", action="store_true")
    args = parser.parse_args()

    payload, results = load_results(args.results)
    baseline = load_baseline(args.baseline) if args.baseline else {}
    missing = coverage_failures(results, args.coverage)
    over_budget = budget_failures(results)
    regressions = regression_failures(results, baseline, args.max_regression_pct)

    history = read_history(args.history)
    current_entry = history_entry(payload, results)
    history_with_current = [*history, current_entry]
    if args.history_out:
        write_history(args.history_out, history_with_current)

    write_text(
        args.dashboard,
        render_dashboard(payload, results, missing, over_budget, regressions, history_with_current),
    )

    errors = []
    if missing:
        errors.append(f"{len(missing)} required benchmark operation(s) missing")
    if over_budget:
        errors.append(f"{len(over_budget)} operation(s) over budget")
    if args.fail_on_regression and regressions:
        errors.append(f"{len(regressions)} operation(s) regressed by more than {args.max_regression_pct:.1f}%")

    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        print(f"Dashboard written to {args.dashboard}", file=sys.stderr)
        return 1

    print(f"Dashboard written to {args.dashboard}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
