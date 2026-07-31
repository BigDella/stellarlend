# Contract Fuzz Testing Pipeline

The fuzzing pipeline uses `cargo-fuzz` under `stellar-lend/fuzz`.

## Targets

- `lending_actions`: structured lending operations covering deposit, borrow,
  liquidate, repay, withdraw, pause controls, oracle price changes, and view
  calls.
- `amm_actions`: AMM operation sequences.
- `bridge_actions`: bridge operation sequences.

Each target has seed corpora under `stellar-lend/fuzz/corpus/<target>`.
`scripts/fuzz/check_corpus.sh` enforces that corpora exist and are non-empty.

## CI Jobs

`.github/workflows/ci-cd.yml` keeps a smoke fuzz run in the default CI path.
`.github/workflows/contract-fuzzing.yml` runs a scheduled/manual long lending
fuzz campaign with `-max_total_time=1800`, giving the required 30-minute
minimum duration while capping the GitHub Actions job at 35 minutes.

The long job uploads:

- libFuzzer logs
- generated coverage output
- crash and timeout artifacts
- copied regression inputs

## Crash Triage

On failure, `scripts/fuzz/triage_crash.sh <target>` copies the first
`crash-*` or `timeout-*` artifact into
`stellar-lend/fuzz/regressions/<target>/` and prints a local reproduction
command:

```bash
bash scripts/fuzz/repro.sh lending_actions stellar-lend/fuzz/regressions/lending_actions/crash-...
```

Keep minimized regression artifacts in the target corpus once the bug is fixed
so future CI runs exercise the edge case.

## Local Commands

```bash
cd stellar-lend
cargo +nightly fuzz run lending_actions fuzz/corpus/lending_actions -- -max_total_time=60
cargo +nightly fuzz coverage lending_actions fuzz/corpus/lending_actions
```
