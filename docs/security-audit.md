# EVM Security Audit Pipeline

The EVM security audit workflow runs Slither and Mythril against Solidity contracts in `contracts/evm`. It is designed to keep scans deterministic enough for pull requests while still preserving full raw tool output as CI artifacts.

## What The Pipeline Checks

- Slither runs with all detectors enabled and emits raw JSON output.
- Mythril runs symbolic execution for each Solidity source file with a bounded timeout.
- Findings are normalized into `security-reports/evm/findings.json`.
- The workflow fails on new, unsuppressed findings at `medium` severity or higher.
- Raw reports, normalized findings, logs, and a Markdown summary are uploaded as artifacts.

## Severity Gate

Severity is normalized to one of `informational`, `low`, `medium`, `high`, or `critical`. The default gate is `medium`, which means any new unsuppressed `medium`, `high`, or `critical` finding fails CI.

The threshold can be changed for manual runs through the `severity_threshold` workflow input.

## Historical Tracking

Reviewed historical findings live in `contracts/evm/security-baseline.json`. A finding in the baseline is still reported, but it does not fail CI. New findings are identified by stable normalized ids built from the tool, detector/SWC id, source location, and description.

## False Positive Suppression

Use `contracts/evm/security-suppressions.json` for narrow suppressions when a finding has been reviewed and confirmed as a false positive. Suppressions can match by exact `id` or by fields such as `tool`, `check`, `source`, and `severity`.

Keep suppressions specific, include a reason, and prefer adding an `expires` date for temporary cases.

Example:

```json
{
  "tool": "slither",
  "check": "reentrancy-eth",
  "source": "contracts/evm/Vault.sol:120",
  "reason": "State update happens before the external call through a guarded adapter.",
  "expires": "2026-12-31"
}
```

## No EVM Contracts Yet

If no `.sol` files exist under `contracts/evm`, the runner writes an empty report and exits successfully. This keeps CI green until EVM contracts are added while preserving the audit gate for future Solidity code.
