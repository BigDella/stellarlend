//! # Incremental Interest Calculation with Caching (issue #631)
//!
//! The naive interest model recomputes the full accrual from scratch on every
//! interaction. This module introduces a cached **cumulative interest index**
//! that is advanced *incrementally* — only the delta accrued since the last
//! update is computed, and only when an input actually changes.
//!
//! ## Cache layout
//!
//! A single [`InterestCache`] record stores:
//! * `last_update_block`  — ledger sequence of the last accrual
//! * `last_update_time`   — ledger timestamp of the last accrual
//! * `cumulative_index`   — compounding index, scaled by [`INDEX_SCALE`]
//! * `last_cached_rate`   — borrow rate (bps) used for the segment just closed
//!
//! ## Incremental update
//!
//! `index_n = index_{n-1} + index_{n-1} * rate * dt / (BPS * SECONDS_PER_YEAR)`
//!
//! Each segment is linear (matching the protocol's simple-interest model), so a
//! position's accrued interest is a single multiply against the ratio of the
//! current index to the index captured when the position was last touched.
//!
//! ## Batching
//!
//! Multiple operations landing in the *same ledger* observe `dt == 0` and skip
//! the write entirely — interest is therefore charged once per block, not once
//! per operation.
//!
//! ## Invalidation
//!
//! [`invalidate`] first accrues up to the present moment (closing the current
//! segment at the old rate) and then refreshes the cached rate. Callers must
//! invoke it on rate-model changes, parameter changes and oracle updates so the
//! next segment compounds at the corrected rate.

use soroban_sdk::{contracterror, contracttype, Env};

/// Fixed-point scale for the cumulative index (`1e9` == 1.0).
pub const INDEX_SCALE: i128 = 1_000_000_000;

/// Basis-point denominator.
const BPS_SCALE: i128 = 10_000;

/// Seconds in a (non-leap) year — matches `borrow.rs`.
const SECONDS_PER_YEAR: i128 = 31_536_000;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum InterestCacheError {
    /// Arithmetic overflow while advancing the index.
    Overflow = 1,
    /// A position index was newer than the global index (inconsistent cache).
    StaleSnapshot = 2,
}

/// Storage keys for the interest cache (namespaced to avoid collisions).
#[contracttype]
#[derive(Clone)]
pub enum InterestCacheKey {
    /// The single global interest cache record.
    Cache,
}

/// Cached intermediate interest state. One record per pool.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct InterestCache {
    /// Ledger sequence (block) of the last accrual.
    pub last_update_block: u32,
    /// Ledger timestamp of the last accrual.
    pub last_update_time: u64,
    /// Compounding index scaled by [`INDEX_SCALE`].
    pub cumulative_index: i128,
    /// Borrow rate (bps) applied to the segment just closed.
    pub last_cached_rate: i128,
}

impl InterestCache {
    /// A fresh cache anchored at the current ledger, index == 1.0.
    fn fresh(env: &Env, rate_bps: i128) -> Self {
        InterestCache {
            last_update_block: env.ledger().sequence(),
            last_update_time: env.ledger().timestamp(),
            cumulative_index: INDEX_SCALE,
            last_cached_rate: rate_bps,
        }
    }
}

/// Pure incremental index step. Exposed for unit testing without an `Env`.
///
/// Returns the new index given the previous index, the rate (bps) charged over
/// the segment and the elapsed seconds. Linear within the segment.
///
/// `delta = index * rate_bps * dt / (BPS * SECONDS_PER_YEAR)`
pub fn advance_index(
    index: i128,
    rate_bps: i128,
    dt_secs: u64,
) -> Result<i128, InterestCacheError> {
    if dt_secs == 0 || rate_bps == 0 || index == 0 {
        return Ok(index);
    }

    let dt = dt_secs as i128;
    // index * rate_bps * dt  — bounded well under i128::MAX for realistic inputs
    // (index ~1e9, rate ~1e4, dt ~3e7 ⇒ ~3e20 ≪ 1.7e38).
    let numerator = index
        .checked_mul(rate_bps)
        .and_then(|v| v.checked_mul(dt))
        .ok_or(InterestCacheError::Overflow)?;

    let denominator = BPS_SCALE
        .checked_mul(SECONDS_PER_YEAR)
        .ok_or(InterestCacheError::Overflow)?;

    let delta = numerator
        .checked_div(denominator)
        .ok_or(InterestCacheError::Overflow)?;

    index.checked_add(delta).ok_or(InterestCacheError::Overflow)
}

/// Compute the interest owed on `principal` given the index when the position
/// was last synced (`entry_index`) and the current global `index`.
///
/// `interest = principal * (index - entry_index) / entry_index`
pub fn interest_for(
    principal: i128,
    entry_index: i128,
    index: i128,
) -> Result<i128, InterestCacheError> {
    if principal <= 0 || entry_index <= 0 {
        return Ok(0);
    }
    if index < entry_index {
        // The position carries an index from a *future* state — only possible
        // after a reorg rewound the global index. Treat as no accrual rather
        // than reporting negative interest.
        return Err(InterestCacheError::StaleSnapshot);
    }
    let growth = index
        .checked_sub(entry_index)
        .ok_or(InterestCacheError::Overflow)?;
    principal
        .checked_mul(growth)
        .ok_or(InterestCacheError::Overflow)?
        .checked_div(entry_index)
        .ok_or(InterestCacheError::Overflow)
}

/// Read the cache, materialising a fresh one (without persisting) if absent.
pub fn get_cache(env: &Env) -> InterestCache {
    env.storage()
        .persistent()
        .get(&InterestCacheKey::Cache)
        .unwrap_or_else(|| InterestCache::fresh(env, current_rate(env)))
}

fn save_cache(env: &Env, cache: &InterestCache) {
    env.storage()
        .persistent()
        .set(&InterestCacheKey::Cache, cache);
}

/// Best-effort current borrow rate. Falls back to 0 on error so the view path
/// never panics.
fn current_rate(env: &Env) -> i128 {
    crate::interest_rate::borrow_rate_bps(env).unwrap_or(0)
}

/// Advance the cached index to the current ledger and persist it.
///
/// * **No-op update:** if the cache is already at this block (or `dt == 0`) the
///   record is returned unchanged and no storage write occurs (batching).
/// * **Reorg safety:** if ledger time appears to move *backwards* the segment is
///   skipped (we never rewind a persisted index), keeping the cache monotonic.
pub fn accrue(env: &Env) -> Result<InterestCache, InterestCacheError> {
    let mut cache = get_cache(env);
    let now = env.ledger().timestamp();
    let block = env.ledger().sequence();

    // Batch: already accrued this block — charge interest once per block.
    if block == cache.last_update_block || now <= cache.last_update_time {
        return Ok(cache);
    }

    let dt = now - cache.last_update_time;
    cache.cumulative_index = advance_index(cache.cumulative_index, cache.last_cached_rate, dt)?;
    cache.last_update_time = now;
    cache.last_update_block = block;
    cache.last_cached_rate = current_rate(env);

    save_cache(env, &cache);
    Ok(cache)
}

/// Read-optimised cumulative index for view calls.
///
/// Computes what the index *would* be at the current ledger **without writing**
/// to storage, so cheap `view` entry points pay no storage-write gas.
pub fn current_index(env: &Env) -> i128 {
    let cache = get_cache(env);
    let now = env.ledger().timestamp();
    if now <= cache.last_update_time {
        return cache.cumulative_index;
    }
    let dt = now - cache.last_update_time;
    advance_index(cache.cumulative_index, cache.last_cached_rate, dt)
        .unwrap_or(cache.cumulative_index)
}

/// Invalidate the cache after an input change (rate model, parameter, oracle).
///
/// Closes the in-flight segment at the *old* rate first, then re-anchors the
/// cached rate to the freshly-read value so subsequent compounding is correct.
pub fn invalidate(env: &Env) -> Result<InterestCache, InterestCacheError> {
    // Close the open segment at the previous rate.
    let mut cache = accrue(env)?;
    // Re-anchor to the new rate even if `accrue` short-circuited (same block).
    cache.last_cached_rate = current_rate(env);
    cache.last_update_block = env.ledger().sequence();
    save_cache(env, &cache);
    Ok(cache)
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn advance_index_zero_dt_is_noop() {
        assert_eq!(advance_index(INDEX_SCALE, 500, 0).unwrap(), INDEX_SCALE);
    }

    #[test]
    fn advance_index_zero_rate_is_noop() {
        assert_eq!(
            advance_index(INDEX_SCALE, 0, SECONDS_PER_YEAR as u64).unwrap(),
            INDEX_SCALE
        );
    }

    #[test]
    fn advance_index_full_year_at_5pct() {
        // 5% (500 bps) over exactly one year ⇒ index grows by 5%.
        let next = advance_index(INDEX_SCALE, 500, SECONDS_PER_YEAR as u64).unwrap();
        assert_eq!(next, INDEX_SCALE + INDEX_SCALE * 5 / 100);
    }

    #[test]
    fn advance_index_is_incremental_and_consistent() {
        // Two half-year linear segments accumulate the same delta as the math.
        let half = (SECONDS_PER_YEAR / 2) as u64;
        let a = advance_index(INDEX_SCALE, 1000, half).unwrap();
        let b = advance_index(a, 1000, half).unwrap();
        // Linear within each segment: total delta == index*rate*1yr/(bps*yr) twice.
        let expected_delta_each =
            INDEX_SCALE * 1000 * (half as i128) / (BPS_SCALE * SECONDS_PER_YEAR);
        assert_eq!(a, INDEX_SCALE + expected_delta_each);
        // Second segment compounds off the larger index.
        let second_delta = a * 1000 * (half as i128) / (BPS_SCALE * SECONDS_PER_YEAR);
        assert_eq!(b, a + second_delta);
        assert!(b > a);
    }

    #[test]
    fn interest_for_basic() {
        // principal 1_000_000 with index grown 10% ⇒ 100_000 interest.
        let idx = INDEX_SCALE + INDEX_SCALE / 10;
        assert_eq!(interest_for(1_000_000, INDEX_SCALE, idx).unwrap(), 100_000);
    }

    #[test]
    fn interest_for_zero_principal() {
        assert_eq!(interest_for(0, INDEX_SCALE, INDEX_SCALE * 2).unwrap(), 0);
    }

    #[test]
    fn interest_for_reorg_rewind_is_flagged() {
        // entry index newer than the (rewound) global index.
        let res = interest_for(1_000, INDEX_SCALE * 2, INDEX_SCALE);
        assert_eq!(res, Err(InterestCacheError::StaleSnapshot));
    }

    #[test]
    fn no_op_when_index_unchanged() {
        // Same index ⇒ no interest.
        assert_eq!(interest_for(5_000, INDEX_SCALE, INDEX_SCALE).unwrap(), 0);
    }
}
