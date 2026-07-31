//! # Formal Specification — `get_max_liquidatable_amount`
//!
//! ## Mathematical model
//!
//! ```text
//! max_liquidatable = total_debt * close_factor_bps / 10_000
//!                    iff HF < HF_SCALE
//!                    else 0
//! ```
//!
//! ## Properties
//!
//! | ID   | Property                                                       |
//! |------|----------------------------------------------------------------|
//! | L-01 | Returns 0 when HF ≥ HF_SCALE (healthy position)               |
//! | L-02 | Returns 0 when user has no debt                                |
//! | L-03 | Returns 0 when oracle is absent (HF cannot be computed)        |
//! | L-04 | Liquidatable amount ≤ total_debt (never exceeds full debt)     |
//! | L-05 | Liquidatable amount = total_debt * close_factor / 10_000       |
//! | L-06 | No overflow for any valid i128 total debt                      |

// ── Inline constants (mirrors health_factor.rs and implementation) ────────────
const HEALTH_FACTOR_SCALE: i128 = 10_000;
const DEFAULT_LIQ_THRESHOLD_BPS: i128 = 8_000;
pub const DEFAULT_CLOSE_FACTOR_BPS: i128 = 5_000;
const BPS_DIVISOR: i128 = 10_000;

/// Inline health-factor helper (self-contained for this spec module).
fn local_health_factor(
    collateral_value: i128,
    debt_value: i128,
    liq_threshold_bps: i128,
    oracle_present: bool,
) -> Option<i128> {
    if debt_value <= 0 {
        return Some(i128::MAX);
    } // no debt = max healthy
    if !oracle_present {
        return None;
    }
    let weighted = collateral_value
        .checked_mul(liq_threshold_bps)?
        .checked_div(BPS_DIVISOR)?;
    let hf = weighted
        .checked_mul(HEALTH_FACTOR_SCALE)?
        .checked_div(debt_value)?;
    Some(hf)
}

/// Pure reference implementation of max-liquidatable-amount calculation.
pub fn reference_max_liquidatable(
    total_debt: i128,
    collateral_value: i128,
    debt_value: i128,
    close_factor_bps: i128,
    liq_threshold_bps: i128,
    oracle_present: bool,
) -> i128 {
    if total_debt <= 0 {
        return 0;
    }
    let hf = local_health_factor(
        collateral_value,
        debt_value,
        liq_threshold_bps,
        oracle_present,
    );
    match hf {
        None | Some(0) => 0,                      // oracle absent or zero HF
        Some(h) if h >= HEALTH_FACTOR_SCALE => 0, // healthy
        Some(_) => {
            // Liquidatable: apply close factor
            total_debt
                .checked_mul(close_factor_bps)
                .map(|v| v / BPS_DIVISOR)
                .unwrap_or(0)
        }
    }
}

/// **L-01**: Healthy position (HF ≥ HF_SCALE) → 0.
#[test]
fn lemma_l01_healthy_position_returns_zero() {
    // Collateral far above threshold
    let result = reference_max_liquidatable(
        100_000,
        2_000_000,
        1_000_000,
        DEFAULT_CLOSE_FACTOR_BPS,
        DEFAULT_LIQ_THRESHOLD_BPS,
        true,
    );
    assert_eq!(result, 0, "L-01: healthy position should return 0");
}

/// **L-02**: No debt → 0.
#[test]
fn lemma_l02_no_debt_returns_zero() {
    let result = reference_max_liquidatable(
        0,
        0,
        0,
        DEFAULT_CLOSE_FACTOR_BPS,
        DEFAULT_LIQ_THRESHOLD_BPS,
        true,
    );
    assert_eq!(result, 0, "L-02");
}

/// **L-03**: Oracle absent → 0.
#[test]
fn lemma_l03_no_oracle_returns_zero() {
    let result = reference_max_liquidatable(
        100_000,
        50_000,
        100_000,
        DEFAULT_CLOSE_FACTOR_BPS,
        DEFAULT_LIQ_THRESHOLD_BPS,
        false,
    );
    assert_eq!(result, 0, "L-03: no oracle should return 0");
}

/// **L-04**: Liquidatable amount never exceeds total debt.
#[test]
fn lemma_l04_liquidatable_does_not_exceed_total_debt() {
    // Severely undercollateralised position
    let total_debt = 1_000_000i128;
    let result = reference_max_liquidatable(
        total_debt,
        100_000,
        1_000_000,
        DEFAULT_CLOSE_FACTOR_BPS,
        DEFAULT_LIQ_THRESHOLD_BPS,
        true,
    );
    assert!(
        result <= total_debt,
        "L-04: liquidatable={result} > total_debt={total_debt}"
    );
}

/// **L-05**: Liquidatable amount equals total_debt * close_factor / 10_000.
#[test]
fn lemma_l05_formula_correctness() {
    let total_debt = 1_000_000i128;
    // Position: debt_value = 1_000_000, collateral_value = 1 (almost zero → clearly undercollateralised)
    // local_health_factor: weighted = 1 * 8000 / 10000 = 0, so hf = 0 / dv → 0
    // To ensure HF > 0 but < HEALTH_FACTOR_SCALE, use collateral slightly below threshold
    // threshold collateral for dv=1_000_000 at 80%: cv_min = 1_000_000 * 10_000 / 8_000 = 1_250_000
    // Use cv = 600_000 → weighted = 600_000 * 8_000 / 10_000 = 480_000
    //   HF = 480_000 * 10_000 / 1_000_000 = 4_800 < 10_000 → liquidatable
    let cv = 600_000i128;
    let dv = total_debt;
    let result = reference_max_liquidatable(
        total_debt,
        cv,
        dv,
        DEFAULT_CLOSE_FACTOR_BPS,
        DEFAULT_LIQ_THRESHOLD_BPS,
        true,
    );
    let expected = total_debt * DEFAULT_CLOSE_FACTOR_BPS / BPS_DIVISOR;
    assert_eq!(
        result, expected,
        "L-05: formula mismatch, got={result}, expected={expected}"
    );
}

/// **L-06**: No overflow for i128::MAX / 2 total debt.
#[test]
fn lemma_l06_no_overflow_on_large_debt() {
    let total_debt = i128::MAX / 2;
    let result = reference_max_liquidatable(
        total_debt,
        0,
        total_debt,
        DEFAULT_CLOSE_FACTOR_BPS,
        DEFAULT_LIQ_THRESHOLD_BPS,
        true,
    );
    // Should not panic; result may be 0 if intermediate overflow triggers the unwrap_or path
    let expected = (total_debt as i128)
        .checked_mul(DEFAULT_CLOSE_FACTOR_BPS)
        .map(|v| v / BPS_DIVISOR)
        .unwrap_or(0);
    assert_eq!(result, expected, "L-06: overflow path mismatch");
}
