use soroban_sdk::{contracttype, Address, Env, IntoVal, TryFromVal, Val, Vec};

#[soroban_sdk::contracttype]
pub struct SnapshotValue {
    pub value: Val,
    pub timestamp: u64,
}

/// Get a value from persistent storage, optionally bypassing an in-memory cache layer.
/// Returns `None` when `force_direct` is false (caller is expected to serve from cache).
/// Returns the stored value when `force_direct` is true.
pub fn get_snapshot<K, T>(env: &Env, key: &K, force_direct: bool) -> Option<T>
where
    K: IntoVal<Env, Val> + TryFromVal<Env, Val>,
    T: IntoVal<Env, Val> + TryFromVal<Env, Val>,
{
    if force_direct {
        return env.storage().persistent().get::<K, T>(key);
    }
    None
}

// ─── Guardian config ──────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug)]
pub struct GuardianConfig {
    pub guardians: Vec<Address>,
    pub threshold: u32,
}

// ─── Governance storage keys ──────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum GovernanceDataKey {
    // Core governance config
    Admin,
    Config,
    NextProposalId,
    // Multisig
    MultisigConfig,
    MultisigAdmins,
    // Guardian / recovery
    GuardianConfig,
    Guardians,
    GuardianThreshold,
    // Proposals
    Proposal(u64),
    UserProposals(Address, u64),
    ProposalApprovals(u64),
    // Votes
    Vote(u64, Address),
    VotePowerSnapshot(u64, Address),
    VoteLock(Address),
    // Delegation
    DelegationRecord(Address),
    // Recovery
    RecoveryRequest,
    RecoveryApprovals,
    // Analytics
    GovernanceAnalytics,
    // Caches
    ProposalSimulationCache(u64),
    ProposalDryRunCache(u64),
    ParameterOptimizationCache,
    // Rate limiting
    ProposalWindowStart(Address),
    ProposalCreationCount(Address),
    // Timelock
    TimelockConfig,
    NextTimelockId,
    TimelockOperation(u64),
    TimelockQueue,
    /// #674 — per-action-type timelock delay override: ActionTypeDelay(action_type_id) -> u64
    ActionTypeDelay(u32),
    /// #674 — guardian approvals collected for an emergency override of a queued
    /// timelock operation: TimelockEmergencyApprovals(operation_id) -> Vec<Address>
    TimelockEmergencyApprovals(u64),
}

// ─── General data keys (used by credit score and other modules) ───────────────

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    CreditScore(Address),
}

// ─── Temporary transaction-local cache keys ─────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum TempDataKey {
    TokenBalanceCache(Address, Address),
    LendingIndexCache,
}

pub fn get_temp_token_balance(env: &Env, token: &Address, owner: &Address) -> Option<i128> {
    env.storage()
        .temporary()
        .get::<TempDataKey, i128>(&TempDataKey::TokenBalanceCache(
            token.clone(),
            owner.clone(),
        ))
}

pub fn set_temp_token_balance(env: &Env, token: &Address, owner: &Address, balance: i128) {
    env.storage()
        .temporary()
        .set(&TempDataKey::TokenBalanceCache(token.clone(), owner.clone()), &balance);
}

pub fn get_temp_lending_index(env: &Env) -> Option<crate::interest_rate::LendingIndex> {
    env.storage()
        .temporary()
        .get::<TempDataKey, crate::interest_rate::LendingIndex>(&TempDataKey::LendingIndexCache)
}

pub fn set_temp_lending_index(env: &Env, index: crate::interest_rate::LendingIndex) {
    env.storage()
        .temporary()
        .set(&TempDataKey::LendingIndexCache, &index);
}

// ─── Temporary transaction-local cache keys ─────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum TempDataKey {
    TokenBalanceCache(Address, Address),
}

pub fn get_temp_token_balance(env: &Env, token: &Address, owner: &Address) -> Option<i128> {
    env.storage()
        .temporary()
        .get::<TempDataKey, i128>(&TempDataKey::TokenBalanceCache(
            token.clone(),
            owner.clone(),
        ))
}

pub fn set_temp_token_balance(env: &Env, token: &Address, owner: &Address, balance: i128) {
    env.storage()
        .temporary()
        .set(&TempDataKey::TokenBalanceCache(token.clone(), owner.clone()), &balance);
}
