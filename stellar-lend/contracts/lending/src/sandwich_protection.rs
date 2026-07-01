//! # Sandwich Attack Protection (issue #637)
//!
//! Protects lending pool transactions from sandwich attacks using a combination
//! of commit-reveal schemes, configurable execution delays, randomized batch
//! ordering, and sandwich pattern detection.
//!
//! ## Protection Levels
//!
//! | Level    | Commit-reveal | Delay | Batch ordering | Detection |
//! |----------|--------------|-------|----------------|-----------|
//! | None     | No           | 0     | FIFO           | No        |
//! | Basic    | No           | Yes   | Randomized     | Yes       |
//! | Max      | Yes          | Yes   | Randomized     | Yes       |

use soroban_sdk::{contracterror, contracttype, Address, Bytes, Crypto, Env, Hash, Vec};

use crate::borrow::get_admin;

// ── Errors ────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SandwichError {
    Unauthorized = 1,
    InvalidProtectionLevel = 2,
    CommitExpired = 3,
    CommitMismatch = 4,
    CommitNotFound = 5,
    DelayNotElapsed = 6,
    InvalidDelay = 7,
    Overflow = 8,
    CommitAlreadyRevealed = 9,
}

// ── Protection level ──────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ProtectionLevel {
    None = 0,
    Basic = 1,
    Max = 2,
}

// ── Storage keys ──────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum SandwichKey {
    /// Global configuration.
    Config,
    /// Pending commit: Hash → Commitment.
    Commitment(Hash),
    /// User protection level preference.
    UserProtection(Address),
    /// Batch execution order seed for the current block.
    BatchSeed,
    /// Pending transactions for current block ordering.
    PendingTx(u32),
    /// Pending transaction count.
    PendingCount,
    /// Sandwich detection alerts.
    DetectionLog,
}

// ── Configuration ─────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SandwichConfig {
    /// Minimum delay in ledger seconds between commit and reveal.
    pub min_delay_secs: u64,
    /// Maximum delay allowed for commit-reveal.
    pub max_delay_secs: u64,
    /// Commit expiry in seconds.
    pub commit_expiry_secs: u64,
    /// Large transaction threshold (above this, commit-reveal is recommended).
    pub large_tx_threshold: i128,
    /// Premium fee bps for sandwich reversal protection.
    pub premium_fee_bps: i128,
}

impl Default for SandwichConfig {
    fn default() -> Self {
        SandwichConfig {
            min_delay_secs: 1,
            max_delay_secs: 10,
            commit_expiry_secs: 60,
            large_tx_threshold: 100_000,
            premium_fee_bps: 50, // 0.5%
        }
    }
}

// ── Commitment ────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Commitment {
    pub owner: Address,
    pub operation_hash: Hash,
    pub asset: Address,
    pub amount: i128,
    pub created_at: u64,
    pub reveal_after: u64,
    pub revealed: bool,
    pub protection_level: ProtectionLevel,
}

// ── Pending transaction (for batch ordering) ──────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PendingTransaction {
    pub index: u32,
    pub owner: Address,
    pub operation_type: u32,
    pub asset: Address,
    pub amount: i128,
    pub submitted_at: u64,
    pub execute_after: u64,
    pub protection_level: ProtectionLevel,
    pub random_nonce: u64,
}

// ── Sandwich detection log ────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SandwichDetection {
    pub suspect_address: Address,
    pub victim_address: Address,
    pub asset: Address,
    pub block_number: u32,
    pub timestamp: u64,
    pub estimated_profit: i128,
    pub reversed: bool,
}

// ── Admin functions ───────────────────────────────────────────────────────

pub fn get_config(env: &Env) -> SandwichConfig {
    env.storage()
        .persistent()
        .get(&SandwichKey::Config)
        .unwrap_or_else(SandwichConfig::default)
}

pub fn set_config(
    env: &Env,
    admin: Address,
    config: SandwichConfig,
) -> Result<SandwichConfig, SandwichError> {
    admin.require_auth();
    let current_admin = get_admin(env).ok_or(SandwichError::Unauthorized)?;
    if admin != current_admin {
        return Err(SandwichError::Unauthorized);
    }

    if config.min_delay_secs == 0
        || config.min_delay_secs >= config.max_delay_secs
        || config.commit_expiry_secs == 0
    {
        return Err(SandwichError::InvalidDelay);
    }

    env.storage()
        .persistent()
        .set(&SandwichKey::Config, &config);
    Ok(config)
}

// ── User protection level ─────────────────────────────────────────────────

pub fn set_user_protection(
    env: &Env,
    user: Address,
    level: ProtectionLevel,
) -> Result<(), SandwichError> {
    user.require_auth();

    let config = get_config(env);
    if config.min_delay_secs == 0 && level != ProtectionLevel::None {
        // Delay-based protection is disabled globally.
    }

    env.storage()
        .persistent()
        .set(&SandwichKey::UserProtection(user.clone()), &(level as u8));

    Ok(())
}

pub fn get_user_protection(env: &Env, user: &Address) -> ProtectionLevel {
    let val: Option<u8> = env
        .storage()
        .persistent()
        .get(&SandwichKey::UserProtection(user.clone()));
    match val {
        Some(0) => ProtectionLevel::None,
        Some(1) => ProtectionLevel::Basic,
        Some(2) => ProtectionLevel::Max,
        _ => ProtectionLevel::Basic, // Default to Basic for safety
    }
}

// ── Commit-reveal (Max protection) ────────────────────────────────────────

pub fn commit_transaction(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    operation_type: u32,
) -> Result<Hash, SandwichError> {
    user.require_auth();

    let config = get_config(env);
    let now = env.ledger().timestamp();

    // Generate commitment hash from user + asset + amount + nonce.
    let crypto = Crypto::new(&env);
    let mut data = Bytes::new(env);
    data.extend_from_slice(&user.to_buffer());
    data.extend_from_slice(&asset.to_buffer());
    data.extend_from_slice(&(amount.to_be_bytes()));
    data.extend_from_slice(&(now.to_be_bytes()));
    data.extend_from_slice(&(operation_type.to_be_bytes()));

    let commit_hash = crypto.hash_sha256(&data);

    let commitment = Commitment {
        owner: user.clone(),
        operation_hash: commit_hash.clone(),
        asset: asset.clone(),
        amount,
        created_at: now,
        reveal_after: now.saturating_add(config.min_delay_secs),
        revealed: false,
        protection_level: ProtectionLevel::Max,
    };

    env.storage()
        .persistent()
        .set(&SandwichKey::Commitment(commit_hash.clone()), &commitment);

    Ok(commit_hash)
}

pub fn reveal_transaction(
    env: &Env,
    user: Address,
    commit_hash: Hash,
    nonce: u64,
) -> Result<(), SandwichError> {
    user.require_auth();

    let key = SandwichKey::Commitment(commit_hash.clone());
    let mut commitment: Commitment = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(SandwichError::CommitNotFound)?;

    if commitment.revealed {
        return Err(SandwichError::CommitAlreadyRevealed);
    }

    if commitment.owner != user {
        return Err(SandwichError::CommitMismatch);
    }

    let now = env.ledger().timestamp();
    let config = get_config(env);

    if now > commitment.created_at.saturating_add(config.commit_expiry_secs) {
        return Err(SandwichError::CommitExpired);
    }

    if now < commitment.reveal_after {
        return Err(SandwichError::DelayNotElapsed);
    }

    commitment.revealed = true;
    env.storage().persistent().set(&key, &commitment);

    Ok(())
}

// ── Batch execution ordering ──────────────────────────────────────────────

/// Submit a transaction to the pending batch.
pub fn submit_pending_transaction(
    env: &Env,
    user: Address,
    operation_type: u32,
    asset: Address,
    amount: i128,
    protection_level: ProtectionLevel,
) -> Result<u32, SandwichError> {
    user.require_auth();

    let config = get_config(env);
    let now = env.ledger().timestamp();
    let delay = match protection_level {
        ProtectionLevel::None => 0,
        ProtectionLevel::Basic => config.min_delay_secs,
        ProtectionLevel::Max => config.min_delay_secs.saturating_mul(2),
    };

    let count: u32 = env
        .storage()
        .instance()
        .get(&SandwichKey::PendingCount)
        .unwrap_or(0);

    // Simple pseudo-random nonce from timestamp + user hash.
    let crypto = Crypto::new(&env);
    let mut nonce_data = Bytes::new(env);
    nonce_data.extend_from_slice(&user.to_buffer());
    nonce_data.extend_from_slice(&(now.to_be_bytes()));
    nonce_data.extend_from_slice(&(count.to_be_bytes()));
    let nonce_hash = crypto.hash_sha256(&nonce_data);
    let nonce_bytes = nonce_hash.to_array();
    let random_nonce =
        u64::from_be_bytes([nonce_bytes[0], nonce_bytes[1], nonce_bytes[2], nonce_bytes[3], nonce_bytes[4], nonce_bytes[5], nonce_bytes[6], nonce_bytes[7]]);

    let pending = PendingTransaction {
        index: count,
        owner: user,
        operation_type,
        asset,
        amount,
        submitted_at: now,
        execute_after: now.saturating_add(delay),
        protection_level,
        random_nonce,
    };

    env.storage()
        .instance()
        .set(&SandwichKey::PendingTx(count), &pending);
    env.storage()
        .instance()
        .set(&SandwichKey::PendingCount, &(count + 1));

    Ok(count)
}

/// Get the randomized execution order for the current block's pending
/// transactions. Uses the random nonce for shuffle ordering.
pub fn get_execution_order(env: &Env) -> Vec<u32> {
    let count: u32 = env
        .storage()
        .instance()
        .get(&SandwichKey::PendingCount)
        .unwrap_or(0);

    let mut indices: Vec<u32> = Vec::new(env);
    let mut entries: Vec<PendingTransaction> = Vec::new(env);
    let now = env.ledger().timestamp();

    for i in 0..count {
        if let Some(tx) = env
            .storage()
            .instance()
            .get::<SandwichKey, PendingTransaction>(&SandwichKey::PendingTx(i))
        {
            if tx.execute_after <= now {
                entries.push_back(tx);
                indices.push_back(i);
            }
        }
    }

    // Sort by random_nonce for randomized ordering.
    // Simple insertion sort for small arrays (typical batch sizes).
    let len = indices.len();
    if len <= 1 {
        return indices;
    }

    // Collect (nonce, index) pairs and sort by nonce.
    let mut pairs: Vec<(u64, u32)> = Vec::new(env);
    for idx in indices.iter() {
        if let Some(tx) = env
            .storage()
            .instance()
            .get::<SandwichKey, PendingTransaction>(&SandwichKey::PendingTx(*idx))
        {
            pairs.push_back((tx.random_nonce, *idx));
        }
    }

    // Bubble sort by nonce (simple, sufficient for small batches).
    let pair_len = pairs.len();
    for i in 0..pair_len {
        for j in 0..pair_len - 1 - i {
            let a = pairs.get(j).unwrap();
            let b = pairs.get(j + 1).unwrap();
            if a.0 > b.0 {
                // Swap.
                let mut pairs_vec: soroban_sdk::Vec<(u64, u32)> = Vec::new(env);
                for k in 0..pair_len {
                    if k == j {
                        pairs_vec.push_back(b);
                    } else if k == j + 1 {
                        pairs_vec.push_back(a);
                    } else {
                        pairs_vec.push_back(pairs.get(k).unwrap());
                    }
                }
                pairs = pairs_vec;
            }
        }
    }

    let mut result: Vec<u32> = Vec::new(env);
    for pair in pairs.iter() {
        result.push_back(pair.1);
    }

    result
}

/// Clear pending transactions for the current block (called after batch execution).
pub fn clear_pending_transactions(env: &Env) {
    let count: u32 = env
        .storage()
        .instance()
        .get(&SandwichKey::PendingCount)
        .unwrap_or(0);

    for i in 0..count {
        env.storage()
            .instance()
            .remove(&SandwichKey::PendingTx(i));
    }

    env.storage()
        .instance()
        .set(&SandwichKey::PendingCount, &0u32);
}

// ── Sandwich detection ────────────────────────────────────────────────────

pub fn detect_sandwich_pattern(
    env: &Env,
    _suspect: &Address,
    _victim: &Address,
    _asset: &Address,
    _amount: i128,
) -> bool {
    // Pattern detection heuristics:
    // 1. Check if suspect had a transaction before and after victim in the same block.
    // 2. Check if the amounts are correlated (typical sandwich: same or similar amounts).
    // 3. Check if price impact would benefit the suspect.
    //
    // For on-chain detection, we rely on the pending transaction ordering and
    // historical patterns. Full mempool analysis happens off-chain.

    // Simplified on-chain check: if the suspect has pending txs both before
    // and after the victim in the execution order, flag it.
    // This is a heuristic; full detection is off-chain.

    false // Placeholder — real implementation would cross-reference pending txs
}

pub fn log_sandwich_detection(
    env: &Env,
    detection: SandwichDetection,
) {
    let mut log: Vec<SandwichDetection> = env
        .storage()
        .persistent()
        .get(&SandwichKey::DetectionLog)
        .unwrap_or(Vec::new(env));

    // Keep last 100 entries.
    if log.len() >= 100 {
        let mut new_log: Vec<SandwichDetection> = Vec::new(env);
        for i in 1..log.len() {
            if let Some(entry) = log.get(i) {
                new_log.push_back(entry);
            }
        }
        log = new_log;
    }

    log.push_back(detection);
    env.storage()
        .persistent()
        .set(&SandwichKey::DetectionLog, &log);
}

pub fn get_detection_log(env: &Env) -> Vec<SandwichDetection> {
    env.storage()
        .persistent()
        .get(&SandwichKey::DetectionLog)
        .unwrap_or(Vec::new(env))
}

// ── Premium protection: reverse sandwiched transaction ────────────────────

pub fn calculate_premium_fee(env: &Env, amount: i128) -> i128 {
    let config = get_config();
    amount
        .saturating_mul(config.premium_fee_bps)
        .saturating_div(10_000)
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn default_config() {
        let config = SandwichConfig::default();
        assert_eq!(config.min_delay_secs, 1);
        assert_eq!(config.max_delay_secs, 10);
        assert_eq!(config.commit_expiry_secs, 60);
        assert_eq!(config.large_tx_threshold, 100_000);
        assert_eq!(config.premium_fee_bps, 50);
    }

    #[test]
    fn user_protection_level_default_is_basic() {
        let env = Env::default();
        let user = Address::generate(&env);
        let level = get_user_protection(&env, &user);
        assert_eq!(level, ProtectionLevel::Basic);
    }

    #[test]
    fn set_user_protection_level() {
        let env = Env::default();
        let user = Address::generate(&env);

        let result = set_user_protection(&env, user.clone(), ProtectionLevel::Max);
        assert!(result.is_ok());

        let level = get_user_protection(&env, &user);
        assert_eq!(level, ProtectionLevel::Max);
    }

    #[test]
    fn commit_reveal_flow() {
        let env = Env::default();
        let user = Address::generate(&env);
        let asset = Address::generate(&env);

        // Commit.
        let hash = commit_transaction(&env, user.clone(), asset.clone(), 1000, 1).unwrap();
        assert!(!hash.to_array().is_empty());

        // Commitment should exist.
        let key = SandwichKey::Commitment(hash.clone());
        let commitment: Commitment = env.storage().persistent().get(&key).unwrap();
        assert_eq!(commitment.owner, user);
        assert!(!commitment.revealed);
    }

    #[test]
    fn protection_level_display() {
        assert_eq!(format!("{:?}", ProtectionLevel::None), "None");
        assert_eq!(format!("{:?}", ProtectionLevel::Basic), "Basic");
        assert_eq!(format!("{:?}", ProtectionLevel::Max), "Max");
    }

    #[test]
    fn pending_count_initializes_to_zero() {
        let env = Env::default();
        let count: u32 = env
            .storage()
            .instance()
            .get(&SandwichKey::PendingCount)
            .unwrap_or(0);
        assert_eq!(count, 0);
    }
}
