//! Error types for overflow-safe math operations.

/// Errors that overflow-safe math operations can return.
///
/// Each variant corresponds to a distinct failure mode proven absent
/// in the formal SMT specifications under `formal-verification/safe-math-proofs/`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MathError {
    /// Arithmetic result exceeds i128::MAX.
    Overflow = 1,
    /// Arithmetic result is below i128::MIN (signed underflow).
    Underflow = 2,
    /// Divisor is zero.
    DivisionByZero = 3,
    /// Argument to sqrt is negative.
    NegativeSqrt = 4,
    /// Exponent too large; intermediate power would overflow.
    ExponentTooLarge = 5,
}
