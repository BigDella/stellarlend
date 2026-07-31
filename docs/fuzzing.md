# Smart Contract Fuzzing

StellarLend uses coverage-guided fuzzing with `cargo-fuzz` and libFuzzer to exercise smart-contract state machines, edge-case accounting, ledger time jumps, oracle manipulation, and protocol invariants.

The fuzz package lives under `stellar-lend/fuzz/`.

## Targets

| Target | Scope |
| --- | --- |
| `lending_critical` | Focused lending path coverage for deposit, borrow, repay, liquidate, oracle price shifts, and time jumps |
| `lending_actions` | Broader lending state-machine coverage, including withdraw, pause toggles, oracle writes, and views |
| `amm_actions` | AMM action coverage for swaps, liquidity changes, and pool views |
| `bridge_actions` | Bridge message/action coverage |

Each target interprets input as a sequence of fixed-size 32-byte actions defined in `stellar-lend/fuzz/src/encoding.rs`.

## Strategy

The harnesses map action bytes to protocol calls and assert invariants after state transitions. Lending fuzzing registers a fuzz-only oracle contract so inputs can mutate per-asset prices while the target calls collateral, debt, health-factor, and liquidation paths.

Performance guardrails:

- Inputs are capped to a bounded number of actions.
- Ledger time deltas are capped per step.
- Harnesses use `try_*` calls so expected rejections keep exploration moving.
- `lending_critical` uses positive bounded amounts and over-collateralized borrow attempts to reach debt and liquidation states quickly.

## Corpus Management

Seed corpora are checked into git:

- `stellar-lend/fuzz/corpus/lending_critical/`
- `stellar-lend/fuzz/corpus/lending_actions/`
- `stellar-lend/fuzz/corpus/amm_actions/`
- `stellar-lend/fuzz/corpus/bridge_actions/`

`scripts/fuzz/check_corpus.sh` enforces a minimum of 10 non-empty files per target. Override with `MIN_CORPUS_FILES` when needed.

## Run Locally

Prerequisites:

- Rust nightly: `rustup toolchain install nightly`
- cargo-fuzz: `cargo +nightly install cargo-fuzz --locked`
- LLVM/clang for libFuzzer

Run focused lending fuzzing:

```bash
cd stellar-lend
cargo +nightly fuzz run lending_critical fuzz/corpus/lending_critical -- -max_total_time=1800 -timeout=15
```

Run smoke fuzzing for every target:

```bash
bash scripts/fuzz/run_ci_smoke.sh
```

## Crash Triage

Replay a crash:

```bash
./scripts/fuzz/repro.sh lending_critical stellar-lend/fuzz/artifacts/lending_critical/crash-* -- -runs=1
```

Promote a crash to a regression corpus fixture and write replay notes:

```bash
./scripts/fuzz/triage_crash.sh lending_critical stellar-lend/fuzz/artifacts/lending_critical/crash-*
```

The triage script copies the input into `stellar-lend/fuzz/corpus/<target>/regression_<sha>` and writes a markdown report under `stellar-lend/fuzz/regressions/<target>/`.

## Coverage Reports

Generate coverage logs and a summary table:

```bash
./scripts/fuzz/coverage_report.sh lending_critical lending_actions
```

The report is written to `stellar-lend/fuzz/coverage/` and uploaded by CI with fuzz artifacts.

## CI

The regular CI pipeline keeps a quick smoke fuzz pass in `scripts/fuzz/run_ci_smoke.sh`.

The dedicated long-running workflow is `.github/workflows/contract-fuzz.yml`. It runs each target with `-max_total_time=1800`, uploads crash artifacts, and generates per-target coverage reports.
