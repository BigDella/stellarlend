use crate::error::MathError;
use crate::rounding::RoundingMode;
use soroban_sdk::{Env, I256};

pub fn mul_div(env: &Env, a: i128, b: i128, denominator: i128) -> Result<i128, MathError> {
    if denominator == 0 {
        return Err(MathError::DivisionByZero);
    }
    let a256 = I256::from_i128(env, a);
    let b256 = I256::from_i128(env, b);
    let d256 = I256::from_i128(env, denominator);
    let product = a256.mul(&b256);
    let result = product.div(&d256);
    result.to_i128().ok_or(MathError::Overflow)
}

pub fn mul_div_round_up(
    env: &Env,
    a: i128,
    b: i128,
    denominator: i128,
) -> Result<i128, MathError> {
    if denominator == 0 {
        return Err(MathError::DivisionByZero);
    }
    let a256 = I256::from_i128(env, a);
    let b256 = I256::from_i128(env, b);
    let d256 = I256::from_i128(env, denominator);
    let product = a256.mul(&b256);
    let remainder = product.rem_euclid(&d256);
    let result = product.div(&d256);
    let zero = I256::from_i128(env, 0);
    if remainder > zero {
        let one = I256::from_i128(env, 1);
        result
            .add(&one)
            .to_i128()
            .ok_or(MathError::Overflow)
    } else {
        result.to_i128().ok_or(MathError::Overflow)
    }
}

pub fn mul_div_floor(env: &Env, a: i128, b: i128, denominator: i128) -> Result<i128, MathError> {
    mul_div(env, a, b, denominator)
}

pub fn mul_div_ceil(env: &Env, a: i128, b: i128, denominator: i128) -> Result<i128, MathError> {
    mul_div_round_up(env, a, b, denominator)
}

pub fn mul_div_with_rounding(
    env: &Env,
    a: i128,
    b: i128,
    denominator: i128,
    mode: RoundingMode,
) -> Result<i128, MathError> {
    match mode {
        RoundingMode::Down | RoundingMode::Nearest => mul_div(env, a, b, denominator),
        RoundingMode::Up => mul_div_round_up(env, a, b, denominator),
    }
}

pub fn mul_div_with_safety(
    env: &Env,
    a: i128,
    b: i128,
    denominator: i128,
    is_liability: bool,
) -> Result<i128, MathError> {
    if is_liability {
        mul_div_round_up(env, a, b, denominator)
    } else {
        mul_div(env, a, b, denominator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    fn env() -> Env {
        Env::default()
    }

    #[test]
    fn mul_div_normal() {
        let e = env();
        assert_eq!(mul_div(&e, 10, 10, 5).unwrap(), 20);
        assert_eq!(mul_div(&e, 100, 100, 50).unwrap(), 200);
    }

    #[test]
    fn mul_div_truncation() {
        let e = env();
        assert_eq!(mul_div(&e, 10, 10, 3).unwrap(), 33);
    }

    #[test]
    fn mul_div_round_up_positive() {
        let e = env();
        assert_eq!(mul_div_round_up(&e, 10, 10, 3).unwrap(), 34);
    }

    #[test]
    fn mul_div_round_up_exact() {
        let e = env();
        assert_eq!(mul_div_round_up(&e, 10, 10, 5).unwrap(), 20);
    }

    #[test]
    fn mul_div_division_by_zero() {
        let e = env();
        assert_eq!(
            mul_div(&e, 10, 10, 0),
            Err(MathError::DivisionByZero)
        );
    }

    #[test]
    fn mul_div_overflow_result() {
        let e = env();
        assert!(mul_div(&e, i128::MAX, i128::MAX, 1).is_err());
    }

    #[test]
    fn mul_div_large_product() {
        let e = env();
        let a: i128 = 1_000_000_000_000_000_000;
        let b: i128 = 1_000_000_000_000_000_000;
        let result = mul_div(&e, a, b, 1_000_000_000_000_000_000i128).unwrap();
        assert_eq!(result, a);
    }

    #[test]
    fn mul_div_safety_rounds_up_for_liability() {
        let e = env();
        let result = mul_div_with_safety(&e, 10, 10, 3, true).unwrap();
        assert_eq!(result, 34);
        let result = mul_div_with_safety(&e, 10, 10, 3, false).unwrap();
        assert_eq!(result, 33);
    }

    #[test]
    fn mul_div_ceil_alias() {
        let e = env();
        assert_eq!(mul_div_ceil(&e, 10, 10, 3).unwrap(), 34);
    }

    #[test]
    fn mul_div_floor_alias() {
        let e = env();
        assert_eq!(mul_div_floor(&e, 10, 10, 3).unwrap(), 33);
    }

    #[test]
    fn boundary_min_values() {
        let e = env();
        assert!(mul_div(&e, i128::MIN + 1, 1, 1).is_ok());
    }

    #[test]
    fn boundary_negative_product() {
        let e = env();
        assert_eq!(mul_div(&e, -10, 10, 5).unwrap(), -20);
        let result = mul_div_round_up(&e, -10, 10, 3).unwrap();
        assert!(result >= -34, "round_up negative should be >= ceiling, got {result}");
    }
}

#[cfg(all(test, feature = "testutils"))]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use soroban_sdk::Env;

    proptest! {
        #[test]
        fn mul_div_consistent(
            a in -10_000i128..10_000,
            b in -10_000i128..10_000,
            d in 1i128..10_000,
        ) {
            let e = Env::default();
            let result = mul_div(&e, a, b, d).unwrap();
            let expected = (a as i128).saturating_mul(b as i128) / d;
            assert_eq!(result, expected);
        }

        #[test]
        fn mul_div_round_up_ge_floor(
            a in -1_000i128..1_000,
            b in -1_000i128..1_000,
            d in 1i128..1_000,
        ) {
            let e = Env::default();
            let floor = mul_div_floor(&e, a, b, d).unwrap();
            let ceil = mul_div_ceil(&e, a, b, d).unwrap();
            assert!(ceil >= floor, "ceil {ceil} < floor {floor}");
        }
    }
}
