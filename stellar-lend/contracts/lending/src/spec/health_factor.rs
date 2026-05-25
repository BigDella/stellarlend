//! # Formal Specification — `compute_health_factor`
//!
//! ## Mathematical model
//!
//! ```text
//! HF = (collateral_value * liq_threshold_bps / 10_000) * HF_SCALE / debt_value
//! ```
//! where `HF_SCALE = 10_000` (so HF = 10_000 means ratio exactly equals threshold).
//!
//! ## Properties
//!
//! | ID   | Property                                                       |
//! |------|----------------------------------------------------------------|
//! | H-01 | Zero debt → HEALTH_FACTOR_NO_DEBT sentinel                     |
//! | H-02 | No oracle → 0 when user has debt                               |
//! | H-03 | HF ≥ 1.0 (i.e. ≥ HF_SCALE) iff position is not liquidatable   |
//! | H-04 | HF is monotonically non-decreasing in collateral_value         |
//! | H-05 | HF is monotonically non-increasing in debt_value               |
//! | H-06 | No intermediate overflow for typical protocol values           |
//! | H-07 | Boundary: exact liquidation threshold yields HF = HF_SCALE     |

pub const HEALTH_FACTOR_SCALE: i128 = 10_000;
pub const HEALTH_FACTOR_NO_DEBT: i128 = 100_000_000;
pub const DEFAULT_LIQ_THRESHOLD_BPS: i128 = 8_000; // 80%
pub const BPS_DIVISOR: i128 = 10_000;

/// Pure reference implementation of health factor computation.
///
/// Returns `None` when oracle is absent and user has debt (cannot compute).
pub fn reference_health_factor(
    collateral_value: i128,
    debt_value: i128,
    has_debt: bool,
    liq_threshold_bps: i128,
    oracle_present: bool,
) -> Option<i128> {
    if debt_value <= 0 {
        if has_debt {
            return None; // Oracle absent; cannot compute
        }
        return Some(HEALTH_FACTOR_NO_DEBT);
    }
    if !oracle_present {
        return None;
    }
    // weighted_collateral = collateral_value * liq_threshold_bps / 10_000
    let weighted = collateral_value
        .checked_mul(liq_threshold_bps)?
        .checked_div(BPS_DIVISOR)?;
    // HF = weighted * HF_SCALE / debt_value
    let hf = weighted
        .checked_mul(HEALTH_FACTOR_SCALE)?
        .checked_div(debt_value)?;
    Some(hf)
}

/// **H-01**: Zero debt returns HEALTH_FACTOR_NO_DEBT.
#[test]
fn lemma_h01_zero_debt_returns_no_debt_sentinel() {
    let hf = reference_health_factor(1_000_000, 0, false, DEFAULT_LIQ_THRESHOLD_BPS, true);
    assert_eq!(hf, Some(HEALTH_FACTOR_NO_DEBT), "H-01");
}

/// **H-02**: No oracle + user has debt returns None (0 in implementation).
#[test]
fn lemma_h02_no_oracle_with_debt_returns_none() {
    let hf = reference_health_factor(1_000_000, 500_000, true, DEFAULT_LIQ_THRESHOLD_BPS, false);
    assert_eq!(hf, None, "H-02: expected None without oracle");
}

/// **H-03**: HF ≥ HF_SCALE iff position is not liquidatable.
#[test]
fn lemma_h03_hf_scale_liquidation_boundary() {
    // At exact threshold: collateral * threshold / 10000 == debt
    // => HF = debt * HF_SCALE / debt = HF_SCALE
    let debt = 1_000_000i128;
    let collateral = debt * BPS_DIVISOR / DEFAULT_LIQ_THRESHOLD_BPS; // = 1_250_000
    let hf = reference_health_factor(collateral, debt, true, DEFAULT_LIQ_THRESHOLD_BPS, true)
        .expect("H-03: exact threshold");
    assert_eq!(
        hf, HEALTH_FACTOR_SCALE,
        "H-03: exact threshold HF should equal HF_SCALE"
    );

    // Use 2x collateral → HF should be 2 * HF_SCALE (clearly healthy)
    let hf_above =
        reference_health_factor(collateral * 2, debt, true, DEFAULT_LIQ_THRESHOLD_BPS, true)
            .expect("H-03 above");
    assert!(
        hf_above > HEALTH_FACTOR_SCALE,
        "H-03: 2x collateral should be healthy, got hf={hf_above}"
    );

    // Half the exact-threshold collateral → HF should be HF_SCALE/2 (liquidatable)
    let hf_below =
        reference_health_factor(collateral / 2, debt, true, DEFAULT_LIQ_THRESHOLD_BPS, true)
            .expect("H-03 below");
    assert!(
        hf_below < HEALTH_FACTOR_SCALE,
        "H-03: half collateral should be liquidatable, got hf={hf_below}"
    );
}

/// **H-04**: HF is monotonically non-decreasing in collateral_value.
#[test]
fn lemma_h04_monotone_increasing_in_collateral() {
    let debt = 500_000i128;
    let collateral_values: &[i128] = &[0, 100_000, 500_000, 800_000, 1_000_000, 5_000_000];
    let mut prev = 0i128;
    for &cv in collateral_values {
        let hf =
            reference_health_factor(cv, debt, true, DEFAULT_LIQ_THRESHOLD_BPS, true).unwrap_or(0);
        assert!(
            hf >= prev,
            "H-04: HF not monotone at cv={cv}: hf={hf} < prev={prev}"
        );
        prev = hf;
    }
}

/// **H-05**: HF is monotonically non-increasing in debt_value.
#[test]
fn lemma_h05_monotone_decreasing_in_debt() {
    let collateral = 1_000_000i128;
    let debts: &[i128] = &[1, 500, 100_000, 500_000, 1_000_000];
    let mut prev = i128::MAX;
    for &dv in debts {
        let hf = reference_health_factor(collateral, dv, true, DEFAULT_LIQ_THRESHOLD_BPS, true)
            .unwrap_or(0);
        assert!(
            hf <= prev,
            "H-05: HF not monotone at dv={dv}: hf={hf} > prev={prev}"
        );
        prev = hf;
    }
}

/// **H-06**: No intermediate overflow for typical protocol values.
#[test]
fn lemma_h06_no_overflow_on_large_values() {
    // Use values up to ~1e15 USD-equivalent (reasonable protocol scale)
    let collateral = 1_000_000_000_000_000i128;
    let debt = 500_000_000_000_000i128;
    let hf = reference_health_factor(collateral, debt, true, DEFAULT_LIQ_THRESHOLD_BPS, true);
    assert!(hf.is_some(), "H-06: unexpected overflow at large values");
    let hf = hf.unwrap();
    assert!(hf > 0, "H-06: HF must be positive for positive inputs");
}

/// **H-07**: Boundary — exact liquidation threshold position yields HF = HF_SCALE.
#[test]
fn lemma_h07_exact_threshold_yields_hf_scale() {
    // At the exact threshold: collateral_value * threshold / 10000 = debt_value
    // => HF = debt_value * HF_SCALE / debt_value = HF_SCALE
    let debt = 100_000i128;
    let collateral = debt * BPS_DIVISOR / DEFAULT_LIQ_THRESHOLD_BPS;
    let hf = reference_health_factor(collateral, debt, true, DEFAULT_LIQ_THRESHOLD_BPS, true)
        .expect("H-07");
    assert_eq!(
        hf, HEALTH_FACTOR_SCALE,
        "H-07: boundary HF should be exactly HF_SCALE"
    );
}

#[cfg(kani)]
#[kani::proof]
#[kani::unwind(2)]
pub fn kani_health_factor_properties() {
    let collateral_value: i128 = kani::any();
    let debt_value: i128 = kani::any();
    kani::assume(collateral_value >= 0 && collateral_value <= 1_000_000_000_000_000i128);
    kani::assume(debt_value >= 0 && debt_value <= 1_000_000_000_000_000i128);

    let hf = reference_health_factor(
        collateral_value,
        debt_value,
        debt_value > 0,
        DEFAULT_LIQ_THRESHOLD_BPS,
        true,
    );

    if debt_value == 0 {
        kani::assert(
            hf == Some(HEALTH_FACTOR_NO_DEBT),
            "H-01: zero debt sentinel",
        );
    }
    if let Some(h) = hf {
        kani::assert(h >= 0, "HF must be non-negative");
    }
}
