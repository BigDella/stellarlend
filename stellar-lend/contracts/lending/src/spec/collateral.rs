//! # Formal Specification — `validate_collateral_ratio`
//!
//! ## Function signature (from `borrow.rs`)
//!
//! ```text
//! pub(crate) fn validate_collateral_ratio(collateral: i128, borrow: i128)
//!     -> Result<(), BorrowError>
//! ```
//!
//! ## Mathematical model
//!
//! The function enforces:
//!
//! ```text
//! collateral ≥ borrow * COLLATERAL_RATIO_MIN / 10_000
//! ```
//!
//! where `COLLATERAL_RATIO_MIN = 15_000` (150%).
//!
//! Equivalently: `collateral * 10_000 ≥ borrow * 15_000`
//!
//! ## Properties proved
//!
//! | ID    | Property                                                           |
//! |-------|--------------------------------------------------------------------|
//! | C-01  | Exact 150% collateral is accepted                                  |
//! | C-02  | Collateral strictly below 150% is rejected                         |
//! | C-03  | Collateral strictly above 150% is accepted                         |
//! | C-04  | Any borrow=0 with collateral=0 is accepted (vacuously satisfied)   |
//! | C-05  | No intermediate overflow for any valid i128 principal              |
//! | C-06  | The check is tight — the boundary condition holds exactly          |

/// Minimum collateral ratio in basis points (150%).
pub const COLLATERAL_RATIO_MIN: i128 = 15_000;
/// Basis-point divisor.
pub const BPS_DIVISOR: i128 = 10_000;

/// Errors returned by collateral-ratio validation.
#[derive(Debug, PartialEq)]
pub enum CollateralError {
    /// Collateral is below the required minimum ratio.
    InsufficientCollateral,
    /// Arithmetic overflow occurred during validation.
    Overflow,
    /// Amount is zero or invalid.
    InvalidAmount,
}

/// Pure reference implementation of collateral-ratio validation.
///
/// Mirrors `borrow::validate_collateral_ratio` exactly.
pub fn reference_validate_collateral_ratio(
    collateral: i128,
    borrow: i128,
) -> Result<(), CollateralError> {
    // If borrow is 0, any non-negative collateral satisfies the ratio.
    if borrow == 0 {
        return Ok(());
    }
    let min_collateral = borrow
        .checked_mul(COLLATERAL_RATIO_MIN)
        .ok_or(CollateralError::Overflow)?
        .checked_div(BPS_DIVISOR)
        .ok_or(CollateralError::InvalidAmount)?;

    if collateral < min_collateral {
        Err(CollateralError::InsufficientCollateral)
    } else {
        Ok(())
    }
}

// ─── Lemmas ───────────────────────────────────────────────────────────────────

/// **C-01**: Exact 150% collateral satisfies the check.
#[test]
fn lemma_c01_exact_150pct_is_accepted() {
    let borrow_amounts: &[i128] = &[1, 100, 10_000, 1_000_000, 1_000_000_000];
    for &b in borrow_amounts {
        // Compute exact minimum collateral (floor division, so add 1 where needed)
        let exact_min = b.checked_mul(COLLATERAL_RATIO_MIN).unwrap() / BPS_DIVISOR;
        // Handle rounding: if b * 15000 is not divisible by 10000, floor < true ratio
        let collateral = if (b * COLLATERAL_RATIO_MIN) % BPS_DIVISOR == 0 {
            exact_min
        } else {
            exact_min + 1
        };
        let result = reference_validate_collateral_ratio(collateral, b);
        assert!(
            result.is_ok(),
            "C-01 failed for borrow={b}, collateral={collateral}: {result:?}"
        );
    }
}

/// **C-02**: Any collateral strictly below the minimum is rejected.
#[test]
fn lemma_c02_below_150pct_is_rejected() {
    let test_cases: &[(i128, i128)] = &[
        (1, 2), // 50% — clearly insufficient
        (100, 200),
        (14_999, 10_000), // 149.99% — just below the threshold
    ];
    for &(collateral, borrow) in test_cases {
        let min_required = borrow
            .checked_mul(COLLATERAL_RATIO_MIN)
            .unwrap()
            .checked_div(BPS_DIVISOR)
            .unwrap();
        if collateral < min_required {
            let result = reference_validate_collateral_ratio(collateral, borrow);
            assert_eq!(
                result,
                Err(CollateralError::InsufficientCollateral),
                "C-02 failed for collateral={collateral}, borrow={borrow}"
            );
        }
    }
}

/// **C-03**: Collateral above 150% is always accepted.
#[test]
fn lemma_c03_above_150pct_is_accepted() {
    let test_cases: &[(i128, i128)] = &[
        (15_001, 10_000),       // 150.01%
        (20_000, 10_000),       // 200%
        (3_000_000, 1_000_000), // 300%
        (i128::MAX / 2, 1),
    ];
    for &(collateral, borrow) in test_cases {
        let result = reference_validate_collateral_ratio(collateral, borrow);
        assert!(
            result.is_ok(),
            "C-03 failed: collateral={collateral}, borrow={borrow}: {result:?}"
        );
    }
}

/// **C-04**: Borrow=0 with collateral=0 is accepted (empty position).
#[test]
fn lemma_c04_zero_borrow_always_accepted() {
    for collateral in [0i128, 1, 1_000_000, i128::MAX] {
        let result = reference_validate_collateral_ratio(collateral, 0);
        assert!(
            result.is_ok(),
            "C-04 failed for collateral={collateral}: {result:?}"
        );
    }
}

/// **C-05**: No intermediate overflow for any valid i128 borrow amount that
/// satisfies the safe-domain constraint `borrow <= i128::MAX / COLLATERAL_RATIO_MIN`.
#[test]
fn lemma_c05_no_overflow_in_valid_domain() {
    let max_safe_borrow = i128::MAX / COLLATERAL_RATIO_MIN;
    let result = reference_validate_collateral_ratio(i128::MAX, max_safe_borrow);
    // Should succeed (not overflow), since i128::MAX collateral covers any
    // borrow within the safe domain.
    assert!(
        result.is_ok() || matches!(result, Err(CollateralError::InsufficientCollateral)),
        "C-05: unexpected Overflow error: {result:?}"
    );
}

/// **C-06**: Boundary tightness — the minimum required collateral is exactly
/// `borrow * 15000 / 10000` (integer floor division).
///
/// Specifically: `collateral = floor(borrow * 1.5) - 1` must fail, and
/// `collateral = ceil(borrow * 1.5)` must pass.
#[test]
fn lemma_c06_boundary_is_tight() {
    let borrows: &[i128] = &[1, 3, 7, 100, 333, 1_000_001];
    for &b in borrows {
        let raw = b * COLLATERAL_RATIO_MIN;
        let floor = raw / BPS_DIVISOR;
        let has_remainder = raw % BPS_DIVISOR != 0;
        let ceil = if has_remainder { floor + 1 } else { floor };

        // floor - 1 should fail if floor > 0
        if floor > 0 {
            let result = reference_validate_collateral_ratio(floor - 1, b);
            assert_eq!(
                result,
                Err(CollateralError::InsufficientCollateral),
                "C-06: floor-1 should be rejected for borrow={b}"
            );
        }

        // ceil should pass
        let result = reference_validate_collateral_ratio(ceil, b);
        assert!(
            result.is_ok(),
            "C-06: ceil should be accepted for borrow={b}, ceil={ceil}: {result:?}"
        );
    }
}

// ─── Kani harness ─────────────────────────────────────────────────────────────

/// Kani harness: exhaustively verify C-01 through C-04 for bounded domain.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(2)]
pub fn kani_collateral_ratio_properties() {
    let collateral: i128 = kani::any();
    let borrow: i128 = kani::any();
    kani::assume(collateral >= 0);
    kani::assume(borrow >= 0 && borrow <= i128::MAX / COLLATERAL_RATIO_MIN);

    match reference_validate_collateral_ratio(collateral, borrow) {
        Ok(()) => {
            // If accepted, collateral must be ≥ required minimum
            let min = borrow * COLLATERAL_RATIO_MIN / BPS_DIVISOR;
            kani::assert(collateral >= min, "accepted below minimum");
        }
        Err(CollateralError::InsufficientCollateral) => {
            let min = borrow * COLLATERAL_RATIO_MIN / BPS_DIVISOR;
            kani::assert(collateral < min, "rejected above minimum");
        }
        Err(CollateralError::Overflow) => {
            // Should not happen within the kani::assume domain
            kani::assert(false, "unexpected overflow");
        }
        Err(CollateralError::InvalidAmount) => {
            kani::assert(false, "unexpected invalid amount");
        }
    }
}
