# StellarLend Gas Benchmark Suite

Comprehensive gas benchmarks for all StellarLend protocol contracts. Tracks CPU instruction counts (the primary Soroban fee proxy) per operation, detects regressions against a stored baseline, and integrates with CI to enforce gas budgets.

## Overview

| Contract | Operations Benchmarked |
|----------|----------------------|
| `lending` | 23 |
| `hello-world` (core lending) | 28 |
| `amm` | 15 |
| `bridge` | 17 |
| **Total** | **83** |

## How Gas is Measured

Soroban charges fees based on **CPU instructions** and **memory bytes** consumed per transaction. The `soroban-sdk` testutils expose `env.cost_estimate()` which provides these counts without deploying to a real network.

Each benchmark:
1. Creates a fresh `Env` with `mock_all_auths()`
2. Resets the cost estimator via `env.cost_estimate().reset()`
3. Invokes the target function
4. Records `cpu_insns()` and `mem_bytes()`

## Cold vs Warm Storage

Soroban charges more for the **first access** to a storage entry (cold read/write) than for subsequent accesses within the same transaction context (warm). Both patterns are benchmarked:

- **Cold benchmarks**: Fresh `Env`, no prior state — measures worst-case cost
- **Warm benchmarks**: State pre-populated, then cost estimator reset — measures amortized cost

## Running Benchmarks

```bash
# From project root
./run-benchmarks.sh                    # Run all benchmarks
./run-benchmarks.sh --compare          # Compare against baseline (fail on regression)
./run-benchmarks.sh --update-baseline  # Run and save as new baseline
```

Or directly with cargo:

```bash
cd stellar-lend
cargo run --bin run_benchmarks
cargo run --bin run_benchmarks -- --compare benchmarks/baseline.json
cargo run --bin run_benchmarks -- --output my-results.json
cargo run --bin run_benchmarks -- --threshold 5.0  # Set custom regression threshold
```

### Environment-Agnostic Configuration

The benchmark suite supports configuration via environment variables:

```bash
export BENCHMARK_REGRESSION_THRESHOLD=5.0
export BENCHMARK_OUTPUT_JSON=custom-results.json
export BENCHMARK_BASELINE=benchmarks/baseline.json
cargo run --bin run_benchmarks
```

Or use the `config.toml` file in the benchmarks directory for persistent configuration.

## Output

Results are written to multiple formats:

- **JSON**: `benchmark-results.json` - Machine-readable full results
- **Markdown**: `benchmark-results.md` - Human-readable report with tables
- **History**: `benchmarks/history.json` - Historical trend data with git metadata

### JSON Output

```json
{
  "version": "0.1.0",
  "timestamp": "2026-04-23T...",
  "total_benchmarks": 83,
  "passed": 83,
  "failed": 0,
  "results": [
    {
      "operation": "lending::deposit",
      "contract": "lending",
      "description": "Deposit asset — first deposit (cold storage write)",
      "instructions": 245000,
      "memory_bytes": 18432,
      "storage_reads": 1,
      "storage_writes": 2,
      "cold_storage": true,
      "budget": 800000,
      "within_budget": true,
      "timestamp": "...",
      "tags": ["deposit", "cold"]
    }
  ],
  "summary_by_contract": { ... }
}
```

### Historical Trend Storage

The `benchmarks/history.json` file stores all benchmark runs with git metadata:

```json
[
  {
    "timestamp": "2026-04-23T...",
    "operation": "lending::deposit",
    "instructions": 245000,
    "memory_bytes": 18432,
    "git_commit": "abc123...",
    "git_branch": "main"
  }
]
```

This enables tracking performance trends over time and correlating changes with specific commits.

## Gas Budgets

Default budgets (instruction count limits) are defined in `src/framework.rs`. They represent conservative upper bounds — tighten them after profiling real workloads.

| Operation Category | Budget (instructions) |
|-------------------|----------------------|
| Initialize | 300k – 500k |
| Simple admin writes | 200k – 400k |
| Deposit / Withdraw | 800k – 1M |
| Borrow / Repay | 1M – 1.2M |
| Liquidate | 1.5M |
| Flash loan | 1.8M |
| Query (read-only) | 200k – 400k |

## Regression Detection

The benchmark suite compares results against `benchmarks/baseline.json` and flags regressions when:

1. **Hard budget violation**: `instructions > budget` for any operation
2. **Relative regression**: Instructions increased by **>5%** vs baseline (configurable)

To update the baseline after an intentional optimization:

```bash
./run-benchmarks.sh --update-baseline
git add stellar-lend/benchmarks/baseline.json
git commit -m "chore: update gas benchmark baseline after optimization"
```

## CI Integration

The `.github/workflows/gas-benchmarks.yml` workflow:

- Runs on every PR touching `contracts/**` or `benchmarks/**`
- Builds and runs the full benchmark suite
- Compares against `baseline.json` if it has recorded results
- Fails the PR if any operation exceeds its gas budget or shows >5% regression
- Uploads `benchmark-results.json`, `benchmark-results.md`, and `benchmarks/history.json` as CI artifacts (retained 90 days)
- Posts a summary table and full markdown report to the GitHub Actions step summary

## Edge Cases Covered

| Edge Case | How Benchmarked |
|-----------|----------------|
| Cold storage access | Fresh `Env` per benchmark |
| Warm storage access | `warm_env_after()` helper resets cost after setup |
| Storage write patterns | Multi-asset / multi-bridge deposit benchmarks |
| Empty vs populated history | Separate benchmarks for empty/populated query results |
| Zero-fee operations | `compute_fee_zero_rate` benchmark |
| Compiler optimization changes | Benchmarks run in `--release` mode (same as production) |

## Adding New Benchmarks

1. Add a function to the relevant `*_benchmarks.rs` file following the existing pattern
2. Register it in the `run_all()` function of that module
3. Add a budget entry in `framework.rs` `default_budgets()`
4. Run `./run-benchmarks.sh --update-baseline` to record the new baseline

```rust
fn bench_my_new_operation(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::my_new_operation";
    let env = fresh_env();
    // ... setup ...

    let (insns, mem) = measure_instructions(&env, || {
        client.my_new_operation(/* args */);
    });

    BenchmarkResult::new(
        op, CONTRACT,
        "Description of what this measures",
        insns, mem,
        /* storage_reads */ 1,
        /* storage_writes */ 1,
        /* cold_storage */ true,
        get_budget(config, op),
        vec!["tag1".into(), "tag2".into()],
    )
}
```
