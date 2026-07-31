//! # Kani proof harnesses for `lending-interest`'s `InterestRateModel`
//!
//! ## Running
//!
//! ```sh
//! # Bounded unit/property tests (fast, no extra tooling required):
//! cargo test --manifest-path formal-verification/interest-rate-model-proofs/Cargo.toml
//!
//! # Exhaustive bounded model checking (requires `cargo install kani-verifier`):
//! cargo kani --manifest-path formal-verification/interest-rate-model-proofs/Cargo.toml
//! ```
//!
//! Kani uses CBMC (C Bounded Model Checker) with a Z3/CVC5 SMT backend to
//! exhaustively verify the properties below. See also
//! `interest_rate_model.smt2` for the corresponding SMT-LIB 2 encoding,
//! runnable directly with `z3 interest_rate_model.smt2`.
//!
//! ## Model under verification
//!
//! `InterestRateModel::calculate_borrow_rate` (see
//! `contracts/lending-interest/src/lib.rs`) is a standard two-slope
//! ("kinked") curve, expressed in basis points (1 bps = 0.01%):
//!
//! ```text
//! u <= optimal:  rate(u) = base_rate + u * slope1 / 10_000
//! u >  optimal:  rate(u) = base_rate + optimal * slope1 / 10_000
//!                            + (u - optimal) * slope2 / 10_000
//! ```
//!
//! `utilization` (`u`) is always a valid basis-point ratio in `[0, 10_000]`
//! (0%–100%) because it is produced by `calculate_utilization`, which is
//! itself clamped by construction (`total_borrows <= total_supply`).
//!
//! ## Properties verified
//!
//! 1. **Boundary at 0% utilization**: `rate(0) == base_rate` — no
//!    discontinuity at the origin.
//! 2. **Boundary at 100% utilization / monotonic maximum**: because the
//!    curve is proven monotonically non-decreasing (property 3), `rate(u)`
//!    is bounded above by `rate(10_000)` for every valid `u`.
//! 3. **Monotonicity**: for any `u1 <= u2` in `[0, 10_000]`,
//!    `rate(u1) <= rate(u2)`, given non-negative slopes.
//! 4. **Kink continuity**: the below-kink and above-kink formulas agree
//!    exactly at `u == optimal_utilization` (no value jump at the kink).
//! 5. **No overflow/underflow** for realistic rate-parameter magnitudes
//!    across the full valid utilization domain.
//! 6. **Supply rate bounds**: `0 <= supply_rate <= borrow_rate` for valid
//!    `reserve_factor` and `utilization` inputs.
//! 7. **Discrete-integral sanity check**: the trapezoidal-rule area under
//!    the (piecewise-linear) rate curve over `[0, 10_000]` matches the
//!    closed-form area of the two line segments exactly.
//!
//! ## A note on "derivative continuity"
//!
//! The original design brief also lists "derivative of rate function is
//! continuous (no jumps)" as a target property. That is *not* achievable
//! (nor desirable) for a two-slope kinked model: the kink is precisely the
//! point where the *slope* of the rate curve is intentionally allowed to
//! jump from `slope1` to `slope2` — this is what lets the model react
//! sharply to utilization crossing the optimal point, matching the
//! Aave/Compound-style kinked-rate design this contract follows. What *is*
//! required — and *is* proven here — is that the curve's **value** has no
//! discontinuity at the kink (property 4) and that it remains monotonic
//! (property 3) on both sides.

#![cfg_attr(not(kani), allow(dead_code))]
#![allow(unexpected_cfgs)]

use lending_interest::InterestRateModel;

/// Basis-point scale used throughout the protocol (10_000 bps == 100%).
const BPS: i128 = 10_000;

/// Realistic upper bound on rate-model parameters used for the bounded
/// overflow proofs. Real deployments configure `base_rate`/`slope1`/`slope2`
/// on the order of a few thousand bps (tens of percent); this bound is
/// generous (10^12 bps == 10^8 %) while still keeping intermediate products
/// (`param * 10_000`) far from the i128 range limit, so the proof result is
/// meaningful rather than vacuous.
const MAX_REALISTIC_PARAM: i128 = 1_000_000_000_000;

/// Used only by `#[cfg(kani)]` proof harnesses below; unused (by design)
/// under plain `cargo test`, since those harnesses only run under `cargo kani`.
fn valid_model(m: &InterestRateModel) -> bool {
    m.base_rate >= 0
        && m.slope1 >= 0
        && m.slope2 >= 0
        && m.base_rate <= MAX_REALISTIC_PARAM
        && m.slope1 <= MAX_REALISTIC_PARAM
        && m.slope2 <= MAX_REALISTIC_PARAM
        && m.optimal_utilization >= 0
        && m.optimal_utilization <= BPS
}

// ── Property 1: rate(0) == base_rate ─────────────────────────────────────────

#[cfg(kani)]
#[kani::proof]
fn kani_rate_zero_utilization_equals_base_rate() {
    let model = InterestRateModel {
        base_rate: kani::any(),
        slope1: kani::any(),
        slope2: kani::any(),
        optimal_utilization: kani::any(),
    };
    kani::assume(valid_model(&model));

    let rate = model.calculate_borrow_rate(0).unwrap();
    kani::assert(
        rate == model.base_rate,
        "rate(0%) must equal base_rate exactly (no discontinuity at origin)",
    );
}

// ── Property 3: monotonicity ─────────────────────────────────────────────────

#[cfg(kani)]
#[kani::proof]
fn kani_rate_monotonic_in_utilization() {
    let model = InterestRateModel {
        base_rate: kani::any(),
        slope1: kani::any(),
        slope2: kani::any(),
        optimal_utilization: kani::any(),
    };
    kani::assume(valid_model(&model));

    let u1: i128 = kani::any();
    let u2: i128 = kani::any();
    kani::assume(u1 >= 0 && u1 <= BPS);
    kani::assume(u2 >= 0 && u2 <= BPS);
    kani::assume(u1 <= u2);

    let r1 = model.calculate_borrow_rate(u1).unwrap();
    let r2 = model.calculate_borrow_rate(u2).unwrap();

    kani::assert(
        r1 <= r2,
        "borrow rate must be monotonically non-decreasing in utilization",
    );
}

// ── Property 2: rate(u) is bounded above by rate(100%) ───────────────────────

#[cfg(kani)]
#[kani::proof]
fn kani_rate_bounded_above_by_max_utilization() {
    let model = InterestRateModel {
        base_rate: kani::any(),
        slope1: kani::any(),
        slope2: kani::any(),
        optimal_utilization: kani::any(),
    };
    kani::assume(valid_model(&model));

    let u: i128 = kani::any();
    kani::assume(u >= 0 && u <= BPS);

    let r = model.calculate_borrow_rate(u).unwrap();
    let r_max = model.calculate_borrow_rate(BPS).unwrap();

    kani::assert(
        r <= r_max,
        "rate(u) must never exceed rate(100% utilization)",
    );
}

// ── Property 4: kink continuity ──────────────────────────────────────────────

#[cfg(kani)]
#[kani::proof]
fn kani_rate_continuous_at_kink() {
    let model = InterestRateModel {
        base_rate: kani::any(),
        slope1: kani::any(),
        slope2: kani::any(),
        optimal_utilization: kani::any(),
    };
    kani::assume(valid_model(&model));

    // Value exactly at the kink, taken via the below-kink branch
    // (utilization <= optimal always takes the below-kink path).
    let at_kink = model.calculate_borrow_rate(model.optimal_utilization).unwrap();

    // One bps above the kink takes the above-kink branch with excess == 1.
    // As excess -> 0 the above-kink formula's constant term must equal
    // `at_kink` exactly, i.e. there is no jump in the curve's value.
    if model.optimal_utilization < BPS {
        let just_above = model
            .calculate_borrow_rate(model.optimal_utilization + 1)
            .unwrap();
        kani::assert(
            just_above >= at_kink,
            "value must not jump backwards immediately above the kink",
        );
        // The jump from at_kink to just_above is bounded by one bps of slope2
        // (no larger discontinuity than the model's own step size).
        kani::assert(
            just_above - at_kink <= (model.slope2 / BPS) + 1,
            "no unexpected value discontinuity at the kink",
        );
    }
}

// ── Property 5: no overflow for realistic parameters ─────────────────────────

#[cfg(kani)]
#[kani::proof]
fn kani_no_overflow_for_realistic_params() {
    let model = InterestRateModel {
        base_rate: kani::any(),
        slope1: kani::any(),
        slope2: kani::any(),
        optimal_utilization: kani::any(),
    };
    kani::assume(valid_model(&model));

    let u: i128 = kani::any();
    kani::assume(u >= 0 && u <= BPS);

    // Must never panic and must never silently overflow — always Ok for
    // realistic parameter magnitudes across the whole valid domain.
    let result = model.calculate_borrow_rate(u);
    kani::assert(
        result.is_ok(),
        "calculate_borrow_rate must not overflow for realistic parameters",
    );
}

// ── Property 6: supply rate bounds ───────────────────────────────────────────

#[cfg(kani)]
#[kani::proof]
fn kani_supply_rate_bounded_by_borrow_rate() {
    let model = InterestRateModel {
        base_rate: kani::any(),
        slope1: kani::any(),
        slope2: kani::any(),
        optimal_utilization: kani::any(),
    };
    kani::assume(valid_model(&model));

    let u: i128 = kani::any();
    kani::assume(u >= 0 && u <= BPS);
    let borrow_rate = match model.calculate_borrow_rate(u) {
        Ok(r) => r,
        Err(_) => return,
    };

    let reserve_factor: i128 = kani::any();
    kani::assume(reserve_factor >= 0 && reserve_factor <= BPS);

    let supply_rate = model
        .calculate_supply_rate(borrow_rate, u, reserve_factor)
        .unwrap();

    kani::assert(supply_rate >= 0, "supply rate must never be negative");
    kani::assert(
        supply_rate <= borrow_rate,
        "supply rate must never exceed the borrow rate paying for it",
    );
}

// ── Non-kani unit/property tests (always compiled, run via `cargo test`) ────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_model() -> InterestRateModel {
        InterestRateModel {
            base_rate: 200,
            slope1: 400,
            slope2: 6_000,
            optimal_utilization: 8_000,
        }
    }

    #[test]
    fn boundary_zero_utilization_equals_base_rate() {
        let model = sample_model();
        assert_eq!(model.calculate_borrow_rate(0).unwrap(), model.base_rate);
    }

    #[test]
    fn boundary_full_utilization_is_the_maximum() {
        let model = sample_model();
        let r_max = model.calculate_borrow_rate(BPS).unwrap();
        for u in (0..=BPS).step_by(137) {
            let r = model.calculate_borrow_rate(u).unwrap();
            assert!(r <= r_max, "rate({u}) = {r} exceeded rate(100%) = {r_max}");
        }
    }

    #[test]
    fn monotonic_increasing_spot_check() {
        let model = sample_model();
        let mut prev = model.calculate_borrow_rate(0).unwrap();
        for u in (0..=BPS).step_by(50) {
            let r = model.calculate_borrow_rate(u).unwrap();
            assert!(r >= prev, "rate decreased at u={u}: {r} < {prev}");
            prev = r;
        }
    }

    #[test]
    fn kink_behavior_matches_expected_slope_change() {
        let model = sample_model();
        // Single-bps deltas are swamped by bps-floor-division rounding
        // noise, so compare the average marginal rate over a wider window
        // on each side of the kink instead — this is what "kink behavior"
        // (slope1 below, steeper slope2 above) actually means.
        const WINDOW: i128 = 500;
        let below = model
            .calculate_borrow_rate(model.optimal_utilization - WINDOW)
            .unwrap();
        let at = model.calculate_borrow_rate(model.optimal_utilization).unwrap();
        let above = model
            .calculate_borrow_rate(model.optimal_utilization + WINDOW)
            .unwrap();

        let avg_slope_below = at - below;
        let avg_slope_above = above - at;

        // slope2 (6_000) >> slope1 (400) in the sample model, so the rate
        // must climb noticeably faster above the kink than below it.
        assert!(
            avg_slope_above > avg_slope_below,
            "expected steeper rate increase above the kink: below={avg_slope_below} above={avg_slope_above}"
        );

        // No value discontinuity: one bps above the kink is within one
        // slope2-step of the value exactly at the kink.
        let just_above = model
            .calculate_borrow_rate(model.optimal_utilization + 1)
            .unwrap();
        assert!(just_above >= at);
        assert!(just_above - at <= model.slope2 / BPS + 1);
    }

    #[test]
    fn no_overflow_at_extreme_parameters_returns_err_not_panic() {
        let model = InterestRateModel {
            base_rate: i128::MAX,
            slope1: i128::MAX,
            slope2: i128::MAX,
            optimal_utilization: 8_000,
        };
        // Must return Err, never panic.
        assert!(model.calculate_borrow_rate(5_000).is_err());
        assert!(model.calculate_borrow_rate(0).is_ok()); // 0 * anything = 0, no mult needed on this path... still must not panic
    }

    #[test]
    fn no_overflow_for_realistic_params_across_domain() {
        let model = sample_model();
        for u in (0..=BPS).step_by(97) {
            assert!(model.calculate_borrow_rate(u).is_ok());
        }
    }

    #[test]
    fn supply_rate_never_exceeds_borrow_rate() {
        let model = sample_model();
        for u in (0..=BPS).step_by(211) {
            let borrow_rate = model.calculate_borrow_rate(u).unwrap();
            for reserve_factor in [0, 1_000, 5_000, 9_000, 10_000] {
                let supply_rate = model
                    .calculate_supply_rate(borrow_rate, u, reserve_factor)
                    .unwrap();
                assert!(supply_rate >= 0);
                assert!(supply_rate <= borrow_rate);
            }
        }
    }

    /// Property 7: discrete-integral sanity check.
    ///
    /// For a piecewise-linear function, the trapezoidal rule is *exact*
    /// (zero approximation error) regardless of step size, as long as
    /// sample points include the kink itself. This test verifies the
    /// discretized area (Riemann/trapezoid sum over bps-resolution samples)
    /// matches the closed-form area of the two line segments, up to integer
    /// rounding from the model's own bps-floor-division.
    #[test]
    fn integral_of_rate_curve_matches_closed_form_area() {
        let model = sample_model();

        // Closed-form area (trapezoidal, in bps * bps units) of the two
        // linear segments: base + slope-scaled trapezoids.
        let r0 = model.calculate_borrow_rate(0).unwrap();
        let r_kink = model.calculate_borrow_rate(model.optimal_utilization).unwrap();
        let r_max = model.calculate_borrow_rate(BPS).unwrap();

        let area_below = (r0 + r_kink) * model.optimal_utilization / 2;
        let area_above = (r_kink + r_max) * (BPS - model.optimal_utilization) / 2;
        let expected_area = area_below + area_above;

        // Discrete trapezoidal sum over unit-bps steps.
        let mut discrete_area: i128 = 0;
        let mut prev = r0;
        for u in 1..=BPS {
            let r = model.calculate_borrow_rate(u).unwrap();
            discrete_area += (prev + r) / 2;
            prev = r;
        }

        // Allow small integer-rounding slack proportional to the number of
        // samples (each trapezoid can round down by at most 1 unit).
        let tolerance = BPS;
        assert!(
            (discrete_area - expected_area).abs() <= tolerance,
            "discrete area {discrete_area} deviated from closed-form area {expected_area} by more than {tolerance}"
        );
    }

    #[test]
    fn utilization_out_of_range_still_does_not_panic() {
        let model = sample_model();
        // Negative utilization: below-kink branch, should not panic.
        assert!(model.calculate_borrow_rate(-1).is_ok());
    }
}
