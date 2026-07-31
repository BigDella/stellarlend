# Gas Benchmark Suite

This repository includes a contract gas benchmark suite for `hello-world` to track optimization impact across releases.

## Coverage

The benchmark test records per-operation CPU and memory costs for all public contract entrypoints in `HelloContract`, including:

- Core lending operations (`deposit_collateral`, `borrow_asset`, `repay_debt`, `withdraw_collateral`, `liquidate`)
- Admin and configuration entrypoints
- Read-only query entrypoints
- Governance and flash-loan related entrypoints

It also captures storage-sensitive scenarios:

- `deposit_collateral` in `write_cold` and `write_warm` modes
- `get_user_position` in `read_cold` and `read_warm` modes

These scenarios are used to detect cold vs warm storage deltas and storage write-pattern regressions.

## Run Benchmarks Locally

From repository root:

```bash
cd stellar-lend
GAS_BENCHMARK_OUTPUT=benchmarks/gas-current.json cargo test --package hello-world gas_benchmark -- --nocapture
python3 scripts/check_gas_benchmarks.py \
  --baseline benchmarks/gas-baseline.json \
  --current benchmarks/gas-current.json \
  --max-regression-pct 10
```

## Historical Comparison

- Baseline file: `stellar-lend/benchmarks/gas-baseline.json`
- Current CI output: `stellar-lend/benchmarks/gas-current.json` (artifact)

CI compares current results to baseline and fails when CPU or memory for any operation regresses beyond the configured threshold.

## CI Budget Alerts

CI benchmark job enforces a regression budget (`10%` by default). When exceeded, the pipeline fails and prints operation-level alerts.

## Notes / Edge Cases

- Network congestion does not affect these contract-level host cost benchmarks because they run in deterministic test environment.
- Compiler/toolchain changes can shift costs; update baseline only after intentional review.
- Storage-heavy operations are benchmarked in cold/warm variants to highlight storage access pattern changes.
