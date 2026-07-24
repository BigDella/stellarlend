//! # Packed Pool Configuration Storage (issue #633)
//!
//! Each pool-configuration parameter previously occupied its own persistent
//! slot, so a pool paid Soroban storage rent on five-plus separate entries.
//! This module bit-packs the configuration into **two** machine words:
//!
//! ## Word 1 — rate parameters (`u128`)
//!
//! Five basis-point parameters, 16 bits each (each ≤ 10 000 ≤ 0xFFFF):
//!
//! | bits     | field                      |
//! |----------|----------------------------|
//! | `0..16`  | loan-to-value (LTV)        |
//! | `16..32` | liquidation threshold      |
//! | `32..48` | reserve factor             |
//! | `48..64` | close factor               |
//! | `64..80` | liquidation incentive      |
//!
//! ## Word 2 — timestamp + flags (`u64`)
//!
//! | bits     | field                      |
//! |----------|----------------------------|
//! | `0..40`  | last-update timestamp (s)  |
//! | `40..48` | status flags (8 booleans)  |
//!
//! Packing/unpacking is **pure integer arithmetic** — reads stay O(1) and do not
//! cost more gas than reading a single slot, while rent drops from N slots to 2.

use soroban_sdk::{contracterror, contracttype, Env};

// ── Field widths / masks ─────────────────────────────────────────────────

/// Width of each basis-point field in the rate word.
const BPS_FIELD_BITS: u32 = 16;
/// Mask for a single 16-bit basis-point field.
const BPS_FIELD_MASK: u128 = 0xFFFF;

/// Timestamp occupies the low 40 bits of the status word (~year 36 800).
const TS_BITS: u32 = 40;
const TS_MASK: u64 = (1u64 << TS_BITS) - 1;
/// Status flags occupy 8 bits above the timestamp.
const FLAGS_SHIFT: u32 = TS_BITS;
const FLAGS_MASK: u64 = 0xFF;

// ── Status-flag bit positions ────────────────────────────────────────────

/// Pool is paused.
pub const FLAG_PAUSED: u8 = 1 << 0;
/// Borrowing is enabled.
pub const FLAG_BORROWING_ENABLED: u8 = 1 << 1;
/// Collateral usage is enabled.
pub const FLAG_COLLATERAL_ENABLED: u8 = 1 << 2;
/// Pool is deprecated (wind-down only).
pub const FLAG_DEPRECATED: u8 = 1 << 3;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PackError {
    /// A basis-point value does not fit the 16-bit packed field.
    BpsFieldOverflow = 1,
    /// Timestamp does not fit the 40-bit packed field.
    TimestampOverflow = 2,
}

/// Storage key for the packed configuration words.
#[contracttype]
#[derive(Clone)]
pub enum PackedConfigKey {
    /// Both packed words `(rate_word: u128, status_word: u64)`.
    Config,
}

/// Logical (unpacked) view of the pool configuration.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PoolConfig {
    pub ltv_bps: i128,
    pub liquidation_threshold_bps: i128,
    pub reserve_factor_bps: i128,
    pub close_factor_bps: i128,
    pub liquidation_incentive_bps: i128,
    pub last_update: u64,
    /// Packed status-flag byte (see `FLAG_*`).
    pub flags: u32,
}

/// The two packed words as persisted on-chain.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PackedConfig {
    pub rate_word: u128,
    pub status_word: u64,
}

// ── Pure pack / unpack ───────────────────────────────────────────────────

fn pack_bps_field(value: i128, slot: u32) -> Result<u128, PackError> {
    if !(0..=BPS_FIELD_MASK as i128).contains(&value) {
        return Err(PackError::BpsFieldOverflow);
    }
    Ok((value as u128) << (slot * BPS_FIELD_BITS))
}

fn unpack_bps_field(word: u128, slot: u32) -> i128 {
    ((word >> (slot * BPS_FIELD_BITS)) & BPS_FIELD_MASK) as i128
}

/// Pack a [`PoolConfig`] into its two-word representation.
pub fn pack(config: &PoolConfig) -> Result<PackedConfig, PackError> {
    let rate_word = pack_bps_field(config.ltv_bps, 0)?
        | pack_bps_field(config.liquidation_threshold_bps, 1)?
        | pack_bps_field(config.reserve_factor_bps, 2)?
        | pack_bps_field(config.close_factor_bps, 3)?
        | pack_bps_field(config.liquidation_incentive_bps, 4)?;

    if config.last_update > TS_MASK {
        return Err(PackError::TimestampOverflow);
    }
    let status_word =
        (config.last_update & TS_MASK) | (((config.flags as u64) & FLAGS_MASK) << FLAGS_SHIFT);

    Ok(PackedConfig {
        rate_word,
        status_word,
    })
}

/// Unpack the two-word representation back into a [`PoolConfig`].
pub fn unpack(packed: &PackedConfig) -> PoolConfig {
    PoolConfig {
        ltv_bps: unpack_bps_field(packed.rate_word, 0),
        liquidation_threshold_bps: unpack_bps_field(packed.rate_word, 1),
        reserve_factor_bps: unpack_bps_field(packed.rate_word, 2),
        close_factor_bps: unpack_bps_field(packed.rate_word, 3),
        liquidation_incentive_bps: unpack_bps_field(packed.rate_word, 4),
        last_update: packed.status_word & TS_MASK,
        flags: ((packed.status_word >> FLAGS_SHIFT) & FLAGS_MASK) as u32,
    }
}

// ── Flag helpers ─────────────────────────────────────────────────────────

/// Read a status flag from a packed flags byte.
pub fn flag_is_set(flags: u8, flag: u8) -> bool {
    flags & flag != 0
}

/// Set/clear a status flag, returning the new flags byte.
pub fn flag_with(flags: u8, flag: u8, on: bool) -> u8 {
    if on {
        flags | flag
    } else {
        flags & !flag
    }
}

// ── Persistence ──────────────────────────────────────────────────────────

/// Read the packed config, or `None` if the pool has not been packed yet.
pub fn load(env: &Env) -> Option<PoolConfig> {
    env.storage()
        .persistent()
        .get::<PackedConfigKey, PackedConfig>(&PackedConfigKey::Config)
        .map(|p| unpack(&p))
}

/// Pack and persist a [`PoolConfig`] into the single packed entry.
pub fn store(env: &Env, config: &PoolConfig) -> Result<(), PackError> {
    let packed = pack(config)?;
    env.storage()
        .persistent()
        .set(&PackedConfigKey::Config, &packed);
    Ok(())
}

/// Migrate an existing pool's loose configuration values into the packed entry.
///
/// Idempotent: if a packed entry already exists it is left untouched. The legacy
/// values are read via the borrow module's getters so the migration reflects the
/// pool's current on-chain parameters.
pub fn migrate_from_legacy(env: &Env) -> Result<PoolConfig, PackError> {
    if let Some(existing) = load(env) {
        return Ok(existing);
    }

    let config = PoolConfig {
        // LTV is not tracked separately in the legacy layout; derive a safe
        // default from the liquidation threshold (callers may override later).
        ltv_bps: crate::borrow::get_liquidation_threshold_bps(env),
        liquidation_threshold_bps: crate::borrow::get_liquidation_threshold_bps(env),
        reserve_factor_bps: 0,
        close_factor_bps: crate::borrow::get_close_factor_bps(env),
        liquidation_incentive_bps: crate::borrow::get_liquidation_incentive_bps(env),
        last_update: env.ledger().timestamp(),
        flags: FLAG_BORROWING_ENABLED | FLAG_COLLATERAL_ENABLED,
    };

    store(env, &config)?;
    Ok(config)
}

#[cfg(test)]
mod unit {
    use super::*;

    fn sample() -> PoolConfig {
        PoolConfig {
            ltv_bps: 7_500,
            liquidation_threshold_bps: 8_000,
            reserve_factor_bps: 1_000,
            close_factor_bps: 5_000,
            liquidation_incentive_bps: 1_000,
            last_update: 1_700_000_000,
            flags: FLAG_BORROWING_ENABLED | FLAG_COLLATERAL_ENABLED,
        }
    }

    #[test]
    fn pack_unpack_round_trips() {
        let cfg = sample();
        let packed = pack(&cfg).unwrap();
        assert_eq!(unpack(&packed), cfg);
    }

    #[test]
    fn fields_are_independent() {
        // Changing one field must not bleed into neighbours.
        let mut cfg = sample();
        cfg.reserve_factor_bps = 0xFFFF; // max field value
        let back = unpack(&pack(&cfg).unwrap());
        assert_eq!(back.reserve_factor_bps, 0xFFFF);
        assert_eq!(back.ltv_bps, 7_500);
        assert_eq!(back.liquidation_threshold_bps, 8_000);
        assert_eq!(back.close_factor_bps, 5_000);
        assert_eq!(back.liquidation_incentive_bps, 1_000);
    }

    #[test]
    fn five_bps_fields_fit_one_word() {
        // All five 16-bit fields occupy the low 80 bits of the u128.
        let packed = pack(&sample()).unwrap();
        assert_eq!(packed.rate_word >> 80, 0);
    }

    #[test]
    fn bps_field_overflow_rejected() {
        let mut cfg = sample();
        cfg.ltv_bps = 0x1_0000; // 17 bits — too wide
        assert_eq!(pack(&cfg), Err(PackError::BpsFieldOverflow));
    }

    #[test]
    fn negative_bps_rejected() {
        let mut cfg = sample();
        cfg.close_factor_bps = -1;
        assert_eq!(pack(&cfg), Err(PackError::BpsFieldOverflow));
    }

    #[test]
    fn timestamp_overflow_rejected() {
        let mut cfg = sample();
        cfg.last_update = 1u64 << 41;
        assert_eq!(pack(&cfg), Err(PackError::TimestampOverflow));
    }

    #[test]
    fn flags_round_trip_and_helpers() {
        let mut cfg = sample();
        cfg.flags = flag_with(cfg.flags, FLAG_PAUSED, true);
        let back = unpack(&pack(&cfg).unwrap());
        assert!(flag_is_set(back.flags, FLAG_PAUSED));
        assert!(flag_is_set(back.flags, FLAG_BORROWING_ENABLED));
        assert!(!flag_is_set(back.flags, FLAG_DEPRECATED));

        let cleared = flag_with(back.flags, FLAG_PAUSED, false);
        assert!(!flag_is_set(cleared, FLAG_PAUSED));
    }

    #[test]
    fn flags_do_not_corrupt_timestamp() {
        let mut cfg = sample();
        cfg.flags = 0xFF; // all flags on
        let back = unpack(&pack(&cfg).unwrap());
        assert_eq!(back.last_update, 1_700_000_000);
        assert_eq!(back.flags, 0xFF);
    }
}
