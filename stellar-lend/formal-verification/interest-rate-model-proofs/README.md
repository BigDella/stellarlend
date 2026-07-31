# interest-rate-model-proofs

Formal verification of `InterestRateModel::calculate_borrow_rate` /
`calculate_supply_rate` (`contracts/lending-interest`) at boundary
conditions: 0% utilization, 100% utilization, and the kink point.

This crate is **not** part of the main Cargo workspace (see the root
`Cargo.toml` `exclude` list), matching the existing
`formal-verification/safe-math-proofs` pattern, so it doesn't affect normal
`cargo build`/`cargo test` runs of the contract workspace.

## Running

Bounded property tests (fast, run in normal CI, no extra tooling):

```sh
cargo test --manifest-path formal-verification/interest-rate-model-proofs/Cargo.toml
```

Exhaustive bounded model checking with [Kani](https://model-checking.github.io/kani/):

```sh
cargo install --locked kani-verifier
cargo kani setup
cargo kani --manifest-path formal-verification/interest-rate-model-proofs/Cargo.toml
```

SMT-LIB spec directly with Z3:

```sh
z3 formal-verification/interest-rate-model-proofs/interest_rate_model.smt2
```

## Properties verified

1. `rate(0%) == base_rate` — no discontinuity at the origin.
2. `rate(u)` is bounded above by `rate(100%)` for all valid `u`.
3. `rate(u)` is monotonically non-decreasing in `u`.
4. The below-kink and above-kink formulas agree exactly at
   `u == optimal_utilization` (no value discontinuity at the kink).
5. No overflow/underflow for realistic rate-parameter magnitudes across the
   full valid utilization domain `[0, 10_000]` bps.
6. `0 <= supply_rate <= borrow_rate` for all valid `reserve_factor` inputs.
7. The discretized (trapezoidal) area under the rate curve over
   `[0, 10_000]` matches the closed-form area of the two line segments.

See `src/lib.rs` for a note on why literal "derivative continuity" at the
kink is not a target property — the kink's entire purpose is an intentional
slope change from `slope1` to `slope2`.
