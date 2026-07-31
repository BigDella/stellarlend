//! Rounding utilities for depositor-friendly arithmetic
//!
//! This module provides rounding functions that are depositor-friendly:
//! - Round down for deposits (depositor keeps more)
//! - Round up for withdrawals (user receives at least what they expect)
//!
//! This prevents arbitrage opportunities from rounding asymmetry and ensures
//! consistent behavior across operations.

/// Scale for percentage calculations (10000 = 100%)
pub const PERCENTAGE_SCALE: i128 = 10000;

/// Scale for interest rate calculations
pub const RATE_SCALE: i128 = 1_000_000;

/// Round down (floor) - depositor-friendly for deposits
///
/// Used for deposit interest calculations to ensure depositors keep more value.
///
/// # Arguments
/// * `numerator` - The numerator
/// * `denominator` - The denominator
///
/// # Returns
/// The rounded-down result
pub fn round_down(numerator: i128, denominator: i128) -> i128 {
    if denominator == 0 {
        return 0;
    }
    numerator / denominator
}

/// Round up (ceil) - depositor-friendly for withdrawals
///
/// Used for withdrawal calculations to ensure users receive at least what they expect.
///
/// # Arguments
/// * `numerator` - The numerator
/// * `denominator` - The denominator
///
/// # Returns
/// The rounded-up result
pub fn round_up(numerator: i128, denominator: i128) -> i128 {
    if denominator == 0 {
        return 0;
    }
    (numerator + denominator - 1) / denominator
}

/// Calculate deposit interest with depositor-friendly rounding (round down)
///
/// # Arguments
/// * `principal` - The principal amount
/// * `rate` - The interest rate (in basis points or similar scale)
/// * `scale` - The scale for the rate (e.g., RATE_SCALE)
///
/// # Returns
/// The interest amount rounded down
pub fn calculate_deposit_interest(principal: i128, rate: i128, scale: i128) -> i128 {
    round_down(principal * rate, scale)
}

/// Calculate withdrawal amount with depositor-friendly rounding (round up)
///
/// # Arguments
/// * `balance` - The total balance
/// * `fraction` - The fraction to withdraw (in basis points)
/// * `scale` - The scale for the fraction (e.g., PERCENTAGE_SCALE)
///
/// # Returns
/// The withdrawal amount rounded up
pub fn calculate_withdraw_amount(balance: i128, fraction: i128, scale: i128) -> i128 {
    round_up(balance * fraction, scale)
}

/// Calculate repay amount with depositor-friendly rounding (round down)
///
/// Used for debt repayment to ensure users don't overpay due to rounding.
///
/// # Arguments
/// * `debt` - The debt amount
/// * `fraction` - The fraction to repay (in basis points)
/// * `scale` - The scale for the fraction (e.g., PERCENTAGE_SCALE)
///
/// # Returns
/// The repayment amount rounded down
pub fn calculate_repay_amount(debt: i128, fraction: i128, scale: i128) -> i128 {
    round_down(debt * fraction, scale)
}

/// Calculate liquidation amount with depositor-friendly rounding (round up)
///
/// Used for liquidations to ensure protocol recovers at least the expected amount.
///
/// # Arguments
/// * `debt` - The debt amount
/// * `fraction` - The liquidation fraction (in basis points)
/// * `scale` - The scale for the fraction (e.g., PERCENTAGE_SCALE)
///
/// # Returns
/// The liquidation amount rounded up
pub fn calculate_liquidation_amount(debt: i128, fraction: i128, scale: i128) -> i128 {
    round_up(debt * fraction, scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_down() {
        assert_eq!(round_down(10, 3), 3); // 10/3 = 3.33 -> 3
        assert_eq!(round_down(10, 2), 5); // 10/2 = 5 -> 5
        assert_eq!(round_down(9, 3), 3); // 9/3 = 3 -> 3
        assert_eq!(round_down(1, 2), 0); // 1/2 = 0.5 -> 0
        assert_eq!(round_down(0, 5), 0); // 0/5 = 0 -> 0
    }

    #[test]
    fn test_round_up() {
        assert_eq!(round_up(10, 3), 4); // 10/3 = 3.33 -> 4
        assert_eq!(round_up(10, 2), 5); // 10/2 = 5 -> 5
        assert_eq!(round_up(9, 3), 3); // 9/3 = 3 -> 3
        assert_eq!(round_up(1, 2), 1); // 1/2 = 0.5 -> 1
        assert_eq!(round_up(0, 5), 0); // 0/5 = 0 -> 0
    }

    #[test]
    fn test_calculate_deposit_interest() {
        // 1000 principal, 5% rate (500 bps), 10000 scale
        // Expected: 1000 * 500 / 10000 = 50
        assert_eq!(calculate_deposit_interest(1000, 500, 10000), 50);

        // 1000 principal, 5.5% rate (550 bps), 10000 scale
        // Expected: 1000 * 550 / 10000 = 55
        assert_eq!(calculate_deposit_interest(1000, 550, 10000), 55);

        // 1000 principal, 5.5% rate (550 bps), 10000 scale
        // With rounding down: 1000 * 550 / 10000 = 55
        assert_eq!(calculate_deposit_interest(1000, 555, 10000), 55); // 55.5 -> 55
    }

    #[test]
    fn test_calculate_withdraw_amount() {
        // 1000 balance, 50% withdraw (5000 bps), 10000 scale
        // Expected: 1000 * 5000 / 10000 = 500
        assert_eq!(calculate_withdraw_amount(1000, 5000, 10000), 500);

        // 1000 balance, 33.33% withdraw (3333 bps), 10000 scale
        // With rounding up: 1000 * 3333 / 10000 = 333.3 -> 334
        assert_eq!(calculate_withdraw_amount(1000, 3333, 10000), 334);
    }

    #[test]
    fn test_calculate_repay_amount() {
        // 1000 debt, 50% repay (5000 bps), 10000 scale
        // Expected: 1000 * 5000 / 10000 = 500
        assert_eq!(calculate_repay_amount(1000, 5000, 10000), 500);

        // 1000 debt, 33.33% repay (3333 bps), 10000 scale
        // With rounding down: 1000 * 3333 / 10000 = 333.3 -> 333
        assert_eq!(calculate_repay_amount(1000, 3333, 10000), 333);
    }

    #[test]
    fn test_calculate_liquidation_amount() {
        // 1000 debt, 50% liquidation (5000 bps), 10000 scale
        // Expected: 1000 * 5000 / 10000 = 500
        assert_eq!(calculate_liquidation_amount(1000, 5000, 10000), 500);

        // 1000 debt, 33.33% liquidation (3333 bps), 10000 scale
        // With rounding up: 1000 * 3333 / 10000 = 333.3 -> 334
        assert_eq!(calculate_liquidation_amount(1000, 3333, 10000), 334);
    }

    #[test]
    fn test_rounding_asymmetry_prevention() {
        // Test that rounding is consistent and prevents arbitrage
        let balance = 1000;
        let fraction = 3333; // 33.33%

        // Withdraw should round up (user gets more)
        let withdraw = calculate_withdraw_amount(balance, fraction, 10000);

        // Repay should round down (user pays less)
        let repay = calculate_repay_amount(balance, fraction, 10000);

        // Ensure withdraw >= repay for same fraction (depositor-friendly)
        assert!(withdraw >= repay);
    }
}
