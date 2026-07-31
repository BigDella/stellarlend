//! # Kani proof harnesses for `lending-risk`'s liquidation math
//!
//! ## Running
//!
//! ```sh
//! # Bounded unit/property tests (fast, no extra tooling required):
//! cargo test --manifest-path formal-verification/liquidation-math-proofs/Cargo.toml
//!
//! # Exhaustive bounded model checking (requires `cargo install kani-verifier`):
//! cargo kani --manifest-path formal-verification/liquidation-math-proofs/Cargo.toml
//! ```
//!
//! See also `liquidation_math.smt2` for the corresponding SMT-LIB 2 encoding.
//!
//! ## Model under verification
//!
//! `RiskManager::apply_liquidation` (`contracts/lending-risk/src/lib.rs`)
//! simulates a partial liquidation using the standard Aave/Compound-style
//! design: a liquidator repays `repay_amount` of a position's debt and
//! seizes `seized = repay_amount + bonus` of collateral, where
//! `bonus = repay_amount * bonus_bps / 10_000`.
//!
//! ## Properties verified
//!
//! 1. **Liquidation discount bounds**: `0 <= bonus <= repay_amount` for any
//!    `bonus_bps` in `[0, 10_000]` (a liquidation bonus can never exceed
//!    100% of the repaid amount).
//! 2. **Conservation / no value leak**: `collateral_value ==
//!    new_collateral_value + seized_collateral` and `debt_value ==
//!    new_debt_value + repay_amount` exactly — nothing is created or
//!    destroyed by the liquidation math itself.
//! 3. **Liquidator payout is exact**: `liquidator_profit == bonus` as
//!    computed by `calculate_liquidation_bonus` — the liquidator receives
//!    precisely the configured discount, no more, no less.
//! 4. **Conditional health-factor monotonicity** (see note below).
//! 5. **No silent overflow/underflow** — `apply_liquidation` either returns
//!    `Ok` with the exact conserved values above, or `Err`, never a wrapped/
//!    truncated result.
//!
//! ## An important, non-obvious finding: liquidation is not *always*
//! ## health-improving
//!
//! The design brief for this verification effort assumed an unconditional
//! invariant: "post-liquidation health factor >= pre-liquidation health
//! factor" for *any* valid liquidation call. **That unconditional claim is
//! false**, and Kani finds the counterexample immediately if you try to
//! prove it as stated.
//!
//! Concretely: with `collateral_value = 50_000`, `debt_value = 90_000`,
//! `repay_amount = 40_000`, `liquidation_bonus_bps = 500` — a deeply
//! under-collateralized ("bad debt") position — the health factor
//! (`collateral * threshold / debt`) *drops* from `4444` to `1280` after
//! liquidation, because the liquidator's bonus is carved out of collateral
//! that is already scarcer than the debt it backs.
//!
//! Working the algebra (see `kani_health_factor_ratio_condition` below and
//! `liquidation_math.smt2`) shows the precise, *provable* condition, stated
//! as two one-directional implications (integer floor-division means the
//! two directions meet at equality, not a clean strict biconditional):
//!
//! ```text
//! collateral_value * repay_amount >= debt_value * seized_collateral
//!     =>  health_factor_after >= health_factor_before
//!
//! collateral_value * repay_amount <  debt_value * seized_collateral
//!     =>  health_factor_after <= health_factor_before
//! ```
//!
//! Equivalently, in real-number terms, the first premise is
//! `collateral_value / debt_value >= 1 + liquidation_bonus_bps / 10_000`.
//! i.e. liquidation only improves health when the position's raw
//! collateral/debt ratio still exceeds the liquidation *seizure* ratio.
//! Below that point, every liquidation call — even though individually
//! "correct" per the discount formula — mechanically accelerates the
//! position toward insolvency. This is a real design implication: a
//! liquidation bot / keeper incentive layer built on top of this math
//! should gate on this ratio (or fall back to a bonus-free/bad-debt
//! socialization path) rather than assuming liquidation is always
//! protective. That gating logic is out of scope for this crate (it lives
//! above `lending-risk`, e.g. in a keeper or a future bad-debt handler) —
//! this crate's job is to make the exact boundary precise and proven rather
//! than asserted.

#![cfg_attr(not(kani), allow(dead_code))]
#![allow(unexpected_cfgs)]
// Both imports are used by `#[cfg(kani)]` proof harnesses (only compiled
// under `cargo kani`) and by the `#[cfg(test)]` unit tests via `super::*`;
// under a plain `cargo test` build that combination still trips the lint.
#![allow(unused_imports)]

use lending_risk::RiskManager;
use lending_types::calculate_health_factor;

const BPS: i128 = 10_000;
/// Same rationale as `interest-rate-model-proofs`: realistic magnitude bound
/// for value/rate parameters so overflow proofs are meaningful, not vacuous.
const MAX_REALISTIC_VALUE: i128 = 1_000_000_000_000;

fn valid_position(collateral_value: i128, debt_value: i128, threshold_bps: i128) -> bool {
    collateral_value > 0
        && collateral_value <= MAX_REALISTIC_VALUE
        && debt_value > 0
        && debt_value <= MAX_REALISTIC_VALUE
        && threshold_bps > 0
        && threshold_bps <= BPS
}

// ── Property 1: liquidation discount bounds ──────────────────────────────────

#[cfg(kani)]
#[kani::proof]
fn kani_liquidation_bonus_bounded() {
    let repay_amount: i128 = kani::any();
    let bonus_bps: i128 = kani::any();
    kani::assume(repay_amount >= 0 && repay_amount <= MAX_REALISTIC_VALUE);
    kani::assume(bonus_bps >= 0 && bonus_bps <= BPS);

    let bonus = RiskManager::calculate_liquidation_bonus(repay_amount, bonus_bps).unwrap();

    kani::assert(bonus >= 0, "liquidation bonus must never be negative");
    kani::assert(
        bonus <= repay_amount,
        "liquidation bonus must never exceed 100% of the repaid amount",
    );
}

// ── Properties 2, 3, 5: conservation, exact payout, no silent overflow ──────

#[cfg(kani)]
#[kani::proof]
fn kani_apply_liquidation_conserves_value() {
    let collateral_value: i128 = kani::any();
    let debt_value: i128 = kani::any();
    let repay_amount: i128 = kani::any();
    let bonus_bps: i128 = kani::any();

    kani::assume(valid_position(collateral_value, debt_value, BPS / 2));
    kani::assume(repay_amount >= 0 && repay_amount <= debt_value);
    kani::assume(bonus_bps >= 0 && bonus_bps <= BPS);

    if let Ok(outcome) =
        RiskManager::apply_liquidation(collateral_value, debt_value, repay_amount, bonus_bps)
    {
        // No value leak: what's removed from the position is exactly what
        // the liquidator receives / the debt that's cleared.
        kani::assert(
            outcome.new_collateral_value + outcome.seized_collateral == collateral_value,
            "collateral conservation: new_collateral + seized == original collateral",
        );
        kani::assert(
            outcome.new_debt_value + repay_amount == debt_value,
            "debt conservation: new_debt + repaid == original debt",
        );

        // Exact payout: liquidator profit is precisely the configured bonus.
        let expected_bonus =
            RiskManager::calculate_liquidation_bonus(repay_amount, bonus_bps).unwrap();
        kani::assert(
            outcome.liquidator_profit == expected_bonus,
            "liquidator profit must exactly equal the configured discount",
        );
        kani::assert(
            outcome.seized_collateral == repay_amount + outcome.liquidator_profit,
            "seized collateral must equal repaid debt plus liquidator profit exactly",
        );

        // Resulting state must remain within valid (non-negative) bounds.
        kani::assert(outcome.new_collateral_value >= 0, "new collateral must be non-negative");
        kani::assert(outcome.new_debt_value >= 0, "new debt must be non-negative");
    }
}

// ── Property 4: the *precise*, conditional health-factor invariant ─────────

#[cfg(kani)]
#[kani::proof]
fn kani_health_factor_ratio_condition() {
    let collateral_value: i128 = kani::any();
    let debt_value: i128 = kani::any();
    let repay_amount: i128 = kani::any();
    let bonus_bps: i128 = kani::any();
    let threshold_bps: i128 = kani::any();

    kani::assume(valid_position(collateral_value, debt_value, threshold_bps));
    // Strict inequality: a full repayment triggers `calculate_health_factor`'s
    // `i128::MAX` short-circuit, which is trivially >= anything and verified
    // separately in the unit tests below.
    kani::assume(repay_amount > 0 && repay_amount < debt_value);
    kani::assume(bonus_bps >= 0 && bonus_bps <= BPS);

    let outcome =
        match RiskManager::apply_liquidation(collateral_value, debt_value, repay_amount, bonus_bps) {
            Ok(o) => o,
            Err(_) => return,
        };

    let hf_before = calculate_health_factor(collateral_value, debt_value, threshold_bps);
    let hf_after = calculate_health_factor(
        outcome.new_collateral_value,
        outcome.new_debt_value,
        threshold_bps,
    );

    // The exact algebraic condition (see module docs for the derivation):
    // collateral * repay >= debt * seized  <=>  hf_after >= hf_before.
    let ratio_holds = match (
        collateral_value.checked_mul(repay_amount),
        debt_value.checked_mul(outcome.seized_collateral),
    ) {
        (Some(lhs), Some(rhs)) => lhs >= rhs,
        _ => return, // proof restricted to the non-overflowing regime
    };

    if ratio_holds {
        kani::assert(
            hf_after >= hf_before,
            "when collateral*repay >= debt*seized, liquidation must not worsen health factor",
        );
    } else {
        // Non-strict: bps-floor-division can make both sides land on the
        // same integer health factor right at the boundary, so this is
        // "does not improve" rather than "strictly worsens" in general.
        // The strict-worsening case is real and regression-pinned with a
        // concrete example in the unit tests below.
        kani::assert(
            hf_after <= hf_before,
            "when collateral*repay < debt*seized, liquidation cannot improve health factor",
        );
    }
}

// ── Non-kani unit/property tests (always compiled, run via `cargo test`) ────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liquidation_bonus_never_exceeds_repay_amount() {
        for repay in [0, 1, 1_000, 100_000, 1_000_000] {
            for bps in [0, 500, 1_000, 5_000, 10_000] {
                let bonus = RiskManager::calculate_liquidation_bonus(repay, bps).unwrap();
                assert!(bonus >= 0);
                assert!(bonus <= repay);
            }
        }
    }

    #[test]
    fn apply_liquidation_conserves_collateral_and_debt() {
        let outcome = RiskManager::apply_liquidation(100_000, 90_000, 40_000, 500).unwrap();
        assert_eq!(outcome.new_collateral_value + outcome.seized_collateral, 100_000);
        assert_eq!(outcome.new_debt_value + 40_000, 90_000);
        assert_eq!(
            outcome.seized_collateral,
            40_000 + outcome.liquidator_profit
        );
    }

    /// The finding documented at the top of this file: liquidation with a
    /// bonus can *worsen* health factor for deeply under-collateralized
    /// ("bad debt") positions. This regression-pins that exact scenario so
    /// it can't silently regress into a false "always improves" assumption.
    #[test]
    fn liquidation_can_worsen_health_factor_for_bad_debt_positions() {
        let threshold_bps = 8_000;
        let (collateral, debt, repay, bonus_bps) = (50_000, 90_000, 40_000, 500);

        let before = calculate_health_factor(collateral, debt, threshold_bps);
        let outcome = RiskManager::apply_liquidation(collateral, debt, repay, bonus_bps).unwrap();
        let after = calculate_health_factor(
            outcome.new_collateral_value,
            outcome.new_debt_value,
            threshold_bps,
        );

        assert_eq!(before, 4_444);
        assert_eq!(after, 1_280);
        assert!(after < before, "expected this bad-debt scenario to worsen, not improve, HF");

        // Confirm this matches the proven ratio condition: collateral*repay
        // < debt*seized  =>  HF must decrease.
        assert!(collateral * repay < debt * outcome.seized_collateral);
    }

    #[test]
    fn liquidation_improves_health_factor_when_ratio_condition_holds() {
        let threshold_bps = 8_000;
        let (collateral, debt, repay, bonus_bps) = (100_000, 90_000, 40_000, 500);

        let before = calculate_health_factor(collateral, debt, threshold_bps);
        let outcome = RiskManager::apply_liquidation(collateral, debt, repay, bonus_bps).unwrap();
        let after = calculate_health_factor(
            outcome.new_collateral_value,
            outcome.new_debt_value,
            threshold_bps,
        );

        assert!(collateral * repay >= debt * outcome.seized_collateral);
        assert!(after >= before);
    }

    #[test]
    fn full_repayment_yields_max_health_factor() {
        let outcome = RiskManager::apply_liquidation(100_000, 90_000, 90_000, 500).unwrap();
        assert_eq!(outcome.new_debt_value, 0);
        let hf = calculate_health_factor(outcome.new_collateral_value, outcome.new_debt_value, 8_000);
        assert_eq!(hf, i128::MAX);
    }

    #[test]
    fn no_overflow_at_extreme_parameters_returns_err_not_panic() {
        assert!(RiskManager::apply_liquidation(i128::MAX, i128::MAX, i128::MAX, i128::MAX).is_err());
        assert!(RiskManager::calculate_liquidation_bonus(i128::MAX, i128::MAX).is_err());
    }
}
