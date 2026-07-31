use soroban_sdk::{contracterror, contracttype, Address, Env, String, Symbol};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MevProtectionError {
    InvalidConfig = 1,
    CommitNotFound = 2,
    CommitNotReady = 3,
    CommitExpired = 4,
    Unauthorized = 5,
    FeeCapExceeded = 6,
    InvalidAmount = 7,
    InvalidOperation = 8,
    SlippageExpired = 9,
    SlippageExceeded = 10,
    AuctionNotFound = 11,
    AuctionNotOpen = 12,
    AuctionNotReady = 13,
    BidNotFound = 14,
    BidTooLow = 15,
    PrivateRouteRequired = 16,
    PrivateRouteNotFound = 17,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SensitiveOperation {
    Borrow,
    Withdraw,
    Liquidate,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TxOrderingHint {
    Default,
    PrivateMempool,
    BatchAuction,
    DelayedReveal,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MevProtectionConfig {
    pub commit_delay_secs: u64,
    pub commit_expiry_secs: u64,
    pub suspicious_window_secs: u64,
    pub fee_smoothing_bps: i128,
    pub base_protection_fee_bps: i128,
    pub surge_protection_fee_bps: i128,
    pub sandwich_threshold_bps: i128,
    pub large_tx_threshold: i128,
    pub default_auction_secs: u64,
    pub min_auction_bid_rebate_bps: i128,
    pub max_private_route_ttl_secs: u64,
    pub private_mempool_enabled: bool,
    pub batching_enabled: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionGuard {
    pub quoted_output_amount: i128,
    pub min_output_amount: i128,
    pub max_slippage_bps: i128,
    pub deadline: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingCommit {
    pub id: u64,
    pub owner: Address,
    pub operation: SensitiveOperation,
    pub asset: Option<Address>,
    pub secondary_asset: Option<Address>,
    pub borrower: Option<Address>,
    pub amount: i128,
    pub max_fee_bps: i128,
    pub hint: TxOrderingHint,
    pub committed_at: u64,
    pub reveal_after: u64,
    pub expires_at: u64,
    pub commit_ledger: u32,
    pub guard: Option<ExecutionGuard>,
    pub private_route: Option<Symbol>,
    pub auction_id: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderingObservation {
    pub actor: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderingStats {
    pub suspicious_sequences: u64,
    pub sandwich_alerts: u64,
    pub last_alert_timestamp: u64,
    pub last_effective_fee_bps: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuctionStatus {
    Open,
    Settled,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiquidationAuction {
    pub id: u64,
    pub opener: Address,
    pub borrower: Address,
    pub debt_asset: Option<Address>,
    pub collateral_asset: Option<Address>,
    pub debt_amount: i128,
    pub min_rebate_bps: i128,
    pub opened_at: u64,
    pub bidding_deadline: u64,
    pub status: AuctionStatus,
    pub best_bid_id: Option<u64>,
    pub winning_liquidator: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiquidationAuctionBid {
    pub id: u64,
    pub auction_id: u64,
    pub liquidator: Address,
    pub repay_amount: i128,
    pub rebate_bps: i128,
    pub max_fee_bps: i128,
    pub min_collateral_out: i128,
    pub private_route: Option<Symbol>,
    pub submitted_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuctionStats {
    pub opened: u64,
    pub bids: u64,
    pub settled: u64,
    pub executed: u64,
    pub last_auction_id: u64,
    pub last_settled_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrivateMempoolRoute {
    pub route_id: Symbol,
    pub relay: Address,
    pub registered_by: Address,
    pub registered_at: u64,
    pub expires_at: u64,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrivateExecutionReceipt {
    pub commit_id: u64,
    pub route_id: Symbol,
    pub relay: Address,
    pub received_at: u64,
    pub expires_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrivateRouteStats {
    pub active_routes: u32,
    pub executions: u64,
    pub last_execution_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GasBidStats {
    pub samples: u64,
    pub min_bid_microlumens: i128,
    pub max_bid_microlumens: i128,
    pub avg_bid_microlumens: i128,
    pub last_bid_microlumens: i128,
    pub avg_inclusion_delay_ledgers: u64,
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MevMonitoringDashboard {
    pub ordering: OrderingStats,
    pub gas_bids: GasBidStats,
    pub auctions: AuctionStats,
    pub private_routes: PrivateRouteStats,
    pub recommended_fee_bps: i128,
    pub recommended_hint: TxOrderingHint,
}

#[contracttype]
#[derive(Clone)]
enum MevDataKey {
    Config,
    NextCommitId,
    Commit(u64),
    NextAuctionId,
    NextBidId,
    Auction(u64),
    AuctionBid(u64),
    AuctionStats,
    PrivateRoute(Symbol),
    PrivateReceipt(u64),
    PrivateStats,
    GasBidStats(Symbol, Option<Address>),
    OrderingStats,
    LatestObservation(Symbol, Option<Address>),
    PreviousObservation(Symbol, Option<Address>),
    SmoothedFee(Symbol, Option<Address>),
}

const MAX_BPS: i128 = 10_000;

pub fn default_config() -> MevProtectionConfig {
    MevProtectionConfig {
        commit_delay_secs: 30,
        commit_expiry_secs: 300,
        suspicious_window_secs: 45,
        fee_smoothing_bps: 2_500,
        base_protection_fee_bps: 10,
        surge_protection_fee_bps: 60,
        sandwich_threshold_bps: 500,
        large_tx_threshold: 100_000,
        default_auction_secs: 60,
        min_auction_bid_rebate_bps: 25,
        max_private_route_ttl_secs: 600,
        private_mempool_enabled: true,
        batching_enabled: true,
    }
}

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

pub fn get_config(env: &Env) -> MevProtectionConfig {
    env.storage()
        .persistent()
        .get(&MevDataKey::Config)
        .unwrap_or_else(default_config)
}

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
    create_commit_with_context(
        env,
        owner,
        operation,
        asset,
        secondary_asset,
        borrower,
        amount,
        max_fee_bps,
        hint,
        None,
        None,
        None,
    )
}

pub fn create_guarded_commit(
    env: &Env,
    owner: Address,
    operation: SensitiveOperation,
    asset: Option<Address>,
    secondary_asset: Option<Address>,
    borrower: Option<Address>,
    amount: i128,
    max_fee_bps: i128,
    hint: TxOrderingHint,
    guard: ExecutionGuard,
    private_route: Option<Symbol>,
    auction_id: Option<u64>,
) -> Result<u64, MevProtectionError> {
    create_commit_with_context(
        env,
        owner,
        operation,
        asset,
        secondary_asset,
        borrower,
        amount,
        max_fee_bps,
        hint,
        Some(guard),
        private_route,
        auction_id,
    )
}

fn create_commit_with_context(
    env: &Env,
    owner: Address,
    operation: SensitiveOperation,
    asset: Option<Address>,
    secondary_asset: Option<Address>,
    borrower: Option<Address>,
    amount: i128,
    max_fee_bps: i128,
    hint: TxOrderingHint,
    guard: Option<ExecutionGuard>,
    private_route: Option<Symbol>,
    auction_id: Option<u64>,
) -> Result<u64, MevProtectionError> {
    owner.require_auth();
    if amount <= 0 {
        return Err(MevProtectionError::InvalidAmount);
    }
    if !(0..=MAX_BPS).contains(&max_fee_bps) {
        return Err(MevProtectionError::InvalidConfig);
    }

    let cfg = get_config(env);
    if let Some(ref execution_guard) = guard {
        validate_guard_config(env, execution_guard)?;
    }
    if let Some(ref route_id) = private_route {
        ensure_private_route(env, route_id)?;
    }
    if let Some(id) = auction_id {
        ensure_auction_for_commit(env, id, &owner)?;
    }

    let id = next_commit_id(env);
    let now = env.ledger().timestamp();
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
        guard,
        private_route,
        auction_id,
    };
    env.storage()
        .persistent()
        .set(&MevDataKey::Commit(id), &commit);
    Ok(id)
}

pub fn get_commit(env: &Env, commit_id: u64) -> Option<PendingCommit> {
    env.storage()
        .persistent()
        .get(&MevDataKey::Commit(commit_id))
}

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

pub fn requires_commit_reveal(env: &Env, amount: i128) -> bool {
    amount >= get_config(env).large_tx_threshold
}

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

pub fn get_ordering_stats(env: &Env) -> OrderingStats {
    env.storage()
        .persistent()
        .get(&MevDataKey::OrderingStats)
        .unwrap_or(OrderingStats {
            suspicious_sequences: 0,
            sandwich_alerts: 0,
            last_alert_timestamp: 0,
            last_effective_fee_bps: 0,
        })
}

pub fn get_auction_stats(env: &Env) -> AuctionStats {
    env.storage()
        .persistent()
        .get(&MevDataKey::AuctionStats)
        .unwrap_or(AuctionStats {
            opened: 0,
            bids: 0,
            settled: 0,
            executed: 0,
            last_auction_id: 0,
            last_settled_timestamp: 0,
        })
}

pub fn get_private_route_stats(env: &Env) -> PrivateRouteStats {
    env.storage()
        .persistent()
        .get(&MevDataKey::PrivateStats)
        .unwrap_or(PrivateRouteStats {
            active_routes: 0,
            executions: 0,
            last_execution_timestamp: 0,
        })
}

pub fn get_gas_bid_stats(
    env: &Env,
    operation: SensitiveOperation,
    asset: Option<Address>,
) -> GasBidStats {
    let key = MevDataKey::GasBidStats(operation_symbol(env, &operation), asset);
    env.storage().persistent().get(&key).unwrap_or(GasBidStats {
        samples: 0,
        min_bid_microlumens: 0,
        max_bid_microlumens: 0,
        avg_bid_microlumens: 0,
        last_bid_microlumens: 0,
        avg_inclusion_delay_ledgers: 0,
        last_updated: 0,
    })
}

pub fn get_monitoring_dashboard(
    env: &Env,
    operation: SensitiveOperation,
    asset: Option<Address>,
    amount: i128,
) -> MevMonitoringDashboard {
    let recommended_hint = execution_hint(env, TxOrderingHint::Default);
    let recommended_fee_bps = preview_fee_bps(env, operation.clone(), asset.clone(), amount);
    MevMonitoringDashboard {
        ordering: get_ordering_stats(env),
        gas_bids: get_gas_bid_stats(env, operation, asset),
        auctions: get_auction_stats(env),
        private_routes: get_private_route_stats(env),
        recommended_fee_bps,
        recommended_hint,
    }
}

pub fn record_gas_bid_sample(
    env: &Env,
    reporter: Address,
    operation: SensitiveOperation,
    asset: Option<Address>,
    bid_microlumens: i128,
    inclusion_delay_ledgers: u64,
) -> Result<GasBidStats, MevProtectionError> {
    reporter.require_auth();
    if bid_microlumens <= 0 {
        return Err(MevProtectionError::InvalidAmount);
    }

    let key = MevDataKey::GasBidStats(operation_symbol(env, &operation), asset);
    let mut stats = env.storage().persistent().get(&key).unwrap_or(GasBidStats {
        samples: 0,
        min_bid_microlumens: bid_microlumens,
        max_bid_microlumens: bid_microlumens,
        avg_bid_microlumens: 0,
        last_bid_microlumens: 0,
        avg_inclusion_delay_ledgers: 0,
        last_updated: 0,
    });

    let next_samples = stats.samples.saturating_add(1);
    stats.min_bid_microlumens = if stats.samples == 0 {
        bid_microlumens
    } else {
        stats.min_bid_microlumens.min(bid_microlumens)
    };
    stats.max_bid_microlumens = stats.max_bid_microlumens.max(bid_microlumens);
    stats.avg_bid_microlumens = stats
        .avg_bid_microlumens
        .saturating_mul(i128::from(stats.samples))
        .saturating_add(bid_microlumens)
        .saturating_div(i128::from(next_samples));
    stats.avg_inclusion_delay_ledgers = stats
        .avg_inclusion_delay_ledgers
        .saturating_mul(stats.samples)
        .saturating_add(inclusion_delay_ledgers)
        .saturating_div(next_samples);
    stats.samples = next_samples;
    stats.last_bid_microlumens = bid_microlumens;
    stats.last_updated = env.ledger().timestamp();

    env.storage().persistent().set(&key, &stats);
    Ok(stats)
}

pub fn register_private_route(
    env: &Env,
    caller: Address,
    route_id: Symbol,
    relay: Address,
    ttl_secs: u64,
) -> Result<PrivateMempoolRoute, MevProtectionError> {
    caller.require_auth();
    let cfg = get_config(env);
    if !cfg.private_mempool_enabled || ttl_secs == 0 || ttl_secs > cfg.max_private_route_ttl_secs {
        return Err(MevProtectionError::InvalidConfig);
    }

    let now = env.ledger().timestamp();
    let existed: Option<PrivateMempoolRoute> = env
        .storage()
        .persistent()
        .get(&MevDataKey::PrivateRoute(route_id.clone()));
    let route = PrivateMempoolRoute {
        route_id: route_id.clone(),
        relay,
        registered_by: caller,
        registered_at: now,
        expires_at: now.saturating_add(ttl_secs),
        active: true,
    };
    env.storage()
        .persistent()
        .set(&MevDataKey::PrivateRoute(route_id), &route);

    if existed.is_none() {
        let mut stats = get_private_route_stats(env);
        stats.active_routes = stats.active_routes.saturating_add(1);
        env.storage()
            .persistent()
            .set(&MevDataKey::PrivateStats, &stats);
    }
    Ok(route)
}

pub fn get_private_route(env: &Env, route_id: Symbol) -> Option<PrivateMempoolRoute> {
    env.storage()
        .persistent()
        .get(&MevDataKey::PrivateRoute(route_id))
}

pub fn record_private_execution(
    env: &Env,
    relay: Address,
    commit_id: u64,
    route_id: Symbol,
) -> Result<PrivateExecutionReceipt, MevProtectionError> {
    relay.require_auth();
    let commit = load_commit(env, commit_id)?;
    let route = ensure_private_route(env, &route_id)?;
    if route.relay != relay {
        return Err(MevProtectionError::Unauthorized);
    }
    if commit.hint != TxOrderingHint::PrivateMempool {
        return Err(MevProtectionError::InvalidOperation);
    }
    if let Some(ref expected_route) = commit.private_route {
        if expected_route != &route_id {
            return Err(MevProtectionError::PrivateRouteRequired);
        }
    }

    let now = env.ledger().timestamp();
    let receipt = PrivateExecutionReceipt {
        commit_id,
        route_id: route.route_id,
        relay,
        received_at: now,
        expires_at: commit.expires_at,
    };
    env.storage()
        .persistent()
        .set(&MevDataKey::PrivateReceipt(commit_id), &receipt);

    let mut stats = get_private_route_stats(env);
    stats.executions = stats.executions.saturating_add(1);
    stats.last_execution_timestamp = now;
    env.storage()
        .persistent()
        .set(&MevDataKey::PrivateStats, &stats);
    Ok(receipt)
}

pub fn open_liquidation_auction(
    env: &Env,
    opener: Address,
    borrower: Address,
    debt_asset: Option<Address>,
    collateral_asset: Option<Address>,
    debt_amount: i128,
    min_rebate_bps: i128,
    bidding_period_secs: u64,
) -> Result<u64, MevProtectionError> {
    opener.require_auth();
    let cfg = get_config(env);
    if !cfg.batching_enabled {
        return Err(MevProtectionError::InvalidConfig);
    }
    if debt_amount <= 0 {
        return Err(MevProtectionError::InvalidAmount);
    }
    if !(0..=MAX_BPS).contains(&min_rebate_bps) || min_rebate_bps < cfg.min_auction_bid_rebate_bps {
        return Err(MevProtectionError::InvalidConfig);
    }

    let now = env.ledger().timestamp();
    let auction_id = next_auction_id(env);
    let auction_secs = if bidding_period_secs == 0 {
        cfg.default_auction_secs
    } else {
        bidding_period_secs
    };
    let auction = LiquidationAuction {
        id: auction_id,
        opener,
        borrower,
        debt_asset,
        collateral_asset,
        debt_amount,
        min_rebate_bps,
        opened_at: now,
        bidding_deadline: now.saturating_add(auction_secs),
        status: AuctionStatus::Open,
        best_bid_id: None,
        winning_liquidator: None,
    };
    env.storage()
        .persistent()
        .set(&MevDataKey::Auction(auction_id), &auction);

    let mut stats = get_auction_stats(env);
    stats.opened = stats.opened.saturating_add(1);
    stats.last_auction_id = auction_id;
    env.storage()
        .persistent()
        .set(&MevDataKey::AuctionStats, &stats);
    Ok(auction_id)
}

pub fn submit_liquidation_bid(
    env: &Env,
    liquidator: Address,
    auction_id: u64,
    repay_amount: i128,
    rebate_bps: i128,
    max_fee_bps: i128,
    min_collateral_out: i128,
    private_route: Option<Symbol>,
) -> Result<u64, MevProtectionError> {
    liquidator.require_auth();
    if repay_amount <= 0 || min_collateral_out < 0 {
        return Err(MevProtectionError::InvalidAmount);
    }
    if !(0..=MAX_BPS).contains(&rebate_bps) || !(0..=MAX_BPS).contains(&max_fee_bps) {
        return Err(MevProtectionError::InvalidConfig);
    }
    if let Some(ref route_id) = private_route {
        ensure_private_route(env, route_id)?;
    }

    let mut auction = load_auction(env, auction_id)?;
    let now = env.ledger().timestamp();
    if auction.status != AuctionStatus::Open || now > auction.bidding_deadline {
        return Err(MevProtectionError::AuctionNotOpen);
    }
    if rebate_bps < auction.min_rebate_bps {
        return Err(MevProtectionError::BidTooLow);
    }

    let bid_id = next_bid_id(env);
    let bid = LiquidationAuctionBid {
        id: bid_id,
        auction_id,
        liquidator,
        repay_amount,
        rebate_bps,
        max_fee_bps,
        min_collateral_out,
        private_route,
        submitted_at: now,
    };

    let replaces_best = match auction.best_bid_id {
        None => true,
        Some(best_id) => {
            let best = load_bid(env, best_id)?;
            bid.rebate_bps > best.rebate_bps
                || (bid.rebate_bps == best.rebate_bps && bid.repay_amount > best.repay_amount)
        }
    };
    if replaces_best {
        auction.best_bid_id = Some(bid_id);
        auction.winning_liquidator = Some(bid.liquidator.clone());
        env.storage()
            .persistent()
            .set(&MevDataKey::Auction(auction_id), &auction);
    }

    env.storage()
        .persistent()
        .set(&MevDataKey::AuctionBid(bid_id), &bid);
    let mut stats = get_auction_stats(env);
    stats.bids = stats.bids.saturating_add(1);
    env.storage()
        .persistent()
        .set(&MevDataKey::AuctionStats, &stats);
    Ok(bid_id)
}

pub fn settle_liquidation_auction(
    env: &Env,
    caller: Address,
    auction_id: u64,
) -> Result<LiquidationAuctionBid, MevProtectionError> {
    caller.require_auth();
    let mut auction = load_auction(env, auction_id)?;
    let now = env.ledger().timestamp();
    if auction.status != AuctionStatus::Open {
        return Err(MevProtectionError::AuctionNotOpen);
    }
    if now <= auction.bidding_deadline {
        return Err(MevProtectionError::AuctionNotReady);
    }
    let best_id = auction.best_bid_id.ok_or(MevProtectionError::BidNotFound)?;
    let best = load_bid(env, best_id)?;

    auction.status = AuctionStatus::Settled;
    auction.winning_liquidator = Some(best.liquidator.clone());
    env.storage()
        .persistent()
        .set(&MevDataKey::Auction(auction_id), &auction);

    let mut stats = get_auction_stats(env);
    stats.settled = stats.settled.saturating_add(1);
    stats.last_settled_timestamp = now;
    env.storage()
        .persistent()
        .set(&MevDataKey::AuctionStats, &stats);
    Ok(best)
}

pub fn get_liquidation_auction(env: &Env, auction_id: u64) -> Option<LiquidationAuction> {
    env.storage()
        .persistent()
        .get(&MevDataKey::Auction(auction_id))
}

pub fn get_liquidation_bid(env: &Env, bid_id: u64) -> Option<LiquidationAuctionBid> {
    env.storage()
        .persistent()
        .get(&MevDataKey::AuctionBid(bid_id))
}

pub fn create_liquidation_auction_commit(
    env: &Env,
    liquidator: Address,
    auction_id: u64,
    guard: ExecutionGuard,
    private_route: Option<Symbol>,
) -> Result<u64, MevProtectionError> {
    let auction = load_auction(env, auction_id)?;
    if auction.status != AuctionStatus::Settled {
        return Err(MevProtectionError::AuctionNotReady);
    }
    let best_id = auction.best_bid_id.ok_or(MevProtectionError::BidNotFound)?;
    let best = load_bid(env, best_id)?;
    if best.liquidator != liquidator {
        return Err(MevProtectionError::Unauthorized);
    }
    if guard.min_output_amount < best.min_collateral_out {
        return Err(MevProtectionError::SlippageExceeded);
    }

    create_commit_with_context(
        env,
        liquidator,
        SensitiveOperation::Liquidate,
        auction.debt_asset,
        auction.collateral_asset,
        Some(auction.borrower),
        auction.debt_amount,
        best.max_fee_bps,
        TxOrderingHint::BatchAuction,
        Some(guard),
        private_route,
        Some(auction_id),
    )
}

pub fn reveal_borrow(
    env: &Env,
    owner: Address,
    commit_id: u64,
) -> Result<(Option<Address>, i128, i128), MevProtectionError> {
    owner.require_auth();
    let commit = validate_reveal(env, &owner, commit_id, SensitiveOperation::Borrow)?;
    validate_guard_if_present(env, &commit, commit.amount)?;
    validate_private_receipt_if_needed(env, &commit)?;
    let effective_fee_bps = preview_fee_bps(
        env,
        SensitiveOperation::Borrow,
        commit.asset.clone(),
        commit.amount,
    );
    if effective_fee_bps > commit.max_fee_bps {
        return Err(MevProtectionError::FeeCapExceeded);
    }
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

pub fn reveal_withdraw(
    env: &Env,
    owner: Address,
    commit_id: u64,
) -> Result<(Option<Address>, i128), MevProtectionError> {
    owner.require_auth();
    let commit = validate_reveal(env, &owner, commit_id, SensitiveOperation::Withdraw)?;
    validate_guard_if_present(env, &commit, commit.amount)?;
    validate_private_receipt_if_needed(env, &commit)?;
    let effective_fee_bps = preview_fee_bps(
        env,
        SensitiveOperation::Withdraw,
        commit.asset.clone(),
        commit.amount,
    );
    if effective_fee_bps > commit.max_fee_bps {
        return Err(MevProtectionError::FeeCapExceeded);
    }
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

pub fn reveal_liquidation(
    env: &Env,
    owner: Address,
    commit_id: u64,
) -> Result<(Address, Option<Address>, Option<Address>, i128), MevProtectionError> {
    owner.require_auth();
    let commit = validate_reveal(env, &owner, commit_id, SensitiveOperation::Liquidate)?;
    if commit.guard.is_some() {
        return Err(MevProtectionError::InvalidOperation);
    }
    validate_private_receipt_if_needed(env, &commit)?;
    validate_auction_commit_if_needed(env, &commit)?;
    let effective_fee_bps = preview_fee_bps(
        env,
        SensitiveOperation::Liquidate,
        commit.asset.clone(),
        commit.amount,
    );
    if effective_fee_bps > commit.max_fee_bps {
        return Err(MevProtectionError::FeeCapExceeded);
    }
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
    record_auction_execution_if_needed(env, &commit);
    Ok((
        borrower,
        commit.asset,
        commit.secondary_asset,
        commit.amount,
    ))
}

pub fn reveal_liquidation_with_output(
    env: &Env,
    owner: Address,
    commit_id: u64,
    expected_collateral_out: i128,
) -> Result<(Address, Option<Address>, Option<Address>, i128), MevProtectionError> {
    owner.require_auth();
    let commit = validate_reveal(env, &owner, commit_id, SensitiveOperation::Liquidate)?;
    validate_guard_if_present(env, &commit, expected_collateral_out)?;
    validate_private_receipt_if_needed(env, &commit)?;
    validate_auction_commit_if_needed(env, &commit)?;

    let effective_fee_bps = preview_fee_bps(
        env,
        SensitiveOperation::Liquidate,
        commit.asset.clone(),
        commit.amount,
    );
    if effective_fee_bps > commit.max_fee_bps {
        return Err(MevProtectionError::FeeCapExceeded);
    }
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
    record_auction_execution_if_needed(env, &commit);
    Ok((
        borrower,
        commit.asset,
        commit.secondary_asset,
        commit.amount,
    ))
}

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
    Ok(commit)
}

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

    if let Some(last) = latest.clone() {
        if now.saturating_sub(last.timestamp) <= cfg.suspicious_window_secs && last.actor != actor {
            stats.suspicious_sequences = stats.suspicious_sequences.saturating_add(1);
        }
    }

    if let (Some(prev), Some(last)) = (previous.clone(), latest.clone()) {
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

fn next_auction_id(env: &Env) -> u64 {
    let id = env
        .storage()
        .persistent()
        .get::<MevDataKey, u64>(&MevDataKey::NextAuctionId)
        .unwrap_or(1);
    env.storage()
        .persistent()
        .set(&MevDataKey::NextAuctionId, &id.saturating_add(1));
    id
}

fn next_bid_id(env: &Env) -> u64 {
    let id = env
        .storage()
        .persistent()
        .get::<MevDataKey, u64>(&MevDataKey::NextBidId)
        .unwrap_or(1);
    env.storage()
        .persistent()
        .set(&MevDataKey::NextBidId, &id.saturating_add(1));
    id
}

fn load_auction(env: &Env, auction_id: u64) -> Result<LiquidationAuction, MevProtectionError> {
    env.storage()
        .persistent()
        .get(&MevDataKey::Auction(auction_id))
        .ok_or(MevProtectionError::AuctionNotFound)
}

fn load_bid(env: &Env, bid_id: u64) -> Result<LiquidationAuctionBid, MevProtectionError> {
    env.storage()
        .persistent()
        .get(&MevDataKey::AuctionBid(bid_id))
        .ok_or(MevProtectionError::BidNotFound)
}

fn ensure_private_route(
    env: &Env,
    route_id: &Symbol,
) -> Result<PrivateMempoolRoute, MevProtectionError> {
    let route: PrivateMempoolRoute = env
        .storage()
        .persistent()
        .get(&MevDataKey::PrivateRoute(route_id.clone()))
        .ok_or(MevProtectionError::PrivateRouteNotFound)?;
    if !route.active || env.ledger().timestamp() > route.expires_at {
        return Err(MevProtectionError::PrivateRouteNotFound);
    }
    Ok(route)
}

fn ensure_auction_for_commit(
    env: &Env,
    auction_id: u64,
    liquidator: &Address,
) -> Result<(), MevProtectionError> {
    let auction = load_auction(env, auction_id)?;
    if auction.status != AuctionStatus::Settled {
        return Err(MevProtectionError::AuctionNotReady);
    }
    if auction.winning_liquidator.as_ref() != Some(liquidator) {
        return Err(MevProtectionError::Unauthorized);
    }
    Ok(())
}

fn validate_guard_config(env: &Env, guard: &ExecutionGuard) -> Result<(), MevProtectionError> {
    if guard.quoted_output_amount <= 0
        || guard.min_output_amount < 0
        || guard.min_output_amount > guard.quoted_output_amount
        || !(0..=MAX_BPS).contains(&guard.max_slippage_bps)
        || guard.deadline <= env.ledger().timestamp()
    {
        return Err(MevProtectionError::InvalidConfig);
    }
    Ok(())
}

fn validate_guard_if_present(
    env: &Env,
    commit: &PendingCommit,
    actual_output_amount: i128,
) -> Result<(), MevProtectionError> {
    let Some(ref guard) = commit.guard else {
        return Ok(());
    };
    if env.ledger().timestamp() > guard.deadline {
        return Err(MevProtectionError::SlippageExpired);
    }

    let slippage_floor = guard
        .quoted_output_amount
        .saturating_mul(MAX_BPS.saturating_sub(guard.max_slippage_bps))
        .saturating_div(MAX_BPS);
    let required_output = guard.min_output_amount.max(slippage_floor);
    if actual_output_amount < required_output {
        return Err(MevProtectionError::SlippageExceeded);
    }
    Ok(())
}

fn validate_private_receipt_if_needed(
    env: &Env,
    commit: &PendingCommit,
) -> Result<(), MevProtectionError> {
    let Some(ref route_id) = commit.private_route else {
        return Ok(());
    };
    let receipt: PrivateExecutionReceipt = env
        .storage()
        .persistent()
        .get(&MevDataKey::PrivateReceipt(commit.id))
        .ok_or(MevProtectionError::PrivateRouteRequired)?;
    if &receipt.route_id != route_id || env.ledger().timestamp() > receipt.expires_at {
        return Err(MevProtectionError::PrivateRouteRequired);
    }
    ensure_private_route(env, route_id)?;
    Ok(())
}

fn validate_auction_commit_if_needed(
    env: &Env,
    commit: &PendingCommit,
) -> Result<(), MevProtectionError> {
    let Some(auction_id) = commit.auction_id else {
        return Ok(());
    };
    ensure_auction_for_commit(env, auction_id, &commit.owner)
}

fn record_auction_execution_if_needed(env: &Env, commit: &PendingCommit) {
    if commit.auction_id.is_none() {
        return;
    }
    let mut stats = get_auction_stats(env);
    stats.executed = stats.executed.saturating_add(1);
    env.storage()
        .persistent()
        .set(&MevDataKey::AuctionStats, &stats);
}

fn validate_config(config: &MevProtectionConfig) -> Result<(), MevProtectionError> {
    if config.commit_delay_secs == 0
        || config.commit_expiry_secs <= config.commit_delay_secs
        || config.suspicious_window_secs == 0
        || !(0..=MAX_BPS).contains(&config.fee_smoothing_bps)
        || !(0..=MAX_BPS).contains(&config.base_protection_fee_bps)
        || !(0..=MAX_BPS).contains(&config.surge_protection_fee_bps)
        || !(0..=MAX_BPS).contains(&config.sandwich_threshold_bps)
        || config.large_tx_threshold <= 0
        || config.default_auction_secs == 0
        || !(0..=MAX_BPS).contains(&config.min_auction_bid_rebate_bps)
        || config.max_private_route_ttl_secs == 0
    {
        return Err(MevProtectionError::InvalidConfig);
    }
    Ok(())
}

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
