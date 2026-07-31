//! Insurance Pool Module
//!
//! Implements a self-funded insurance pool that:
//! - Accumulates funds from protocol fees (fee contributions)
//! - Provides dynamic coverage pricing per asset
//! - Handles claim submission and admin evaluation
//! - Enforces per-asset coverage limits
//! - Maintains an emergency fund allocation
//! - Exposes analytics for pool health monitoring

use soroban_sdk::{contractevent, contracttype, Address, Env, Vec};

// ─── Constants ───────────────────────────────────────────────────────────────

/// Maximum coverage limit per asset in basis points of pool balance (5000 = 50%)
const MAX_COVERAGE_LIMIT_BPS: i128 = 5000;
/// Emergency fund allocation in basis points of total pool (2000 = 20%)
const EMERGENCY_FUND_BPS: i128 = 2000;
/// Minimum premium rate in basis points (10 = 0.1%)
const MIN_PREMIUM_BPS: i128 = 10;
/// Maximum premium rate in basis points (500 = 5%)
const MAX_PREMIUM_BPS: i128 = 500;
/// Base premium rate in basis points (50 = 0.5%)
const BASE_PREMIUM_BPS: i128 = 50;
/// Scale factor for basis point calculations
const BPS_SCALE: i128 = 10_000;

// ─── Errors ──────────────────────────────────────────────────────────────────

#[soroban_sdk::contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum InsuranceError {
    /// Caller is not the protocol admin
    Unauthorized = 1,
    /// Amount is zero or negative
    InvalidAmount = 2,
    /// Claim amount exceeds per-asset coverage limit
    ExceedsCoverageLimit = 3,
    /// Claim ID does not exist
    ClaimNotFound = 4,
    /// Claim is not in Pending state
    ClaimNotPending = 5,
    /// Pool has insufficient funds to pay claim
    InsufficientPoolFunds = 6,
    /// Coverage limit bps out of range (0..=MAX_COVERAGE_LIMIT_BPS)
    InvalidCoverageLimit = 7,
    /// Pool not yet initialized
    NotInitialized = 8,
    /// Pool already initialized
    AlreadyInitialized = 9,
    /// Arithmetic overflow
    Overflow = 10,
}

// ─── Storage keys ────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum InsuranceKey {
    /// Admin address
    Admin,
    /// Total pool balance (all assets combined, in protocol units)
    PoolBalance,
    /// Emergency fund balance (subset of pool)
    EmergencyFund,
    /// Per-asset coverage limit in basis points
    CoverageLimit(Address),
    /// Per-asset premium rate in basis points (dynamic)
    PremiumRate(Address),
    /// Total claims paid out (analytics)
    TotalClaimsPaid,
    /// Total premiums collected (analytics)
    TotalPremiumsCollected,
    /// Total fee contributions received (analytics)
    TotalFeeContributions,
    /// Claim record by ID
    Claim(u64),
    /// Next claim ID counter
    NextClaimId,
    /// List of all claim IDs for history iteration
    ClaimsList,
}

// ─── Types ───────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ClaimStatus {
    Pending = 0,
    Approved = 1,
    Rejected = 2,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct InsuranceClaim {
    pub id: u64,
    pub claimant: Address,
    pub asset: Address,
    pub amount: i128,
    pub status: ClaimStatus,
    pub submitted_at: u64,
    pub resolved_at: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct InsuranceAnalytics {
    pub pool_balance: i128,
    pub emergency_fund: i128,
    pub total_claims_paid: i128,
    pub total_premiums_collected: i128,
    pub total_fee_contributions: i128,
    pub available_balance: i128,
}

// ─── Events ──────────────────────────────────────────────────────────────────

#[contractevent(topics = ["ins_funded"])]
#[derive(Clone, Debug)]
pub struct InsuranceFundedEvent {
    pub amount: i128,
    pub new_pool_balance: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["ins_claim_sub"])]
#[derive(Clone, Debug)]
pub struct InsuranceClaimSubmittedEvent {
    pub claim_id: u64,
    pub claimant: Address,
    pub asset: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["ins_claim_res"])]
#[derive(Clone, Debug)]
pub struct InsuranceClaimResolvedEvent {
    pub claim_id: u64,
    pub approved: bool,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["ins_cov_limit"])]
#[derive(Clone, Debug)]
pub struct InsuranceCoverageLimitSetEvent {
    pub asset: Address,
    pub limit_bps: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["ins_premium"])]
#[derive(Clone, Debug)]
pub struct InsurancePremiumCollectedEvent {
    pub payer: Address,
    pub asset: Address,
    pub premium: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["ins_claim_cancelled"])]
#[derive(Clone, Debug)]
pub struct InsuranceClaimCancelledEvent {
    pub claim_id: u64,
    pub claimant: Address,
    pub timestamp: u64,
}

#[contractevent(topics = ["ins_claim_history_cleared"])]
#[derive(Clone, Debug)]
pub struct InsuranceClaimHistoryClearedEvent {
    pub admin: Address,
    pub timestamp: u64,
}

// ─── Storage helpers ─────────────────────────────────────────────────────────

fn get_admin(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&InsuranceKey::Admin)
}

fn require_admin(env: &Env, caller: &Address) -> Result<(), InsuranceError> {
    let admin = get_admin(env).ok_or(InsuranceError::NotInitialized)?;
    if *caller != admin {
        return Err(InsuranceError::Unauthorized);
    }
    caller.require_auth();
    Ok(())
}

fn get_pool_balance(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&InsuranceKey::PoolBalance)
        .unwrap_or(0)
}

fn set_pool_balance(env: &Env, balance: i128) {
    env.storage()
        .persistent()
        .set(&InsuranceKey::PoolBalance, &balance);
}

fn get_emergency_fund(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&InsuranceKey::EmergencyFund)
        .unwrap_or(0)
}

fn set_emergency_fund(env: &Env, amount: i128) {
    env.storage()
        .persistent()
        .set(&InsuranceKey::EmergencyFund, &amount);
}

fn get_next_claim_id(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&InsuranceKey::NextClaimId)
        .unwrap_or(1)
}

fn increment_claim_id(env: &Env) -> u64 {
    let id = get_next_claim_id(env);
    env.storage()
        .persistent()
        .set(&InsuranceKey::NextClaimId, &(id + 1));
    id
}

fn get_claim(env: &Env, id: u64) -> Option<InsuranceClaim> {
    env.storage().persistent().get(&InsuranceKey::Claim(id))
}

fn save_claim(env: &Env, claim: &InsuranceClaim) {
    env.storage()
        .persistent()
        .set(&InsuranceKey::Claim(claim.id), claim);
}

fn get_coverage_limit_bps(env: &Env, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&InsuranceKey::CoverageLimit(asset.clone()))
        .unwrap_or(MAX_COVERAGE_LIMIT_BPS)
}

fn get_premium_rate_bps(env: &Env, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&InsuranceKey::PremiumRate(asset.clone()))
        .unwrap_or(BASE_PREMIUM_BPS)
}

fn add_to_analytics(env: &Env, key: InsuranceKey, amount: i128) {
    let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
    env.storage()
        .persistent()
        .set(&key, &current.saturating_add(amount));
}

// ─── Dynamic premium calculation ─────────────────────────────────────────────

/// Calculates dynamic premium based on pool utilisation.
///
/// Formula: `premium_bps = BASE + (utilisation_bps * (MAX - BASE) / BPS_SCALE)`
/// where `utilisation_bps = claims_paid * BPS_SCALE / pool_balance`.
///
/// Higher utilisation → higher premium to replenish the pool.
fn compute_dynamic_premium(env: &Env) -> i128 {
    let pool = get_pool_balance(env);
    if pool == 0 {
        return MAX_PREMIUM_BPS;
    }
    let claims_paid: i128 = env
        .storage()
        .persistent()
        .get(&InsuranceKey::TotalClaimsPaid)
        .unwrap_or(0);

    let utilisation_bps = claims_paid
        .saturating_mul(BPS_SCALE)
        .checked_div(pool)
        .unwrap_or(BPS_SCALE);

    let dynamic = BASE_PREMIUM_BPS
        .saturating_add(
            utilisation_bps
                .saturating_mul(MAX_PREMIUM_BPS - BASE_PREMIUM_BPS)
                .checked_div(BPS_SCALE)
                .unwrap_or(0),
        )
        .clamp(MIN_PREMIUM_BPS, MAX_PREMIUM_BPS);

    dynamic
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Initialize the insurance pool. Must be called once by the admin.
pub fn initialize(env: &Env, admin: &Address) -> Result<(), InsuranceError> {
    if get_admin(env).is_some() {
        return Err(InsuranceError::AlreadyInitialized);
    }
    admin.require_auth();
    env.storage().persistent().set(&InsuranceKey::Admin, admin);
    Ok(())
}

/// Contribute protocol fees to the insurance pool.
/// Updates pool balance and allocates emergency fund share.
pub fn fund_pool(env: &Env, amount: i128) -> Result<(), InsuranceError> {
    if get_admin(env).is_none() {
        return Err(InsuranceError::NotInitialized);
    }
    if amount <= 0 {
        return Err(InsuranceError::InvalidAmount);
    }

    let new_balance = get_pool_balance(env)
        .checked_add(amount)
        .ok_or(InsuranceError::Overflow)?;
    set_pool_balance(env, new_balance);

    // Allocate emergency fund share
    let emergency_share = amount
        .saturating_mul(EMERGENCY_FUND_BPS)
        .checked_div(BPS_SCALE)
        .unwrap_or(0);
    let new_emergency = get_emergency_fund(env)
        .checked_add(emergency_share)
        .ok_or(InsuranceError::Overflow)?;
    set_emergency_fund(env, new_emergency);

    add_to_analytics(env, InsuranceKey::TotalFeeContributions, amount);

    InsuranceFundedEvent {
        amount,
        new_pool_balance: new_balance,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Collect a coverage premium from a user for a given asset.
/// Premium is dynamically priced based on pool utilisation.
/// Returns the premium amount charged.
pub fn collect_premium(
    env: &Env,
    payer: Address,
    asset: Address,
    coverage_amount: i128,
) -> Result<i128, InsuranceError> {
    if get_admin(env).is_none() {
        return Err(InsuranceError::NotInitialized);
    }
    if coverage_amount <= 0 {
        return Err(InsuranceError::InvalidAmount);
    }
    payer.require_auth();

    // Update dynamic premium rate for this asset
    let dynamic_rate = compute_dynamic_premium(env);
    env.storage()
        .persistent()
        .set(&InsuranceKey::PremiumRate(asset.clone()), &dynamic_rate);

    let premium = coverage_amount
        .saturating_mul(dynamic_rate)
        .checked_div(BPS_SCALE)
        .unwrap_or(0);

    // Add premium to pool
    let new_balance = get_pool_balance(env)
        .checked_add(premium)
        .ok_or(InsuranceError::Overflow)?;
    set_pool_balance(env, new_balance);

    add_to_analytics(env, InsuranceKey::TotalPremiumsCollected, premium);

    InsurancePremiumCollectedEvent {
        payer: payer.clone(),
        asset: asset.clone(),
        premium,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(premium)
}

/// Submit an insurance claim. Returns the new claim ID.
pub fn submit_claim(
    env: &Env,
    claimant: Address,
    asset: Address,
    amount: i128,
) -> Result<u64, InsuranceError> {
    if get_admin(env).is_none() {
        return Err(InsuranceError::NotInitialized);
    }
    if amount <= 0 {
        return Err(InsuranceError::InvalidAmount);
    }
    claimant.require_auth();

    // Enforce per-asset coverage limit
    let pool = get_pool_balance(env);
    let limit_bps = get_coverage_limit_bps(env, &asset);
    let max_payout = pool
        .saturating_mul(limit_bps)
        .checked_div(BPS_SCALE)
        .unwrap_or(0);

    if amount > max_payout {
        return Err(InsuranceError::ExceedsCoverageLimit);
    }

    let id = increment_claim_id(env);
    let claim = InsuranceClaim {
        id,
        claimant: claimant.clone(),
        asset: asset.clone(),
        amount,
        status: ClaimStatus::Pending,
        submitted_at: env.ledger().timestamp(),
        resolved_at: 0,
    };
    save_claim(env, &claim);

    // Track in claims list for history
    let mut all_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&InsuranceKey::ClaimsList)
        .unwrap_or_else(|| Vec::new(env));
    all_ids.push_back(id);
    env.storage()
        .persistent()
        .set(&InsuranceKey::ClaimsList, &all_ids);

    InsuranceClaimSubmittedEvent {
        claim_id: id,
        claimant,
        asset,
        amount,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(id)
}

/// Evaluate a pending claim (admin only).
/// If approved, deducts from pool balance.
pub fn evaluate_claim(
    env: &Env,
    admin: Address,
    claim_id: u64,
    approve: bool,
) -> Result<(), InsuranceError> {
    require_admin(env, &admin)?;

    let mut claim = get_claim(env, claim_id).ok_or(InsuranceError::ClaimNotFound)?;

    if claim.status != ClaimStatus::Pending {
        return Err(InsuranceError::ClaimNotPending);
    }

    if approve {
        let pool = get_pool_balance(env);
        // Protect emergency fund — only pay from available (non-emergency) balance
        let emergency = get_emergency_fund(env);
        let available = pool.saturating_sub(emergency);

        if claim.amount > available {
            return Err(InsuranceError::InsufficientPoolFunds);
        }

        let new_balance = pool
            .checked_sub(claim.amount)
            .ok_or(InsuranceError::Overflow)?;
        set_pool_balance(env, new_balance);

        add_to_analytics(env, InsuranceKey::TotalClaimsPaid, claim.amount);

        claim.status = ClaimStatus::Approved;
    } else {
        claim.status = ClaimStatus::Rejected;
    }

    claim.resolved_at = env.ledger().timestamp();
    save_claim(env, &claim);

    InsuranceClaimResolvedEvent {
        claim_id,
        approved: approve,
        amount: claim.amount,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Set per-asset coverage limit in basis points (admin only).
pub fn set_coverage_limit(
    env: &Env,
    admin: Address,
    asset: Address,
    limit_bps: i128,
) -> Result<(), InsuranceError> {
    require_admin(env, &admin)?;

    if limit_bps < 0 || limit_bps > MAX_COVERAGE_LIMIT_BPS {
        return Err(InsuranceError::InvalidCoverageLimit);
    }

    env.storage()
        .persistent()
        .set(&InsuranceKey::CoverageLimit(asset.clone()), &limit_bps);

    InsuranceCoverageLimitSetEvent {
        asset,
        limit_bps,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Cancel a pending claim (claimant only).
pub fn cancel_claim(env: &Env, claimant: Address, claim_id: u64) -> Result<(), InsuranceError> {
    claimant.require_auth();

    let mut claim = get_claim(env, claim_id).ok_or(InsuranceError::ClaimNotFound)?;

    if claim.claimant != claimant {
        return Err(InsuranceError::Unauthorized);
    }

    if claim.status != ClaimStatus::Pending {
        return Err(InsuranceError::ClaimNotPending);
    }

    claim.status = ClaimStatus::Rejected;
    claim.resolved_at = env.ledger().timestamp();
    save_claim(env, &claim);

    InsuranceClaimCancelledEvent {
        claim_id,
        claimant,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Get all claim IDs for history tracking.
pub fn get_all_claim_ids(env: &Env) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&InsuranceKey::ClaimsList)
        .unwrap_or_else(|| Vec::new(env))
}

/// Get all claims (full history).
pub fn get_all_claims(env: &Env) -> Vec<InsuranceClaim> {
    let ids: Vec<u64> = get_all_claim_ids(env);
    let mut claims = Vec::new(env);
    for id in ids.iter() {
        if let Some(claim) = get_claim(env, id) {
            claims.push_back(claim);
        }
    }
    claims
}

/// Get a claim by ID.
pub fn get_claim_by_id(env: &Env, claim_id: u64) -> Option<InsuranceClaim> {
    get_claim(env, claim_id)
}

/// Get current dynamic premium rate for an asset (in basis points).
pub fn get_premium_rate(env: &Env, asset: &Address) -> i128 {
    get_premium_rate_bps(env, asset)
}

/// Get per-asset coverage limit in basis points.
pub fn get_coverage_limit(env: &Env, asset: &Address) -> i128 {
    get_coverage_limit_bps(env, asset)
}

/// Get insurance pool analytics.
pub fn get_analytics(env: &Env) -> InsuranceAnalytics {
    let pool_balance = get_pool_balance(env);
    let emergency_fund = get_emergency_fund(env);
    let total_claims_paid = env
        .storage()
        .persistent()
        .get(&InsuranceKey::TotalClaimsPaid)
        .unwrap_or(0);
    let total_premiums_collected = env
        .storage()
        .persistent()
        .get(&InsuranceKey::TotalPremiumsCollected)
        .unwrap_or(0);
    let total_fee_contributions = env
        .storage()
        .persistent()
        .get(&InsuranceKey::TotalFeeContributions)
        .unwrap_or(0);
    let available_balance = pool_balance.saturating_sub(emergency_fund);

    InsuranceAnalytics {
        pool_balance,
        emergency_fund,
        total_claims_paid,
        total_premiums_collected,
        total_fee_contributions,
        available_balance,
    }
}
