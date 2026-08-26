//! Bridge helpers for migrating `hello-world/src/liquidate.rs`'s hardcoded
//! liquidation-incentive logic onto this crate's pluggable strategy system
//! (issue #817).
//!
//! `register_strategy`/`calculate_discount` take an opaque `Bytes` blob for
//! strategy parameters, but nothing in this crate encodes or decodes that
//! blob — callers have to agree on a wire format out of band. This module
//! defines that format for [`FixedDiscountParams`] (the strategy closest to
//! `hello-world`'s current fixed liquidation-incentive-bps behavior) as a
//! big-endian `i128`, matching the 16-byte minimum length the strategies
//! already validate against.

use crate::FixedDiscountParams;
use soroban_sdk::{Bytes, Env};

/// Encodes a [`FixedDiscountParams`] as the 16-byte big-endian wire format
/// `register_strategy`/`calculate_discount` expect in their `parameters: Bytes`.
pub fn encode_fixed_discount_params(env: &Env, params: &FixedDiscountParams) -> Bytes {
    Bytes::from_slice(env, &params.discount_bps.to_be_bytes())
}

/// Decodes the wire format produced by [`encode_fixed_discount_params`].
/// Returns `None` if `params` is shorter than 16 bytes (same threshold the
/// strategies' own `validate` already enforces).
pub fn decode_fixed_discount_params(params: &Bytes) -> Option<FixedDiscountParams> {
    if params.len() < 16 {
        return None;
    }
    let mut buf = [0u8; 16];
    params.slice(0..16).copy_into_slice(&mut buf);
    Some(FixedDiscountParams {
        discount_bps: i128::from_be_bytes(buf),
    })
}

/// Builds the `FixedDiscountParams` wire bytes directly from `hello-world`'s
/// `liquidation_incentive` risk parameter (basis points), so a caller
/// migrating off `hello-world`'s hardcoded incentive can register an
/// equivalent fixed-discount strategy here without hand-rolling the encoding.
pub fn fixed_discount_params_from_liquidation_incentive_bps(
    env: &Env,
    liquidation_incentive_bps: i128,
) -> Bytes {
    encode_fixed_discount_params(
        env,
        &FixedDiscountParams {
            discount_bps: liquidation_incentive_bps,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn encode_then_decode_round_trips() {
        let env = Env::default();
        let params = FixedDiscountParams { discount_bps: 1_000 };
        let encoded = encode_fixed_discount_params(&env, &params);
        assert_eq!(encoded.len(), 16);
        let decoded = decode_fixed_discount_params(&encoded).expect("decode should succeed");
        assert_eq!(decoded, params);
    }

    #[test]
    fn decode_rejects_short_buffers() {
        let env = Env::default();
        let short = Bytes::from_slice(&env, &[1, 2, 3]);
        assert_eq!(decode_fixed_discount_params(&short), None);
    }

    #[test]
    fn from_liquidation_incentive_bps_matches_manual_encoding() {
        let env = Env::default();
        let from_helper = fixed_discount_params_from_liquidation_incentive_bps(&env, 1_500);
        let manual = encode_fixed_discount_params(&env, &FixedDiscountParams { discount_bps: 1_500 });
        assert_eq!(from_helper, manual);
    }
}
