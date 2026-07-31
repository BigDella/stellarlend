pub fn assert_non_negative(value: i128, message: &str) {
    assert!(
        value >= 0,
        "{}: value must be non-negative, got {}",
        message,
        value
    );
}

pub fn assert_positive(value: i128, message: &str) {
    assert!(
        value > 0,
        "{}: value must be positive, got {}",
        message,
        value
    );
}

pub fn assert_in_range(value: i128, min: i128, max: i128, message: &str) {
    assert!(
        value >= min && value <= max,
        "{}: value {} must be between {} and {}",
        message,
        value,
        min,
        max
    );
}

pub fn assert_balance_non_negative(balance: i128, account: &str) {
    assert_non_negative(balance, &format!("{} balance", account));
}

pub fn assert_balances_equal(actual: i128, expected: i128, message: &str) {
    assert_eq!(
        actual, expected,
        "{}: expected {}, got {}",
        message, expected, actual
    );
}

pub fn assert_approximately_equal(actual: i128, expected: i128, tolerance: i128, message: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "{}: expected {} ± {}, got {} (diff: {})",
        message,
        expected,
        tolerance,
        actual,
        diff
    );
}

pub fn assert_percentage_in_range(
    value: i128,
    percentage_low: i128,
    percentage_high: i128,
    base: i128,
    message: &str,
) {
    let expected_low = base * percentage_low / 10_000;
    let expected_high = base * percentage_high / 10_000;
    assert_in_range(value, expected_low, expected_high, message);
}

pub fn assert_greater_than(actual: i128, threshold: i128, message: &str) {
    assert!(
        actual > threshold,
        "{}: expected value > {}, got {}",
        message,
        threshold,
        actual
    );
}

pub fn assert_less_than(actual: i128, threshold: i128, message: &str) {
    assert!(
        actual < threshold,
        "{}: expected value < {}, got {}",
        message,
        threshold,
        actual
    );
}

pub fn assert_zero(value: i128, message: &str) {
    assert_eq!(value, 0, "{}: expected 0, got {}", message, value);
}

pub fn assert_not_zero(value: i128, message: &str) {
    assert_ne!(value, 0, "{}: expected non-zero value, got 0", message);
}
