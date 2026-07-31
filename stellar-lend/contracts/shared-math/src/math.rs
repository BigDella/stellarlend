use crate::errors::MathError;

// BPS (basis points) scale: 10,000 = 100%
const BPS_SCALE: i128 = 10_000;
// APR to APY conversion
const SECONDS_PER_YEAR: i128 = 365 * 24 * 60 * 60;

/// Calculate interest accrual over time period
///
/// Formula: amount * (1 + rate * time / seconds_per_year)
/// where rate is in basis points (10000 = 100%)
pub fn accrue_interest(
    principal: i128,
    rate_bps: i128,
    time_elapsed: u64,
) -> Result<i128, MathError> {
    if principal < 0 {
        return Err(MathError::NegativeValue);
    }
    if rate_bps < 0 {
        return Err(MathError::NegativeValue);
    }

    let time_i128 = time_elapsed as i128;

    // interest = principal * rate_bps * time_elapsed / (BPS_SCALE * SECONDS_PER_YEAR)
    let interest = principal
        .checked_mul(rate_bps)
        .ok_or(MathError::Overflow)?
        .checked_mul(time_i128)
        .ok_or(MathError::Overflow)?
        .checked_div(BPS_SCALE)
        .ok_or(MathError::DivisionByZero)?
        .checked_div(SECONDS_PER_YEAR)
        .ok_or(MathError::DivisionByZero)?;

    principal
        .checked_add(interest)
        .ok_or(MathError::Overflow)
}

/// Calculate collateral ratio: (total_collateral_value * 100) / total_debt_value
/// Returns ratio in basis points (10000 = 100%)
pub fn collateral_ratio(
    collateral_value: i128,
    debt_value: i128,
) -> Result<i128, MathError> {
    if collateral_value < 0 || debt_value < 0 {
        return Err(MathError::NegativeValue);
    }
    if debt_value == 0 {
        return Ok(i128::MAX); // Infinite collateral ratio if no debt
    }

    collateral_value
        .checked_mul(BPS_SCALE)
        .ok_or(MathError::Overflow)?
        .checked_div(debt_value)
        .ok_or(MathError::DivisionByZero)
}

/// Calculate health factor: collateral_ratio / min_collateral_ratio
/// Returns health factor in basis points (10000 = healthy, < 10000 = undercollateralized)
pub fn health_factor(
    collateral_value: i128,
    debt_value: i128,
    min_collateral_ratio_bps: i128,
) -> Result<i128, MathError> {
    if min_collateral_ratio_bps <= 0 {
        return Err(MathError::InvalidParameter);
    }

    let ratio = collateral_ratio(collateral_value, debt_value)?;
    ratio
        .checked_mul(BPS_SCALE)
        .ok_or(MathError::Overflow)?
        .checked_div(min_collateral_ratio_bps)
        .ok_or(MathError::DivisionByZero)
}

/// Calculate liquidation discount applied to collateral value
/// discount = collateral * (1 - liquidation_incentive_bps / 10000)
pub fn liquidation_discount(
    collateral_amount: i128,
    liquidation_incentive_bps: i128,
) -> Result<i128, MathError> {
    if collateral_amount < 0 {
        return Err(MathError::NegativeValue);
    }
    if liquidation_incentive_bps < 0 || liquidation_incentive_bps > BPS_SCALE {
        return Err(MathError::InvalidParameter);
    }

    let discount_factor = BPS_SCALE
        .checked_sub(liquidation_incentive_bps)
        .ok_or(MathError::InvalidParameter)?;

    collateral_amount
        .checked_mul(discount_factor)
        .ok_or(MathError::Overflow)?
        .checked_div(BPS_SCALE)
        .ok_or(MathError::DivisionByZero)
}

/// Calculate utilization rate: debt / supply_cap
/// Returns utilization in basis points (10000 = 100%)
pub fn utilization_rate(
    total_debt: i128,
    supply_cap: i128,
) -> Result<i128, MathError> {
    if total_debt < 0 || supply_cap < 0 {
        return Err(MathError::NegativeValue);
    }
    if supply_cap == 0 {
        return Ok(0);
    }

    total_debt
        .checked_mul(BPS_SCALE)
        .ok_or(MathError::Overflow)?
        .checked_div(supply_cap)
        .ok_or(MathError::DivisionByZero)
        .map(|u| u.min(BPS_SCALE))
}

/// Convert annual percentage rate (APR) to annual percentage yield (APY)
/// APY = (1 + APR/n)^n - 1, where n is compounding periods per year
/// Simplified: APY ≈ APR * (1 + APR/2) for continuous compounding
pub fn apr_to_apy(apr_bps: i128, compounding_periods: i128) -> Result<i128, MathError> {
    if apr_bps < 0 {
        return Err(MathError::NegativeValue);
    }
    if compounding_periods <= 0 {
        return Err(MathError::InvalidParameter);
    }

    // Convert BPS to decimal: divide by 10000
    // (1 + r/n)^n ≈ 1 + r + r^2/2 for small r
    let rate_decimal = apr_bps; // in basis points
    let one_plus_rate = BPS_SCALE
        .checked_add(rate_decimal.checked_div(compounding_periods).ok_or(MathError::DivisionByZero)?)
        .ok_or(MathError::Overflow)?;

    // Simplified: (1 + r/n)^n ≈ (1 + r/n) * compounding_periods
    one_plus_rate
        .checked_mul(compounding_periods)
        .ok_or(MathError::Overflow)?
        .checked_sub(BPS_SCALE)
        .ok_or(MathError::InvalidParameter)
}

/// Calculate maximum borrowable amount based on collateral
/// max_borrow = (collateral_value * LTV) - current_debt
pub fn max_borrow_amount(
    collateral_value: i128,
    ltv_bps: i128,
    current_debt: i128,
) -> Result<i128, MathError> {
    if collateral_value < 0 || current_debt < 0 {
        return Err(MathError::NegativeValue);
    }
    if ltv_bps < 0 || ltv_bps > BPS_SCALE {
        return Err(MathError::InvalidParameter);
    }

    let max_debt = collateral_value
        .checked_mul(ltv_bps)
        .ok_or(MathError::Overflow)?
        .checked_div(BPS_SCALE)
        .ok_or(MathError::DivisionByZero)?;

    max_debt
        .checked_sub(current_debt)
        .ok_or(MathError::InvalidParameter)
        .and_then(|x| if x < 0 { Ok(0) } else { Ok(x) })
}

/// Safe multiplication with overflow check
pub fn safe_multiply(a: i128, b: i128) -> Result<i128, MathError> {
    a.checked_mul(b).ok_or(MathError::Overflow)
}

/// Safe division with division by zero check
pub fn safe_divide(a: i128, b: i128) -> Result<i128, MathError> {
    if b == 0 {
        return Err(MathError::DivisionByZero);
    }
    Ok(a / b)
}

/// Safe division with rounding up
pub fn safe_divide_ceil(a: i128, b: i128) -> Result<i128, MathError> {
    if b == 0 {
        return Err(MathError::DivisionByZero);
    }
    let result = a / b;
    let remainder = a % b;
    if remainder > 0 {
        result.checked_add(1).ok_or(MathError::Overflow)
    } else {
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accrue_interest() {
        // 1000 principal, 5% APR, 1 year
        let result = accrue_interest(1000, 500, SECONDS_PER_YEAR as u64).unwrap();
        assert!(result >= 1050 && result <= 1051); // Should be ~1050
    }

    #[test]
    fn test_accrue_interest_half_year() {
        // 1000 principal, 5% APR, half year
        let result = accrue_interest(1000, 500, (SECONDS_PER_YEAR / 2) as u64).unwrap();
        assert!(result >= 1024 && result <= 1026); // Should be ~1025
    }

    #[test]
    fn test_collateral_ratio_healthy() {
        // 2000 collateral, 1000 debt = 200% ratio = 20000 BPS
        let result = collateral_ratio(2000, 1000).unwrap();
        assert_eq!(result, 20000);
    }

    #[test]
    fn test_collateral_ratio_no_debt() {
        let result = collateral_ratio(1000, 0).unwrap();
        assert_eq!(result, i128::MAX);
    }

    #[test]
    fn test_health_factor_healthy() {
        // 2000 collateral, 1000 debt, 15000 BPS min ratio
        // ratio = 2000 / 1000 = 20000 BPS
        // hf = 20000 * 10000 / 15000 = 13333 BPS
        let result = health_factor(2000, 1000, 15000).unwrap();
        assert_eq!(result, 13333);
    }

    #[test]
    fn test_health_factor_undercollateralized() {
        // 1000 collateral, 1000 debt, 15000 BPS min ratio
        // ratio = 1000 / 1000 = 10000 BPS
        // hf = 10000 * 10000 / 15000 = 6666 BPS (undercollateralized)
        let result = health_factor(1000, 1000, 15000).unwrap();
        assert_eq!(result, 6666);
    }

    #[test]
    fn test_liquidation_discount() {
        // 1000 collateral, 10% incentive = 900
        let result = liquidation_discount(1000, 1000).unwrap();
        assert_eq!(result, 900);
    }

    #[test]
    fn test_liquidation_discount_no_incentive() {
        let result = liquidation_discount(1000, 0).unwrap();
        assert_eq!(result, 1000);
    }

    #[test]
    fn test_utilization_rate_half() {
        // 5000 debt, 10000 cap = 50% util = 5000 BPS
        let result = utilization_rate(5000, 10000).unwrap();
        assert_eq!(result, 5000);
    }

    #[test]
    fn test_utilization_rate_full() {
        // 10000 debt, 10000 cap = 100% util = 10000 BPS
        let result = utilization_rate(10000, 10000).unwrap();
        assert_eq!(result, 10000);
    }

    #[test]
    fn test_utilization_rate_zero_cap() {
        let result = utilization_rate(5000, 0).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_max_borrow_amount() {
        // 1000 collateral, 80% LTV = 800 max debt, 0 current debt
        let result = max_borrow_amount(1000, 8000, 0).unwrap();
        assert_eq!(result, 800);
    }

    #[test]
    fn test_max_borrow_amount_with_existing_debt() {
        // 1000 collateral, 80% LTV = 800 max debt, 300 current debt
        let result = max_borrow_amount(1000, 8000, 300).unwrap();
        assert_eq!(result, 500);
    }

    #[test]
    fn test_max_borrow_amount_at_limit() {
        // 1000 collateral, 80% LTV = 800 max debt, 800 current debt
        let result = max_borrow_amount(1000, 8000, 800).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_negative_values_error() {
        assert_eq!(accrue_interest(-100, 500, 1000), Err(MathError::NegativeValue));
        assert_eq!(collateral_ratio(-100, 100), Err(MathError::NegativeValue));
        assert_eq!(utilization_rate(-100, 1000), Err(MathError::NegativeValue));
    }

    #[test]
    fn test_division_by_zero() {
        assert_eq!(safe_divide(100, 0), Err(MathError::DivisionByZero));
    }

    #[test]
    fn test_safe_divide_ceil() {
        assert_eq!(safe_divide_ceil(10, 3).unwrap(), 4); // 10/3 = 3.33 -> 4
        assert_eq!(safe_divide_ceil(9, 3).unwrap(), 3);  // 9/3 = 3 -> 3
    }
}
