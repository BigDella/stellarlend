//! # Cross-Chain Lending Bridge  — Issue #799
//!
//! Extends the base bridge contract with multi-chain lending primitives:
//!
//! ## Features
//! 1. **Cross-chain lending positions** — a user can initiate a borrow on a
//!    remote chain by locking collateral on Stellar, sending a bridge message,
//!    and later repaying / unlocking through the same channel.
//! 2. **Collateral locking** — collateral locked here cannot be transferred or
//!    double-spent until the remote borrow is repaid or the lock is cancelled
//!    by an admin (emergency only).
//! 3. **Liquidity routing** — track which bridge carries cross-chain liquidity
//!    between lending pools so the off-chain router can pick the cheapest path.
//! 4. **Cross-chain health factor oracle** — validators submit remote health
//!    factor reports that the contract stores; the on-chain value is used to
//!    gate whether a user may increase their cross-chain position.
//!
//! ## Design notes
//! • All state is stored in `persistent` storage with typed keys so it can
//!   co-exist with the base bridge keys without collision.
//! • Payment / token transfer is NOT implemented on-chain — the SEP-41
//!   collateral custody is handled by a separate vault contract that calls
//!   back here. The lock records the amount and vault contract for auditability.
//! • Health factor reports expire after `HEALTH_REPORT_TTL_LEDGERS` ledgers.

#![allow(unused)]

use soroban_sdk::{
    contracterror, contractevent, contracttype, Address, Env, String, Symbol, Vec,
};

// ─── Constants ────────────────────────────────────────────────────────────────

/// Maximum number of liquidity routes retained in the registry.
const MAX_ROUTES: u32 = 50;

/// Maximum number of remote health-factor reports retained per user.
const MAX_HEALTH_REPORTS: u32 = 10;

/// Ledgers after which a health-factor report is considered stale.
const HEALTH_REPORT_TTL_LEDGERS: u32 = 100;

/// Minimum health factor (in bps) required to initiate a new cross-chain borrow.
/// 12_000 bps = 1.2× collateralization.
const MIN_HEALTH_FACTOR_TO_BORROW_BPS: i128 = 12_000;

// ─── Errors ───────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum LendingBridgeError {
    /// Caller is not authorized.
    Unauthorized = 100,
    /// The lending position does not exist.
    PositionNotFound = 101,
    /// A position already exists for this user on this chain/pool.
    PositionAlreadyExists = 102,
    /// The collateral lock does not exist.
    LockNotFound = 103,
    /// Collateral is still locked and cannot be released.
    CollateralStillLocked = 104,
    /// Amount must be positive.
    InvalidAmount = 105,
    /// Health factor is too low to initiate a new cross-chain borrow.
    InsufficientHealthFactor = 106,
    /// The liquidity route already exists.
    RouteAlreadyExists = 107,
    /// The liquidity route was not found.
    RouteNotFound = 108,
    /// Arithmetic overflow.
    Overflow = 109,
    /// Remote health factor report not found or stale.
    StaleHealthReport = 110,
    /// The position is already marked repaid.
    AlreadyRepaid = 111,
}

// ─── Storage keys ─────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug)]
pub enum LendingBridgeKey {
    /// Cross-chain lending position: LendingPosition(user, remote_chain, remote_pool)
    LendingPosition(Address, String, String),
    /// Collateral lock for a lending position: CollateralLock(user, remote_chain, remote_pool)
    CollateralLock(Address, String, String),
    /// Liquidity route registry: LiquidityRoute(bridge_id) -> LiquidityRoute
    LiquidityRoute(String),
    /// Ordered list of registered route bridge_ids: RouteList -> Vec<String>
    RouteList,
    /// Latest remote health factor report for a user: HealthReport(user, remote_chain)
    HealthReport(Address, String),
    /// Global lending bridge statistics: LendingStats -> LendingBridgeStats
    LendingStats,
}

// ─── Data types ───────────────────────────────────────────────────────────────

/// Status of a cross-chain lending position.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum PositionStatus {
    /// Collateral locked, borrow initiated on remote chain.
    Active,
    /// Remote borrow fully repaid; collateral can be unlocked.
    Repaid,
    /// Position cancelled by admin (emergency).
    Cancelled,
}

/// A cross-chain lending position initiated from Stellar.
///
/// Represents: user locked `collateral_amount` of `collateral_asset` on
/// Stellar and borrowed `borrow_amount` of `borrow_asset` on `remote_chain`
/// against pool `remote_pool`.
#[contracttype]
#[derive(Clone, Debug)]
pub struct CrossChainLendingPosition {
    pub user: Address,
    /// Identifier of the remote chain (e.g. "ethereum", "polygon").
    pub remote_chain: String,
    /// Identifier of the remote lending pool.
    pub remote_pool: String,
    /// Bridge used to route this position.
    pub bridge_id: String,
    /// Asset address of the Stellar-side collateral (SEP-41).
    pub collateral_asset: Address,
    /// Amount of collateral locked on Stellar.
    pub collateral_amount: i128,
    /// Asset borrowed on the remote chain (identifier string).
    pub borrow_asset: String,
    /// Amount borrowed on the remote chain.
    pub borrow_amount: i128,
    /// Interest rate agreed at initiation (basis points).
    pub interest_rate_bps: i128,
    /// The bridge message ID that relayed this position to the remote chain.
    pub bridge_message_id: u64,
    pub status: PositionStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Collateral locked on Stellar backing a cross-chain borrow.
#[contracttype]
#[derive(Clone, Debug)]
pub struct CollateralLock {
    pub user: Address,
    pub asset: Address,
    /// Vault contract holding the actual tokens.
    pub vault_contract: Address,
    pub amount: i128,
    pub remote_chain: String,
    pub remote_pool: String,
    /// True once the lock has been released (position repaid/cancelled).
    pub released: bool,
    pub locked_at: u64,
    pub released_at: u64,
}

/// A registered liquidity route between a Stellar lending pool and a remote pool.
///
/// The off-chain router uses these records to calculate the cheapest path for
/// cross-chain lending operations.
#[contracttype]
#[derive(Clone, Debug)]
pub struct LiquidityRoute {
    /// Bridge contract routing liquidity on this path.
    pub bridge_id: String,
    /// Stellar-side lending pool contract.
    pub stellar_pool: Address,
    /// Remote chain identifier.
    pub remote_chain: String,
    /// Remote pool identifier.
    pub remote_pool: String,
    /// Supported asset on Stellar side (SEP-41 contract).
    pub stellar_asset: Address,
    /// Fee in basis points charged by this route.
    pub route_fee_bps: u64,
    /// Maximum liquidity capacity (0 = unlimited).
    pub max_capacity: i128,
    /// Whether this route is currently active.
    pub active: bool,
    pub registered_at: u64,
}

/// A remote health-factor report submitted by a validator.
#[contracttype]
#[derive(Clone, Debug)]
pub struct RemoteHealthReport {
    pub user: Address,
    pub remote_chain: String,
    /// Health factor in basis points (10_000 = 1.0×).
    pub health_factor_bps: i128,
    /// Total collateral value on the remote chain (in a common denomination).
    pub remote_collateral_value: i128,
    /// Total debt value on the remote chain.
    pub remote_debt_value: i128,
    /// Ledger sequence at which this was submitted.
    pub submitted_at_ledger: u32,
    pub submitted_at_timestamp: u64,
    /// Validator who submitted this report.
    pub validator: Address,
}

/// Global statistics for the lending bridge module.
#[contracttype]
#[derive(Clone, Debug)]
pub struct LendingBridgeStats {
    pub total_positions_opened: u64,
    pub total_positions_repaid: u64,
    pub total_positions_cancelled: u64,
    pub total_collateral_locked: i128,
    pub total_collateral_released: i128,
    pub total_routes_registered: u32,
}

// ─── Events ───────────────────────────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug)]
pub struct CrossChainPositionOpenedEvent {
    pub user: Address,
    pub remote_chain: String,
    pub remote_pool: String,
    pub bridge_id: String,
    pub collateral_amount: i128,
    pub borrow_amount: i128,
    pub bridge_message_id: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct CrossChainPositionRepaidEvent {
    pub user: Address,
    pub remote_chain: String,
    pub remote_pool: String,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct CollateralLockedEvent {
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
    pub remote_chain: String,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct CollateralReleasedEvent {
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
    pub remote_chain: String,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct LiquidityRouteRegisteredEvent {
    pub bridge_id: String,
    pub remote_chain: String,
    pub remote_pool: String,
    pub route_fee_bps: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct RemoteHealthReportSubmittedEvent {
    pub user: Address,
    pub remote_chain: String,
    pub health_factor_bps: i128,
    pub validator: Address,
    pub timestamp: u64,
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

fn get_stats(env: &Env) -> LendingBridgeStats {
    env.storage()
        .persistent()
        .get(&LendingBridgeKey::LendingStats)
        .unwrap_or(LendingBridgeStats {
            total_positions_opened: 0,
            total_positions_repaid: 0,
            total_positions_cancelled: 0,
            total_collateral_locked: 0,
            total_collateral_released: 0,
            total_routes_registered: 0,
        })
}

fn save_stats(env: &Env, stats: &LendingBridgeStats) {
    env.storage()
        .persistent()
        .set(&LendingBridgeKey::LendingStats, stats);
}

fn position_key(
    user: &Address,
    remote_chain: &String,
    remote_pool: &String,
) -> LendingBridgeKey {
    LendingBridgeKey::LendingPosition(user.clone(), remote_chain.clone(), remote_pool.clone())
}

fn lock_key(
    user: &Address,
    remote_chain: &String,
    remote_pool: &String,
) -> LendingBridgeKey {
    LendingBridgeKey::CollateralLock(user.clone(), remote_chain.clone(), remote_pool.clone())
}

// ─── Cross-chain lending position API ─────────────────────────────────────────

/// Open a new cross-chain lending position.
///
/// Records the intent to borrow `borrow_amount` of `borrow_asset` on
/// `remote_chain`/`remote_pool` using `collateral_amount` of
/// `collateral_asset` locked on Stellar. The caller must have already
/// called `lock_collateral` (or the vault contract must call it atomically).
///
/// If a valid, non-stale remote health report exists for the user on
/// `remote_chain`, it is checked against `MIN_HEALTH_FACTOR_TO_BORROW_BPS`
/// before the position is created. If no report exists the check is skipped
/// (first-time cross-chain borrower).
///
/// # Arguments
/// * `caller`             — must be `user` (auth required)
/// * `bridge_message_id`  — the already-submitted cross-chain message ID
pub fn open_lending_position(
    env: &Env,
    caller: Address,
    remote_chain: String,
    remote_pool: String,
    bridge_id: String,
    collateral_asset: Address,
    collateral_amount: i128,
    borrow_asset: String,
    borrow_amount: i128,
    interest_rate_bps: i128,
    bridge_message_id: u64,
) -> Result<CrossChainLendingPosition, LendingBridgeError> {
    caller.require_auth();

    if collateral_amount <= 0 || borrow_amount <= 0 {
        return Err(LendingBridgeError::InvalidAmount);
    }

    // Guard against duplicate open positions.
    let pk = position_key(&caller, &remote_chain, &remote_pool);
    if env.storage().persistent().has(&pk) {
        let existing: CrossChainLendingPosition = env.storage().persistent().get(&pk).unwrap();
        if existing.status == PositionStatus::Active {
            return Err(LendingBridgeError::PositionAlreadyExists);
        }
    }

    // Check remote health factor if a fresh report is available.
    let report_key = LendingBridgeKey::HealthReport(caller.clone(), remote_chain.clone());
    if let Some(report) = env
        .storage()
        .persistent()
        .get::<LendingBridgeKey, RemoteHealthReport>(&report_key)
    {
        let age_ledgers = env
            .ledger()
            .sequence()
            .saturating_sub(report.submitted_at_ledger);
        if age_ledgers <= HEALTH_REPORT_TTL_LEDGERS
            && report.health_factor_bps < MIN_HEALTH_FACTOR_TO_BORROW_BPS
        {
            return Err(LendingBridgeError::InsufficientHealthFactor);
        }
    }

    let now = env.ledger().timestamp();
    let position = CrossChainLendingPosition {
        user: caller.clone(),
        remote_chain: remote_chain.clone(),
        remote_pool: remote_pool.clone(),
        bridge_id: bridge_id.clone(),
        collateral_asset: collateral_asset.clone(),
        collateral_amount,
        borrow_asset: borrow_asset.clone(),
        borrow_amount,
        interest_rate_bps,
        bridge_message_id,
        status: PositionStatus::Active,
        created_at: now,
        updated_at: now,
    };

    env.storage().persistent().set(&pk, &position);

    let mut stats = get_stats(env);
    stats.total_positions_opened = stats.total_positions_opened.saturating_add(1);
    stats.total_collateral_locked = stats
        .total_collateral_locked
        .checked_add(collateral_amount)
        .unwrap_or(stats.total_collateral_locked);
    save_stats(env, &stats);

    CrossChainPositionOpenedEvent {
        user: caller,
        remote_chain,
        remote_pool,
        bridge_id,
        collateral_amount,
        borrow_amount,
        bridge_message_id,
        timestamp: now,
    }
    .publish(env);

    Ok(position)
}

/// Mark a cross-chain lending position as repaid.
///
/// Called by the bridge validator relayer once the remote repayment message
/// has been verified and executed. Sets `status = Repaid` so the user can
/// call `release_collateral`.
///
/// Only the admin or a validator holding the `bridge_relayer` role may call
/// this (verified via caller auth; role check deferred to the bridge contract
/// layer that wraps this module).
pub fn mark_position_repaid(
    env: &Env,
    caller: Address,
    user: Address,
    remote_chain: String,
    remote_pool: String,
) -> Result<(), LendingBridgeError> {
    caller.require_auth();

    let pk = position_key(&user, &remote_chain, &remote_pool);
    let mut position: CrossChainLendingPosition = env
        .storage()
        .persistent()
        .get(&pk)
        .ok_or(LendingBridgeError::PositionNotFound)?;

    if position.status == PositionStatus::Repaid {
        return Err(LendingBridgeError::AlreadyRepaid);
    }

    position.status = PositionStatus::Repaid;
    position.updated_at = env.ledger().timestamp();
    env.storage().persistent().set(&pk, &position);

    let mut stats = get_stats(env);
    stats.total_positions_repaid = stats.total_positions_repaid.saturating_add(1);
    save_stats(env, &stats);

    CrossChainPositionRepaidEvent {
        user,
        remote_chain,
        remote_pool,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Admin-only: cancel a stuck position (emergency).
pub fn cancel_position(
    env: &Env,
    admin: Address,
    user: Address,
    remote_chain: String,
    remote_pool: String,
) -> Result<(), LendingBridgeError> {
    admin.require_auth();

    let pk = position_key(&user, &remote_chain, &remote_pool);
    let mut position: CrossChainLendingPosition = env
        .storage()
        .persistent()
        .get(&pk)
        .ok_or(LendingBridgeError::PositionNotFound)?;

    position.status = PositionStatus::Cancelled;
    position.updated_at = env.ledger().timestamp();
    env.storage().persistent().set(&pk, &position);

    let mut stats = get_stats(env);
    stats.total_positions_cancelled = stats.total_positions_cancelled.saturating_add(1);
    save_stats(env, &stats);

    Ok(())
}

/// Read-only: get a lending position.
pub fn get_lending_position(
    env: &Env,
    user: Address,
    remote_chain: String,
    remote_pool: String,
) -> Option<CrossChainLendingPosition> {
    env.storage()
        .persistent()
        .get(&position_key(&user, &remote_chain, &remote_pool))
}

// ─── Collateral lock API ───────────────────────────────────────────────────────

/// Record a collateral lock for a cross-chain borrow.
///
/// This does NOT move tokens — it only records metadata. The vault contract
/// that custody the tokens should call this to register the lock so the
/// rest of the protocol can verify it.
pub fn lock_collateral(
    env: &Env,
    caller: Address,
    user: Address,
    asset: Address,
    vault_contract: Address,
    amount: i128,
    remote_chain: String,
    remote_pool: String,
) -> Result<CollateralLock, LendingBridgeError> {
    caller.require_auth();

    if amount <= 0 {
        return Err(LendingBridgeError::InvalidAmount);
    }

    let lk = lock_key(&user, &remote_chain, &remote_pool);
    if env.storage().persistent().has(&lk) {
        let existing: CollateralLock = env.storage().persistent().get(&lk).unwrap();
        if !existing.released {
            return Err(LendingBridgeError::CollateralStillLocked);
        }
    }

    let now = env.ledger().timestamp();
    let lock = CollateralLock {
        user: user.clone(),
        asset: asset.clone(),
        vault_contract,
        amount,
        remote_chain: remote_chain.clone(),
        remote_pool: remote_pool.clone(),
        released: false,
        locked_at: now,
        released_at: 0,
    };

    env.storage().persistent().set(&lk, &lock);

    CollateralLockedEvent {
        user,
        asset,
        amount,
        remote_chain,
        timestamp: now,
    }
    .publish(env);

    Ok(lock)
}

/// Release a collateral lock once the position is repaid or cancelled.
///
/// Sets `released = true` and records the release timestamp. The vault
/// contract should then transfer the tokens back to the user.
pub fn release_collateral(
    env: &Env,
    caller: Address,
    user: Address,
    remote_chain: String,
    remote_pool: String,
) -> Result<CollateralLock, LendingBridgeError> {
    caller.require_auth();

    let pk = position_key(&user, &remote_chain, &remote_pool);
    let position: Option<CrossChainLendingPosition> = env.storage().persistent().get(&pk);

    // Position must be Repaid or Cancelled before collateral is released.
    if let Some(pos) = position {
        if pos.status == PositionStatus::Active {
            return Err(LendingBridgeError::CollateralStillLocked);
        }
    }

    let lk = lock_key(&user, &remote_chain, &remote_pool);
    let mut lock: CollateralLock = env
        .storage()
        .persistent()
        .get(&lk)
        .ok_or(LendingBridgeError::LockNotFound)?;

    if lock.released {
        return Err(LendingBridgeError::LockNotFound);
    }

    let now = env.ledger().timestamp();
    lock.released = true;
    lock.released_at = now;
    env.storage().persistent().set(&lk, &lock);

    let mut stats = get_stats(env);
    stats.total_collateral_released = stats
        .total_collateral_released
        .checked_add(lock.amount)
        .unwrap_or(stats.total_collateral_released);
    save_stats(env, &stats);

    CollateralReleasedEvent {
        user,
        asset: lock.asset.clone(),
        amount: lock.amount,
        remote_chain,
        timestamp: now,
    }
    .publish(env);

    Ok(lock)
}

/// Read-only: get the collateral lock for a position.
pub fn get_collateral_lock(
    env: &Env,
    user: Address,
    remote_chain: String,
    remote_pool: String,
) -> Option<CollateralLock> {
    env.storage()
        .persistent()
        .get(&lock_key(&user, &remote_chain, &remote_pool))
}

// ─── Liquidity route registry ──────────────────────────────────────────────────

/// Register a new liquidity route (admin-only).
pub fn register_liquidity_route(
    env: &Env,
    caller: Address,
    bridge_id: String,
    stellar_pool: Address,
    remote_chain: String,
    remote_pool: String,
    stellar_asset: Address,
    route_fee_bps: u64,
    max_capacity: i128,
) -> Result<LiquidityRoute, LendingBridgeError> {
    caller.require_auth();

    let rk = LendingBridgeKey::LiquidityRoute(bridge_id.clone());
    if env.storage().persistent().has(&rk) {
        return Err(LendingBridgeError::RouteAlreadyExists);
    }

    let now = env.ledger().timestamp();
    let route = LiquidityRoute {
        bridge_id: bridge_id.clone(),
        stellar_pool,
        remote_chain: remote_chain.clone(),
        remote_pool: remote_pool.clone(),
        stellar_asset,
        route_fee_bps,
        max_capacity,
        active: true,
        registered_at: now,
    };

    env.storage().persistent().set(&rk, &route);

    // Maintain enumeration list.
    let list_key = LendingBridgeKey::RouteList;
    let mut list: Vec<String> = env
        .storage()
        .persistent()
        .get(&list_key)
        .unwrap_or_else(|| Vec::new(env));
    list.push_back(bridge_id.clone());
    while list.len() > MAX_ROUTES {
        list.remove(0);
    }
    env.storage().persistent().set(&list_key, &list);

    let mut stats = get_stats(env);
    stats.total_routes_registered = stats.total_routes_registered.saturating_add(1);
    save_stats(env, &stats);

    LiquidityRouteRegisteredEvent {
        bridge_id,
        remote_chain,
        remote_pool,
        route_fee_bps,
        timestamp: now,
    }
    .publish(env);

    Ok(route)
}

/// Update a route's active status and fee (admin-only).
pub fn update_liquidity_route(
    env: &Env,
    caller: Address,
    bridge_id: String,
    active: bool,
    route_fee_bps: u64,
    max_capacity: i128,
) -> Result<LiquidityRoute, LendingBridgeError> {
    caller.require_auth();

    let rk = LendingBridgeKey::LiquidityRoute(bridge_id.clone());
    let mut route: LiquidityRoute = env
        .storage()
        .persistent()
        .get(&rk)
        .ok_or(LendingBridgeError::RouteNotFound)?;

    route.active = active;
    route.route_fee_bps = route_fee_bps;
    route.max_capacity = max_capacity;
    env.storage().persistent().set(&rk, &route);

    Ok(route)
}

/// Read-only: get a liquidity route by bridge_id.
pub fn get_liquidity_route(env: &Env, bridge_id: String) -> Option<LiquidityRoute> {
    env.storage()
        .persistent()
        .get(&LendingBridgeKey::LiquidityRoute(bridge_id))
}

/// Read-only: list all registered route bridge_ids.
pub fn list_liquidity_routes(env: &Env) -> Vec<String> {
    env.storage()
        .persistent()
        .get(&LendingBridgeKey::RouteList)
        .unwrap_or_else(|| Vec::new(env))
}

/// Read-only: find the cheapest active route for a (remote_chain, remote_pool) pair.
///
/// Returns the bridge_id of the route with the lowest `route_fee_bps` that
/// is currently active and has sufficient capacity.
pub fn get_best_route(
    env: &Env,
    remote_chain: String,
    remote_pool: String,
    required_capacity: i128,
) -> Option<String> {
    let list = list_liquidity_routes(env);
    let mut best_fee: Option<u64> = None;
    let mut best_bridge: Option<String> = None;

    for i in 0..list.len() {
        let bridge_id = list.get(i).unwrap();
        let rk = LendingBridgeKey::LiquidityRoute(bridge_id.clone());
        if let Some(route) = env
            .storage()
            .persistent()
            .get::<LendingBridgeKey, LiquidityRoute>(&rk)
        {
            if !route.active {
                continue;
            }
            if route.remote_chain != remote_chain || route.remote_pool != remote_pool {
                continue;
            }
            if route.max_capacity > 0 && route.max_capacity < required_capacity {
                continue;
            }
            if best_fee.is_none() || route.route_fee_bps < best_fee.unwrap() {
                best_fee = Some(route.route_fee_bps);
                best_bridge = Some(bridge_id);
            }
        }
    }

    best_bridge
}

// ─── Remote health-factor oracle ──────────────────────────────────────────────

/// Submit a remote health-factor report for a user (validator-only).
///
/// Callers must hold the `bridge_validator` role (enforced by the wrapper in
/// `lib.rs` which checks `admin::has_role`). The report is stored with the
/// current ledger sequence so staleness can be detected.
pub fn submit_health_report(
    env: &Env,
    validator: Address,
    user: Address,
    remote_chain: String,
    health_factor_bps: i128,
    remote_collateral_value: i128,
    remote_debt_value: i128,
) -> Result<RemoteHealthReport, LendingBridgeError> {
    validator.require_auth();

    if health_factor_bps < 0 {
        return Err(LendingBridgeError::InvalidAmount);
    }

    let now = env.ledger().timestamp();
    let report = RemoteHealthReport {
        user: user.clone(),
        remote_chain: remote_chain.clone(),
        health_factor_bps,
        remote_collateral_value,
        remote_debt_value,
        submitted_at_ledger: env.ledger().sequence(),
        submitted_at_timestamp: now,
        validator: validator.clone(),
    };

    let rk = LendingBridgeKey::HealthReport(user.clone(), remote_chain.clone());
    env.storage().persistent().set(&rk, &report);

    RemoteHealthReportSubmittedEvent {
        user,
        remote_chain,
        health_factor_bps,
        validator,
        timestamp: now,
    }
    .publish(env);

    Ok(report)
}

/// Read-only: get the latest remote health report for a user.
///
/// Returns `None` if no report exists. Callers should check `submitted_at_ledger`
/// against the current ledger to detect staleness.
pub fn get_health_report(
    env: &Env,
    user: Address,
    remote_chain: String,
) -> Option<RemoteHealthReport> {
    env.storage()
        .persistent()
        .get(&LendingBridgeKey::HealthReport(user, remote_chain))
}

/// Read-only: check whether the latest health report is still fresh.
pub fn is_health_report_fresh(env: &Env, user: Address, remote_chain: String) -> bool {
    match get_health_report(env, user, remote_chain) {
        None => false,
        Some(report) => {
            let age = env
                .ledger()
                .sequence()
                .saturating_sub(report.submitted_at_ledger);
            age <= HEALTH_REPORT_TTL_LEDGERS
        }
    }
}

// ─── Analytics ────────────────────────────────────────────────────────────────

/// Read-only: global lending bridge statistics.
pub fn get_lending_bridge_stats(env: &Env) -> LendingBridgeStats {
    get_stats(env)
}
