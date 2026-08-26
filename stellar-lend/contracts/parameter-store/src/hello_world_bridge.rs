//! Bridge between `hello-world`'s in-contract [`RiskParams`]-style config
//! (four `i128` fields updated in place, no timelock/audit trail) and this
//! crate's on-chain governance parameter store.
//!
//! Migrating `hello-world/src/governance.rs` off ad-hoc config storage is a
//! multi-step process (see issue #818): this module provides the pure,
//! side-effect-free conversion helpers a caller needs for that migration —
//! translating the four risk-relevant `hello-world` fields into
//! [`ParameterType`]/value pairs that can be proposed through
//! [`crate::ParameterStoreContract::propose_change`], and reconstructing a
//! `hello-world`-shaped tuple from parameters already accepted here.
//!
//! `min_collateral_ratio` (a "how much collateral per unit of debt" figure)
//! and [`ParameterType::LTV`] (a "how much debt per unit of collateral"
//! figure) are reciprocal concepts, so the conversion below is an
//! approximation, not a lossless round trip — see [`ltv_bps_from_min_collateral_ratio`].

use crate::{ParameterType, BPS_DIVISOR};

/// The four `hello-world` risk fields this bridge knows how to migrate.
/// Mirrors `hello_world::risk_params::RiskParams` without depending on that
/// crate (hello-world is a WASM contract binary, not a library dependency).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HelloWorldRiskParams {
    pub min_collateral_ratio: i128,
    pub liquidation_threshold: i128,
    pub close_factor: i128,
    pub liquidation_incentive: i128,
}

/// Approximates an LTV (max debt per unit of collateral, in bps) from a
/// minimum collateral ratio (min collateral per unit of debt, in bps).
/// `ltv_bps = BPS_DIVISOR^2 / min_collateral_ratio_bps`.
pub fn ltv_bps_from_min_collateral_ratio(min_collateral_ratio_bps: i128) -> i128 {
    if min_collateral_ratio_bps <= 0 {
        return 0;
    }
    (BPS_DIVISOR * BPS_DIVISOR) / min_collateral_ratio_bps
}

/// Inverse of [`ltv_bps_from_min_collateral_ratio`].
pub fn min_collateral_ratio_from_ltv_bps(ltv_bps: i128) -> i128 {
    if ltv_bps <= 0 {
        return 0;
    }
    (BPS_DIVISOR * BPS_DIVISOR) / ltv_bps
}

/// Splits a `hello-world` risk-params snapshot into the individual
/// `(ParameterType, value)` pairs to propose in the parameter store.
pub fn to_parameter_changes(params: &HelloWorldRiskParams) -> [(ParameterType, i128); 4] {
    [
        (
            ParameterType::LTV,
            ltv_bps_from_min_collateral_ratio(params.min_collateral_ratio),
        ),
        (
            ParameterType::LiquidationThreshold,
            params.liquidation_threshold,
        ),
        (ParameterType::CloseFactor, params.close_factor),
        (
            ParameterType::LiquidationIncentive,
            params.liquidation_incentive,
        ),
    ]
}

/// Reassembles a `hello-world`-shaped risk-params snapshot from parameter
/// values already read out of the store (e.g. via `get_parameter`).
pub fn from_parameter_values(
    ltv_bps: i128,
    liquidation_threshold: i128,
    close_factor: i128,
    liquidation_incentive: i128,
) -> HelloWorldRiskParams {
    HelloWorldRiskParams {
        min_collateral_ratio: min_collateral_ratio_from_ltv_bps(ltv_bps),
        liquidation_threshold,
        close_factor,
        liquidation_incentive,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ltv_round_trip_is_stable_for_common_ratios() {
        // 150% min collateral ratio <-> ~66.67% LTV
        let ltv = ltv_bps_from_min_collateral_ratio(15_000);
        assert_eq!(ltv, 6_666);
        let back = min_collateral_ratio_from_ltv_bps(ltv);
        // Integer division means this won't be exact, but should be close.
        assert!((back - 15_000).abs() < 5);
    }

    #[test]
    fn zero_and_negative_inputs_are_handled_without_panicking() {
        assert_eq!(ltv_bps_from_min_collateral_ratio(0), 0);
        assert_eq!(ltv_bps_from_min_collateral_ratio(-100), 0);
        assert_eq!(min_collateral_ratio_from_ltv_bps(0), 0);
    }

    #[test]
    fn to_parameter_changes_covers_all_four_fields() {
        let params = HelloWorldRiskParams {
            min_collateral_ratio: 15_000,
            liquidation_threshold: 10_500,
            close_factor: 5_000,
            liquidation_incentive: 1_000,
        };
        let changes = to_parameter_changes(&params);
        assert_eq!(changes[0].0, ParameterType::LTV);
        assert_eq!(changes[1], (ParameterType::LiquidationThreshold, 10_500));
        assert_eq!(changes[2], (ParameterType::CloseFactor, 5_000));
        assert_eq!(changes[3], (ParameterType::LiquidationIncentive, 1_000));
    }

    #[test]
    fn from_parameter_values_reassembles_a_hello_world_snapshot() {
        let rebuilt = from_parameter_values(6_666, 10_500, 5_000, 1_000);
        assert_eq!(rebuilt.liquidation_threshold, 10_500);
        assert_eq!(rebuilt.close_factor, 5_000);
        assert_eq!(rebuilt.liquidation_incentive, 1_000);
        assert!((rebuilt.min_collateral_ratio - 15_000).abs() < 5);
    }
}
