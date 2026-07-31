//! # Rate Limiter Module
//!
//! Provides per-user and global-per-pool rate limiting for sensitive operations.
//! Implemented as a token bucket with integer fixed-point math (1e6 scale).
//!
//! ## Adaptive (congestion-aware) limits
//!
//! The steady-state rate and burst capacity can be scaled down automatically when the
//! network is congested, and restored (up to the configured ceiling) when it is quiet.
//!
//! Soroban exposes **no** gas-price or surge-pricing oracle to contracts, so there is no
//! way to read the real inclusion fee on-chain. Two signals are used instead:
//!
//! 1. **Keeper-reported congestion** (`report_congestion`) — an off-chain monitor holding
//!    the `congestion_reporter` role pushes a congestion index in basis points, where
//!    `10_000` means "normal". This is the accurate signal, and it is the integration
//!    point for an off-chain network-monitoring service. Reports expire after
//!    `report_ttl_seconds` so a dead reporter cannot pin limits indefinitely.
//! 2. **Ledger close interval** (permissionless fallback) — average seconds-per-ledger
//!    observed between `consume` calls, compared against a configured baseline. This is
//!    derived purely from `env.ledger()` and needs no trusted party, but it is a coarse
//!    proxy: Stellar targets a fixed close time, so it degrades only under real network
//!    stress and will not detect fee-market congestion.
//!
//! Limits are only ever scaled within the admin-configured
//! `[min_factor_bps, max_factor_bps]` band, and the effective allowance never drops below
//! one call per window, so congestion adaptation can throttle but never lock users out.

#![allow(unused)]

use soroban_sdk::{contracterror, contracttype, Address, Env, Symbol, Vec};

use crate::admin;

const TOKEN_SCALE: i128 = 1_000_000; // fixed-point scale

/// Basis-point scale. `BPS_SCALE` congestion == normal, `BPS_SCALE` factor == unscaled.
const BPS_SCALE: i128 = 10_000;

/// Minimum number of ledgers between close-interval samples. Sampling over too few
/// ledgers makes the derived interval extremely noisy.
const MIN_SAMPLE_LEDGERS: u32 = 10;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RateLimitError {
    /// Caller exceeded their rate limit.
    RateLimited = 1,
    /// Invalid configuration parameters.
    InvalidConfig = 2,
    /// Unauthorized configuration call.
    Unauthorized = 3,
    /// Arithmetic overflow.
    Overflow = 4,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RateLimitConfig {
    /// Time window (seconds) used to derive refill rate.
    pub window_seconds: u64,
    /// Allowed calls per window (steady-state rate).
    pub max_calls_per_window: u32,
    /// Additional burst capacity on top of max_calls_per_window.
    pub burst_calls: u32,
    /// Extra burst calls granted to whitelisted/high-frequency users.
    pub grace_burst_calls: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct BucketState {
    /// Current tokens (scaled by TOKEN_SCALE).
    pub tokens: i128,
    /// Last refill timestamp (ledger timestamp seconds).
    pub last_refill: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RateLimitStatus {
    pub config: RateLimitConfig,
    pub bucket: BucketState,
    /// Capacity in tokens (scaled).
    pub capacity_tokens: i128,
    /// Refill rate per second (scaled tokens/sec).
    pub refill_per_second: i128,
    /// Whether this address is considered grace-enabled for this op.
    pub grace_enabled: bool,
}

/// Admin-tunable parameters controlling congestion-based adaptation.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CongestionConfig {
    /// Master switch. When false, limits behave exactly as the static configuration.
    pub enabled: bool,
    /// Expected seconds per ledger under normal conditions (Stellar targets ~5s).
    pub baseline_secs_per_ledger: u32,
    /// How long a keeper-reported congestion value stays authoritative.
    pub report_ttl_seconds: u64,
    /// Lower bound on the scaling factor applied to limits, in bps (e.g. 2_500 = 25%).
    pub min_factor_bps: i128,
    /// Upper bound on the scaling factor applied to limits, in bps (e.g. 20_000 = 200%).
    pub max_factor_bps: i128,
}

/// Latest keeper-reported congestion index.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CongestionReport {
    /// Congestion index in bps; `10_000` is normal, higher is more congested.
    pub congestion_bps: i128,
    /// Ledger timestamp at which this was reported.
    pub reported_at: u64,
    /// Reporter address, for auditability.
    pub reporter: Address,
}

/// Rolling ledger-close-interval sample used as the permissionless fallback signal.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct LedgerIntervalSample {
    /// Ledger sequence at the start of the current sampling window.
    pub anchor_sequence: u32,
    /// Ledger timestamp at the start of the current sampling window.
    pub anchor_timestamp: u64,
    /// Last fully observed average seconds-per-ledger (0 = not yet observed).
    pub observed_secs_per_ledger: u32,
}

/// Read-only view of the adaptive state, for monitoring and off-chain dashboards.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CongestionState {
    pub config: CongestionConfig,
    /// Effective congestion index in bps currently in force.
    pub congestion_bps: i128,
    /// Scaling factor in bps derived from `congestion_bps` and clamped to the config band.
    pub factor_bps: i128,
    /// Which signal produced `congestion_bps`.
    pub source: Symbol,
    /// The keeper report currently on file, if any (may be stale).
    pub report: Option<CongestionReport>,
    /// The ledger-interval sample currently on file, if any.
    pub sample: Option<LedgerIntervalSample>,
}

#[contracttype]
#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum RateLimitDataKey {
    /// Default config for an operation
    OpConfig(Symbol),
    /// Config override for an operation + pool
    OpPoolConfig(Symbol, Address),
    /// Per-user bucket for (user, operation, pool)
    UserBucket(Address, Symbol, Address),
    /// Global bucket for (operation, pool)
    GlobalBucket(Symbol, Address),
    /// Optional per-user grace enable flag for an operation
    UserGrace(Address, Symbol),
    /// Global congestion adaptation config
    CongestionConfig,
    /// Latest keeper-reported congestion index
    CongestionReport,
    /// Rolling ledger-close-interval sample
    LedgerSample,
}

fn default_congestion_config() -> CongestionConfig {
    CongestionConfig {
        // Off by default: enabling it changes effective limits for live users, so it is an
        // explicit admin decision rather than a silent behaviour change on upgrade.
        enabled: false,
        baseline_secs_per_ledger: 5,
        report_ttl_seconds: 300,
        min_factor_bps: 2_500,  // never throttle below 25% of configured limits
        max_factor_bps: 10_000, // do not exceed configured limits unless admin opts in
    }
}

pub fn get_congestion_config(env: &Env) -> CongestionConfig {
    env.storage()
        .persistent()
        .get::<RateLimitDataKey, CongestionConfig>(&RateLimitDataKey::CongestionConfig)
        .unwrap_or_else(default_congestion_config)
}

fn validate_congestion_config(cfg: &CongestionConfig) -> Result<(), RateLimitError> {
    if cfg.baseline_secs_per_ledger == 0 {
        return Err(RateLimitError::InvalidConfig);
    }
    if cfg.min_factor_bps <= 0 || cfg.max_factor_bps <= 0 {
        return Err(RateLimitError::InvalidConfig);
    }
    if cfg.min_factor_bps > cfg.max_factor_bps {
        return Err(RateLimitError::InvalidConfig);
    }
    Ok(())
}

/// Configure congestion-based adaptation (admin-only).
pub fn configure_congestion(
    env: &Env,
    caller: Address,
    cfg: CongestionConfig,
) -> Result<(), RateLimitError> {
    admin::require_admin(env, &caller).map_err(|_| RateLimitError::Unauthorized)?;
    validate_congestion_config(&cfg)?;
    env.storage()
        .persistent()
        .set(&RateLimitDataKey::CongestionConfig, &cfg);
    Ok(())
}

/// Report the current network congestion index in bps (`10_000` == normal).
///
/// Callable by the admin or any address holding the `congestion_reporter` role. This is
/// the hook for an off-chain network monitor, which can observe the fee market that
/// contracts cannot see. Reports go stale after `report_ttl_seconds`.
pub fn report_congestion(
    env: &Env,
    caller: Address,
    congestion_bps: i128,
) -> Result<(), RateLimitError> {
    let is_admin = admin::get_admin(env)
        .map(|a| a == caller)
        .unwrap_or(false);
    if !is_admin
        && !admin::has_role(env, Symbol::new(env, "congestion_reporter"), caller.clone())
    {
        return Err(RateLimitError::Unauthorized);
    }
    caller.require_auth();
    if congestion_bps <= 0 {
        return Err(RateLimitError::InvalidConfig);
    }
    let report = CongestionReport {
        congestion_bps,
        reported_at: env.ledger().timestamp(),
        reporter: caller,
    };
    env.storage()
        .persistent()
        .set(&RateLimitDataKey::CongestionReport, &report);
    Ok(())
}

fn get_report(env: &Env) -> Option<CongestionReport> {
    env.storage()
        .persistent()
        .get::<RateLimitDataKey, CongestionReport>(&RateLimitDataKey::CongestionReport)
}

fn get_sample(env: &Env) -> Option<LedgerIntervalSample> {
    env.storage()
        .persistent()
        .get::<RateLimitDataKey, LedgerIntervalSample>(&RateLimitDataKey::LedgerSample)
}

/// Advance the ledger-close-interval sampler. Called on the hot path from `consume`.
///
/// Cheap and self-healing: it anchors on the first observation, and once at least
/// `MIN_SAMPLE_LEDGERS` ledgers have elapsed it records the average close interval over
/// that span and re-anchors.
fn observe_ledger_interval(env: &Env) {
    let seq = env.ledger().sequence();
    let now = env.ledger().timestamp();

    let mut sample = get_sample(env).unwrap_or(LedgerIntervalSample {
        anchor_sequence: seq,
        anchor_timestamp: now,
        observed_secs_per_ledger: 0,
    });

    // Guard against a re-anchor going backwards (e.g. after a reset in tests).
    if seq < sample.anchor_sequence || now < sample.anchor_timestamp {
        sample.anchor_sequence = seq;
        sample.anchor_timestamp = now;
        env.storage()
            .persistent()
            .set(&RateLimitDataKey::LedgerSample, &sample);
        return;
    }

    let ledgers = seq - sample.anchor_sequence;
    if ledgers >= MIN_SAMPLE_LEDGERS {
        let elapsed = now - sample.anchor_timestamp;
        let per_ledger = elapsed / (ledgers as u64);
        sample.observed_secs_per_ledger = per_ledger.min(u32::MAX as u64) as u32;
        sample.anchor_sequence = seq;
        sample.anchor_timestamp = now;
        env.storage()
            .persistent()
            .set(&RateLimitDataKey::LedgerSample, &sample);
    } else if get_sample(env).is_none() {
        env.storage()
            .persistent()
            .set(&RateLimitDataKey::LedgerSample, &sample);
    }
}

/// Resolve the congestion index in force, and the signal it came from.
fn resolve_congestion_bps(env: &Env, cfg: &CongestionConfig) -> (i128, Symbol) {
    if !cfg.enabled {
        return (BPS_SCALE, Symbol::new(env, "disabled"));
    }

    // 1. A fresh keeper report wins.
    if let Some(report) = get_report(env) {
        let now = env.ledger().timestamp();
        if now >= report.reported_at && now - report.reported_at <= cfg.report_ttl_seconds {
            return (report.congestion_bps, Symbol::new(env, "reported"));
        }
    }

    // 2. Fall back to the observed ledger close interval vs. baseline.
    if let Some(sample) = get_sample(env) {
        if sample.observed_secs_per_ledger > 0 {
            let observed = sample.observed_secs_per_ledger as i128;
            let baseline = cfg.baseline_secs_per_ledger as i128;
            let ratio = observed
                .checked_mul(BPS_SCALE)
                .and_then(|v| v.checked_div(baseline))
                .unwrap_or(BPS_SCALE);
            return (ratio.max(1), Symbol::new(env, "ledger"));
        }
    }

    // 3. No usable signal — assume normal.
    (BPS_SCALE, Symbol::new(env, "none"))
}

/// Convert a congestion index into a limit scaling factor, clamped to the config band.
///
/// The relationship is inverse: twice the normal congestion halves the allowance.
fn factor_bps_from_congestion(cfg: &CongestionConfig, congestion_bps: i128) -> i128 {
    if !cfg.enabled {
        return BPS_SCALE;
    }
    if congestion_bps <= 0 {
        return cfg.max_factor_bps.min(BPS_SCALE.max(cfg.min_factor_bps));
    }
    let raw = BPS_SCALE
        .checked_mul(BPS_SCALE)
        .and_then(|v| v.checked_div(congestion_bps))
        .unwrap_or(BPS_SCALE);
    raw.max(cfg.min_factor_bps).min(cfg.max_factor_bps)
}

fn scale_calls(value: u32, factor_bps: i128, floor: u32) -> u32 {
    if factor_bps == BPS_SCALE {
        return value;
    }
    let scaled = (value as i128)
        .checked_mul(factor_bps)
        .and_then(|v| v.checked_div(BPS_SCALE))
        .unwrap_or(value as i128);
    scaled.max(floor as i128).min(u32::MAX as i128) as u32
}

/// Apply the current congestion factor to a static config.
///
/// `max_calls_per_window` keeps a floor of 1 so throttling can never fully deny an
/// operation; burst and grace capacity are allowed to scale down to zero.
fn apply_congestion(cfg: &RateLimitConfig, factor_bps: i128) -> RateLimitConfig {
    if factor_bps == BPS_SCALE {
        return cfg.clone();
    }
    RateLimitConfig {
        window_seconds: cfg.window_seconds,
        max_calls_per_window: scale_calls(cfg.max_calls_per_window, factor_bps, 1),
        burst_calls: scale_calls(cfg.burst_calls, factor_bps, 0),
        grace_burst_calls: scale_calls(cfg.grace_burst_calls, factor_bps, 0),
    }
}

/// The config actually enforced for `(op, pool)` right now, congestion included.
fn effective_config(env: &Env, op: &Symbol, pool: &Address) -> RateLimitConfig {
    let base = get_pool_config(env, op, pool);
    let cfg = get_congestion_config(env);
    if !cfg.enabled {
        return base;
    }
    let (congestion_bps, _) = resolve_congestion_bps(env, &cfg);
    apply_congestion(&base, factor_bps_from_congestion(&cfg, congestion_bps))
}

/// Read-only: current adaptive state, for monitoring and configuration UIs.
pub fn get_congestion_state(env: &Env) -> CongestionState {
    let cfg = get_congestion_config(env);
    let (congestion_bps, source) = resolve_congestion_bps(env, &cfg);
    let factor_bps = factor_bps_from_congestion(&cfg, congestion_bps);
    CongestionState {
        config: cfg,
        congestion_bps,
        factor_bps,
        source,
        report: get_report(env),
        sample: get_sample(env),
    }
}

fn default_config(env: &Env, op: &Symbol) -> RateLimitConfig {
    // Conservative defaults: set low but non-zero limits to make abuse harder,
    // while allowing typical UX. Admin can tune per operation/pool.
    // Borrow and liquidate are the primary targets.
    let name = op.to_string();
    if name == "borrow" {
        RateLimitConfig {
            window_seconds: 60,
            max_calls_per_window: 5,
            burst_calls: 3,
            grace_burst_calls: 10,
        }
    } else if name == "liquidate" {
        RateLimitConfig {
            window_seconds: 60,
            max_calls_per_window: 10,
            burst_calls: 5,
            grace_burst_calls: 20,
        }
    } else {
        RateLimitConfig {
            window_seconds: 60,
            max_calls_per_window: 30,
            burst_calls: 10,
            grace_burst_calls: 0,
        }
    }
}

fn get_op_config(env: &Env, op: &Symbol) -> RateLimitConfig {
    let key = RateLimitDataKey::OpConfig(op.clone());
    env.storage()
        .persistent()
        .get::<RateLimitDataKey, RateLimitConfig>(&key)
        .unwrap_or_else(|| default_config(env, op))
}

fn get_pool_config(env: &Env, op: &Symbol, pool: &Address) -> RateLimitConfig {
    let key = RateLimitDataKey::OpPoolConfig(op.clone(), pool.clone());
    env.storage()
        .persistent()
        .get::<RateLimitDataKey, RateLimitConfig>(&key)
        .unwrap_or_else(|| get_op_config(env, op))
}

fn is_grace_enabled(env: &Env, user: &Address, op: &Symbol) -> bool {
    let key = RateLimitDataKey::UserGrace(user.clone(), op.clone());
    env.storage()
        .persistent()
        .get::<RateLimitDataKey, bool>(&key)
        .unwrap_or(false)
}

fn capacity_tokens(cfg: &RateLimitConfig, grace: bool) -> i128 {
    // Capacity is derived from config; validate_config ensures non-zero window and max calls.
    let base = (cfg.max_calls_per_window as i128)
        .checked_add(cfg.burst_calls as i128)
        .unwrap_or(i128::MAX);
    let extra = if grace {
        cfg.grace_burst_calls as i128
    } else {
        0
    };
    base.checked_add(extra)
        .and_then(|v| v.checked_mul(TOKEN_SCALE))
        .unwrap_or(i128::MAX)
}

fn refill_per_second(cfg: &RateLimitConfig) -> Result<i128, RateLimitError> {
    if cfg.window_seconds == 0 || cfg.max_calls_per_window == 0 {
        return Err(RateLimitError::InvalidConfig);
    }
    let per_window_tokens = (cfg.max_calls_per_window as i128)
        .checked_mul(TOKEN_SCALE)
        .ok_or(RateLimitError::Overflow)?;
    per_window_tokens
        .checked_div(cfg.window_seconds as i128)
        .ok_or(RateLimitError::Overflow)
}

fn refill_bucket(
    env: &Env,
    mut bucket: BucketState,
    cfg: &RateLimitConfig,
    cap_tokens: i128,
) -> Result<BucketState, RateLimitError> {
    let now = env.ledger().timestamp();
    if now <= bucket.last_refill {
        return Ok(bucket);
    }
    let dt = now - bucket.last_refill;
    let rate = refill_per_second(cfg)?;
    let add = rate
        .checked_mul(dt as i128)
        .ok_or(RateLimitError::Overflow)?;
    bucket.tokens = bucket
        .tokens
        .checked_add(add)
        .ok_or(RateLimitError::Overflow)?
        .min(cap_tokens);
    bucket.last_refill = now;
    Ok(bucket)
}

fn get_or_init_bucket(env: &Env, key: &RateLimitDataKey, cap_tokens: i128) -> BucketState {
    env.storage()
        .persistent()
        .get::<RateLimitDataKey, BucketState>(key)
        .unwrap_or(BucketState {
            tokens: cap_tokens,
            last_refill: env.ledger().timestamp(),
        })
}

fn set_bucket(env: &Env, key: &RateLimitDataKey, bucket: &BucketState) {
    env.storage().persistent().set(key, bucket);
}

fn is_bypassed(env: &Env, caller: &Address) -> bool {
    // Governance/admin bypass: admin can always act (e.g., emergency actions),
    // and a dedicated role can be granted for bots/keepers.
    if admin::get_admin(env).map(|a| a == *caller).unwrap_or(false) {
        return true;
    }
    admin::has_role(env, Symbol::new(env, "rate_limit_bypass"), caller.clone())
}

/// Configure default rate limit parameters for an operation (admin-only).
pub fn configure_operation_limit(
    env: &Env,
    caller: Address,
    op: Symbol,
    cfg: RateLimitConfig,
) -> Result<(), RateLimitError> {
    admin::require_admin(env, &caller).map_err(|_| RateLimitError::Unauthorized)?;
    validate_config(&cfg)?;
    let key = RateLimitDataKey::OpConfig(op);
    env.storage().persistent().set(&key, &cfg);
    Ok(())
}

/// Configure per-pool global rate limit parameters for an operation (admin-only).
pub fn configure_pool_limit(
    env: &Env,
    caller: Address,
    op: Symbol,
    pool: Address,
    cfg: RateLimitConfig,
) -> Result<(), RateLimitError> {
    admin::require_admin(env, &caller).map_err(|_| RateLimitError::Unauthorized)?;
    validate_config(&cfg)?;
    let key = RateLimitDataKey::OpPoolConfig(op, pool);
    env.storage().persistent().set(&key, &cfg);
    Ok(())
}

/// Enable/disable grace for a (user, operation) pair (admin-only).
pub fn set_user_grace(
    env: &Env,
    caller: Address,
    user: Address,
    op: Symbol,
    enabled: bool,
) -> Result<(), RateLimitError> {
    admin::require_admin(env, &caller).map_err(|_| RateLimitError::Unauthorized)?;
    let key = RateLimitDataKey::UserGrace(user, op);
    env.storage().persistent().set(&key, &enabled);
    Ok(())
}

fn validate_config(cfg: &RateLimitConfig) -> Result<(), RateLimitError> {
    if cfg.window_seconds == 0 {
        return Err(RateLimitError::InvalidConfig);
    }
    if cfg.max_calls_per_window == 0 {
        return Err(RateLimitError::InvalidConfig);
    }
    Ok(())
}

/// Consume one unit from the per-user and global-per-pool limit buckets.
///
/// This should be called at the beginning of sensitive entrypoints.
pub fn consume(
    env: &Env,
    caller: &Address,
    user: &Address,
    op: &Symbol,
    pool: &Address,
) -> Result<(), RateLimitError> {
    if is_bypassed(env, caller) {
        return Ok(());
    }

    // Keep the permissionless congestion signal fresh. Done before resolving the config
    // so a newly completed sample takes effect on this same call.
    let congestion_cfg = get_congestion_config(env);
    if congestion_cfg.enabled {
        observe_ledger_interval(env);
    }

    let cfg = effective_config(env, op, pool);
    let grace = is_grace_enabled(env, user, op);
    let cap = capacity_tokens(&cfg, grace);

    // Per-user bucket
    let user_key = RateLimitDataKey::UserBucket(user.clone(), op.clone(), pool.clone());
    let user_bucket = get_or_init_bucket(env, &user_key, cap);
    let mut user_bucket = refill_bucket(env, user_bucket, &cfg, cap)?;
    if user_bucket.tokens < TOKEN_SCALE {
        return Err(RateLimitError::RateLimited);
    }
    user_bucket.tokens = user_bucket
        .tokens
        .checked_sub(TOKEN_SCALE)
        .ok_or(RateLimitError::Overflow)?;
    set_bucket(env, &user_key, &user_bucket);

    // Global bucket (per pool)
    let global_key = RateLimitDataKey::GlobalBucket(op.clone(), pool.clone());
    let global_bucket = get_or_init_bucket(env, &global_key, cap);
    let mut global_bucket = refill_bucket(env, global_bucket, &cfg, cap)?;
    if global_bucket.tokens < TOKEN_SCALE {
        return Err(RateLimitError::RateLimited);
    }
    global_bucket.tokens = global_bucket
        .tokens
        .checked_sub(TOKEN_SCALE)
        .ok_or(RateLimitError::Overflow)?;
    set_bucket(env, &global_key, &global_bucket);

    Ok(())
}

/// Read-only: return the current effective status for a per-user bucket.
pub fn get_user_status(env: &Env, user: Address, op: Symbol, pool: Address) -> RateLimitStatus {
    let cfg = effective_config(env, &op, &pool);
    let grace = is_grace_enabled(env, &user, &op);
    let cap = capacity_tokens(&cfg, grace);
    let key = RateLimitDataKey::UserBucket(user.clone(), op.clone(), pool.clone());
    let bucket = get_or_init_bucket(env, &key, cap);
    let refill = refill_per_second(&cfg).unwrap_or(0);
    RateLimitStatus {
        config: cfg,
        bucket,
        capacity_tokens: cap,
        refill_per_second: refill,
        grace_enabled: grace,
    }
}

/// Read-only: return the current effective status for the global-per-pool bucket.
pub fn get_global_status(env: &Env, op: Symbol, pool: Address) -> RateLimitStatus {
    let cfg = effective_config(env, &op, &pool);
    let cap = capacity_tokens(&cfg, false);
    let key = RateLimitDataKey::GlobalBucket(op.clone(), pool.clone());
    let bucket = get_or_init_bucket(env, &key, cap);
    let refill = refill_per_second(&cfg).unwrap_or(0);
    RateLimitStatus {
        config: cfg,
        bucket,
        capacity_tokens: cap,
        refill_per_second: refill,
        grace_enabled: false,
    }
}
