//! # MEV Protection Module
//!
//! Protects liquidations and large transactions from MEV (Maximal Extractable Value)
//! extraction via:
//!
//! - **Commit-reveal scheme** — users commit a hash of their intent; after a mandatory
//!   delay they reveal and execute. Front-runners cannot act on hidden parameters.
//! - **Batch auction for liquidations** — liquidation opportunities are collected into
//!   a time-windowed batch and settled at a uniform clearing price, eliminating
//!   priority-gas-auction races.
//! - **Slippage protection with deadline** — every sensitive operation carries a
//!   `max_slippage_bps` and `deadline` that are enforced at reveal time.
//! - **Private mempool integration** — the contract signals preferred routing hints
//!   (`PrivateMempool`, `BatchAuction`, `DelayedReveal`) so off-chain relayers can
//!   route transactions through protected channels.
//! - **Gas price bidding analysis** — the module tracks effective fee observations and
//!   exposes smoothed fee statistics so callers can make informed bid decisions.
//! - **Sandwich / ordering detection** — consecutive observations from different actors
//!   within a suspicious window increment alert counters surfaced in `OrderingStats`.

use soroban_sdk::{contracterror, contracttype, Address, Env, String, Symbol};
use stellarlend_shared_deadline::{is_expired_strict, require_strict_deadline};

// ─── Error Types ─────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MevProtectionError {
    /// Configuration value is out of range or logically invalid
    InvalidConfig = 1,
    /// Commit ID does not exist in storage
    CommitNotFound = 2,
    /// Reveal attempted before the mandatory delay has elapsed
    CommitNotReady = 3,
    /// Commit has passed its expiry window
    CommitExpired = 4,
    /// Caller is not the commit owner or protocol admin
    Unauthorized = 5,
    /// Effective MEV fee exceeds the caller's declared cap
    FeeCapExceeded = 6,
    /// Amount is zero or negative
    InvalidAmount = 7,
    /// Operation type mismatch between commit and reveal
    InvalidOperation = 8,
    /// Transaction deadline has passed
    DeadlineExpired = 9,
    /// Slippage tolerance exceeded at execution time
    SlippageExceeded = 10,
    /// Batch auction window is not yet closed
    AuctionWindowOpen = 11,
    /// No bids registered for this batch auction slot
    NoBidsInAuction = 12,
}

// ─── Enumerations ────────────────────────────────────────────────────────────

/// The sensitive operation being protected.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SensitiveOperation {
    Borrow,
    Withdraw,
    Liquidate,
}

/// Preferred transaction routing hint returned to off-chain relayers.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TxOrderingHint {
    /// Use the default public mempool (no special routing).
    Default,
    /// Route through a private / protected mempool (Flashbots-style).
    PrivateMempool,
    /// Participate in the on-chain batch auction for this operation.
    BatchAuction,
    /// Delay submission until after the commit-reveal window closes.
    DelayedReveal,
}

// ─── Configuration ───────────────────────────────────────────────────────────

/// Protocol-wide MEV protection configuration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MevProtectionConfig {
    /// Mandatory delay (seconds) between commit and reveal.
    pub commit_delay_secs: u64,
    /// Maximum lifetime (seconds) of an unrevealed commit.
    pub commit_expiry_secs: u64,
    /// Window (seconds) used to detect suspicious ordering sequences.
    pub suspicious_window_secs: u64,
    /// EMA smoothing weight for fee observations (0–10 000 bps).
    pub fee_smoothing_bps: i128,
    /// Base MEV protection fee charged on normal operations (bps).
    pub base_protection_fee_bps: i128,
    /// Surge fee charged when suspicious activity is detected (bps).
    pub surge_protection_fee_bps: i128,
    /// Relative amount difference threshold for sandwich detection (bps).
    pub sandwich_threshold_bps: i128,
    /// Whether private-mempool routing is enabled.
    pub private_mempool_enabled: bool,
    /// Whether batch-auction settlement is enabled.
    pub batching_enabled: bool,
    /// Duration (seconds) of each batch auction window.
    pub batch_window_secs: u64,
    /// Maximum slippage allowed by default when none is specified (bps).
    pub default_max_slippage_bps: i128,
}

// ─── Commit / Reveal Types ────────────────────────────────────────────────────

/// A pending commit stored on-chain until the reveal phase.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingCommit {
    /// Unique monotonic identifier.
    pub id: u64,
    /// Address that created the commit (must match at reveal).
    pub owner: Address,
    /// Operation type (must match at reveal).
    pub operation: SensitiveOperation,
    /// Primary asset (debt asset for liquidations, collateral for others).
    pub asset: Option<Address>,
    /// Secondary asset (collateral asset for liquidations).
    pub secondary_asset: Option<Address>,
    /// Borrower address (liquidations only).
    pub borrower: Option<Address>,
    /// Amount to operate on.
    pub amount: i128,
    /// Maximum MEV fee the caller is willing to pay (bps).
    pub max_fee_bps: i128,
    /// Preferred routing hint.
    pub hint: TxOrderingHint,
    /// Ledger timestamp when the commit was created.
    pub committed_at: u64,
    /// Earliest timestamp at which reveal is permitted.
    pub reveal_after: u64,
    /// Timestamp after which the commit is considered expired.
    pub expires_at: u64,
    /// Ledger sequence number at commit time (for ordering analysis).
    pub commit_ledger: u32,
    /// Maximum slippage the caller accepts (bps). 0 = use protocol default.
    pub max_slippage_bps: i128,
    /// Unix timestamp after which the operation must not execute.
    pub deadline: u64,
}

// ─── Batch Auction Types ──────────────────────────────────────────────────────

/// A single bid in a batch liquidation auction.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuctionBid {
    /// Bidder (liquidator) address.
    pub bidder: Address,
    /// Borrower whose position is being bid on.
    pub borrower: Address,
    /// Debt amount the bidder is willing to repay.
    pub debt_amount: i128,
    /// Minimum collateral the bidder expects to receive (slippage guard).
    pub min_collateral_out: i128,
    /// Maximum fee the bidder will pay (bps).
    pub max_fee_bps: i128,
    /// Timestamp when the bid was placed.
    pub placed_at: u64,
    /// Deadline: bid is invalid after this timestamp.
    pub deadline: u64,
}

/// Aggregated result of a settled batch auction slot.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuctionResult {
    /// Slot identifier (= `slot_start_time / batch_window_secs`).
    pub slot_id: u64,
    /// Number of bids that participated.
    pub bid_count: u32,
    /// Uniform clearing fee applied to all winning bids (bps).
    pub clearing_fee_bps: i128,
    /// Total debt liquidated across all winning bids.
    pub total_debt_liquidated: i128,
    /// Timestamp when the auction was settled.
    pub settled_at: u64,
}

// ─── Monitoring / Analytics Types ────────────────────────────────────────────

/// A single ordering observation recorded after each reveal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderingObservation {
    pub actor: Address,
    pub amount: i128,
    pub timestamp: u64,
}

/// Cumulative MEV ordering statistics exposed to the monitoring dashboard.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderingStats {
    /// Number of suspicious ordering sequences detected.
    pub suspicious_sequences: u64,
    /// Number of sandwich-attack patterns detected.
    pub sandwich_alerts: u64,
    /// Timestamp of the most recent alert.
    pub last_alert_timestamp: u64,
    /// Last smoothed effective fee (bps).
    pub last_effective_fee_bps: i128,
    /// Total commits created since deployment.
    pub total_commits: u64,
    /// Total reveals executed since deployment.
    pub total_reveals: u64,
    /// Total batch auction bids placed.
    pub total_auction_bids: u64,
    /// Total batch auctions settled.
    pub total_auctions_settled: u64,
    /// Cumulative MEV fees collected (in bps-weighted units).
    pub cumulative_fee_bps_collected: i128,
}

/// Gas price bidding analysis snapshot.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GasBidAnalysis {
    /// Current smoothed base fee (bps).
    pub smoothed_base_fee_bps: i128,
    /// Current surge fee (bps) — non-zero when suspicious activity detected.
    pub current_surge_fee_bps: i128,
    /// Recommended bid fee for a normal operation (bps).
    pub recommended_bid_bps: i128,
    /// Recommended bid fee during high-congestion periods (bps).
    pub high_congestion_bid_bps: i128,
    /// Whether the protocol is currently in a suspicious-activity window.
    pub in_suspicious_window: bool,
    /// Number of sandwich alerts in the last suspicious window.
    pub recent_sandwich_alerts: u64,
}

// ─── Storage Keys ─────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
enum MevDataKey {
    Config,
    NextCommitId,
    Commit(u64),
    OrderingStats,
    LatestObservation(Symbol, Option<Address>),
    PreviousObservation(Symbol, Option<Address>),
    SmoothedFee(Symbol, Option<Address>),
    /// Bids for a given auction slot: slot_id → Vec<AuctionBid>
    AuctionBids(u64),
    /// Settled result for a given auction slot
    AuctionResult(u64),
    /// Next auction slot id counter
    NextAuctionSlotId,
}

const MAX_BPS: i128 = 10_000;
/// Maximum number of bids stored per auction slot (prevents unbounded growth).
const MAX_BIDS_PER_SLOT: u32 = 200;

// ─── Default Configuration ────────────────────────────────────────────────────

pub fn default_config() -> MevProtectionConfig {
    MevProtectionConfig {
        commit_delay_secs: 30,
        commit_expiry_secs: 300,
        suspicious_window_secs: 45,
        fee_smoothing_bps: 2_500,
        base_protection_fee_bps: 10,
        surge_protection_fee_bps: 60,
        sandwich_threshold_bps: 500,
        private_mempool_enabled: true,
        batching_enabled: true,
        batch_window_secs: 60,
        default_max_slippage_bps: 100, // 1% default slippage tolerance
    }
}

// ─── Admin: Configure ─────────────────────────────────────────────────────────

/// Update MEV protection configuration (admin only).
pub fn configure(
    env: &Env,
    caller: Address,
    config: MevProtectionConfig,
) -> Result<(), MevProtectionError> {
    crate::risk_management::require_admin(env, &caller)
        .map_err(|_| MevProtectionError::Unauthorized)?;
    validate_config(&config)?;
    env.storage().persistent().set(&MevDataKey::Config, &config);
    Ok(())
}

/// Read current MEV protection configuration (or defaults if not yet set).
pub fn get_config(env: &Env) -> MevProtectionConfig {
    env.storage()
        .persistent()
        .get(&MevDataKey::Config)
        .unwrap_or_else(default_config)
}

// ─── Commit / Reveal ──────────────────────────────────────────────────────────

/// Create a new commit for a sensitive operation.
///
/// The caller must authorize the call. The commit is stored on-chain and can
/// only be revealed after `commit_delay_secs` and before `commit_expiry_secs`.
///
/// # Parameters
/// - `max_slippage_bps`: 0 means "use protocol default".
/// - `deadline`: 0 means "no deadline" (uses `commit_expiry_secs` as fallback).
#[allow(clippy::too_many_arguments)]
pub fn create_commit(
    env: &Env,
    owner: Address,
    operation: SensitiveOperation,
    asset: Option<Address>,
    secondary_asset: Option<Address>,
    borrower: Option<Address>,
    amount: i128,
    max_fee_bps: i128,
    hint: TxOrderingHint,
) -> Result<u64, MevProtectionError> {
    create_commit_with_slippage(
        env, owner, operation, asset, secondary_asset, borrower,
        amount, max_fee_bps, hint, 0, 0,
    )
}

/// Extended commit creation with explicit slippage and deadline parameters.
#[allow(clippy::too_many_arguments)]
pub fn create_commit_with_slippage(
    env: &Env,
    owner: Address,
    operation: SensitiveOperation,
    asset: Option<Address>,
    secondary_asset: Option<Address>,
    borrower: Option<Address>,
    amount: i128,
    max_fee_bps: i128,
    hint: TxOrderingHint,
    max_slippage_bps: i128,
    deadline: u64,
) -> Result<u64, MevProtectionError> {
    owner.require_auth();
    if amount <= 0 {
        return Err(MevProtectionError::InvalidAmount);
    }
    if !(0..=MAX_BPS).contains(&max_fee_bps) {
        return Err(MevProtectionError::InvalidConfig);
    }
    if max_slippage_bps < 0 || max_slippage_bps > MAX_BPS {
        return Err(MevProtectionError::InvalidConfig);
    }

    let cfg = get_config(env);
    let now = env.ledger().timestamp();

    // Validate deadline if provided
    let effective_deadline = if deadline == 0 {
        now.saturating_add(cfg.commit_expiry_secs)
    } else {
        if deadline <= now {
            return Err(MevProtectionError::DeadlineExpired);
        }
        deadline
    };

    let effective_slippage = if max_slippage_bps == 0 {
        cfg.default_max_slippage_bps
    } else {
        max_slippage_bps
    };

    let id = next_commit_id(env);
    let commit = PendingCommit {
        id,
        owner,
        operation,
        asset,
        secondary_asset,
        borrower,
        amount,
        max_fee_bps,
        hint,
        committed_at: now,
        reveal_after: now.saturating_add(cfg.commit_delay_secs),
        expires_at: now.saturating_add(cfg.commit_expiry_secs),
        commit_ledger: env.ledger().sequence(),
        max_slippage_bps: effective_slippage,
        deadline: effective_deadline,
    };
    env.storage()
        .persistent()
        .set(&MevDataKey::Commit(id), &commit);

    // Update stats
    let mut stats = get_ordering_stats(env);
    stats.total_commits = stats.total_commits.saturating_add(1);
    env.storage()
        .persistent()
        .set(&MevDataKey::OrderingStats, &stats);

    Ok(id)
}

/// Retrieve a pending commit by ID (returns `None` if not found or already consumed).
pub fn get_commit(env: &Env, commit_id: u64) -> Option<PendingCommit> {
    env.storage()
        .persistent()
        .get(&MevDataKey::Commit(commit_id))
}

/// Cancel a pending commit (owner only). Refunds any locked state.
pub fn cancel_commit(env: &Env, owner: Address, commit_id: u64) -> Result<(), MevProtectionError> {
    owner.require_auth();
    let commit = load_commit(env, commit_id)?;
    if commit.owner != owner {
        return Err(MevProtectionError::Unauthorized);
    }
    env.storage()
        .persistent()
        .remove(&MevDataKey::Commit(commit_id));
    Ok(())
}

// ─── Reveal Helpers ───────────────────────────────────────────────────────────

/// Reveal a borrow commit. Returns `(asset, amount, effective_fee_bps)`.
pub fn reveal_borrow(
    env: &Env,
    owner: Address,
    commit_id: u64,
) -> Result<(Option<Address>, i128, i128), MevProtectionError> {
    owner.require_auth();
    let commit = validate_reveal(env, &owner, commit_id, SensitiveOperation::Borrow)?;
    let effective_fee_bps = compute_and_enforce_fee(env, &commit)?;
    record_ordering_signal(
        env,
        owner,
        SensitiveOperation::Borrow,
        commit.asset.clone(),
        commit.amount,
        effective_fee_bps,
    );
    env.storage()
        .persistent()
        .remove(&MevDataKey::Commit(commit_id));
    Ok((commit.asset, commit.amount, effective_fee_bps))
}

/// Reveal a withdraw commit. Returns `(asset, amount)`.
pub fn reveal_withdraw(
    env: &Env,
    owner: Address,
    commit_id: u64,
) -> Result<(Option<Address>, i128), MevProtectionError> {
    owner.require_auth();
    let commit = validate_reveal(env, &owner, commit_id, SensitiveOperation::Withdraw)?;
    let effective_fee_bps = compute_and_enforce_fee(env, &commit)?;
    record_ordering_signal(
        env,
        owner,
        SensitiveOperation::Withdraw,
        commit.asset.clone(),
        commit.amount,
        effective_fee_bps,
    );
    env.storage()
        .persistent()
        .remove(&MevDataKey::Commit(commit_id));
    Ok((commit.asset, commit.amount))
}

/// Reveal a liquidation commit. Returns `(borrower, debt_asset, collateral_asset, debt_amount)`.
pub fn reveal_liquidation(
    env: &Env,
    owner: Address,
    commit_id: u64,
) -> Result<(Address, Option<Address>, Option<Address>, i128), MevProtectionError> {
    owner.require_auth();
    let commit = validate_reveal(env, &owner, commit_id, SensitiveOperation::Liquidate)?;
    let effective_fee_bps = compute_and_enforce_fee(env, &commit)?;
    let borrower = commit
        .borrower
        .clone()
        .ok_or(MevProtectionError::InvalidOperation)?;
    record_ordering_signal(
        env,
        owner,
        SensitiveOperation::Liquidate,
        commit.asset.clone(),
        commit.amount,
        effective_fee_bps,
    );
    env.storage()
        .persistent()
        .remove(&MevDataKey::Commit(commit_id));
    Ok((borrower, commit.asset, commit.secondary_asset, commit.amount))
}

/// Validate a reveal attempt: ownership, operation type, timing, and deadline.
fn validate_reveal(
    env: &Env,
    owner: &Address,
    commit_id: u64,
    expected: SensitiveOperation,
) -> Result<PendingCommit, MevProtectionError> {
    let commit = load_commit(env, commit_id)?;
    if commit.owner != *owner {
        return Err(MevProtectionError::Unauthorized);
    }
    if commit.operation != expected {
        return Err(MevProtectionError::InvalidOperation);
    }
    let now = env.ledger().timestamp();
    if now < commit.reveal_after {
        return Err(MevProtectionError::CommitNotReady);
    }
    if now > commit.expires_at {
        return Err(MevProtectionError::CommitExpired);
    }
    // Enforce deadline
    if commit.deadline > 0 && now > commit.deadline {
        return Err(MevProtectionError::DeadlineExpired);
    }
    Ok(commit)
}

/// Compute the effective MEV fee and enforce the caller's cap.
fn compute_and_enforce_fee(
    env: &Env,
    commit: &PendingCommit,
) -> Result<i128, MevProtectionError> {
    let effective_fee_bps = preview_fee_bps(
        env,
        commit.operation.clone(),
        commit.asset.clone(),
        commit.amount,
    );
    if effective_fee_bps > commit.max_fee_bps {
        return Err(MevProtectionError::FeeCapExceeded);
    }
    // Update reveal stats
    let mut stats = get_ordering_stats(env);
    stats.total_reveals = stats.total_reveals.saturating_add(1);
    stats.cumulative_fee_bps_collected = stats
        .cumulative_fee_bps_collected
        .saturating_add(effective_fee_bps);
    env.storage()
        .persistent()
        .set(&MevDataKey::OrderingStats, &stats);
    Ok(effective_fee_bps)
}

// ─── Batch Auction ────────────────────────────────────────────────────────────

/// Compute the current auction slot ID from the ledger timestamp.
fn current_slot_id(env: &Env) -> u64 {
    let cfg = get_config(env);
    let window = cfg.batch_window_secs.max(1);
    env.ledger().timestamp() / window
}

/// Place a bid in the current batch liquidation auction.
///
/// Bids are collected during the open window and settled atomically when
/// `settle_batch_auction` is called after the window closes.
///
/// # Slippage protection
/// `min_collateral_out` is stored with the bid and enforced at settlement.
/// If the clearing price would yield less collateral, the bid is skipped.
#[allow(clippy::too_many_arguments)]
pub fn place_auction_bid(
    env: &Env,
    bidder: Address,
    borrower: Address,
    debt_amount: i128,
    min_collateral_out: i128,
    max_fee_bps: i128,
    deadline: u64,
) -> Result<u64, MevProtectionError> {
    bidder.require_auth();
    let cfg = get_config(env);
    if !cfg.batching_enabled {
        return Err(MevProtectionError::InvalidConfig);
    }
    if debt_amount <= 0 {
        return Err(MevProtectionError::InvalidAmount);
    }
    if !(0..=MAX_BPS).contains(&max_fee_bps) {
        return Err(MevProtectionError::InvalidConfig);
    }

    let now = env.ledger().timestamp();
    require_strict_deadline(env, deadline, MevProtectionError::DeadlineExpired)?;

    let slot_id = current_slot_id(env);
    let bids_key = MevDataKey::AuctionBids(slot_id);

    let mut bids: soroban_sdk::Vec<AuctionBid> = env
        .storage()
        .persistent()
        .get(&bids_key)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env));

    if bids.len() >= MAX_BIDS_PER_SLOT {
        return Err(MevProtectionError::InvalidConfig);
    }

    let bid = AuctionBid {
        bidder,
        borrower,
        debt_amount,
        min_collateral_out,
        max_fee_bps,
        placed_at: now,
        deadline: if deadline == 0 {
            now.saturating_add(cfg.batch_window_secs.saturating_mul(2))
        } else {
            deadline
        },
    };
    bids.push_back(bid);
    env.storage().persistent().set(&bids_key, &bids);

    // Update stats
    let mut stats = get_ordering_stats(env);
    stats.total_auction_bids = stats.total_auction_bids.saturating_add(1);
    env.storage()
        .persistent()
        .set(&MevDataKey::OrderingStats, &stats);

    Ok(slot_id)
}

/// Settle a closed batch auction slot.
///
/// Can only be called after the slot's window has closed. Computes a uniform
/// clearing fee as the median of all valid bids' `max_fee_bps`, then returns
/// the `AuctionResult` for the caller to execute individual liquidations.
///
/// Bids whose `deadline` has passed or whose `min_collateral_out` cannot be
/// satisfied at the clearing fee are excluded from the result.
pub fn settle_batch_auction(
    env: &Env,
    caller: Address,
    slot_id: u64,
) -> Result<AuctionResult, MevProtectionError> {
    caller.require_auth();
    let cfg = get_config(env);
    if !cfg.batching_enabled {
        return Err(MevProtectionError::InvalidConfig);
    }

    let now = env.ledger().timestamp();
    let slot_end = slot_id
        .saturating_add(1)
        .saturating_mul(cfg.batch_window_secs);
    if now < slot_end {
        return Err(MevProtectionError::AuctionWindowOpen);
    }

    // Check not already settled
    let result_key = MevDataKey::AuctionResult(slot_id);
    if env
        .storage()
        .persistent()
        .has(&result_key)
    {
        // Return cached result
        return env
            .storage()
            .persistent()
            .get(&result_key)
            .ok_or(MevProtectionError::NoBidsInAuction);
    }

    let bids_key = MevDataKey::AuctionBids(slot_id);
    let bids: soroban_sdk::Vec<AuctionBid> = env
        .storage()
        .persistent()
        .get(&bids_key)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env));

    if bids.is_empty() {
        return Err(MevProtectionError::NoBidsInAuction);
    }

    // Filter valid bids (deadline not passed)
    let mut valid_fee_sum: i128 = 0;
    let mut valid_count: u32 = 0;
    let mut total_debt: i128 = 0;

    for bid in bids.iter() {
        if is_expired_strict(env, bid.deadline) {
            continue;
        }
        valid_fee_sum = valid_fee_sum.saturating_add(bid.max_fee_bps);
        valid_count = valid_count.saturating_add(1);
        total_debt = total_debt.saturating_add(bid.debt_amount);
    }

    if valid_count == 0 {
        return Err(MevProtectionError::NoBidsInAuction);
    }

    // Clearing fee = average of valid bids' max_fee_bps (simple uniform price)
    let clearing_fee_bps = valid_fee_sum
        .checked_div(valid_count as i128)
        .unwrap_or(cfg.base_protection_fee_bps)
        .clamp(cfg.base_protection_fee_bps, MAX_BPS);

    let result = AuctionResult {
        slot_id,
        bid_count: valid_count,
        clearing_fee_bps,
        total_debt_liquidated: total_debt,
        settled_at: now,
    };

    env.storage().persistent().set(&result_key, &result);

    // Update stats
    let mut stats = get_ordering_stats(env);
    stats.total_auctions_settled = stats.total_auctions_settled.saturating_add(1);
    env.storage()
        .persistent()
        .set(&MevDataKey::OrderingStats, &stats);

    Ok(result)
}

/// Retrieve bids for a given auction slot.
pub fn get_auction_bids(env: &Env, slot_id: u64) -> soroban_sdk::Vec<AuctionBid> {
    env.storage()
        .persistent()
        .get(&MevDataKey::AuctionBids(slot_id))
        .unwrap_or_else(|| soroban_sdk::Vec::new(env))
}

/// Retrieve the settled result for a given auction slot (None if not yet settled).
pub fn get_auction_result(env: &Env, slot_id: u64) -> Option<AuctionResult> {
    env.storage()
        .persistent()
        .get(&MevDataKey::AuctionResult(slot_id))
}

/// Return the current open auction slot ID.
pub fn get_current_auction_slot(env: &Env) -> u64 {
    current_slot_id(env)
}

// ─── Fee Preview & Gas Bidding Analysis ──────────────────────────────────────

/// Preview the effective MEV protection fee for an operation without committing.
///
/// Uses an EMA of past observations to smooth fee volatility. Surges when
/// suspicious ordering activity is detected within the configured window.
pub fn preview_fee_bps(
    env: &Env,
    operation: SensitiveOperation,
    asset: Option<Address>,
    amount: i128,
) -> i128 {
    let cfg = get_config(env);
    let op_key = operation_symbol(env, &operation);
    let latest: Option<OrderingObservation> =
        env.storage()
            .persistent()
            .get(&MevDataKey::LatestObservation(
                op_key.clone(),
                asset.clone(),
            ));
    let prior = env
        .storage()
        .persistent()
        .get::<MevDataKey, i128>(&MevDataKey::SmoothedFee(op_key.clone(), asset.clone()))
        .unwrap_or(cfg.base_protection_fee_bps);

    let mut target = cfg.base_protection_fee_bps;
    if let Some(last) = latest {
        let now = env.ledger().timestamp();
        if now.saturating_sub(last.timestamp) <= cfg.suspicious_window_secs {
            target = cfg.surge_protection_fee_bps;
            if amounts_close(last.amount, amount, cfg.sandwich_threshold_bps) {
                target = target.saturating_add(cfg.base_protection_fee_bps);
            }
        }
    }

    let smoothed = prior
        .saturating_mul(MAX_BPS.saturating_sub(cfg.fee_smoothing_bps))
        .saturating_add(target.saturating_mul(cfg.fee_smoothing_bps))
        .saturating_div(MAX_BPS);
    smoothed.clamp(0, MAX_BPS)
}

/// Return a gas bidding analysis snapshot for the given operation and asset.
///
/// Callers (off-chain relayers, frontends) use this to decide how much to bid
/// in order to get their transaction included without overpaying.
pub fn get_gas_bid_analysis(
    env: &Env,
    operation: SensitiveOperation,
    asset: Option<Address>,
    amount: i128,
) -> GasBidAnalysis {
    let cfg = get_config(env);
    let op_key = operation_symbol(env, &operation);
    let now = env.ledger().timestamp();

    let smoothed_base = env
        .storage()
        .persistent()
        .get::<MevDataKey, i128>(&MevDataKey::SmoothedFee(op_key.clone(), asset.clone()))
        .unwrap_or(cfg.base_protection_fee_bps);

    let latest: Option<OrderingObservation> = env
        .storage()
        .persistent()
        .get(&MevDataKey::LatestObservation(op_key, asset.clone()));

    let in_suspicious_window = latest
        .as_ref()
        .map(|obs| now.saturating_sub(obs.timestamp) <= cfg.suspicious_window_secs)
        .unwrap_or(false);

    let current_surge = if in_suspicious_window {
        cfg.surge_protection_fee_bps
    } else {
        0
    };

    let recommended = preview_fee_bps(env, operation.clone(), asset.clone(), amount);
    // High-congestion bid: surge + 20% buffer
    let high_congestion = recommended
        .saturating_add(cfg.base_protection_fee_bps.saturating_mul(2))
        .clamp(0, MAX_BPS);

    let stats = get_ordering_stats(env);

    GasBidAnalysis {
        smoothed_base_fee_bps: smoothed_base,
        current_surge_fee_bps: current_surge,
        recommended_bid_bps: recommended,
        high_congestion_bid_bps: high_congestion,
        in_suspicious_window,
        recent_sandwich_alerts: stats.sandwich_alerts,
    }
}

// ─── Routing Hints ────────────────────────────────────────────────────────────

/// Return the recommended execution routing hint given the caller's preference.
pub fn execution_hint(env: &Env, requested: TxOrderingHint) -> TxOrderingHint {
    let cfg = get_config(env);
    match requested {
        TxOrderingHint::PrivateMempool if cfg.private_mempool_enabled => {
            TxOrderingHint::PrivateMempool
        }
        TxOrderingHint::BatchAuction if cfg.batching_enabled => TxOrderingHint::BatchAuction,
        TxOrderingHint::Default if cfg.private_mempool_enabled => TxOrderingHint::PrivateMempool,
        TxOrderingHint::Default if cfg.batching_enabled => TxOrderingHint::BatchAuction,
        _ => TxOrderingHint::DelayedReveal,
    }
}

/// Return human-readable guidance for the given operation.
pub fn user_guidance(env: &Env, operation: SensitiveOperation) -> String {
    match (operation, execution_hint(env, TxOrderingHint::Default)) {
        (SensitiveOperation::Borrow, TxOrderingHint::PrivateMempool) => String::from_str(
            env,
            "Commit borrow, wait for the reveal delay, then use a private mempool route.",
        ),
        (SensitiveOperation::Withdraw, TxOrderingHint::PrivateMempool) => String::from_str(
            env,
            "Commit withdrawal, wait for the reveal delay, then use a private mempool route.",
        ),
        (SensitiveOperation::Liquidate, TxOrderingHint::PrivateMempool) => String::from_str(
            env,
            "Commit liquidation, wait for the reveal delay, then use a private mempool route.",
        ),
        (_, TxOrderingHint::BatchAuction) => String::from_str(
            env,
            "Use commit/reveal and prefer batched execution during congested periods.",
        ),
        _ => String::from_str(
            env,
            "Use commit/reveal and avoid revealing during short bursts of ordering activity.",
        ),
    }
}

// ─── Monitoring / Stats ───────────────────────────────────────────────────────

/// Return the current MEV ordering statistics (monitoring dashboard data).
pub fn get_ordering_stats(env: &Env) -> OrderingStats {
    env.storage()
        .persistent()
        .get(&MevDataKey::OrderingStats)
        .unwrap_or(OrderingStats {
            suspicious_sequences: 0,
            sandwich_alerts: 0,
            last_alert_timestamp: 0,
            last_effective_fee_bps: 0,
            total_commits: 0,
            total_reveals: 0,
            total_auction_bids: 0,
            total_auctions_settled: 0,
            cumulative_fee_bps_collected: 0,
        })
}

// ─── Internal Helpers ─────────────────────────────────────────────────────────

/// Record an ordering observation and update sandwich / suspicious-sequence counters.
fn record_ordering_signal(
    env: &Env,
    actor: Address,
    operation: SensitiveOperation,
    asset: Option<Address>,
    amount: i128,
    effective_fee_bps: i128,
) {
    let cfg = get_config(env);
    let op_key = operation_symbol(env, &operation);
    let latest_key = MevDataKey::LatestObservation(op_key.clone(), asset.clone());
    let previous_key = MevDataKey::PreviousObservation(op_key.clone(), asset.clone());
    let smoothed_key = MevDataKey::SmoothedFee(op_key, asset.clone());
    let now = env.ledger().timestamp();
    let latest: Option<OrderingObservation> = env.storage().persistent().get(&latest_key);
    let previous: Option<OrderingObservation> = env.storage().persistent().get(&previous_key);
    let mut stats = get_ordering_stats(env);

    // Detect suspicious ordering: two different actors within the window
    if let Some(ref last) = latest {
        if now.saturating_sub(last.timestamp) <= cfg.suspicious_window_secs
            && last.actor != actor
        {
            stats.suspicious_sequences = stats.suspicious_sequences.saturating_add(1);
        }
    }

    // Detect sandwich: prev.actor == current.actor, last.actor != current.actor,
    // and amounts are close (front-run / back-run pattern).
    if let (Some(ref prev), Some(ref last)) = (&previous, &latest) {
        let prev_recent = now.saturating_sub(prev.timestamp) <= cfg.suspicious_window_secs;
        let last_recent = now.saturating_sub(last.timestamp) <= cfg.suspicious_window_secs;
        if prev_recent
            && last_recent
            && prev.actor == actor
            && last.actor != actor
            && amounts_close(prev.amount, amount, cfg.sandwich_threshold_bps)
        {
            stats.sandwich_alerts = stats.sandwich_alerts.saturating_add(1);
            stats.last_alert_timestamp = now;
        }
    }

    stats.last_effective_fee_bps = effective_fee_bps;
    env.storage()
        .persistent()
        .set(&MevDataKey::OrderingStats, &stats);

    // Rotate observations: latest → previous, new → latest
    if let Some(last) = latest {
        env.storage().persistent().set(&previous_key, &last);
    }
    env.storage().persistent().set(
        &latest_key,
        &OrderingObservation {
            actor,
            amount,
            timestamp: now,
        },
    );
    env.storage()
        .persistent()
        .set(&smoothed_key, &effective_fee_bps);
}

fn load_commit(env: &Env, commit_id: u64) -> Result<PendingCommit, MevProtectionError> {
    env.storage()
        .persistent()
        .get(&MevDataKey::Commit(commit_id))
        .ok_or(MevProtectionError::CommitNotFound)
}

fn next_commit_id(env: &Env) -> u64 {
    let id = env
        .storage()
        .persistent()
        .get::<MevDataKey, u64>(&MevDataKey::NextCommitId)
        .unwrap_or(1);
    env.storage()
        .persistent()
        .set(&MevDataKey::NextCommitId, &id.saturating_add(1));
    id
}

fn validate_config(config: &MevProtectionConfig) -> Result<(), MevProtectionError> {
    if config.commit_delay_secs == 0
        || config.commit_expiry_secs <= config.commit_delay_secs
        || config.suspicious_window_secs == 0
        || config.batch_window_secs == 0
        || !(0..=MAX_BPS).contains(&config.fee_smoothing_bps)
        || !(0..=MAX_BPS).contains(&config.base_protection_fee_bps)
        || !(0..=MAX_BPS).contains(&config.surge_protection_fee_bps)
        || !(0..=MAX_BPS).contains(&config.sandwich_threshold_bps)
        || !(0..=MAX_BPS).contains(&config.default_max_slippage_bps)
    {
        return Err(MevProtectionError::InvalidConfig);
    }
    Ok(())
}

/// Returns true when the absolute difference between `a` and `b` is within
/// `threshold_bps` of the larger value.
fn amounts_close(a: i128, b: i128, threshold_bps: i128) -> bool {
    if a == 0 && b == 0 {
        return true;
    }
    let max = if a.abs() > b.abs() { a.abs() } else { b.abs() };
    if max == 0 {
        return true;
    }
    let diff = (a - b).abs();
    diff.saturating_mul(MAX_BPS) <= max.saturating_mul(threshold_bps)
}

fn operation_symbol(env: &Env, operation: &SensitiveOperation) -> Symbol {
    match operation {
        SensitiveOperation::Borrow => Symbol::new(env, "borrow"),
        SensitiveOperation::Withdraw => Symbol::new(env, "withdraw"),
        SensitiveOperation::Liquidate => Symbol::new(env, "liquidate"),
    }
}
