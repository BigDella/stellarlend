# EVM Security Audit Configuration

This directory is the reserved home for Solidity/EVM contracts and their security audit configuration. The CI pipeline in `.github/workflows/security-audit.yml` scans every `.sol` file under this directory with Slither and Mythril.

## Files

- `slither.config.json` configures Slither without excluding detectors.
- `security-suppressions.json` records reviewed false positives.
- `security-baseline.json` tracks accepted historical findings so CI can gate on new medium-or-higher severity findings.

## Local Run

```bash
python -m pip install slither-analyzer mythril
python scripts/security/run_evm_audit.py \
  --contracts-dir contracts/evm \
  --out-dir security-reports/evm \
  --baseline contracts/evm/security-baseline.json \
  --suppressions contracts/evm/security-suppressions.json \
  --severity-threshold medium
```

The runner exits with a non-zero status when it finds a new, unsuppressed finding at or above the configured severity threshold.
