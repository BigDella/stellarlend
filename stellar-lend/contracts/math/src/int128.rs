use crate::error::MathError;

// ── Safe (checked) operations ────────────────────────────────────────────────

#[inline]
pub fn safe_add(a: i128, b: i128) -> Result<i128, MathError> {
    a.checked_add(b).ok_or(if b > 0 {
        MathError::Overflow
    } else {
        MathError::Underflow
    })
}

#[inline]
pub fn safe_sub(a: i128, b: i128) -> Result<i128, MathError> {
    a.checked_sub(b).ok_or(if b > 0 {
        MathError::Underflow
    } else {
        MathError::Overflow
    })
}

#[inline]
pub fn safe_mul(a: i128, b: i128) -> Result<i128, MathError> {
    a.checked_mul(b).ok_or_else(|| {
        if (a > 0 && b > 0) || (a < 0 && b < 0) {
            MathError::Overflow
        } else {
            MathError::Underflow
        }
    })
}

#[inline]
pub fn safe_div(a: i128, b: i128) -> Result<i128, MathError> {
    if b == 0 {
        return Err(MathError::DivisionByZero);
    }
    a.checked_div(b).ok_or(MathError::Overflow)
}

pub fn safe_pow(base: i128, exp: u32) -> Result<i128, MathError> {
    if exp == 0 {
        return Ok(1);
    }
    let mut result: i128 = 1;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = safe_mul(result, b)?;
        }
        e >>= 1;
        if e > 0 {
            b = safe_mul(b, b)?;
        }
    }
    Ok(result)
}

pub fn safe_sqrt(n: i128) -> Result<i128, MathError> {
    if n < 0 {
        return Err(MathError::NegativeSqrt);
    }
    if n < 4 {
        return Ok(if n == 0 { 0 } else { 1 });
    }
    let mut x = n / 2;
    loop {
        let next = (x + n / x) / 2;
        if next >= x {
            break;
        }
        x = next;
    }
    Ok(x)
}

// ── Unchecked helpers (gas-optimized, caller must guarantee safety) ──────────

#[inline]
pub fn unchecked_add(a: i128, b: i128) -> i128 {
    a + b
}

#[inline]
pub fn unchecked_sub(a: i128, b: i128) -> i128 {
    a - b
}

#[inline]
pub fn unchecked_mul(a: i128, b: i128) -> i128 {
    a * b
}

#[inline]
pub fn unchecked_div(a: i128, b: i128) -> i128 {
    a / b
}

// ── Unsigned variants (u128) ─────────────────────────────────────────────────

#[inline]
pub fn safe_add_u128(a: u128, b: u128) -> Result<u128, MathError> {
    a.checked_add(b).ok_or(MathError::Overflow)
}

#[inline]
pub fn safe_sub_u128(a: u128, b: u128) -> Result<u128, MathError> {
    a.checked_sub(b).ok_or(MathError::Underflow)
}

#[inline]
pub fn safe_mul_u128(a: u128, b: u128) -> Result<u128, MathError> {
    a.checked_mul(b).ok_or(MathError::Overflow)
}

#[inline]
pub fn safe_div_u128(a: u128, b: u128) -> Result<u128, MathError> {
    if b == 0 {
        return Err(MathError::DivisionByZero);
    }
    Ok(a / b)
}

pub fn safe_sqrt_u128(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) >> 1;
    while y < x {
        x = y;
        y = (x + n / x) >> 1;
    }
    x
}

#[inline]
pub fn unchecked_add_u128(a: u128, b: u128) -> u128 {
    a + b
}

#[inline]
pub fn unchecked_sub_u128(a: u128, b: u128) -> u128 {
    a - b
}

#[inline]
pub fn unchecked_mul_u128(a: u128, b: u128) -> u128 {
    a * b
}

// ── Basis-point helpers ──────────────────────────────────────────────────────

#[inline]
pub fn bps_mul(amount: i128, bps: i128) -> Result<i128, MathError> {
    safe_mul(amount, bps).and_then(|v| safe_div(v, 10_000))
}

#[inline]
pub fn bps_mul_u128(amount: u128, bps: u128) -> Result<u128, MathError> {
    safe_mul_u128(amount, bps).and_then(|v| safe_div_u128(v, 10_000))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_normal() {
        assert_eq!(safe_add(3, 4), Ok(7));
        assert_eq!(safe_add(-3, 4), Ok(1));
        assert_eq!(safe_add(0, i128::MAX), Ok(i128::MAX));
    }

    #[test]
    fn add_overflow() {
        assert_eq!(safe_add(i128::MAX, 1), Err(MathError::Overflow));
    }

    #[test]
    fn add_underflow() {
        assert_eq!(safe_add(i128::MIN, -1), Err(MathError::Underflow));
    }

    #[test]
    fn sub_normal() {
        assert_eq!(safe_sub(10, 3), Ok(7));
        assert_eq!(safe_sub(i128::MIN, 0), Ok(i128::MIN));
    }

    #[test]
    fn sub_underflow() {
        assert_eq!(safe_sub(i128::MIN, 1), Err(MathError::Underflow));
    }

    #[test]
    fn sub_overflow() {
        assert_eq!(safe_sub(i128::MAX, -1), Err(MathError::Overflow));
    }

    #[test]
    fn mul_normal() {
        assert_eq!(safe_mul(6, 7), Ok(42));
        assert_eq!(safe_mul(-3, 4), Ok(-12));
        assert_eq!(safe_mul(0, i128::MAX), Ok(0));
    }

    #[test]
    fn mul_overflow() {
        assert_eq!(safe_mul(i128::MAX, 2), Err(MathError::Overflow));
        assert_eq!(safe_mul(i128::MIN, -1), Err(MathError::Overflow));
    }

    #[test]
    fn mul_underflow() {
        assert_eq!(safe_mul(i128::MAX, -2), Err(MathError::Underflow));
    }

    #[test]
    fn div_normal() {
        assert_eq!(safe_div(10, 3), Ok(3));
        assert_eq!(safe_div(-10, 3), Ok(-3));
    }

    #[test]
    fn div_by_zero() {
        assert_eq!(safe_div(42, 0), Err(MathError::DivisionByZero));
        assert_eq!(safe_div(0, 0), Err(MathError::DivisionByZero));
    }

    #[test]
    fn div_min_neg_one_overflow() {
        assert_eq!(safe_div(i128::MIN, -1), Err(MathError::Overflow));
    }

    #[test]
    fn pow_zero_exp() {
        assert_eq!(safe_pow(0, 0), Ok(1));
        assert_eq!(safe_pow(i128::MAX, 0), Ok(1));
    }

    #[test]
    fn pow_normal() {
        assert_eq!(safe_pow(2, 10), Ok(1024));
        assert_eq!(safe_pow(-2, 3), Ok(-8));
    }

    #[test]
    fn pow_overflow() {
        assert!(safe_pow(2, 127).is_err());
    }

    #[test]
    fn sqrt_normal() {
        assert_eq!(safe_sqrt(0), Ok(0));
        assert_eq!(safe_sqrt(1), Ok(1));
        assert_eq!(safe_sqrt(4), Ok(2));
        assert_eq!(safe_sqrt(16), Ok(4));
        assert_eq!(safe_sqrt(2), Ok(1));
    }

    #[test]
    fn sqrt_negative() {
        assert_eq!(safe_sqrt(-1), Err(MathError::NegativeSqrt));
    }

    #[test]
    fn sqrt_large() {
        let r = safe_sqrt(i128::MAX).unwrap();
        assert!((r as u128).saturating_mul(r as u128) <= i128::MAX as u128);
    }

    #[test]
    fn bps_mul_normal() {
        assert_eq!(bps_mul(1_000_000, 100), Ok(10_000));
        assert_eq!(bps_mul(10_000, 10_000), Ok(10_000));
        assert_eq!(bps_mul(0, 9999), Ok(0));
    }

    #[test]
    fn bps_mul_overflow() {
        assert!(bps_mul(i128::MAX, 2).is_err());
    }

    #[test]
    fn unchecked_helpers_smoke() {
        assert_eq!(unchecked_add(3, 4), 7);
        assert_eq!(unchecked_sub(10, 3), 7);
        assert_eq!(unchecked_mul(6, 7), 42);
        assert_eq!(unchecked_div(10, 3), 3);
    }

    #[test]
    fn property_add_matches_checked() {
        let samples: &[i128] = &[
            0, 1, -1, i128::MAX, i128::MIN, i128::MAX / 2, i128::MIN / 2, 42, -42,
        ];
        for &a in samples {
            for &b in samples {
                let expected = a.checked_add(b);
                assert_eq!(safe_add(a, b).ok(), expected, "safe_add({a}, {b})");
            }
        }
    }

    #[test]
    fn property_mul_matches_checked() {
        let samples: &[i128] = &[
            0, 1, -1, 2, -2, i128::MAX, i128::MIN, i128::MAX / 2, i128::MIN / 2,
        ];
        for &a in samples {
            for &b in samples {
                let expected = a.checked_mul(b);
                assert_eq!(safe_mul(a, b).ok(), expected, "safe_mul({a}, {b})");
            }
        }
    }
}

#[cfg(all(test, feature = "testutils"))]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn safe_add_matches_checked(a: i128, b: i128) {
            assert_eq!(safe_add(a, b).ok(), a.checked_add(b));
        }

        #[test]
        fn safe_mul_matches_checked(a: i128, b: i128) {
            assert_eq!(safe_mul(a, b).ok(), a.checked_mul(b));
        }

        #[test]
        fn safe_sub_matches_checked(a: i128, b: i128) {
            assert_eq!(safe_sub(a, b).ok(), a.checked_sub(b));
        }
    }
}
