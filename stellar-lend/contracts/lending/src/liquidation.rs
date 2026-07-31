//! # Gas-Optimised Liquidation (issue #632)
//!
//! Liquidations are gas-heavy and frequently revert *after* spending gas on
//! state preparation. This module front-loads validation and orders the checks
//! **cheapest-first** so a doomed liquidation aborts before any storage write.
//!
//! ## Optimisations
//!
//! 1. **Early validation** — health factor, profitability and oracle freshness
//!    are checked before any mutation.
//! 2. **Cheapest checks first** — pure arithmetic guards run ahead of storage
//!    reads, which run ahead of cross-contract oracle calls.
//! 3. **Batched reads** — every value needed is gathered once into a
//!    [`PositionSnapshot`] instead of being re-read per check.
//! 4. **Gas-vs-profit guard** — [`abort_if_unprofitable`] rejects liquidations
//!    whose estimated execution cost exceeds the seize bonus.
//!
//! The core math is implemented as pure functions so it can be unit-tested and
//! reused by an off-chain gas-benchmark harness without a Soroban `Env`.

use soroban_sdk::{contracterror, contracttype};

const BPS_SCALE: i128 = 10_000;

/// Health factor at exactly 1.0, expressed in basis points.
pub const HEALTH_FACTOR_ONE: i128 = 10_000;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum LiquidationError {
    /// `repay_amount` (or another input) was zero or negative.
    InvalidAmount = 1,
    /// Position health factor is at/above 1.0 — not liquidatable.
    PositionHealthy = 2,
    /// Oracle price is older than the freshness window.
    StaleOracle = 3,
    /// Estimated gas cost exceeds the liquidation bonus — not worth executing.
    Unprofitable = 4,
    /// Arithmetic overflow.
    Overflow = 5,
}

/// All state needed to validate and price a liquidation, read in one batch.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PositionSnapshot {
    /// Borrower collateral value in the common unit.
    pub collateral_value: i128,
    /// Borrower debt value (principal + accrued interest) in the common unit.
    pub debt_value: i128,
    /// Liquidation threshold in bps (e.g. 8000 = 80%).
    pub liquidation_threshold_bps: i128,
    /// Close factor in bps (max fraction of debt repayable in one call).
    pub close_factor_bps: i128,
    /// Liquidation incentive/bonus in bps (e.g. 1000 = 10%).
    pub liquidation_incentive_bps: i128,
    /// Timestamp of the oracle price used to build this snapshot.
    pub oracle_timestamp: u64,
}

/// Outcome of a validated liquidation plan.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidationPlan {
    /// Debt value actually repaid by the liquidator.
    pub repay_value: i128,
    /// Collateral value seized (repay + bonus).
    pub seize_value: i128,
    /// Liquidator's gross bonus (`seize_value - repay_value`).
    pub bonus_value: i128,
}

// ── Pure helpers (cheapest → most expensive) ──────────────────────────────

/// Health factor in bps. `hf = collateral_value * threshold / debt_value`.
/// A position with no debt is maximally healthy.
pub fn health_factor_bps(
    collateral_value: i128,
    debt_value: i128,
    threshold_bps: i128,
) -> Result<i128, LiquidationError> {
    if debt_value <= 0 {
        return Ok(i128::MAX);
    }
    if collateral_value <= 0 {
        return Ok(0);
    }
    collateral_value
        .checked_mul(threshold_bps)
        .ok_or(LiquidationError::Overflow)?
        .checked_div(debt_value)
        .ok_or(LiquidationError::Overflow)
}

/// A position is liquidatable when its health factor drops below 1.0.
pub fn is_liquidatable(health_factor_bps: i128) -> bool {
    health_factor_bps < HEALTH_FACTOR_ONE
}

/// Maximum debt value repayable in a single call (`debt * close_factor`).
pub fn max_repay_value(debt_value: i128, close_factor_bps: i128) -> Result<i128, LiquidationError> {
    debt_value
        .checked_mul(close_factor_bps)
        .ok_or(LiquidationError::Overflow)?
        .checked_div(BPS_SCALE)
        .ok_or(LiquidationError::Overflow)
}

/// Collateral value seized for a given repay value: `repay * (1 + incentive)`.
pub fn seize_value(repay_value: i128, incentive_bps: i128) -> Result<i128, LiquidationError> {
    let bonus = repay_value
        .checked_mul(incentive_bps)
        .ok_or(LiquidationError::Overflow)?
        .checked_div(BPS_SCALE)
        .ok_or(LiquidationError::Overflow)?;
    repay_value
        .checked_add(bonus)
        .ok_or(LiquidationError::Overflow)
}

/// Is the oracle price fresh enough? Pure so it can be tested deterministically.
pub fn is_oracle_fresh(oracle_timestamp: u64, now: u64, max_age_secs: u64) -> bool {
    now.saturating_sub(oracle_timestamp) <= max_age_secs
}

/// Abort when the bonus cannot cover the estimated execution cost.
pub fn abort_if_unprofitable(
    bonus_value: i128,
    est_gas_cost: i128,
) -> Result<(), LiquidationError> {
    if bonus_value <= est_gas_cost {
        return Err(LiquidationError::Unprofitable);
    }
    Ok(())
}

/// Validate and build a [`LiquidationPlan`] running checks **cheapest-first**.
///
/// Ordering (ascending gas cost):
/// 1. amount sign check — no reads
/// 2. health-factor check — pure arithmetic over the already-batched snapshot
/// 3. profitability/close-factor clamp — pure arithmetic
/// 4. oracle freshness — depends on the (already fetched) oracle timestamp
/// 5. gas-vs-profit guard — final pure comparison
///
/// All checks complete before the caller performs any storage write or token
/// transfer, so a rejected liquidation costs only the batched read.
#[allow(clippy::too_many_arguments)]
pub fn plan_liquidation(
    snapshot: &PositionSnapshot,
    requested_repay_value: i128,
    now: u64,
    max_oracle_age_secs: u64,
    est_gas_cost: i128,
) -> Result<LiquidationPlan, LiquidationError> {
    // 1. Cheapest: input sign check (no storage).
    if requested_repay_value <= 0 {
        return Err(LiquidationError::InvalidAmount);
    }

    // 2. Health factor — pure arithmetic on the batched snapshot.
    let hf = health_factor_bps(
        snapshot.collateral_value,
        snapshot.debt_value,
        snapshot.liquidation_threshold_bps,
    )?;
    if !is_liquidatable(hf) {
        return Err(LiquidationError::PositionHealthy);
    }

    // 3. Profitability/close-factor clamp — pure arithmetic.
    let cap = max_repay_value(snapshot.debt_value, snapshot.close_factor_bps)?;
    let repay_value = requested_repay_value.min(cap);
    if repay_value <= 0 {
        return Err(LiquidationError::InvalidAmount);
    }

    // 4. Oracle freshness — uses the timestamp already in the snapshot.
    if !is_oracle_fresh(snapshot.oracle_timestamp, now, max_oracle_age_secs) {
        return Err(LiquidationError::StaleOracle);
    }

    // 5. Gas-vs-profit guard — final pure comparison.
    let seize = seize_value(repay_value, snapshot.liquidation_incentive_bps)?;
    let bonus = seize
        .checked_sub(repay_value)
        .ok_or(LiquidationError::Overflow)?;
    abort_if_unprofitable(bonus, est_gas_cost)?;

    Ok(LiquidationPlan {
        repay_value,
        seize_value: seize,
        bonus_value: bonus,
    })
}

#[cfg(test)]
mod unit {
    use super::*;

    fn snap() -> PositionSnapshot {
        PositionSnapshot {
            collateral_value: 1_000,
            debt_value: 1_000,
            liquidation_threshold_bps: 8_000, // 80%
            close_factor_bps: 5_000,          // 50%
            liquidation_incentive_bps: 1_000, // 10%
            oracle_timestamp: 100,
        }
    }

    #[test]
    fn healthy_position_rejected() {
        let s = PositionSnapshot {
            collateral_value: 2_000,
            debt_value: 1_000,
            ..snap()
        };
        // hf = 2000*8000/1000 = 16000 > 10000 ⇒ healthy.
        let res = plan_liquidation(&s, 100, 100, 60, 0);
        assert_eq!(res, Err(LiquidationError::PositionHealthy));
    }

    #[test]
    fn invalid_amount_is_cheapest_check() {
        // Even a healthy/stale position rejects a non-positive amount first.
        let res = plan_liquidation(&snap(), 0, 1_000_000, 60, 0);
        assert_eq!(res, Err(LiquidationError::InvalidAmount));
    }

    #[test]
    fn stale_oracle_rejected() {
        // hf = 1000*8000/1000 = 8000 < 10000 ⇒ liquidatable, but oracle is old.
        let res = plan_liquidation(&snap(), 100, 1_000, 60, 0);
        assert_eq!(res, Err(LiquidationError::StaleOracle));
    }

    #[test]
    fn unprofitable_rejected() {
        // repay 100 ⇒ bonus 10; gas cost 50 ⇒ abort.
        let res = plan_liquidation(&snap(), 100, 150, 60, 50);
        assert_eq!(res, Err(LiquidationError::Unprofitable));
    }

    #[test]
    fn happy_path_clamps_to_close_factor() {
        // request 900 but close factor caps repay at 500; bonus = 50.
        let plan = plan_liquidation(&snap(), 900, 150, 60, 10).unwrap();
        assert_eq!(plan.repay_value, 500);
        assert_eq!(plan.seize_value, 550);
        assert_eq!(plan.bonus_value, 50);
    }

    #[test]
    fn no_debt_is_healthy() {
        assert_eq!(health_factor_bps(1_000, 0, 8_000).unwrap(), i128::MAX);
        assert!(!is_liquidatable(i128::MAX));
    }

    #[test]
    fn oracle_freshness_boundary() {
        assert!(is_oracle_fresh(100, 160, 60));
        assert!(!is_oracle_fresh(100, 161, 60));
    }
}
