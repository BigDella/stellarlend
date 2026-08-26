# Formal Verification Specification

This document describes the formal verification specification suite
introduced for **Issue #224** (DevTooling: formal verification of critical
functions) and extended for **Issue #384** (protocol-level lending
invariants).

## Overview

Critical lending protocol functions now have machine-checkable **formal
specifications** written as Rust property tests.  Each specification:

* Identifies the **mathematical model** the function implements
* States **named lemmas** that must hold for all valid inputs
* Provides **pure reference implementations** for cross-checking
* Ships optional **Kani bounded model-checking harnesses** for exhaustive
  bounded verification

All spec modules are compiled only under `#[cfg(any(test, feature = "spec"))]`
and are completely absent from the production WASM binary.

---

## Critical Functions Covered

| Module | Function | Lemmas |
|---|---|---|
| `spec::interest` | `calculate_interest` | I-01 … I-08 |
| `spec::collateral` | `validate_collateral_ratio` | C-01 … C-06 |
| `spec::accounting` | `borrow`, `repay` (accounting invariants) | A-01 … A-08 |
| `spec::health_factor` | `compute_health_factor` | H-01 … H-07 |
| `spec::liquidation` | `get_max_liquidatable_amount` | L-01 … L-06 |
| `spec::rewards` | `update_global_index`, `update_user`, `claim` | R-01 … R-07 |
| `spec::deposit_spec` | `deposit` | D-01 … D-07 |
| `spec::withdraw_spec` | `withdraw` | W-01 … W-07 |
| `spec::protocol_invariants` | protocol accounting, health factor, interest index | P-01 ... P-05, H-01 ... H-05, I-01 ... I-03 |

**Total: 69 lemmas across 9 critical function groups**

---

## Running the Specs

### Fast property tests (CI / local)

```bash
# From stellar-lend/ workspace root:
cargo check -p stellarlend-lending --features spec
cargo test -p stellarlend-lending --features spec --test protocol_formal_specs

# Single spec module:
cargo test -p stellarlend-lending --features spec --test protocol_formal_specs \
  lemma_p01_accounting_identity_holds_after_deposits

# With backtrace on failure:
RUST_BACKTRACE=1 cargo test -p stellarlend-lending --features spec \
  --test protocol_formal_specs
```

Expected output:

```
test result: ok. 13 passed; 0 failed; 0 ignored
```

The `protocol_formal_specs` integration target is the CI gate for Issue #384.
It compiles the lending crate with the `spec` feature but avoids unrelated
legacy unit-test modules, so failures map directly to the formal invariant
suite.

### Kani bounded model-checking (exhaustive, slow)

Install Kani:

```bash
cargo install --locked kani-verifier
cargo kani setup
```

Run all harnesses:

```bash
cargo kani \
  --package stellarlend-lending \
  --features spec \
  --harness spec::protocol_invariants::kani_protocol_accounting_preserves_identity \
  --harness spec::protocol_invariants::kani_health_factor_consistency \
  --harness spec::protocol_invariants::kani_interest_index_monotonic \
  --harness spec::interest::kani_interest_properties \
  --harness spec::collateral::kani_collateral_ratio_properties \
  --harness spec::accounting::kani_accounting_invariants \
  --harness spec::health_factor::kani_health_factor_properties \
  --unwind 8
```

Or trigger via GitHub Actions manual dispatch:

> Actions → "Formal Verification Specs" → Run workflow → `run_kani: true`

---

## Spec Architecture

```
contracts/lending/src/
└── spec/
    ├── mod.rs           — Root; sub-module declarations
    ├── interest.rs      — I-01…I-08: calculate_interest lemmas
    ├── collateral.rs    — C-01…C-06: validate_collateral_ratio lemmas
    ├── accounting.rs    — A-01…A-08: borrow/repay accounting invariants
    ├── health_factor.rs — H-01…H-07: compute_health_factor lemmas
    ├── protocol_invariants.rs — P/H/I protocol-level conservation lemmas
    ├── liquidation.rs   — L-01…L-06: get_max_liquidatable_amount lemmas
    ├── rewards.rs       — R-01…R-07: rewards distribution lemmas
    ├── deposit_spec.rs  — D-01…D-07: deposit lemmas
    └── withdraw_spec.rs — W-01…W-07: withdraw lemmas
```

Each spec file follows a consistent layout:

1. **Mathematical model** — LaTeX-style formula in a doc comment
2. **Property table** — ID, informal statement
3. **Pure reference implementation** — stateless, no Soroban `Env`, uses
   checked i128 arithmetic (where possible) so the spec can run in any Rust
   test harness
4. **Lemma functions** (`#[test]` fns, named `lemma_<ID>_<description>`)
5. **Kani harness** (`#[cfg(kani)] #[kani::proof]`)

### Note on i128 vs I256

The production contract uses Soroban's `I256` type for all intermediate
arithmetic, which means it can never overflow for any `i128` input.  The
reference implementations in the spec files use native i128 (no `soroban-sdk`
dependency) to keep the specs runnable without the Soroban test harness.  Where
the i128 reference hits its own safe domain boundary (e.g., `P * rate_bps`),
the safe domain is explicitly documented in the lemma comment.

---

## Assumptions (Global)

* All `i128` fields are non-negative in valid protocol state (amounts, rates,
  timestamps).
* Ledger timestamps are monotonically non-decreasing within a ledger.
* Oracle prices are non-negative.
* `overflow-checks = true` is set in `Cargo.toml` release profile, so
  unchecked arithmetic panics in debug and checked arithmetic returns `Err`
  in production paths.

---

## Key Properties Proved

### Absence of Overflow/Underflow

Every arithmetic lemma (I-06, C-05, A-01…A-08, H-06, L-06, R-07, D-07) tests
that the reference implementation does not return `None` (overflow sentinel)
for any input in the valid domain.

### Correctness of Accounting

Invariants INV-1 through INV-6 (from `spec::accounting`) prove that:
- Total debt equals the sum of all user principals (INV-1)
- Total debt is always non-negative (INV-2)
- Total debt is bounded by the ceiling (INV-6)
- Borrow and repay are inverse operations (A-05)

The protocol-level model in `spec::protocol_invariants` additionally proves:
- `total_deposits = total_debt + reserves` after deposits, borrows, and repays
- over-borrow and over-repay paths reject without mutating state
- health factor calculations track the liquidation threshold before and after
  collateral/debt changes
- interest index accrual never decreases when timestamps move forward

### Mathematical Correctness

The interest formula (I-07), collateral boundary (C-06), health factor boundary
(H-07), and close-factor formula (L-05) are proved to match the mathematical
specification with at most ±1 rounding tolerance.

### Monotonicity

- Interest is non-decreasing in both principal and elapsed time (I-04, I-05)
- Health factor is non-decreasing in collateral value (H-04)
- Health factor is non-increasing in debt value (H-05)
- Reward global index is non-decreasing over time (R-01)

### Edge Cases

- Zero inputs yield zero/sentinel outputs (I-01, I-02, H-01, L-02, R-02, R-03)
- Over-repay is rejected (A-07)
- Ceiling breach is rejected (A-04, A-08, D-03)
- Oracle absence returns safe defaults (H-02, L-03)
- Same-timestamp reward update is idempotent (R-05)
- Claim zeroes accrued rewards (R-06)

---

## Maintaining Specs

When modifying a function covered by a spec:

1. **Run the spec tests first** to confirm the current baseline passes.
2. **Update the mathematical model** in the spec doc comment.
3. **Update the reference implementation** to match the new logic.
4. **Update or add lemmas** that cover the new behaviour.
5. **Re-run** `cargo check -p stellarlend-lending --features spec` and
   `cargo test -p stellarlend-lending --features spec --test protocol_formal_specs`
   and confirm green.

The GitHub Actions workflow enforces this gate on every PR touching the
affected files.

---

## CI Integration

The workflow at `.github/workflows/formal-verification.yml` runs:

| Job | Trigger | Purpose |
|---|---|---|
| `spec-lemmas` | Every push/PR | Run protocol formal spec lemmas (ubuntu + macos) |
| `spec-isolation` | Every push/PR | Assert no spec symbols in production WASM |
| `spec-lint` | Every push/PR | Clippy on spec modules |
| `kani-harnesses` | Manual dispatch | Bounded model-checking (slow) |

Kani is used as the Rust/Soroban model checker for this repository. It fills
the same role that Certora Prover fills for Solidity/EVM projects while keeping
the verification target native to the contract language used here.
