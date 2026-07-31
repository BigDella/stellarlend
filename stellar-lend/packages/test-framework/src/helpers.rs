use soroban_sdk::Env;

pub fn assert_equal_with_tolerance(actual: i128, expected: i128, tolerance_bps: i128) {
    let max_diff = (expected.abs() * tolerance_bps) / 10_000;
    let actual_diff = (actual - expected).abs();
    assert!(
        actual_diff <= max_diff,
        "Value {} not within {} bps of {}",
        actual,
        tolerance_bps,
        expected
    );
}

pub fn health_factor_bps(
    collateral_value: i128,
    debt_value: i128,
    threshold_bps: i128,
) -> i128 {
    if debt_value <= 0 {
        return i128::MAX;
    }
    if collateral_value <= 0 {
        return 0;
    }
    (collateral_value * threshold_bps) / debt_value
}

pub fn is_liquidatable(health_factor: i128) -> bool {
    health_factor < 10_000
}

pub fn calculate_interest_accrual(
    principal: i128,
    rate_bps: i128,
    seconds: u64,
) -> i128 {
    let seconds_per_year = 365 * 24 * 3600;
    (principal * rate_bps as i128 * seconds as i128) / (10_000 * seconds_per_year as i128)
}

pub fn assert_close(actual: i128, expected: i128, tolerance: i128) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "Expected {}, got {}, difference {}",
        expected,
        actual,
        diff
    );
}
