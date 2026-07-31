# Differential Test Report

## What This Tests

Differential testing runs the **same inputs against two independent contract implementations** and asserts their outputs are identical. This catches subtle behavioral regressions that unit tests miss — especially after upgrades or refactors.

Two flavors are covered:

1. **Same implementation, two instances** (`differential_test.rs`) — regression guard: two fresh `hello-world` instances must always agree.
2. **Genuinely different implementations** (`hello_world_vs_lending_test.rs`) — compares `hello-world` against the separate `lending` contract crate via a shared `ContractAdapter` trait.

## Files

| File | Purpose |
|---|---|
| `src/tests/diff_harness.rs` | `HwAdapter`, `LendingAdapter`, `ContractAdapter` trait, `PositionSnapshot`, `DivergenceReport` — core harness |
| `src/tests/differential_test.rs` | Same-implementation property comparison tests (deposit, borrow, repay, zero-amount, sequential) |
| `src/tests/hello_world_vs_lending_test.rs` | Cross-implementation comparison: `hello-world` vs `lending` (deposit only — see below) |
| `src/tests/migration_verification_test.rs` | hello-world: storage read-back sanity across a re-created client handle (weak — see "Known Structural Differences") |
| `../lending/src/migration_verification_test.rs` | lending: drives the *real* `UpgradeManager` governance lifecycle (propose → approve → queue timelock → execute/rollback) and confirms it doesn't disturb lending's own application state |

## Running Locally

```bash
cd stellar-lend
cargo test --package hello-world --lib tests::differential_test -- --nocapture
cargo test --package hello-world --lib tests::hello_world_vs_lending_test -- --nocapture
cargo test --package hello-world --lib tests::migration_verification_test -- --nocapture
cargo test --package stellarlend-lending --lib migration_verification_test -- --nocapture
```

## How Divergences Are Reported

If two instances/implementations return different results for the same input, the test panics with:

```
[DIVERGENCE] deposit: v1=Ok(true) v2=Err(())
[DIVERGENCE] get_position: v1=PositionSnapshot { collateral: 1000, debt: 0 } v2=PositionSnapshot { collateral: 999, debt: 0 }
```

## Edge Cases Covered

| Edge Case | How Handled |
|---|---|
| Non-deterministic behavior | Ledger timestamp pinned via `env.ledger().set_timestamp()` before each test |
| State-dependent outputs | Tests run full sequences: deposit → borrow → repay → check position |
| Zero-amount inputs | Explicit tests asserting both same-implementation instances *and* both cross-implementation contracts reject consistently |
| Storage layout across upgrades | `migration_verification_test.rs` (hello-world) reads raw storage keys via `env.as_contract()`; `migration_verification_test.rs` (lending) drives the real `UpgradeManager` state machine including its 48h timelock |
| Multiple users | Multi-user migration test with 5 users and distinct amounts |
| Performance differences | Not covered — no benchmark comparison between implementations yet |

## Known Structural Differences (why some operations aren't cross-implementation-diffed)

`hello-world` and `lending` have genuinely different domain models, discovered while building the cross-implementation harness:

- **Borrow**: `hello-world.borrow_asset` borrows against previously-deposited collateral. `lending.borrow` is atomic — it deposits new collateral *and* borrows in the same call, and rejects `collateral_amount <= 0`. There is no way to call it "borrow only, against existing collateral" the way hello-world does, so a literal same-inputs comparison would require synthetically inventing a matching action for one side, which would test the harness's own workaround rather than the contracts. **Not compared.**
- **Asset model**: `hello-world` takes `asset: Option<Address>` (single/native asset). `lending` requires `asset: Address` on every call (multi-asset). The `ContractAdapter` trait picks one fixed `Address` per adapter instance to keep `deposit` comparable.
- **Position shape**: `hello-world::Position { collateral, debt }` vs `lending::UserPositionSummary { collateral_balance, debt_balance, collateral_value, debt_value, health_factor }`. Only the two directly-equivalent raw balance fields are compared; value/health-factor fields depend on an oracle neither adapter configures.

If `lending`'s API changes to make borrow/repay/withdraw genuinely comparable (e.g. a non-atomic borrow-against-existing-collateral entry point is added), extend `ContractAdapter` and `hello_world_vs_lending_test.rs` accordingly.

## Known Limitation: No Real WASM-Swap Migration Test

Neither `hello-world` nor `lending` currently exposes a real "upgrade this contract's code" entry point — `UpgradeManager` (`common/src/upgrade.rs`, used by `lending`/`amm`/`bridge`) only tracks an *approved* WASM hash + version in storage; it never calls Soroban's `env.deployer().update_current_contract_wasm(..)`. Genuinely testing storage-layout survival across a real code upgrade would require compiling and checking in a separate `.wasm` artifact for an "old" version and loading it via `env.register_contract_wasm(..)` — a build-pipeline addition, not a test-code change, and out of scope here. `lending/src/migration_verification_test.rs` instead verifies the real, available claim: driving the actual governance lifecycle to completion (and to rollback) does not disturb a live position. hello-world's own `migration_verification_test.rs` is weaker (it only re-creates a client handle to an untouched contract) because hello-world doesn't integrate `UpgradeManager` at all.

Separately, `scripts/migration-simulator` and `environments/migration-sandbox` already provide real migration dry-run tooling against a forked network — that's operational tooling, not part of this Rust unit-test suite.

## CI Integration

Differential and migration-verification tests (both crates) run as a dedicated CI step in `.github/workflows/ci-cd.yml` and upload `differential-test-report.txt` as an artifact on every push/PR. A failure here means a behavioral regression was introduced.

## Known Acceptable Divergences

None currently. If an intentional behavioral change is made, document it here with the PR number and rationale before merging.
