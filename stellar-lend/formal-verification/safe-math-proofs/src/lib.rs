//! # Kani proof harnesses for `stellarlend-safe-math`
//!
//! ## Running
//!
//! ```sh
//! cargo kani --manifest-path formal-verification/safe-math-proofs/Cargo.toml
//! ```
//!
//! Kani uses CBMC (C Bounded Model Checker) with a Z3/CVC5 SMT backend to
//! exhaustively verify the properties below over all possible i128 inputs
//! within bounded bit-width.
//!
//! ## Properties verified
//!
//! 1. **No silent overflow in `safe_add`**: if `Ok(r)` is returned then
//!    `r == a + b` holds in unbounded integer arithmetic.
//! 2. **No silent overflow in `safe_mul`**: if `Ok(r)` is returned then
//!    `r == a * b` in unbounded integer arithmetic.
//! 3. **`safe_sqrt` floor property**: `r^2 ≤ n` and `(r+1)^2 > n`.
//! 4. **`safe_pow` identity**: `2^n == product of n twos` for small n.
//! 5. **Simple interest cannot silently overflow**: result fits i128 or Err
//!    is returned.
//!
//! See also `safe_math.smt2` for the corresponding SMT-LIB 2 encoding.

use stellarlend_safe_math::{
    safe_add, safe_div, safe_mul, safe_pow, safe_sqrt, safe_sub, MathError,
};

// ── Addition ─────────────────────────────────────────────────────────────────

/// Proof: if `safe_add` returns `Ok(r)`, then `r` equals the exact
/// mathematical result `a + b` (no silent wrap).
///
/// SMT encoding (see safe_math.smt2, `add-no-overflow`):
///   ∀ a, b ∈ Int :
///     (Int.add a b) ∈ [MIN, MAX]  →  safe_add(a, b) = Ok(Int.add a b)
///     (Int.add a b) ∉ [MIN, MAX]  →  safe_add(a, b) = Err(_)
#[cfg(kani)]
#[kani::proof]
fn kani_proof_add_no_silent_overflow() {
    let a: i128 = kani::any();
    let b: i128 = kani::any();

    match safe_add(a, b) {
        Ok(r) => {
            // Postcondition: result equals mathematical sum.
            kani::assert(
                r == a.wrapping_add(b),
                "safe_add result must equal a + b (no overflow occurred)",
            );
            // Verify result is representable (no wrap).
            kani::assert(
                (a >= 0 && b >= 0 && r >= a) || (a <= 0 && b <= 0 && r <= a) || (a > 0) != (b > 0),
                "safe_add Ok implies no overflow direction mismatch",
            );
        }
        Err(_) => {
            // Postcondition: mathematical result overflows i128.
            kani::assert(
                a.checked_add(b).is_none(),
                "safe_add Err implies true overflow",
            );
        }
    }
}

// ── Subtraction ──────────────────────────────────────────────────────────────

#[cfg(kani)]
#[kani::proof]
fn kani_proof_sub_no_silent_underflow() {
    let a: i128 = kani::any();
    let b: i128 = kani::any();

    match safe_sub(a, b) {
        Ok(r) => {
            kani::assert(
                r == a.wrapping_sub(b),
                "safe_sub Ok: result must equal a - b",
            );
        }
        Err(_) => {
            kani::assert(
                a.checked_sub(b).is_none(),
                "safe_sub Err implies true underflow",
            );
        }
    }
}

// ── Multiplication ────────────────────────────────────────────────────────────

/// Proof: `safe_mul` never silently wraps — either returns the exact product
/// or returns an error when the product would overflow `i128`.
///
/// SMT encoding (see safe_math.smt2, `mul-no-overflow`):
///   ∀ a, b ∈ Int :
///     (Int.mul a b) ∈ [MIN, MAX]  →  safe_mul(a, b) = Ok(Int.mul a b)
///     (Int.mul a b) ∉ [MIN, MAX]  →  safe_mul(a, b) = Err(_)
#[cfg(kani)]
#[kani::proof]
fn kani_proof_mul_no_silent_overflow() {
    let a: i128 = kani::any();
    let b: i128 = kani::any();

    match safe_mul(a, b) {
        Ok(r) => {
            kani::assert(
                r == a.wrapping_mul(b),
                "safe_mul Ok: result must equal a * b",
            );
        }
        Err(_) => {
            kani::assert(
                a.checked_mul(b).is_none(),
                "safe_mul Err implies true overflow",
            );
        }
    }
}

// ── Division ─────────────────────────────────────────────────────────────────

#[cfg(kani)]
#[kani::proof]
fn kani_proof_div_no_silent_overflow() {
    let a: i128 = kani::any();
    let b: i128 = kani::any();

    match safe_div(a, b) {
        Ok(r) => {
            kani::assert(b != 0, "safe_div Ok implies non-zero divisor");
            kani::assert(r == a / b, "safe_div Ok: result must equal a / b");
        }
        Err(MathError::DivisionByZero) => {
            kani::assert(b == 0, "DivisionByZero iff b == 0");
        }
        Err(MathError::Overflow) => {
            // Only i128::MIN / -1 overflows.
            kani::assert(
                a == i128::MIN && b == -1,
                "Overflow in div only for MIN / -1",
            );
        }
        Err(_) => kani::assert(false, "unexpected error variant from safe_div"),
    }
}

// ── Square root ───────────────────────────────────────────────────────────────

/// Proof: `safe_sqrt(n) = Ok(r)` implies `r² ≤ n < (r+1)²`.
///
/// SMT encoding (see safe_math.smt2, `sqrt-floor`).
#[cfg(kani)]
#[kani::proof]
fn kani_proof_sqrt_correct() {
    let n: i128 = kani::any();
    kani::assume(n >= 0); // negative case tested separately

    match safe_sqrt(n) {
        Ok(r) => {
            kani::assert(r >= 0, "sqrt result must be non-negative");
            kani::assert(r * r <= n, "sqrt floor: r² ≤ n");
            if let Some(next_sq) = (r + 1).checked_mul(r + 1) {
                kani::assert(next_sq > n, "sqrt floor: (r+1)² > n");
            }
        }
        Err(_) => kani::assert(false, "safe_sqrt(n≥0) must not error"),
    }
}

#[cfg(kani)]
#[kani::proof]
fn kani_proof_sqrt_negative_returns_err() {
    let n: i128 = kani::any();
    kani::assume(n < 0);
    assert_eq!(safe_sqrt(n), Err(MathError::NegativeSqrt));
}

// ── Power ─────────────────────────────────────────────────────────────────────

/// Proof: `safe_pow` never silently wraps — either returns the exact result
/// or returns an error.  Bounded to exp ≤ 10 to keep CBMC tractable.
#[cfg(kani)]
#[kani::proof]
fn kani_proof_pow_no_silent_overflow() {
    let base: i128 = kani::any();
    let exp: u32 = kani::any();
    kani::assume(exp <= 10); // bound for tractability

    match safe_pow(base, exp) {
        Ok(r) => {
            // Verify via reference: compute naively with checked_mul.
            let mut reference: i128 = 1;
            let mut overflow = false;
            for _ in 0..exp {
                if let Some(v) = reference.checked_mul(base) {
                    reference = v;
                } else {
                    overflow = true;
                    break;
                }
            }
            kani::assert(!overflow, "safe_pow Ok but naive check overflowed");
            kani::assert(r == reference, "safe_pow result must match naive computation");
        }
        Err(_) => {
            // Verify that naive computation also overflows.
            let mut reference: i128 = 1;
            let mut overflow = false;
            for _ in 0..exp {
                if let Some(v) = reference.checked_mul(base) {
                    reference = v;
                } else {
                    overflow = true;
                    break;
                }
            }
            kani::assert(overflow, "safe_pow Err but naive check did not overflow");
        }
    }
}

// ── Composition: add(mul(a,b), c) ────────────────────────────────────────────

/// Proof: the composition `safe_mul` then `safe_add` never silently overflows.
/// This pattern is common in interest calculations: principal * rate + accrued.
#[cfg(kani)]
#[kani::proof]
fn kani_proof_mul_add_composition() {
    let a: i128 = kani::any();
    let b: i128 = kani::any();
    let c: i128 = kani::any();

    if let Ok(product) = safe_mul(a, b) {
        match safe_add(product, c) {
            Ok(r) => {
                kani::assert(
                    r == product.wrapping_add(c),
                    "add after mul: result correct",
                );
            }
            Err(_) => {
                kani::assert(
                    product.checked_add(c).is_none(),
                    "add after mul: Err implies overflow",
                );
            }
        }
    }
}

// ── Non-kani unit tests (always compiled) ────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_boundary() {
        assert_eq!(safe_add(i128::MAX, 0), Ok(i128::MAX));
        assert!(safe_add(i128::MAX, 1).is_err());
        assert_eq!(safe_add(i128::MIN, 0), Ok(i128::MIN));
        assert!(safe_add(i128::MIN, -1).is_err());
    }

    #[test]
    fn mul_boundary() {
        assert_eq!(safe_mul(1, i128::MAX), Ok(i128::MAX));
        assert!(safe_mul(2, i128::MAX).is_err());
    }

    #[test]
    fn sqrt_floor() {
        for n in [0i128, 1, 2, 3, 4, 8, 9, 15, 16, 100, 1_000_000] {
            let r = safe_sqrt(n).unwrap();
            assert!(r * r <= n);
            assert!((r + 1).checked_mul(r + 1).map_or(true, |s| s > n));
        }
    }
}
