#![no_std]

use lending_types::{calculate_health_factor, is_healthy, BPS_DIVISOR};
use stellarlend_safe_math::{safe_add, safe_div, safe_mul, safe_sub, MathError};

pub struct RiskManager;

impl RiskManager {
    pub fn check_liquidation_eligibility(
        collateral_value: i128,
        debt_value: i128,
        liquidation_threshold_bps: i128,
    ) -> bool {
        let health_factor =
            calculate_health_factor(collateral_value, debt_value, liquidation_threshold_bps);
        !is_healthy(health_factor)
    }

    pub fn calculate_liquidation_bonus(
        debt_amount: i128,
        bonus_bps: i128,
    ) -> Result<i128, MathError> {
        safe_mul(debt_amount, bonus_bps).and_then(|v| safe_div(v, BPS_DIVISOR))
    }

    pub fn calculate_max_liquidatable(
        debt_value: i128,
        close_factor_bps: i128,
    ) -> Result<i128, MathError> {
        safe_mul(debt_value, close_factor_bps).and_then(|v| safe_div(v, BPS_DIVISOR))
    }

    pub fn validate_borrow_capacity(
        collateral_value: i128,
        existing_debt: i128,
        new_borrow: i128,
        collateral_factor_bps: i128,
    ) -> Result<bool, MathError> {
        let max_borrow =
            safe_mul(collateral_value, collateral_factor_bps).and_then(|v| safe_div(v, BPS_DIVISOR))?;
        let total_debt = safe_add(existing_debt, new_borrow)?;
        Ok(total_debt <= max_borrow)
    }

    pub fn calculate_ltv(debt_value: i128, collateral_value: i128) -> Result<i128, MathError> {
        if collateral_value == 0 {
            return Ok(BPS_DIVISOR);
        }
        safe_mul(debt_value, BPS_DIVISOR).and_then(|v| safe_div(v, collateral_value))
    }

    pub fn check_concentration_risk(
        asset_value: i128,
        total_pool_value: i128,
        max_concentration_bps: i128,
    ) -> Result<bool, MathError> {
        if total_pool_value == 0 {
            return Ok(true);
        }
        let concentration =
            safe_mul(asset_value, BPS_DIVISOR).and_then(|v| safe_div(v, total_pool_value))?;
        Ok(concentration <= max_concentration_bps)
    }

    /// Simulates a partial liquidation of an under-collateralized position.
    ///
    /// A liquidator repays `repay_amount` of the position's debt and, in
    /// exchange, seizes collateral equal to the repaid amount plus a
    /// liquidation bonus (the standard Aave/Compound-style liquidation
    /// incentive: `seized = repay_amount * (1 + bonus_bps / 10_000)`).
    ///
    /// Returns `Err` — rather than panicking or silently clamping — for any
    /// input that would violate protocol solvency: repaying more than the
    /// outstanding debt, or seizing more collateral than the position holds.
    pub fn apply_liquidation(
        collateral_value: i128,
        debt_value: i128,
        repay_amount: i128,
        liquidation_bonus_bps: i128,
    ) -> Result<LiquidationOutcome, MathError> {
        if repay_amount < 0 || repay_amount > debt_value {
            return Err(MathError::Underflow);
        }

        let bonus = Self::calculate_liquidation_bonus(repay_amount, liquidation_bonus_bps)?;
        let seized_collateral = safe_add(repay_amount, bonus)?;

        if seized_collateral > collateral_value {
            return Err(MathError::Underflow);
        }

        let new_collateral_value = safe_sub(collateral_value, seized_collateral)?;
        let new_debt_value = safe_sub(debt_value, repay_amount)?;

        Ok(LiquidationOutcome {
            new_collateral_value,
            new_debt_value,
            seized_collateral,
            liquidator_profit: bonus,
        })
    }
}

/// Resulting position state and liquidator payout from
/// [`RiskManager::apply_liquidation`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiquidationOutcome {
    pub new_collateral_value: i128,
    pub new_debt_value: i128,
    pub seized_collateral: i128,
    pub liquidator_profit: i128,
}

pub struct RiskMetrics {
    pub health_factor: i128,
    pub ltv_ratio: i128,
    pub liquidation_price: i128,
    pub borrow_capacity: i128,
}

impl RiskMetrics {
    pub fn calculate(
        collateral_value: i128,
        debt_value: i128,
        collateral_factor_bps: i128,
        liquidation_threshold_bps: i128,
    ) -> Result<Self, MathError> {
        let health_factor =
            calculate_health_factor(collateral_value, debt_value, liquidation_threshold_bps);
        let ltv_ratio = RiskManager::calculate_ltv(debt_value, collateral_value)?;
        let borrow_capacity =
            safe_mul(collateral_value, collateral_factor_bps).and_then(|v| safe_div(v, BPS_DIVISOR))?;

        let liquidation_price = if collateral_value > 0 {
            let effective_collateral =
                safe_mul(collateral_value, liquidation_threshold_bps).and_then(|v| safe_div(v, BPS_DIVISOR))?;
            if effective_collateral == 0 {
                0
            } else {
                safe_mul(debt_value, BPS_DIVISOR).and_then(|v| safe_div(v, effective_collateral))?
            }
        } else {
            0
        };

        Ok(Self {
            health_factor,
            ltv_ratio,
            liquidation_price,
            borrow_capacity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liquidation_eligibility() {
        assert!(RiskManager::check_liquidation_eligibility(
            100_000, 90_000, 8_000
        ));
        assert!(!RiskManager::check_liquidation_eligibility(
            100_000, 50_000, 8_000
        ));
    }

    #[test]
    fn test_liquidation_bonus() {
        let bonus = RiskManager::calculate_liquidation_bonus(100_000, 500).unwrap();
        assert_eq!(bonus, 5_000);
    }

    #[test]
    fn test_ltv_calculation() {
        assert_eq!(RiskManager::calculate_ltv(50_000, 100_000).unwrap(), 5_000);
        assert_eq!(RiskManager::calculate_ltv(80_000, 100_000).unwrap(), 8_000);
    }

    #[test]
    fn test_borrow_capacity() {
        assert!(RiskManager::validate_borrow_capacity(100_000, 50_000, 10_000, 7_500).unwrap());
        assert!(!RiskManager::validate_borrow_capacity(100_000, 70_000, 10_000, 7_500).unwrap());
    }

    #[test]
    fn test_concentration_risk() {
        assert!(RiskManager::check_concentration_risk(30_000, 100_000, 5_000).unwrap());
        assert!(!RiskManager::check_concentration_risk(60_000, 100_000, 5_000).unwrap());
    }

    #[test]
    fn test_risk_metrics() {
        let metrics = RiskMetrics::calculate(100_000, 50_000, 7_500, 8_000).unwrap();
        assert_eq!(metrics.ltv_ratio, 5_000);
        assert!(metrics.health_factor > 10_000);
        assert_eq!(metrics.borrow_capacity, 75_000);
    }

    #[test]
    fn test_apply_liquidation_improves_health_factor() {
        // Unhealthy position: collateral 100_000, debt 90_000, threshold 80%.
        let threshold_bps = 8_000;
        let before = calculate_health_factor(100_000, 90_000, threshold_bps);
        assert!(before < BPS_DIVISOR);

        let outcome = RiskManager::apply_liquidation(100_000, 90_000, 40_000, 500).unwrap();
        let after = calculate_health_factor(
            outcome.new_collateral_value,
            outcome.new_debt_value,
            threshold_bps,
        );
        assert!(after > before, "liquidation must improve health factor");
    }

    #[test]
    fn test_apply_liquidation_profit_matches_bonus() {
        let outcome = RiskManager::apply_liquidation(100_000, 90_000, 40_000, 500).unwrap();
        assert_eq!(outcome.liquidator_profit, 2_000); // 40_000 * 500 / 10_000
        assert_eq!(outcome.seized_collateral, 42_000);
        assert_eq!(outcome.new_debt_value, 50_000);
        assert_eq!(outcome.new_collateral_value, 58_000);
    }

    #[test]
    fn test_apply_liquidation_rejects_overpayment() {
        assert!(RiskManager::apply_liquidation(100_000, 90_000, 90_001, 500).is_err());
    }

    #[test]
    fn test_apply_liquidation_rejects_insufficient_collateral() {
        // Seizing repay + bonus would exceed available collateral.
        assert!(RiskManager::apply_liquidation(40_000, 90_000, 39_000, 500).is_err());
    }

    #[test]
    fn test_liquidation_bonus_overflow_is_err() {
        assert!(RiskManager::calculate_liquidation_bonus(i128::MAX, i128::MAX).is_err());
    }
}
