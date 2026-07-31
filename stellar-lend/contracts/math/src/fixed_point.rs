use crate::error::MathError;
use crate::int128::safe_sqrt;
use soroban_sdk::{Env, I256};

pub const WAD: i128 = 1_000_000_000_000_000_000;
pub const RAY: i128 = 1_000_000_000_000_000_000_000_000_000;
pub const HALF_WAD: i128 = 500_000_000_000_000_000;
pub const SECONDS_PER_YEAR: u64 = 31_536_000;

// ── WAD operations ───────────────────────────────────────────────────────────

pub fn fp_mul(env: &Env, a: i128, b: i128) -> Result<i128, MathError> {
    let a256 = I256::from_i128(env, a);
    let b256 = I256::from_i128(env, b);
    let wad256 = I256::from_i128(env, WAD);
    let product = a256.mul(&b256);
    let result = product.div(&wad256);
    result.to_i128().ok_or(MathError::Overflow)
}

pub fn fp_div(env: &Env, a: i128, b: i128) -> Result<i128, MathError> {
    if b == 0 {
        return Err(MathError::DivisionByZero);
    }
    let a256 = I256::from_i128(env, a);
    let wad256 = I256::from_i128(env, WAD);
    let b256 = I256::from_i128(env, b);
    let numerator = a256.mul(&wad256);
    let result = numerator.div(&b256);
    result.to_i128().ok_or(MathError::Overflow)
}

#[inline]
pub fn fp_add(a: i128, b: i128) -> Result<i128, MathError> {
    crate::int128::safe_add(a, b)
}

#[inline]
pub fn fp_sub(a: i128, b: i128) -> Result<i128, MathError> {
    crate::int128::safe_sub(a, b)
}

pub fn fp_sqrt(env: &Env, a: i128) -> Result<i128, MathError> {
    if a < 0 {
        return Err(MathError::NegativeSqrt);
    }
    if a == 0 {
        return Ok(0);
    }
    let a256 = I256::from_i128(env, a);
    let wad256 = I256::from_i128(env, WAD);
    let scaled = a256.mul(&wad256);
    let scaled_i128 = scaled.to_i128();
    if let Some(s) = scaled_i128 {
        safe_sqrt(s)
    } else {
        i256_isqrt(env, scaled)
    }
}

pub fn fp_pow(env: &Env, base: i128, exp: u32) -> Result<i128, MathError> {
    if exp == 0 {
        return Ok(WAD);
    }
    let mut result = WAD;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = fp_mul(env, result, b)?;
        }
        e >>= 1;
        if e > 0 {
            b = fp_mul(env, b, b)?;
        }
    }
    Ok(result)
}

// ── RAY operations ───────────────────────────────────────────────────────────

pub fn ray_mul(env: &Env, a: i128, b: i128) -> Result<i128, MathError> {
    let a256 = I256::from_i128(env, a);
    let b256 = I256::from_i128(env, b);
    let ray256 = I256::from_i128(env, RAY);
    let product = a256.mul(&b256);
    let result = product.div(&ray256);
    result.to_i128().ok_or(MathError::Overflow)
}

pub fn ray_div(env: &Env, a: i128, b: i128) -> Result<i128, MathError> {
    if b == 0 {
        return Err(MathError::DivisionByZero);
    }
    let a256 = I256::from_i128(env, a);
    let ray256 = I256::from_i128(env, RAY);
    let b256 = I256::from_i128(env, b);
    let numerator = a256.mul(&ray256);
    let result = numerator.div(&b256);
    result.to_i128().ok_or(MathError::Overflow)
}

pub fn ray_to_wad(env: &Env, a: i128) -> Result<i128, MathError> {
    if a == 0 {
        return Ok(0);
    }
    let a256 = I256::from_i128(env, a);
    let factor = I256::from_i128(env, 10i128.pow(9));
    let result = a256.div(&factor);
    result.to_i128().ok_or(MathError::Overflow)
}

pub fn wad_to_ray(env: &Env, a: i128) -> Result<i128, MathError> {
    if a == 0 {
        return Ok(0);
    }
    let a256 = I256::from_i128(env, a);
    let factor = I256::from_i128(env, 10i128.pow(9));
    let result = a256.mul(&factor);
    result.to_i128().ok_or(MathError::Overflow)
}

// ── Interest-rate helpers ────────────────────────────────────────────────────

pub fn simple_interest(
    env: &Env,
    principal: i128,
    rate_bps: i128,
    elapsed_secs: u64,
) -> Result<i128, MathError> {
    if elapsed_secs == 0 || principal == 0 || rate_bps == 0 {
        return Ok(0);
    }
    let p256 = I256::from_i128(env, principal);
    let r256 = I256::from_i128(env, rate_bps);
    let t256 = I256::from_i128(env, elapsed_secs as i128);
    let bps256 = I256::from_i128(env, 10_000);
    let spy256 = I256::from_i128(env, SECONDS_PER_YEAR as i128);
    let result = p256.mul(&r256).mul(&t256).div(&bps256).div(&spy256);
    result.to_i128().ok_or(MathError::Overflow)
}

pub fn bps_ratio(env: &Env, numerator: i128, denominator: i128) -> Result<i128, MathError> {
    if denominator == 0 {
        return Err(MathError::DivisionByZero);
    }
    let n256 = I256::from_i128(env, numerator);
    let bps256 = I256::from_i128(env, 10_000);
    let d256 = I256::from_i128(env, denominator);
    let result = n256.mul(&bps256).div(&d256);
    result.to_i128().ok_or(MathError::Overflow)
}

fn i256_isqrt(env: &Env, n: I256) -> Result<i128, MathError> {
    let zero = I256::from_i128(env, 0);
    let two = I256::from_i128(env, 2);
    if n == zero {
        return Ok(0);
    }
    let mut x = n.div(&two);
    let mut y = x.add(&n.div(&x)).div(&two);
    while y < x {
        x = y.clone();
        y = x.add(&n.div(&x)).div(&two);
    }
    x.to_i128().ok_or(MathError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    fn env() -> Env {
        Env::default()
    }

    #[test]
    fn fp_mul_unit() {
        let e = env();
        assert_eq!(fp_mul(&e, WAD, WAD), Ok(WAD));
        let one_half = WAD + WAD / 2;
        assert_eq!(fp_mul(&e, one_half, 2 * WAD), Ok(3 * WAD));
    }

    #[test]
    fn fp_mul_zero() {
        let e = env();
        assert_eq!(fp_mul(&e, 0, WAD), Ok(0));
    }

    #[test]
    fn fp_mul_overflow_result() {
        let e = env();
        assert!(fp_mul(&e, i128::MAX, i128::MAX).is_err());
    }

    #[test]
    fn fp_div_unit() {
        let e = env();
        assert_eq!(fp_div(&e, 3 * WAD, 2 * WAD), Ok(WAD + WAD / 2));
        assert_eq!(fp_div(&e, WAD, WAD), Ok(WAD));
    }

    #[test]
    fn fp_div_by_zero() {
        let e = env();
        assert_eq!(fp_div(&e, WAD, 0), Err(MathError::DivisionByZero));
    }

    #[test]
    fn fp_sqrt_unit() {
        let e = env();
        assert_eq!(fp_sqrt(&e, WAD), Ok(WAD));
        assert_eq!(fp_sqrt(&e, 4 * WAD), Ok(2 * WAD));
        assert_eq!(fp_sqrt(&e, 0), Ok(0));
    }

    #[test]
    fn fp_sqrt_floor_property() {
        let e = env();
        let inputs: &[i128] = &[WAD, 2 * WAD, 3 * WAD, 100 * WAD, 1_000_000 * WAD];
        for &a in inputs {
            let r = fp_sqrt(&e, a).unwrap();
            let r_sq = fp_mul(&e, r, r).unwrap();
            assert!(r_sq <= a, "fp_sqrt floor fail: r²={r_sq} > a={a}");
        }
    }

    #[test]
    fn fp_sqrt_negative() {
        let e = env();
        assert_eq!(fp_sqrt(&e, -1), Err(MathError::NegativeSqrt));
    }

    #[test]
    fn fp_pow_zero_exp() {
        let e = env();
        assert_eq!(fp_pow(&e, 2 * WAD, 0), Ok(WAD));
    }

    #[test]
    fn fp_pow_square() {
        let e = env();
        assert_eq!(fp_pow(&e, 2 * WAD, 2), Ok(4 * WAD));
        let one_five = WAD + WAD / 2;
        let two_twenty_five = 2 * WAD + WAD / 4;
        assert_eq!(fp_pow(&e, one_five, 2), Ok(two_twenty_five));
    }

    #[test]
    fn fp_pow_exponent_one() {
        let e = env();
        assert_eq!(fp_pow(&e, 5 * WAD, 1), Ok(5 * WAD));
    }

    #[test]
    fn ray_mul_normal() {
        let e = env();
        assert_eq!(ray_mul(&e, RAY, RAY), Ok(RAY));
    }

    #[test]
    fn ray_div_normal() {
        let e = env();
        assert_eq!(ray_div(&e, RAY, RAY), Ok(RAY));
    }

    #[test]
    fn ray_div_by_zero() {
        let e = env();
        assert_eq!(ray_div(&e, RAY, 0), Err(MathError::DivisionByZero));
    }

    #[test]
    fn ray_wad_conversion_round_trip() {
        let e = env();
        let wad_val = WAD;
        let ray_val = wad_to_ray(&e, wad_val).unwrap();
        let back = ray_to_wad(&e, ray_val).unwrap();
        assert_eq!(wad_val, back);
    }

    #[test]
    fn ray_wad_conversion_zero() {
        let e = env();
        assert_eq!(ray_to_wad(&e, 0), Ok(0));
        assert_eq!(wad_to_ray(&e, 0), Ok(0));
    }

    #[test]
    fn simple_interest_annual() {
        let e = env();
        let interest = simple_interest(&e, 100_000, 500, SECONDS_PER_YEAR).unwrap();
        assert_eq!(interest, 5_000);
    }

    #[test]
    fn simple_interest_zero_elapsed() {
        let e = env();
        assert_eq!(simple_interest(&e, 1_000_000, 500, 0), Ok(0));
    }

    #[test]
    fn simple_interest_large_principal() {
        let e = env();
        let principal = 1_000_000_000_000_000_000_000_000_000_000i128;
        let interest = simple_interest(&e, principal, 500, SECONDS_PER_YEAR);
        assert!(interest.is_ok());
        assert!(interest.unwrap() > 10_000_000_000_000_000_000_000_000_000i128);
    }

    #[test]
    fn bps_ratio_normal() {
        let e = env();
        assert_eq!(bps_ratio(&e, 5_000, 10_000), Ok(5_000));
        assert_eq!(bps_ratio(&e, 10_000, 10_000), Ok(10_000));
    }

    #[test]
    fn bps_ratio_div_zero() {
        let e = env();
        assert_eq!(bps_ratio(&e, 1_000, 0), Err(MathError::DivisionByZero));
    }

    #[test]
    fn fp_sqrt_precision_boundary() {
        let e = env();
        let just_under = WAD - 1;
        let result = fp_sqrt(&e, just_under).unwrap();
        assert!(result <= WAD);
    }
}
