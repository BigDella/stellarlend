//! Fixed-point decimal arithmetic with 18-decimal precision (WAD scale).
//!
//! # Notation
//!
//! A *WAD-scaled* integer `x` represents the real value `x / WAD` where
//! `WAD = 10^18`. For example, `1.5` is stored as `1_500_000_000_000_000_000`.
//!
//! # Overflow Strategy
//!
//! Multiplication and division both pass through `soroban_sdk::I256` to
//! accommodate intermediate products that exceed `i128::MAX`.  The final
//! result is then range-checked and narrowed back to `i128`.
//!
//! # Mathematical Guarantees
//!
//! For all `a, b ∈ [i128::MIN, i128::MAX]`:
//!
//! * `fp_mul(a, b)  = Ok(r)` ⟹ `r = round_down(a × b / WAD)` and `r ∈ [i128::MIN, i128::MAX]`
//! * `fp_div(a, b)  = Ok(r)` ⟹ `r = round_down(a × WAD / b)` and `r ∈ [i128::MIN, i128::MAX]`
//! * `fp_sqrt(a)    = Ok(r)` ⟹ `r² ≤ a × WAD` and `r = ⌊√(a × WAD)⌋`
//!
//! These properties are specified in `formal-verification/safe-math-proofs/safe_math.smt2`.

use crate::error::MathError;
use crate::int128::safe_sqrt;
use soroban_sdk::{Env, I256};

/// 10^18 — the WAD scaling factor for 18-decimal fixed-point numbers.
pub const WAD: i128 = 1_000_000_000_000_000_000;

/// 10^9 — the RAY scaling factor for 27-decimal fixed-point numbers.
pub const RAY: i128 = 1_000_000_000_000_000_000_000_000_000;

/// Seconds per year (365 days), used in interest rate calculations.
pub const SECONDS_PER_YEAR: u64 = 31_536_000;

// ── Core WAD operations ──────────────────────────────────────────────────────

/// Fixed-point multiply: `a × b / WAD`.
///
/// Uses I256 for the intermediate product to prevent overflow when
/// `a × b > i128::MAX`. Returns `Err(Overflow)` if the final
/// result does not fit in `i128`.
///
/// **Formula:** r = ⌊(a × b) / 10^18⌋
pub fn fp_mul(env: &Env, a: i128, b: i128) -> Result<i128, MathError> {
    let a256 = I256::from_i128(env, a);
    let b256 = I256::from_i128(env, b);
    let wad256 = I256::from_i128(env, WAD);

    let product = a256.mul(&b256);
    let result = product.div(&wad256);

    result.to_i128().ok_or(MathError::Overflow)
}

/// Fixed-point divide: `a × WAD / b`.
///
/// Uses I256 for the intermediate product `a × WAD`. Returns
/// `Err(DivisionByZero)` when `b = 0`, or `Err(Overflow)` when the
/// result does not fit in `i128`.
///
/// **Formula:** r = ⌊(a × 10^18) / b⌋
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

/// Fixed-point add: `a + b` (WAD-scale preserving).
///
/// Delegates to `safe_add` — addition does not change scale.
///
/// **Formula:** r = a + b
#[inline]
pub fn fp_add(a: i128, b: i128) -> Result<i128, MathError> {
    crate::int128::safe_add(a, b)
}

/// Fixed-point subtract: `a - b` (WAD-scale preserving).
///
/// Delegates to `safe_sub` — subtraction does not change scale.
///
/// **Formula:** r = a - b
#[inline]
pub fn fp_sub(a: i128, b: i128) -> Result<i128, MathError> {
    crate::int128::safe_sub(a, b)
}

/// Fixed-point square root: `⌊√(a × WAD)⌋`, returning a WAD-scaled result.
///
/// For a WAD-scaled input `a` representing real value `A = a / WAD`,
/// this computes `⌊√A × WAD⌋ = ⌊√(a × WAD)⌋`.
///
/// Uses I256 for the intermediate `a × WAD` to prevent overflow.
/// Returns `Err(NegativeSqrt)` when `a < 0`.
///
/// **Formula:** r = ⌊√(a / WAD)⌋ × WAD = ⌊√(a × WAD)⌋
pub fn fp_sqrt(env: &Env, a: i128) -> Result<i128, MathError> {
    if a < 0 {
        return Err(MathError::NegativeSqrt);
    }
    if a == 0 {
        return Ok(0);
    }
    // Scale up to preserve precision through the sqrt.
    let a256 = I256::from_i128(env, a);
    let wad256 = I256::from_i128(env, WAD);
    let scaled = a256.mul(&wad256);

    // Narrow back to i128 for the integer sqrt.  The product a × WAD may be
    // up to (i128::MAX × 10^18) ≈ 10^56, which overflows i128 but fits I256.
    // We take the sqrt on the I256 value using Newton's method.
    let scaled_i128 = scaled.to_i128();
    if let Some(s) = scaled_i128 {
        // Fast path: fits in i128.
        safe_sqrt(s)
    } else {
        // Slow path: Newton's method on I256 magnitude.
        i256_isqrt(env, scaled)
    }
}

/// Fixed-point power: `base^exp` with WAD scale adjusted per exponent.
///
/// Each multiplication divides out one WAD to keep the result in WAD units.
/// **Formula:** r = base^exp (both input and output in WAD scale)
///
/// - exp = 0 → returns WAD (representing 1.0)
/// - Uses I256 intermediates for each step
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

// ── Interest-rate helpers ────────────────────────────────────────────────────

/// Compute simple interest: `principal × rate_bps × elapsed / (BPS × SECONDS_PER_YEAR)`.
///
/// All intermediate products are checked via I256 to prevent overflow on
/// large principal values (common in lending pools).
///
/// **Formula:** interest = principal × rate_bps × Δt / (10 000 × 31 536 000)
///
/// Proof reference: `kani_proof_simple_interest_no_overflow` in safe-math-proofs.
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

/// Compute a BPS-scaled ratio using I256 to prevent overflow on large inputs.
///
/// **Formula:** r = numerator × 10 000 / denominator
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

// ── Internal helpers ─────────────────────────────────────────────────────────

/// Integer square root of an I256 value using Newton's method.
///
/// Used when `a × WAD` overflows i128 but fits I256.  The result is
/// guaranteed to fit in i128 for all valid fixed-point inputs.
fn i256_isqrt(env: &Env, n: I256) -> Result<i128, MathError> {
    let zero = I256::from_i128(env, 0);
    let two = I256::from_i128(env, 2);

    if n == zero {
        return Ok(0);
    }

    // Initial estimate: n / 2.
    let mut x = n.div(&two);
    let mut y = x.add(&n.div(&x)).div(&two);

    // Newton convergence.
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

    // ── fp_mul ──────────────────────────────────────────────────────────────

    #[test]
    fn fp_mul_unit() {
        let e = env();
        // 1.0 * 1.0 = 1.0
        assert_eq!(fp_mul(&e, WAD, WAD), Ok(WAD));
        // 1.5 * 2.0 = 3.0
        let one_half = WAD + WAD / 2;
        assert_eq!(fp_mul(&e, one_half, 2 * WAD), Ok(3 * WAD));
    }

    #[test]
    fn fp_mul_zero() {
        let e = env();
        assert_eq!(fp_mul(&e, 0, WAD), Ok(0));
        assert_eq!(fp_mul(&e, WAD, 0), Ok(0));
    }

    #[test]
    fn fp_mul_large_no_overflow() {
        let e = env();
        // 10^9 ETH in WAD = 10^9 * 10^18 = 10^27; squared = 10^54 / WAD = 10^36 (fits i128).
        let amount = 1_000_000_000i128 * WAD; // 10^27
        let result = fp_mul(&e, amount, WAD); // multiply by 1.0
        assert_eq!(result, Ok(amount));
    }

    #[test]
    fn fp_mul_overflow_result() {
        let e = env();
        // i128::MAX * i128::MAX / WAD overflows i128.
        assert!(fp_mul(&e, i128::MAX, i128::MAX).is_err());
    }

    // ── fp_div ──────────────────────────────────────────────────────────────

    #[test]
    fn fp_div_unit() {
        let e = env();
        // 3.0 / 2.0 = 1.5
        assert_eq!(fp_div(&e, 3 * WAD, 2 * WAD), Ok(WAD + WAD / 2));
        // a / 1.0 = a
        assert_eq!(fp_div(&e, WAD, WAD), Ok(WAD));
    }

    #[test]
    fn fp_div_by_zero() {
        let e = env();
        assert_eq!(fp_div(&e, WAD, 0), Err(MathError::DivisionByZero));
    }

    // ── fp_sqrt ─────────────────────────────────────────────────────────────

    #[test]
    fn fp_sqrt_unit() {
        let e = env();
        // sqrt(1.0) = 1.0
        assert_eq!(fp_sqrt(&e, WAD), Ok(WAD));
        // sqrt(4.0) = 2.0
        assert_eq!(fp_sqrt(&e, 4 * WAD), Ok(2 * WAD));
        // sqrt(0) = 0
        assert_eq!(fp_sqrt(&e, 0), Ok(0));
    }

    #[test]
    fn fp_sqrt_floor_property() {
        let e = env();
        let inputs: &[i128] = &[WAD, 2 * WAD, 3 * WAD, 100 * WAD, 1_000_000 * WAD];
        for &a in inputs {
            let r = fp_sqrt(&e, a).unwrap();
            // r² / WAD ≤ a  (r is WAD-scaled, so r * r / WAD = real r² in WAD)
            let r_sq = fp_mul(&e, r, r).unwrap();
            assert!(r_sq <= a, "fp_sqrt floor fail: r²={r_sq} > a={a}");
        }
    }

    #[test]
    fn fp_sqrt_negative() {
        let e = env();
        assert_eq!(fp_sqrt(&e, -1), Err(MathError::NegativeSqrt));
    }

    // ── fp_pow ──────────────────────────────────────────────────────────────

    #[test]
    fn fp_pow_zero_exp() {
        let e = env();
        assert_eq!(fp_pow(&e, 2 * WAD, 0), Ok(WAD)); // any^0 = 1.0
    }

    #[test]
    fn fp_pow_square() {
        let e = env();
        // 2.0^2 = 4.0
        assert_eq!(fp_pow(&e, 2 * WAD, 2), Ok(4 * WAD));
        // 1.5^2 = 2.25
        let one_five = WAD + WAD / 2;
        let two_twenty_five = 2 * WAD + WAD / 4;
        assert_eq!(fp_pow(&e, one_five, 2), Ok(two_twenty_five));
    }

    // ── simple_interest ─────────────────────────────────────────────────────

    #[test]
    fn simple_interest_annual() {
        let e = env();
        // 100_000 at 5% APR for 1 year = 5_000
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
        // 10^30 (large pool), 5% APR, 1 year — tests I256 path.
        let principal = 1_000_000_000_000_000_000_000_000_000_000i128; // 10^30
        let interest = simple_interest(&e, principal, 500, SECONDS_PER_YEAR);
        assert!(interest.is_ok());
        let i = interest.unwrap();
        assert!(i > 10_000_000_000_000_000_000_000_000_000i128); // > 10^28
    }

    // ── bps_ratio ───────────────────────────────────────────────────────────

    #[test]
    fn bps_ratio_normal() {
        let e = env();
        // 50% utilization
        assert_eq!(bps_ratio(&e, 5_000, 10_000), Ok(5_000));
        // 100%
        assert_eq!(bps_ratio(&e, 10_000, 10_000), Ok(10_000));
    }

    #[test]
    fn bps_ratio_div_zero() {
        let e = env();
        assert_eq!(bps_ratio(&e, 1_000, 0), Err(MathError::DivisionByZero));
    }
}
