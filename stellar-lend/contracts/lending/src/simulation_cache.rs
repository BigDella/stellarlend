//! # Transaction Simulation Cache (issue #636)
//!
//! Block-scoped LRU cache for common pool operation simulations.
//! Identical simulation parameters within the same block reuse cached results,
//! saving compute gas. Cache is automatically invalidated on new blocks.
//!
//! ## Cache key
//!
//! `(operation_type, pool_address, user_address, asset, amount)` — hashed
//! into a `u64` key for fast storage lookup.
//!
//! ## Eviction
//!
//! LRU eviction with a configurable max size (default 64 entries per block).

use soroban_sdk::{contracterror, contracttype, Address, Crypto, Env, Vec};

// ── Errors ────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SimCacheError {
    CacheFull = 1,
    InvalidOperation = 2,
    Overflow = 3,
}

// ── Storage keys ──────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum SimCacheKey {
    /// Current cache block number.
    CurrentBlock,
    /// Cache entry by index.
    Entry(u32),
    /// LRU index tracking.
    LruOrder(u32),
    /// Current cache count.
    Count,
    /// Cache stats (hits, misses).
    Stats,
    /// Configuration.
    Config,
}

// ── Configuration ─────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SimCacheConfig {
    /// Maximum cache entries before LRU eviction.
    pub max_entries: u32,
    /// Whether caching is enabled.
    pub enabled: bool,
}

impl Default for SimCacheConfig {
    fn default() -> Self {
        SimCacheConfig {
            max_entries: 64,
            enabled: true,
        }
    }
}

// ── Cache entry ───────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SimCacheEntry {
    /// Hash of the cache key components.
    pub key_hash: u64,
    /// Operation type: 0=deposit, 1=withdraw, 2=borrow, 3=repay, 4=liquidate.
    pub operation_type: u32,
    /// Pool address.
    pub pool: Address,
    /// User address.
    pub user: Address,
    /// Asset address.
    pub asset: Address,
    /// Amount.
    pub amount: i128,
    /// Cached result: health factor after operation (scaled by 10000).
    pub health_after: i128,
    /// Cached result: collateral value after.
    pub collateral_value_after: i128,
    /// Cached result: debt value after.
    pub debt_value_after: i128,
    /// Whether the simulation would succeed.
    pub would_succeed: bool,
    /// Block number when cached.
    pub cached_block: u32,
    /// Timestamp of cache write.
    pub cached_at: u64,
    /// LRU access counter (higher = more recently used).
    pub access_count: u64,
}

// ── Simulation result ─────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SimulationResult {
    pub health_after: i128,
    pub collateral_value_after: i128,
    pub debt_value_after: i128,
    pub would_succeed: bool,
    pub cached: bool,
}

// ── Cache statistics ──────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SimCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_entries: u32,
    pub current_block: u32,
}

// ── Key hashing ───────────────────────────────────────────────────────────

fn compute_key_hash(
    env: &Env,
    operation_type: u32,
    pool: &Address,
    user: &Address,
    asset: &Address,
    amount: i128,
) -> u64 {
    let crypto = Crypto::new(env);
    let mut data = soroban_sdk::Bytes::new(env);
    data.extend_from_slice(&(operation_type.to_be_bytes()));
    data.extend_from_slice(&pool.to_buffer());
    data.extend_from_slice(&user.to_buffer());
    data.extend_from_slice(&asset.to_buffer());
    data.extend_from_slice(&amount.to_be_bytes());

    let hash = crypto.hash_sha256(&data);
    let bytes = hash.to_array();

    // Use first 8 bytes as u64 key.
    u64::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

// ── Block validation ──────────────────────────────────────────────────────

fn ensure_current_block(env: &Env) {
    let current_block = env.ledger().sequence();
    let stored_block: u32 = env
        .storage()
        .instance()
        .get(&SimCacheKey::CurrentBlock)
        .unwrap_or(0);

    if stored_block != current_block {
        // New block — clear cache.
        let count: u32 = env
            .storage()
            .instance()
            .get(&SimCacheKey::Count)
            .unwrap_or(0);

        for i in 0..count {
            env.storage()
                .instance()
                .remove(&SimCacheKey::Entry(i));
            env.storage()
                .instance()
                .remove(&SimCacheKey::LruOrder(i));
        }

        env.storage()
            .instance()
            .set(&SimCacheKey::Count, &0u32);
        env.storage()
            .instance()
            .set(&SimCacheKey::CurrentBlock, &current_block);

        // Reset stats for new block.
        env.storage().instance().set(
            &SimCacheKey::Stats,
            &SimCacheStats {
                hits: 0,
                misses: 0,
                evictions: 0,
                total_entries: 0,
                current_block,
            },
        );
    }
}

// ── Cache read/write ──────────────────────────────────────────────────────

pub fn get_config(env: &Env) -> SimCacheConfig {
    env.storage()
        .instance()
        .get(&SimCacheKey::Config)
        .unwrap_or_else(SimCacheConfig::default)
}

pub fn set_config(env: &Env, config: SimCacheConfig) {
    env.storage()
        .instance()
        .set(&SimCacheKey::Config, &config);
}

/// Look up a cached simulation result.
pub fn cache_lookup(
    env: &Env,
    operation_type: u32,
    pool: &Address,
    user: &Address,
    asset: &Address,
    amount: i128,
) -> Option<SimulationResult> {
    let config = get_config(env);
    if !config.enabled {
        return None;
    }

    ensure_current_block(env);

    let key_hash = compute_key_hash(env, operation_type, pool, user, asset, amount);
    let count: u32 = env
        .storage()
        .instance()
        .get(&SimCacheKey::Count)
        .unwrap_or(0);

    // Linear scan for matching hash (small cache, acceptable).
    for i in 0..count {
        if let Some(entry) = env
            .storage()
            .instance()
            .get::<SimCacheKey, SimCacheEntry>(&SimCacheKey::Entry(i))
        {
            if entry.key_hash == key_hash {
                // Found — update LRU and return.
                let mut updated = entry.clone();
                updated.access_count = updated.access_count.saturating_add(1);
                env.storage()
                    .instance()
                    .set(&SimCacheKey::Entry(i), &updated);

                // Update stats.
                update_stats_hit(env);

                return Some(SimulationResult {
                    health_after: updated.health_after,
                    collateral_value_after: updated.collateral_value_after,
                    debt_value_after: updated.debt_value_after,
                    would_succeed: updated.would_succeed,
                    cached: true,
                });
            }
        }
    }

    // Not found.
    update_stats_miss(env);
    None
}

/// Insert a simulation result into the cache.
pub fn cache_insert(
    env: &Env,
    operation_type: u32,
    pool: &Address,
    user: &Address,
    asset: &Address,
    amount: i128,
    health_after: i128,
    collateral_value_after: i128,
    debt_value_after: i128,
    would_succeed: bool,
) -> Result<(), SimCacheError> {
    let config = get_config(env);
    if !config.enabled {
        return Ok(());
    }

    ensure_current_block(env);

    let key_hash = compute_key_hash(env, operation_type, pool, user, asset, amount);
    let count: u32 = env
        .storage()
        .instance()
        .get(&SimCacheKey::Count)
        .unwrap_or(0);

    // Check if entry already exists (update).
    for i in 0..count {
        if let Some(entry) = env
            .storage()
            .instance()
            .get::<SimCacheKey, SimCacheEntry>(&SimCacheKey::Entry(i))
        {
            if entry.key_hash == key_hash {
                let updated = SimCacheEntry {
                    key_hash,
                    operation_type,
                    pool: pool.clone(),
                    user: user.clone(),
                    asset: asset.clone(),
                    amount,
                    health_after,
                    collateral_value_after,
                    debt_value_after,
                    would_succeed,
                    cached_block: env.ledger().sequence(),
                    cached_at: env.ledger().timestamp(),
                    access_count: entry.access_count.saturating_add(1),
                };
                env.storage()
                    .instance()
                    .set(&SimCacheKey::Entry(i), &updated);
                return Ok(());
            }
        }
    }

    // New entry — check capacity.
    if count >= config.max_entries {
        // LRU eviction: find the entry with the lowest access_count.
        let mut min_idx = 0u32;
        let mut min_count = u64::MAX;

        for i in 0..count {
            if let Some(entry) = env
                .storage()
                .instance()
                .get::<SimCacheKey, SimCacheEntry>(&SimCacheKey::Entry(i))
            {
                if entry.access_count < min_count {
                    min_count = entry.access_count;
                    min_idx = i;
                }
            }
        }

        // Evict.
        env.storage()
            .instance()
            .remove(&SimCacheKey::Entry(min_idx));
        update_stats_eviction(env);

        // Swap the last entry into the evicted slot.
        let last_idx = count - 1;
        if last_idx != min_idx {
            if let Some(last_entry) = env
                .storage()
                .instance()
                .get::<SimCacheKey, SimCacheEntry>(&SimCacheKey::Entry(last_idx))
            {
                env.storage()
                    .instance()
                    .set(&SimCacheKey::Entry(min_idx), &last_entry);
                env.storage()
                    .instance()
                    .remove(&SimCacheKey::Entry(last_idx));
            }
        }

        // Insert at the end.
        let new_entry = SimCacheEntry {
            key_hash,
            operation_type,
            pool: pool.clone(),
            user: user.clone(),
            asset: asset.clone(),
            amount,
            health_after,
            collateral_value_after,
            debt_value_after,
            would_succeed,
            cached_block: env.ledger().sequence(),
            cached_at: env.ledger().timestamp(),
            access_count: 1,
        };
        env.storage()
            .instance()
            .set(&SimCacheKey::Entry(min_idx), &new_entry);
    } else {
        // Insert at next available slot.
        let new_entry = SimCacheEntry {
            key_hash,
            operation_type,
            pool: pool.clone(),
            user: user.clone(),
            asset: asset.clone(),
            amount,
            health_after,
            collateral_value_after,
            debt_value_after,
            would_succeed,
            cached_block: env.ledger().sequence(),
            cached_at: env.ledger().timestamp(),
            access_count: 1,
        };
        env.storage()
            .instance()
            .set(&SimCacheKey::Entry(count), &new_entry);
        env.storage()
            .instance()
            .set(&SimCacheKey::Count, &(count + 1));
    }

    Ok(())
}

/// Clear the entire cache.
pub fn cache_clear(env: &Env) {
    let count: u32 = env
        .storage()
        .instance()
        .get(&SimCacheKey::Count)
        .unwrap_or(0);

    for i in 0..count {
        env.storage()
            .instance()
            .remove(&SimCacheKey::Entry(i));
    }

    env.storage()
        .instance()
        .set(&SimCacheKey::Count, &0u32);
    env.storage().instance().set(
        &SimCacheKey::Stats,
        &SimCacheStats {
            hits: 0,
            misses: 0,
            evictions: 0,
            total_entries: 0,
            current_block: env.ledger().sequence(),
        },
    );
}

/// Get cache statistics.
pub fn get_stats(env: &Env) -> SimCacheStats {
    let mut stats: SimCacheStats = env
        .storage()
        .instance()
        .get(&SimCacheKey::Stats)
        .unwrap_or(SimCacheStats {
            hits: 0,
            misses: 0,
            evictions: 0,
            total_entries: 0,
            current_block: env.ledger().sequence(),
        });

    stats.total_entries = env
        .storage()
        .instance()
        .get(&SimCacheKey::Count)
        .unwrap_or(0);
    stats.current_block = env.ledger().sequence();

    stats
}

fn update_stats_hit(env: &Env) {
    let mut stats = get_stats(env);
    stats.hits = stats.hits.saturating_add(1);
    env.storage().instance().set(&SimCacheKey::Stats, &stats);
}

fn update_stats_miss(env: &Env) {
    let mut stats = get_stats(env);
    stats.misses = stats.misses.saturating_add(1);
    env.storage().instance().set(&SimCacheKey::Stats, &stats);
}

fn update_stats_eviction(env: &Env) {
    let mut stats = get_stats(env);
    stats.evictions = stats.evictions.saturating_add(1);
    env.storage().instance().set(&SimCacheKey::Stats, &stats);
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn default_config() {
        let config = SimCacheConfig::default();
        assert_eq!(config.max_entries, 64);
        assert!(config.enabled);
    }

    #[test]
    fn cache_miss_on_empty() {
        let env = Env::default();
        let pool = Address::generate(&env);
        let user = Address::generate(&env);
        let asset = Address::generate(&env);

        let result = cache_lookup(&env, 0, &pool, &user, &asset, 1000);
        assert!(result.is_none());
    }

    #[test]
    fn cache_insert_and_lookup() {
        let env = Env::default();
        let pool = Address::generate(&env);
        let user = Address::generate(&env);
        let asset = Address::generate(&env);

        let insert_result = cache_insert(
            &env,
            0,
            &pool,
            &user,
            &asset,
            1000,
            15000,
            10000,
            5000,
            true,
        );
        assert!(insert_result.is_ok());

        let result = cache_lookup(&env, 0, &pool, &user, &asset, 1000);
        assert!(result.is_some());
        let sim = result.unwrap();
        assert!(sim.cached);
        assert!(sim.would_succeed);
        assert_eq!(sim.health_after, 15000);
    }

    #[test]
    fn cache_stats() {
        let env = Env::default();
        let stats = get_stats(&env);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn cache_clear() {
        let env = Env::default();
        let pool = Address::generate(&env);
        let user = Address::generate(&env);
        let asset = Address::generate(&env);

        let _ = cache_insert(&env, 0, &pool, &user, &asset, 1000, 15000, 10000, 5000, true);

        cache_clear(&env);

        let result = cache_lookup(&env, 0, &pool, &user, &asset, 1000);
        assert!(result.is_none());
    }

    #[test]
    fn cache_disabled_config() {
        let env = Env::default();
        let pool = Address::generate(&env);
        let user = Address::generate(&env);
        let asset = Address::generate(&env);

        set_config(
            &env,
            SimCacheConfig {
                enabled: false,
                ..Default::default()
            },
        );

        let result = cache_lookup(&env, 0, &pool, &user, &asset, 1000);
        assert!(result.is_none());

        let insert_result = cache_insert(
            &env,
            0,
            &pool,
            &user,
            &asset,
            1000,
            15000,
            10000,
            5000,
            true,
        );
        assert!(insert_result.is_ok());

        // Should still miss since caching is disabled.
        let result = cache_lookup(&env, 0, &pool, &user, &asset, 1000);
        assert!(result.is_none());
    }
}
