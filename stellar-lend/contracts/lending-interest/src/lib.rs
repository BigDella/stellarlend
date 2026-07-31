#![no_std]

use soroban_sdk::Env;
use stellarlend_safe_math::{bps_mul, safe_add, safe_div, safe_mul, MathError};

pub struct InterestRateModel {
    pub base_rate: i128,
    pub slope1: i128,
    pub slope2: i128,
    pub optimal_utilization: i128,
}

impl InterestRateModel {
    /// Variable-slope borrow rate in basis points.
    ///
    /// Below kink:  `base_rate + utilization × slope1 / 10 000`
    /// Above kink:  `base_rate + kink × slope1 / 10 000 + excess × slope2 / 10 000`
    pub fn calculate_borrow_rate(&self, utilization: i128) -> Result<i128, MathError> {
        if utilization <= self.optimal_utilization {
            let inc = safe_mul(utilization, self.slope1).and_then(|v| safe_div(v, 10_000))?;
            safe_add(self.base_rate, inc)
        } else {
            let excess = safe_add(utilization, -self.optimal_utilization)?;
            let kink_component = safe_mul(self.optimal_utilization, self.slope1)
                .and_then(|v| safe_div(v, 10_000))?;
            let excess_component =
                safe_mul(excess, self.slope2).and_then(|v| safe_div(v, 10_000))?;
            safe_add(self.base_rate, kink_component).and_then(|v| safe_add(v, excess_component))
        }
    }

    /// Supply rate: `borrow_rate × (10 000 − reserve_factor) / 10 000 × utilization / 10 000`
    pub fn calculate_supply_rate(
        &self,
        borrow_rate: i128,
        utilization: i128,
        reserve_factor: i128,
    ) -> Result<i128, MathError> {
        let net_factor = safe_add(10_000, -reserve_factor)?;
        let rate_to_pool = safe_mul(borrow_rate, net_factor).and_then(|v| safe_div(v, 10_000))?;
        safe_mul(rate_to_pool, utilization).and_then(|v| safe_div(v, 10_000))
    }
}

/// Utilization rate in basis points: `total_borrows × 10 000 / total_supply`.
pub fn calculate_utilization(total_borrows: i128, total_supply: i128) -> Result<i128, MathError> {
    if total_supply == 0 {
        return Ok(0);
    }
    safe_mul(total_borrows, 10_000).and_then(|v| safe_div(v, total_supply))
}

/// Simple interest via I256 intermediates: `principal × rate × elapsed / (SPY × 10 000)`.
///
/// Replaces the old `unwrap_or(0)` implementation which silently returned 0
/// on overflow.  Now returns `Err(MathError::Overflow)` for very large inputs.
pub fn accrue_interest(
    env: &Env,
    principal: i128,
    rate: i128,
    time_elapsed: u64,
) -> Result<i128, MathError> {
    if time_elapsed == 0 {
        return Ok(0);
    }
    stellarlend_safe_math::simple_interest(env, principal, rate, time_elapsed)
}

/// Compound interest over discrete periods using bps_mul for each step.
pub fn compound_interest(principal: i128, rate: i128, periods: u64) -> Result<i128, MathError> {
    let mut result = principal;
    for _ in 0..periods {
        let interest = bps_mul(result, rate)?;
        result = safe_add(result, interest)?;
    }
    safe_add(result, -principal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_utilization_calculation() {
        assert_eq!(calculate_utilization(50_000, 100_000), Ok(5_000));
        assert_eq!(calculate_utilization(80_000, 100_000), Ok(8_000));
        assert_eq!(calculate_utilization(0, 100_000), Ok(0));
        assert_eq!(calculate_utilization(100_000, 0), Ok(0));
    }

    #[test]
    fn test_interest_rate_model_below_kink() {
        let model = InterestRateModel {
            base_rate: 200,
            slope1: 400,
            slope2: 6_000,
            optimal_utilization: 8_000,
        };

        let rate_at_50 = model.calculate_borrow_rate(5_000).unwrap();
        assert!(rate_at_50 > model.base_rate);

        let rate_at_90 = model.calculate_borrow_rate(9_000).unwrap();
        assert!(rate_at_90 > rate_at_50);
    }

    #[test]
    fn test_accrue_interest_annual() {
        let env = Env::default();
        let interest =
            accrue_interest(&env, 100_000, 500, stellarlend_safe_math::SECONDS_PER_YEAR).unwrap();
        assert_eq!(interest, 5_000);
    }

    #[test]
    fn test_accrue_interest_zero_elapsed() {
        let env = Env::default();
        assert_eq!(accrue_interest(&env, 1_000_000, 500, 0), Ok(0));
    }

    #[test]
    fn test_utilization_overflow_is_err() {
        // total_borrows near MAX: safe_mul(MAX, 10_000) overflows.
        let result = calculate_utilization(i128::MAX, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_borrow_rate_overflow_inputs_err() {
        let model = InterestRateModel {
            base_rate: i128::MAX,
            slope1: i128::MAX,
            slope2: i128::MAX,
            optimal_utilization: 8_000,
        };
        assert!(model.calculate_borrow_rate(5_000).is_err());
    }

    #[test]
    fn test_supply_rate_zero_pool() {
        let model = InterestRateModel {
            base_rate: 200,
            slope1: 400,
            slope2: 6_000,
            optimal_utilization: 8_000,
        };
        // reserve_factor = 10_000 → net_factor = 0 → supply rate = 0.
        let rate = model.calculate_supply_rate(500, 5_000, 10_000).unwrap();
        assert_eq!(rate, 0);
    }
}
