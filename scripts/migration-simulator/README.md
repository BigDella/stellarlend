# Migration Dry-Run Simulator

Simulates the entire migration lifecycle against a forked mainnet state before any
real funds or production state are touched.

## Process

```
pubnet state
    │
    ▼
snapshot.sh ──► snapshot-<label>.json
    │
    ▼
simulate.sh ──► simulate-<label>.json   (local Quickstart fork)
    │
    ▼
report.ts   ──► migration-plan-<label>.json  (gas + time estimates, invariant results)
    │
    ▼
approve.sh  ──► approval-<label>.json   (signed report binding)
    │
    ▼
index.ts    ──► verifies approval, gates mainnet migration
```

## Quick Start

### Prerequisites

- `stellar` CLI installed
- `node` ≥ 20
- `ts-node` (or `npx ts-node`)
- Docker (for local Quickstart sandbox)

### Run all phases

```bash
export LENDING_CONTRACT_ID=C...
export ADMIN_SECRET_KEY=S...
export APPROVER_SECRET_KEY=S...

ts-node scripts/migration-simulator/index.ts \
  --source-network pubnet \
  --lending-contract-id "$LENDING_CONTRACT_ID" \
  --admin-secret "$ADMIN_SECRET_KEY" \
  --approver-secret "$APPROVER_SECRET_KEY" \
  --dry-run
```

### Run individual phases

```bash
# 1. Snapshot mainnet state
bash scripts/migration-simulator/snapshot.sh \
  --source-network pubnet \
  --lending-contract-id "$LENDING_CONTRACT_ID"

# 2. Simulate migration in local fork
bash scripts/migration-simulator/simulate.sh \
  --snapshot-file .migration-sandbox/snapshots/snapshot-<label>.json \
  --lending-contract-id "$LENDING_CONTRACT_ID" \
  --admin-secret "$ADMIN_SECRET_KEY"

# 3. Generate report
npx ts-node scripts/migration-simulator/report.ts \
  --result   .migration-sandbox/results/simulate-<label>.json \
  --snapshot .migration-sandbox/snapshots/snapshot-<label>.json

# 4. Approve (sign the report)
bash scripts/migration-simulator/approve.sh \
  --report-file  .migration-sandbox/reports/migration-plan-<label>.json \
  --approver-key "$APPROVER_SECRET_KEY"
```

### Using the Docker sandbox

```bash
docker-compose \
  -f docker-compose.yml \
  -f environments/migration-sandbox/docker-compose.override.yml \
  up migration-sandbox
```

Then pass `--skip-sandbox-start` and `--rpc-url http://localhost:8000/soroban/rpc` to the scripts.

## Output Files

| File | Description |
|------|-------------|
| `.migration-sandbox/snapshots/snapshot-<label>.json` | Mainnet state snapshot |
| `.migration-sandbox/results/simulate-<label>.json` | Per-step simulation results |
| `.migration-sandbox/reports/migration-plan-<label>.json` | Full report with gas/time estimates |
| `.migration-sandbox/approvals/approval-<label>.json` | Signed approval bound to report hash |

## Invariants Checked

- `get_protocol_params` is readable post-migration
- `get_risk_config` is readable post-migration
- `get_system_stats` is readable post-migration
- `min_collateral_ratio` ≥ 100

## Approval Workflow

The approval file binds a cryptographic signature to the report's `contentHash`
(SHA-256 of the report body). `index.ts` verifies this binding before allowing
any mainnet migration to proceed, ensuring the approved report matches exactly
what was simulated.

## Configuration

Edit `environments/migration-sandbox/sandbox.config.json` to adjust:

- `gasEstimation.cpuInsnsPerXlm` — XLM cost model
- `gasEstimation.networkThroughputTxPerLedger` — assumed network throughput
- `gasEstimation.ledgerCloseTimeSec` — ledger close time for time estimates
- `invariants` — which invariant checks are active
