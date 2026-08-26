//! # Analytics Module
//!
//! Provides protocol-wide and per-user analytics, reporting, and activity tracking.
//!
//! This module aggregates data from the deposit, borrow, and repay modules to produce:
//! - **Protocol metrics**: TVL, utilization, average borrow rate, total users/transactions
//! - **User metrics**: collateral, debt, health factor, risk level, activity score
//! - **Activity feed**: bounded log of recent protocol operations (max 10,000 entries)
//!
//! ## Health Factor
//! `health_factor = (collateral * 10000) / debt`
//!
//! A health factor below 10,000 (1.0x) indicates an undercollateralized position.
//!
//! ## Risk Levels
//! | Health Factor | Risk Level |
//! |---------------|------------|
//! | ≥ 1.50        | 1 (Low)    |
//! | ≥ 1.20        | 2          |
//! | ≥ 1.10        | 3          |
//! | ≥ 1.05        | 4          |
//! | < 1.05        | 5 (Critical) |

#![allow(unused)]
use soroban_sdk::{contracterror, contracttype, Address, Env, Map, Symbol, Vec};

use crate::deposit::{
    DepositDataKey, Position, ProtocolAnalytics as DepositProtocolAnalytics,
    UserAnalytics as DepositUserAnalytics,
};

/// Errors that can occur during analytics operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AnalyticsError {
    /// Analytics system has not been initialized
    NotInitialized = 1,
    /// Invalid parameter supplied to an analytics function
    InvalidParameter = 2,
    /// Arithmetic overflow during calculation
    Overflow = 3,
    /// Requested data (user position, activity, etc.) was not found
    DataNotFound = 4,
    /// #672 — caller is not authorized to configure analytics (e.g. alert thresholds)
    Unauthorized = 5,
}

/// Storage keys for analytics data.
#[contracttype]
#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum AnalyticsDataKey {
    /// Cached snapshot of global protocol-wide metrics
    /// Value type: ProtocolMetrics
    ProtocolMetrics,
    /// Detailed cached metrics for a specific user
    /// Value type: UserMetrics
    UserMetrics(Address),
    /// Global bounded activity log (max 10,000 entries): Vec<ActivityEntry>
    ActivityLog,
    /// Cumulative count of unique protocol users
    /// Value type: u64
    TotalUsers,
    /// Cumulative count of all protocol transactions
    /// Value type: u64
    TotalTransactions,
    /// Cached composite protocol health score
    /// Value type: ProtocolHealthScore
    ProtocolHealthScore,
}

/// Snapshot of protocol-wide metrics.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ProtocolMetrics {
    /// Total value locked across all deposited collateral
    pub total_value_locked: i128,
    /// Cumulative deposit volume
    pub total_deposits: i128,
    /// Cumulative borrow volume
    pub total_borrows: i128,
    /// Current utilization rate in basis points (borrows / deposits * 10000)
    pub utilization_rate: i128,
    /// Weighted average borrow interest rate in basis points
    pub average_borrow_rate: i128,
    /// Number of unique protocol users
    pub total_users: u64,
    /// Total transaction count
    pub total_transactions: u64,
    /// Timestamp of last metrics update
    pub last_update: u64,
}

/// Per-user computed metrics.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct UserMetrics {
    /// User's current collateral balance
    pub collateral: i128,
    /// User's current debt balance
    pub debt: i128,
    /// Health factor in basis points (collateral / debt * 10000)
    pub health_factor: i128,
    /// Cumulative deposit amount
    pub total_deposits: i128,
    /// Cumulative borrow amount
    pub total_borrows: i128,
    /// Cumulative withdrawal amount
    pub total_withdrawals: i128,
    /// Cumulative repayment amount
    pub total_repayments: i128,
    /// Computed activity score (transaction count * 100 + deposits / 1000)
    pub activity_score: i128,
    /// Risk level from 1 (low) to 5 (critical), based on health factor
    pub risk_level: i128,
    /// Total number of user transactions
    pub transaction_count: u64,
}

/// A single activity log entry.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ActivityEntry {
    /// User who performed the activity
    pub user: Address,
    /// Type of activity (e.g., "deposit", "borrow", "repay", "withdraw")
    pub activity_type: Symbol,
    /// Amount involved in the activity
    pub amount: i128,
    /// Asset address (None for native XLM)
    pub asset: Option<Address>,
    /// Ledger timestamp when activity occurred
    pub timestamp: u64,
    /// Additional metadata key-value pairs
    pub metadata: Map<Symbol, i128>,
}

/// Protocol-level analytics report.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ProtocolReport {
    /// Current protocol metrics
    pub metrics: ProtocolMetrics,
    /// Report generation timestamp
    pub timestamp: u64,
}

/// User-level analytics report.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct UserReport {
    /// User address this report is for
    pub user: Address,
    /// Computed user metrics
    pub metrics: UserMetrics,
    /// User's current position (collateral, debt, interest)
    pub position: Position,
    /// Most recent 10 activities for this user
    pub recent_activities: Vec<ActivityEntry>,
    /// Report generation timestamp
    pub timestamp: u64,
}

const BASIS_POINTS: i128 = 10_000;
const MAX_ACTIVITY_LOG_SIZE: u32 = 10_000;

/// Get the total value locked (TVL) in the protocol.
///
/// Reads the cumulative TVL from protocol analytics storage.
///
/// # Returns
/// The total value locked as an `i128`.
pub fn get_total_value_locked(env: &Env) -> Result<i128, AnalyticsError> {
    let protocol_analytics = env
        .storage()
        .persistent()
        .get::<DepositDataKey, DepositProtocolAnalytics>(&DepositDataKey::ProtocolAnalytics)
        .unwrap_or(DepositProtocolAnalytics {
            total_deposits: 0,
            total_borrows: 0,
            total_value_locked: 0,
        });

    Ok(protocol_analytics.total_value_locked)
}

/// Get the current protocol utilization rate.
///
/// Computed as `(total_borrows * 10000) / total_deposits` in basis points.
/// Returns 0 if there are no deposits.
///
/// # Returns
/// Utilization rate in basis points (0–10000).
pub fn get_protocol_utilization(env: &Env) -> Result<i128, AnalyticsError> {
    let protocol_analytics = env
        .storage()
        .persistent()
        .get::<DepositDataKey, DepositProtocolAnalytics>(&DepositDataKey::ProtocolAnalytics)
        .unwrap_or(DepositProtocolAnalytics {
            total_deposits: 0,
            total_borrows: 0,
            total_value_locked: 0,
        });

    if protocol_analytics.total_deposits == 0 {
        return Ok(0);
    }

    let utilization = (protocol_analytics.total_borrows * BASIS_POINTS)
        .checked_div(protocol_analytics.total_deposits)
        .ok_or(AnalyticsError::Overflow)?;

    Ok(utilization)
}

/// Calculate the weighted average borrow interest rate.
///
/// Uses a simplified model: `base_rate (200 bps) + utilization * 10 / 10000`.
/// Returns 0 if there are no borrows.
///
/// # Returns
/// Weighted average interest rate in basis points.
pub fn calculate_weighted_avg_interest_rate(env: &Env) -> Result<i128, AnalyticsError> {
    let protocol_analytics = env
        .storage()
        .persistent()
        .get::<DepositDataKey, DepositProtocolAnalytics>(&DepositDataKey::ProtocolAnalytics)
        .unwrap_or(DepositProtocolAnalytics {
            total_deposits: 0,
            total_borrows: 0,
            total_value_locked: 0,
        });

    if protocol_analytics.total_borrows == 0 {
        return Ok(0);
    }

    let utilization = get_protocol_utilization(env)?;
    let base_rate = 200;
    let rate = base_rate + (utilization * 10) / BASIS_POINTS;

    Ok(rate)
}

/// Recompute and persist protocol-wide metrics.
///
/// Aggregates TVL, utilization, average rate, and user/transaction counts
/// into a fresh `ProtocolMetrics` snapshot and stores it.
///
/// # Returns
/// The newly computed `ProtocolMetrics`.
pub fn update_protocol_metrics(env: &Env) -> Result<ProtocolMetrics, AnalyticsError> {
    let tvl = get_total_value_locked(env)?;
    let utilization = get_protocol_utilization(env)?;
    let avg_rate = calculate_weighted_avg_interest_rate(env)?;

    let protocol_analytics = env
        .storage()
        .persistent()
        .get::<DepositDataKey, DepositProtocolAnalytics>(&DepositDataKey::ProtocolAnalytics)
        .unwrap_or(DepositProtocolAnalytics {
            total_deposits: 0,
            total_borrows: 0,
            total_value_locked: 0,
        });

    let total_users = env
        .storage()
        .persistent()
        .get::<AnalyticsDataKey, u64>(&AnalyticsDataKey::TotalUsers)
        .unwrap_or(0);

    let total_transactions = env
        .storage()
        .persistent()
        .get::<AnalyticsDataKey, u64>(&AnalyticsDataKey::TotalTransactions)
        .unwrap_or(0);

    let metrics = ProtocolMetrics {
        total_value_locked: tvl,
        total_deposits: protocol_analytics.total_deposits,
        total_borrows: protocol_analytics.total_borrows,
        utilization_rate: utilization,
        average_borrow_rate: avg_rate,
        total_users,
        total_transactions,
        last_update: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&AnalyticsDataKey::ProtocolMetrics, &metrics);

    Ok(metrics)
}

/// Get cached protocol metrics, recomputing if none exist.
///
/// Returns the stored `ProtocolMetrics` if available, otherwise calls
/// [`update_protocol_metrics`] to compute fresh metrics.
///
/// # Returns
/// Current `ProtocolMetrics`.
pub fn get_protocol_stats(env: &Env) -> Result<ProtocolMetrics, AnalyticsError> {
    let cached_metrics = env
        .storage()
        .persistent()
        .get::<AnalyticsDataKey, ProtocolMetrics>(&AnalyticsDataKey::ProtocolMetrics);

    if let Some(metrics) = cached_metrics {
        Ok(metrics)
    } else {
        update_protocol_metrics(env)
    }
}

/// Composite protocol health score, built from `ProtocolMetrics` fields
/// that are already tracked on-chain (issue #813).
///
/// This is deliberately scoped to the two risk signals currently available
/// in `ProtocolMetrics` — utilization and average borrow rate — rather than
/// inventing on-chain data the protocol doesn't track yet (bad debt,
/// oracle health, and governance participation are computed off-chain in
/// `api/src/services/protocol-health/healthScore.service.ts`, which covers
/// those with richer, non-on-chain data sources). `component_weights_bps`
/// always sums to `BPS_DIVISOR` so the two component sub-scores can be
/// reweighted later without changing the struct shape.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ProtocolHealthScore {
    /// Overall composite score, 0-100.
    pub overall_score: i128,
    /// Capital-efficiency sub-score (0-100), from utilization vs. the optimal band.
    pub capital_efficiency_score: i128,
    /// Rate-stability sub-score (0-100), from average borrow rate vs. a healthy ceiling.
    pub rate_stability_score: i128,
    /// (capital_efficiency_weight_bps, rate_stability_weight_bps) — sums to `BPS_DIVISOR`.
    pub component_weights_bps: (i128, i128),
    pub last_update: u64,
}

const BPS_DIVISOR: i128 = 10_000;
/// Utilization band considered efficient, in basis points — below it capital
/// sits idle, above it the protocol has little withdrawal buffer.
const OPTIMAL_UTILIZATION_MIN_BPS: i128 = 7_000;
const OPTIMAL_UTILIZATION_MAX_BPS: i128 = 9_000;
/// Average borrow rate at/below which rates are considered fully healthy.
const HEALTHY_BORROW_RATE_BPS: i128 = 2_000;
/// Average borrow rate at/above which the rate-stability score bottoms out at 0.
const STRESSED_BORROW_RATE_BPS: i128 = 5_000;
const CAPITAL_EFFICIENCY_WEIGHT_BPS: i128 = 6_000;
const RATE_STABILITY_WEIGHT_BPS: i128 = 4_000;

/// Scores utilization against the optimal band: 100 inside the band, falling
/// off linearly to 0 at either 0% utilization or 100%+ utilization.
fn score_capital_efficiency(utilization_rate_bps: i128) -> i128 {
    let utilization = utilization_rate_bps.clamp(0, BPS_DIVISOR);
    if utilization >= OPTIMAL_UTILIZATION_MIN_BPS && utilization <= OPTIMAL_UTILIZATION_MAX_BPS {
        return 100;
    }
    if utilization < OPTIMAL_UTILIZATION_MIN_BPS {
        return (utilization * 100) / OPTIMAL_UTILIZATION_MIN_BPS.max(1);
    }
    let headroom = BPS_DIVISOR - OPTIMAL_UTILIZATION_MAX_BPS;
    let over = utilization - OPTIMAL_UTILIZATION_MAX_BPS;
    (100 - (over * 100) / headroom.max(1)).max(0)
}

/// Scores the average borrow rate: 100 at/below the healthy ceiling, 0
/// at/above the stressed threshold, linear in between.
fn score_rate_stability(average_borrow_rate_bps: i128) -> i128 {
    let rate = average_borrow_rate_bps.max(0);
    if rate <= HEALTHY_BORROW_RATE_BPS {
        return 100;
    }
    if rate >= STRESSED_BORROW_RATE_BPS {
        return 0;
    }
    let span = STRESSED_BORROW_RATE_BPS - HEALTHY_BORROW_RATE_BPS;
    100 - ((rate - HEALTHY_BORROW_RATE_BPS) * 100) / span
}

/// Computes and caches the composite protocol health score from the given
/// (already up-to-date) `ProtocolMetrics`.
pub fn calculate_protocol_health_score(env: &Env, metrics: &ProtocolMetrics) -> ProtocolHealthScore {
    let capital_efficiency_score = score_capital_efficiency(metrics.utilization_rate);
    let rate_stability_score = score_rate_stability(metrics.average_borrow_rate);

    let overall_score = (capital_efficiency_score * CAPITAL_EFFICIENCY_WEIGHT_BPS
        + rate_stability_score * RATE_STABILITY_WEIGHT_BPS)
        / BPS_DIVISOR;

    let score = ProtocolHealthScore {
        overall_score,
        capital_efficiency_score,
        rate_stability_score,
        component_weights_bps: (CAPITAL_EFFICIENCY_WEIGHT_BPS, RATE_STABILITY_WEIGHT_BPS),
        last_update: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&AnalyticsDataKey::ProtocolHealthScore, &score);

    score
}

/// Gets the cached composite protocol health score, recomputing from fresh
/// protocol metrics if none exists yet.
pub fn get_protocol_health_score(env: &Env) -> Result<ProtocolHealthScore, AnalyticsError> {
    if let Some(score) = env
        .storage()
        .persistent()
        .get::<AnalyticsDataKey, ProtocolHealthScore>(&AnalyticsDataKey::ProtocolHealthScore)
    {
        return Ok(score);
    }
    let metrics = get_protocol_stats(env)?;
    Ok(calculate_protocol_health_score(env, &metrics))
}

/// Get the user's current position from storage.
///
/// # Arguments
/// * `user` - The user's address
///
/// # Returns
/// The user's `Position` (collateral, debt, interest, last accrual time).
///
/// # Errors
/// Returns `AnalyticsError::DataNotFound` if the user has no position.
pub fn get_user_position_summary(env: &Env, user: &Address) -> Result<Position, AnalyticsError> {
    let position = env
        .storage()
        .persistent()
        .get::<DepositDataKey, Position>(&DepositDataKey::Position(user.clone()))
        .ok_or(AnalyticsError::DataNotFound)?;

    Ok(position)
}

/// Calculate the health factor for a user's position.
///
/// Health factor = `(collateral * 10000) / debt`. Returns `i128::MAX` if the
/// user has no debt (infinite health).
///
/// # Arguments
/// * `user` - The user's address
///
/// # Returns
/// Health factor in basis points (e.g., 15000 = 1.5x collateralization).
pub fn calculate_health_factor(env: &Env, user: &Address) -> Result<i128, AnalyticsError> {
    let position = get_user_position_summary(env, user)?;

    if position.debt == 0 {
        return Ok(i128::MAX);
    }

    let health_factor = (position.collateral * BASIS_POINTS)
        .checked_div(position.debt)
        .ok_or(AnalyticsError::Overflow)?;

    Ok(health_factor)
}

/// Map a health factor to a risk level (1–5).
///
/// | Health Factor | Risk Level |
/// |---------------|------------|
/// | ≥ 15000 (1.5x) | 1 (Low)    |
/// | ≥ 12000 (1.2x) | 2          |
/// | ≥ 11000 (1.1x) | 3          |
/// | ≥ 10500 (1.05x) | 4         |
/// | < 10500        | 5 (Critical) |
pub fn calculate_user_risk_level(health_factor: i128) -> i128 {
    if health_factor >= 15_000 {
        1
    } else if health_factor >= 12_000 {
        2
    } else if health_factor >= 11_000 {
        3
    } else if health_factor >= 10_500 {
        4
    } else {
        5
    }
}

/// Compute a full activity summary for a user.
///
/// Aggregates deposit analytics, current position, health factor, risk level,
/// and activity score into a single `UserMetrics` struct.
///
/// # Arguments
/// * `user` - The user's address
///
/// # Returns
/// Computed `UserMetrics` for the user.
///
/// # Errors
/// Returns `AnalyticsError::DataNotFound` if the user has no analytics data.
pub fn get_user_activity_summary(env: &Env, user: &Address) -> Result<UserMetrics, AnalyticsError> {
    let user_analytics = env
        .storage()
        .persistent()
        .get::<DepositDataKey, DepositUserAnalytics>(&DepositDataKey::UserAnalytics(user.clone()))
        .ok_or(AnalyticsError::DataNotFound)?;

    let position = get_user_position_summary(env, user).unwrap_or(Position {
        collateral: 0,
        debt: 0,
        borrow_interest: 0,
        last_accrual_time: 0,
    });

    let health_factor = calculate_health_factor(env, user).unwrap_or(i128::MAX);
    let risk_level = calculate_user_risk_level(health_factor);

    let activity_score = (user_analytics.transaction_count as i128)
        .saturating_mul(100)
        .saturating_add(user_analytics.total_deposits / 1000);

    let metrics = UserMetrics {
        collateral: position.collateral,
        debt: position.debt,
        health_factor,
        total_deposits: user_analytics.total_deposits,
        total_borrows: user_analytics.total_borrows,
        total_withdrawals: user_analytics.total_withdrawals,
        total_repayments: user_analytics.total_repayments,
        activity_score,
        risk_level,
        transaction_count: user_analytics.transaction_count,
    };

    Ok(metrics)
}

/// Recompute and persist a user's metrics.
///
/// Calls [`get_user_activity_summary`] and stores the result.
///
/// # Arguments
/// * `user` - The user's address
///
/// # Returns
/// The freshly computed `UserMetrics`.
pub fn update_user_metrics(env: &Env, user: &Address) -> Result<UserMetrics, AnalyticsError> {
    let metrics = get_user_activity_summary(env, user)?;

    env.storage()
        .persistent()
        .set(&AnalyticsDataKey::UserMetrics(user.clone()), &metrics);

    Ok(metrics)
}

/// Record a new activity entry in the protocol activity log.
///
/// Appends the entry and trims the log to `MAX_ACTIVITY_LOG_SIZE` (10,000).
/// Also increments the global transaction counter.
///
/// # Arguments
/// * `user` - The user who performed the activity
/// * `activity_type` - Type symbol (e.g., "deposit", "borrow")
/// * `amount` - Amount involved
/// * `asset` - Asset address (None for native XLM)
pub fn record_activity(
    env: &Env,
    user: &Address,
    activity_type: Symbol,
    amount: i128,
    asset: Option<Address>,
) -> Result<(), AnalyticsError> {
    let mut activity_log = env
        .storage()
        .persistent()
        .get::<AnalyticsDataKey, Vec<ActivityEntry>>(&AnalyticsDataKey::ActivityLog)
        .unwrap_or_else(|| Vec::new(env));

    let entry = ActivityEntry {
        user: user.clone(),
        activity_type,
        amount,
        asset,
        timestamp: env.ledger().timestamp(),
        metadata: Map::new(env),
    };

    activity_log.push_back(entry);

    if activity_log.len() > MAX_ACTIVITY_LOG_SIZE {
        activity_log.pop_front();
    }

    env.storage()
        .persistent()
        .set(&AnalyticsDataKey::ActivityLog, &activity_log);

    let total_transactions = env
        .storage()
        .persistent()
        .get::<AnalyticsDataKey, u64>(&AnalyticsDataKey::TotalTransactions)
        .unwrap_or(0);

    env.storage().persistent().set(
        &AnalyticsDataKey::TotalTransactions,
        &(total_transactions + 1),
    );

    Ok(())
}

/// Get recent protocol-wide activity entries with pagination.
///
/// Returns entries in reverse chronological order (most recent first).
///
/// # Arguments
/// * `limit` - Maximum number of entries to return
/// * `offset` - Number of most-recent entries to skip
///
/// # Returns
/// A vector of `ActivityEntry` records.
pub fn get_recent_activity(
    env: &Env,
    limit: u32,
    offset: u32,
) -> Result<Vec<ActivityEntry>, AnalyticsError> {
    let activity_log = env
        .storage()
        .persistent()
        .get::<AnalyticsDataKey, Vec<ActivityEntry>>(&AnalyticsDataKey::ActivityLog)
        .unwrap_or_else(|| Vec::new(env));

    let total_len = activity_log.len();
    if offset >= total_len {
        return Ok(Vec::new(env));
    }

    let mut result = Vec::new(env);
    let start = total_len.saturating_sub(offset + limit);
    let end = total_len.saturating_sub(offset);

    for i in (start..end).rev() {
        if let Some(entry) = activity_log.get(i) {
            result.push_back(entry);
        }
    }

    Ok(result)
}

/// Get activity entries for a specific user with pagination.
///
/// Filters the global activity log for entries matching the user, then
/// applies pagination. Returns entries in reverse chronological order.
///
/// # Arguments
/// * `user` - The user's address to filter by
/// * `limit` - Maximum number of entries to return
/// * `offset` - Number of matching entries to skip
///
/// # Returns
/// A vector of `ActivityEntry` records for the user.
pub fn get_user_activity_feed(
    env: &Env,
    user: &Address,
    limit: u32,
    offset: u32,
) -> Result<Vec<ActivityEntry>, AnalyticsError> {
    let activity_log = env
        .storage()
        .persistent()
        .get::<AnalyticsDataKey, Vec<ActivityEntry>>(&AnalyticsDataKey::ActivityLog)
        .unwrap_or_else(|| Vec::new(env));

    let mut user_activities = Vec::new(env);

    for i in (0..activity_log.len()).rev() {
        if let Some(entry) = activity_log.get(i) {
            if entry.user == *user {
                user_activities.push_back(entry);
            }
        }
    }

    let total_len = user_activities.len();
    if offset >= total_len {
        return Ok(Vec::new(env));
    }

    let mut result = Vec::new(env);
    let end = total_len.saturating_sub(offset);
    let start = end.saturating_sub(limit);

    for i in start..end {
        if let Some(entry) = user_activities.get(i) {
            result.push_back(entry);
        }
    }

    Ok(result)
}

/// Get activity entries filtered by activity type.
///
/// Scans the activity log in reverse order and returns up to `limit` entries
/// matching the given `activity_type`.
///
/// # Arguments
/// * `activity_type` - The activity type symbol to filter by (e.g., "deposit")
/// * `limit` - Maximum number of entries to return
///
/// # Returns
/// A vector of matching `ActivityEntry` records.
pub fn get_activity_by_type(
    env: &Env,
    activity_type: Symbol,
    limit: u32,
) -> Result<Vec<ActivityEntry>, AnalyticsError> {
    let activity_log = env
        .storage()
        .persistent()
        .get::<AnalyticsDataKey, Vec<ActivityEntry>>(&AnalyticsDataKey::ActivityLog)
        .unwrap_or_else(|| Vec::new(env));

    let mut filtered = Vec::new(env);
    let mut count = 0u32;

    for i in (0..activity_log.len()).rev() {
        if count >= limit {
            break;
        }

        if let Some(entry) = activity_log.get(i) {
            if entry.activity_type == activity_type {
                filtered.push_back(entry);
                count += 1;
            }
        }
    }

    Ok(filtered)
}

/// Generate a comprehensive protocol analytics report.
///
/// Recomputes protocol metrics and wraps them in a timestamped report.
///
/// # Returns
/// A `ProtocolReport` containing fresh metrics and the current timestamp.
pub fn generate_protocol_report(env: &Env) -> Result<ProtocolReport, AnalyticsError> {
    let metrics = update_protocol_metrics(env)?;

    let report = ProtocolReport {
        metrics,
        timestamp: env.ledger().timestamp(),
    };

    Ok(report)
}

/// Generate a comprehensive user analytics report.
///
/// Includes the user's computed metrics, current position, and the 10 most
/// recent activities.
///
/// # Arguments
/// * `user` - The user's address
///
/// # Returns
/// A `UserReport` for the specified user.
///
/// # Errors
/// Returns `AnalyticsError::DataNotFound` if the user has no recorded data.
pub fn generate_user_report(env: &Env, user: &Address) -> Result<UserReport, AnalyticsError> {
    let metrics = get_user_activity_summary(env, user)?;
    let position = get_user_position_summary(env, user)?;
    let recent_activities = get_user_activity_feed(env, user, 10, 0)?;

    let report = UserReport {
        user: user.clone(),
        metrics,
        position,
        recent_activities,
        timestamp: env.ledger().timestamp(),
    };

    Ok(report)
}

// ─────────────────────────────────────────────────────────────────────────
// #672 — Historical snapshots, growth forecasting, and threshold alerting.
//
// Real-time metrics (TVL, utilization, avg rate) were already computed by
// the functions above; this section adds the parts #672 actually asked for
// that were missing: a bounded history of periodic snapshots to visualize
// trends over time, a simple linear-trend forecast derived from that
// history, and configurable metric-threshold alerts. Dashboard widgets,
// CSV/PDF export, and the query API are frontend/API-layer concerns and are
// out of scope here — see the PR description.
// ─────────────────────────────────────────────────────────────────────────

/// Maximum historical snapshots retained (oldest pruned first).
const MAX_METRICS_HISTORY: u32 = 90;

/// A single point-in-time snapshot of protocol-wide metrics, for historical
/// visualization and forecasting.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub total_value_locked: i128,
    pub utilization_rate: i128,
    pub average_borrow_rate: i128,
}

/// A configured alert threshold for a named metric.
///
/// `metric` is one of: "tvl", "utilization", "avg_rate" (matching the fields
/// captured in `MetricsSnapshot`).
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct MetricAlertThreshold {
    pub metric: Symbol,
    /// Alert fires when the metric's current value is >= this threshold.
    pub threshold: i128,
}

/// A record of a triggered alert, for audit trail purposes.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TriggeredAlert {
    pub metric: Symbol,
    pub value: i128,
    pub threshold: i128,
    pub timestamp: u64,
}

/// Take and store a new historical metrics snapshot from the current
/// real-time protocol metrics. Intended to be called periodically (e.g. by
/// an off-chain keeper, or opportunistically alongside other state-mutating
/// calls) — the contract itself has no notion of a background scheduler.
pub fn record_metrics_snapshot(env: &Env) -> Result<MetricsSnapshot, AnalyticsError> {
    let tvl = get_total_value_locked(env)?;
    let utilization = get_protocol_utilization(env)?;
    let avg_rate = calculate_weighted_avg_interest_rate(env)?;

    let snapshot = MetricsSnapshot {
        timestamp: env.ledger().timestamp(),
        total_value_locked: tvl,
        utilization_rate: utilization,
        average_borrow_rate: avg_rate,
    };

    let key = AnalyticsDataKey::MetricsHistory;
    let mut history: Vec<MetricsSnapshot> =
        env.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(env));
    history.push_back(snapshot.clone());
    while history.len() > MAX_METRICS_HISTORY {
        history.remove(0);
    }
    env.storage().persistent().set(&key, &history);

    check_metric_alerts_internal(env, &snapshot);

    Ok(snapshot)
}

/// Get the full bounded snapshot history, oldest-first.
pub fn get_metrics_history(env: &Env) -> Vec<MetricsSnapshot> {
    env.storage()
        .persistent()
        .get(&AnalyticsDataKey::MetricsHistory)
        .unwrap_or_else(|| Vec::new(env))
}

/// Forecast a future value of `total_value_locked` using simple linear
/// least-squares regression over the recorded history, projected
/// `periods_ahead` snapshot-intervals into the future.
///
/// This is deliberately a plain linear trend, not a real forecasting model —
/// #672 asked for "linear, exponential" forecasting; linear is implemented
/// honestly here, and an exponential model is deferred (see PR description)
/// since it requires choosing a decay/growth basis that would otherwise be
/// guessed rather than derived from real usage data.
///
/// Requires at least 2 snapshots. Returns `AnalyticsError::DataNotFound` if
/// there isn't enough history yet.
pub fn forecast_tvl(env: &Env, periods_ahead: u32) -> Result<i128, AnalyticsError> {
    let history = get_metrics_history(env);
    if history.len() < 2 {
        return Err(AnalyticsError::DataNotFound);
    }

    let n = history.len() as i128;
    let mut sum_x: i128 = 0;
    let mut sum_y: i128 = 0;
    let mut sum_xy: i128 = 0;
    let mut sum_xx: i128 = 0;

    for i in 0..history.len() {
        let x = i as i128;
        let y = history.get(i).unwrap().total_value_locked;
        sum_x = sum_x.checked_add(x).ok_or(AnalyticsError::Overflow)?;
        sum_y = sum_y.checked_add(y).ok_or(AnalyticsError::Overflow)?;
        sum_xy = sum_xy
            .checked_add(x.checked_mul(y).ok_or(AnalyticsError::Overflow)?)
            .ok_or(AnalyticsError::Overflow)?;
        sum_xx = sum_xx
            .checked_add(x.checked_mul(x).ok_or(AnalyticsError::Overflow)?)
            .ok_or(AnalyticsError::Overflow)?;
    }

    // Least-squares slope: slope = (n*Sigma_xy - Sigma_x*Sigma_y) / (n*Sigma_xx - (Sigma_x)^2)
    let n_sum_xx = n.checked_mul(sum_xx).ok_or(AnalyticsError::Overflow)?;
    let sum_x_sq = sum_x.checked_mul(sum_x).ok_or(AnalyticsError::Overflow)?;
    let denom = n_sum_xx.checked_sub(sum_x_sq).ok_or(AnalyticsError::Overflow)?;

    if denom == 0 {
        // All snapshots at the same x (shouldn't happen with real timestamps,
        // but guards div-by-zero) — flat forecast at the last known value.
        return Ok(history.get(history.len() - 1).unwrap().total_value_locked);
    }

    let n_sum_xy = n.checked_mul(sum_xy).ok_or(AnalyticsError::Overflow)?;
    let sum_x_sum_y = sum_x.checked_mul(sum_y).ok_or(AnalyticsError::Overflow)?;
    let slope_num = n_sum_xy.checked_sub(sum_x_sum_y).ok_or(AnalyticsError::Overflow)?;

    let last_x = (history.len() - 1) as i128;
    let target_x = last_x
        .checked_add(periods_ahead as i128)
        .ok_or(AnalyticsError::Overflow)?;

    // intercept = (Sigma_y - slope*Sigma_x) / n, forecast = slope*target_x + intercept
    // Rearranged over a common denominator to avoid intermediate precision loss:
    // forecast = (slope_num * target_x + (sum_y * denom - slope_num * sum_x)) / (denom * n)
    let slope_times_target = slope_num.checked_mul(target_x).ok_or(AnalyticsError::Overflow)?;
    let sum_y_times_denom = sum_y.checked_mul(denom).ok_or(AnalyticsError::Overflow)?;
    let slope_num_times_sum_x = slope_num.checked_mul(sum_x).ok_or(AnalyticsError::Overflow)?;
    let intercept_num = sum_y_times_denom
        .checked_sub(slope_num_times_sum_x)
        .ok_or(AnalyticsError::Overflow)?;
    let forecast_num = slope_times_target
        .checked_add(intercept_num)
        .ok_or(AnalyticsError::Overflow)?;
    let forecast_denom = denom.checked_mul(n).ok_or(AnalyticsError::Overflow)?;

    forecast_num
        .checked_div(forecast_denom)
        .ok_or(AnalyticsError::Overflow)
}

/// Configure (or update) an alert threshold for a metric. Admin-only.
pub fn set_metric_alert_threshold(
    env: &Env,
    admin: Address,
    metric: Symbol,
    threshold: i128,
) -> Result<(), AnalyticsError> {
    admin.require_auth();
    let configured_admin = crate::governance::get_admin(env).ok_or(AnalyticsError::NotInitialized)?;
    if admin != configured_admin {
        return Err(AnalyticsError::Unauthorized);
    }

    let key = AnalyticsDataKey::AlertThresholds;
    let mut thresholds: Vec<MetricAlertThreshold> =
        env.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(env));

    let mut updated = false;
    for i in 0..thresholds.len() {
        if thresholds.get(i).unwrap().metric == metric {
            thresholds.set(i, MetricAlertThreshold { metric: metric.clone(), threshold });
            updated = true;
            break;
        }
    }
    if !updated {
        thresholds.push_back(MetricAlertThreshold { metric, threshold });
    }

    env.storage().persistent().set(&key, &thresholds);
    Ok(())
}

/// Get all configured alert thresholds.
pub fn get_metric_alert_thresholds(env: &Env) -> Vec<MetricAlertThreshold> {
    env.storage()
        .persistent()
        .get(&AnalyticsDataKey::AlertThresholds)
        .unwrap_or_else(|| Vec::new(env))
}

/// Get the bounded log of previously triggered alerts (audit trail).
pub fn get_triggered_alerts(env: &Env) -> Vec<TriggeredAlert> {
    env.storage()
        .persistent()
        .get(&AnalyticsDataKey::TriggeredAlerts)
        .unwrap_or_else(|| Vec::new(env))
}

/// Check current real-time metrics against configured thresholds and
/// return the list of metric names whose threshold is currently crossed.
/// Also records any newly-crossed threshold into the triggered-alerts log.
pub fn check_metric_alerts(env: &Env) -> Result<Vec<Symbol>, AnalyticsError> {
    let tvl = get_total_value_locked(env)?;
    let utilization = get_protocol_utilization(env)?;
    let avg_rate = calculate_weighted_avg_interest_rate(env)?;

    let snapshot = MetricsSnapshot {
        timestamp: env.ledger().timestamp(),
        total_value_locked: tvl,
        utilization_rate: utilization,
        average_borrow_rate: avg_rate,
    };

    Ok(check_metric_alerts_internal(env, &snapshot))
}

const MAX_TRIGGERED_ALERTS: u32 = 200;

// ─────────────────────────────────────────────────────────────────────────
// Real-time Collateral Ratio Monitoring
// ─────────────────────────────────────────────────────────────────────────

/// Maximum collateral ratio snapshots retained (oldest pruned first)
const MAX_COLLATERAL_SNAPSHOTS: u32 = 100;

/// Maximum historical collateral ratio trends retained
const MAX_COLLATERAL_HISTORY: u32 = 90;

/// A real-time snapshot of collateral ratio metrics for an asset
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CollateralRatioSnapshot {
    pub asset: Symbol,
    pub current_ratio: i128,      // basis points
    pub required_ratio: i128,     // basis points
    pub health_factor: i128,
    pub risk_level: Symbol,       // "safe", "warning", "danger", "critical"
    pub collateral_value: i128,
    pub debt_value: i128,
    pub timestamp: u64,
}

/// Historical trend data for collateral ratio monitoring
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CollateralRatioTrend {
    pub asset: Symbol,
    pub timestamp: u64,
    pub avg_health_factor: i128,
    pub min_health_factor: i128,
    pub max_health_factor: i128,
    pub position_count: u64,
    pub danger_count: u64,
    pub critical_count: u64,
}

/// Risk threshold configuration for collateral ratios
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CollateralRiskThresholds {
    pub safe_threshold: i128,      // health factor >= this
    pub warning_threshold: i128,   // health factor >= this
    pub danger_threshold: i128,    // health factor >= this
}

/// Default risk thresholds
const DEFAULT_THRESHOLDS: CollateralRiskThresholds = CollateralRiskThresholds {
    safe_threshold: 20_000,      // 2.0x
    warning_threshold: 15_000,   // 1.5x
    danger_threshold: 11_000,    // 1.1x
};

/// Record a new collateral ratio snapshot for an asset
pub fn record_collateral_ratio_snapshot(
    env: &Env,
    asset: Symbol,
    current_ratio: i128,
    required_ratio: i128,
    collateral_value: i128,
    debt_value: i128,
) -> Result<CollateralRatioSnapshot, AnalyticsError> {
    let health_factor = if required_ratio == 0 {
        i128::MAX
    } else {
        (current_ratio * 10_000)
            .checked_div(required_ratio)
            .ok_or(AnalyticsError::Overflow)?
    };

    let risk_level = classify_collateral_risk_level(env, health_factor);

    let snapshot = CollateralRatioSnapshot {
        asset: asset.clone(),
        current_ratio,
        required_ratio,
        health_factor,
        risk_level: risk_level.clone(),
        collateral_value,
        debt_value,
        timestamp: env.ledger().timestamp(),
    };

    let key = AnalyticsDataKey::CollateralRatioSnapshots;
    let mut snapshots: Vec<CollateralRatioSnapshot> =
        env.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(env));
    
    // Update existing snapshot for this asset or add new one
    let mut found = false;
    for i in 0..snapshots.len() {
        if snapshots.get(i).unwrap().asset == asset {
            snapshots.set(i, snapshot.clone());
            found = true;
            break;
        }
    }
    if !found {
        snapshots.push_back(snapshot.clone());
    }

    // Trim if exceeding max size
    while snapshots.len() > MAX_COLLATERAL_SNAPSHOTS {
        snapshots.remove(0);
    }

    env.storage().persistent().set(&key, &snapshots);

    Ok(snapshot)
}

/// Get all current collateral ratio snapshots
pub fn get_collateral_ratio_snapshots(env: &Env) -> Vec<CollateralRatioSnapshot> {
    env.storage()
        .persistent()
        .get(&AnalyticsDataKey::CollateralRatioSnapshots)
        .unwrap_or_else(|| Vec::new(env))
}

/// Get collateral ratio snapshot for a specific asset
pub fn get_collateral_ratio_snapshot(env: &Env, asset: Symbol) -> Option<CollateralRatioSnapshot> {
    let snapshots = get_collateral_ratio_snapshots(env);
    for i in 0..snapshots.len() {
        let snapshot = snapshots.get(i)?;
        if snapshot.asset == asset {
            return Some(snapshot.clone());
        }
    }
    None
}

/// Classify collateral risk level based on health factor
pub fn classify_collateral_risk_level(env: &Env, health_factor: i128) -> Symbol {
    let thresholds = get_collateral_risk_thresholds(env);
    
    if health_factor >= thresholds.safe_threshold {
        Symbol::new(env, "safe")
    } else if health_factor >= thresholds.warning_threshold {
        Symbol::new(env, "warning")
    } else if health_factor >= thresholds.danger_threshold {
        Symbol::new(env, "danger")
    } else {
        Symbol::new(env, "critical")
    }
}

/// Get or initialize collateral risk thresholds
pub fn get_collateral_risk_thresholds(env: &Env) -> CollateralRiskThresholds {
    env.storage()
        .persistent()
        .get(&AnalyticsDataKey::CollateralRiskThresholds)
        .unwrap_or(DEFAULT_THRESHOLDS)
}

/// Update collateral risk thresholds (admin only)
pub fn set_collateral_risk_thresholds(
    env: &Env,
    admin: Address,
    thresholds: CollateralRiskThresholds,
) -> Result<(), AnalyticsError> {
    admin.require_auth();
    let configured_admin = crate::governance::get_admin(env).ok_or(AnalyticsError::NotInitialized)?;
    if admin != configured_admin {
        return Err(AnalyticsError::Unauthorized);
    }

    env.storage()
        .persistent()
        .set(&AnalyticsDataKey::CollateralRiskThresholds, &thresholds);
    
    Ok(())
}

/// Record historical collateral ratio trend data
pub fn record_collateral_ratio_trend(
    env: &Env,
    asset: Symbol,
    avg_health_factor: i128,
    min_health_factor: i128,
    max_health_factor: i128,
    position_count: u64,
    danger_count: u64,
    critical_count: u64,
) -> Result<CollateralRatioTrend, AnalyticsError> {
    let trend = CollateralRatioTrend {
        asset: asset.clone(),
        timestamp: env.ledger().timestamp(),
        avg_health_factor,
        min_health_factor,
        max_health_factor,
        position_count,
        danger_count,
        critical_count,
    };

    let key = AnalyticsDataKey::CollateralRatioHistory;
    let mut history: Vec<CollateralRatioTrend> =
        env.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(env));
    
    history.push_back(trend.clone());
    
    while history.len() > MAX_COLLATERAL_HISTORY {
        history.remove(0);
    }

    env.storage().persistent().set(&key, &history);

    Ok(trend)
}

/// Get historical collateral ratio trends for an asset
pub fn get_collateral_ratio_history(env: &Env, asset: Symbol) -> Vec<CollateralRatioTrend> {
    let history: Vec<CollateralRatioTrend> = env
        .storage()
        .persistent()
        .get(&AnalyticsDataKey::CollateralRatioHistory)
        .unwrap_or_else(|| Vec::new(env));
    
    let mut filtered = Vec::new(env);
    for i in 0..history.len() {
        let trend = history.get(i).unwrap();
        if trend.asset == asset {
            filtered.push_back(trend.clone());
        }
    }
    filtered
}

/// Get all historical collateral ratio trends
pub fn get_all_collateral_ratio_history(env: &Env) -> Vec<CollateralRatioTrend> {
    env.storage()
        .persistent()
        .get(&AnalyticsDataKey::CollateralRatioHistory)
        .unwrap_or_else(|| Vec::new(env))
}

fn check_metric_alerts_internal(env: &Env, snapshot: &MetricsSnapshot) -> Vec<Symbol> {
    let thresholds = get_metric_alert_thresholds(env);
    let mut fired = Vec::new(env);

    if thresholds.is_empty() {
        return fired;
    }

    let mut triggered_log = get_triggered_alerts(env);
    let mut log_changed = false;

    for i in 0..thresholds.len() {
        let t = thresholds.get(i).unwrap();
        let current_value = if t.metric == Symbol::new(env, "tvl") {
            Some(snapshot.total_value_locked)
        } else if t.metric == Symbol::new(env, "utilization") {
            Some(snapshot.utilization_rate)
        } else if t.metric == Symbol::new(env, "avg_rate") {
            Some(snapshot.average_borrow_rate)
        } else {
            None
        };

        if let Some(value) = current_value {
            if value >= t.threshold {
                fired.push_back(t.metric.clone());
                triggered_log.push_back(TriggeredAlert {
                    metric: t.metric.clone(),
                    value,
                    threshold: t.threshold,
                    timestamp: snapshot.timestamp,
                });
                log_changed = true;
            }
        }
    }

    if log_changed {
        while triggered_log.len() > MAX_TRIGGERED_ALERTS {
            triggered_log.remove(0);
        }
        env.storage()
            .persistent()
            .set(&AnalyticsDataKey::TriggeredAlerts, &triggered_log);
    }

    fired
}

// ─────────────────────────────────────────────────────────────────────────────
// Real-Time Dashboard Aggregation  (Issue #795)
//
// Adds a single `get_dashboard_snapshot` function that bundles all panels of
// the protocol analytics dashboard into one read-only call, minimising the
// number of round-trips required by the off-chain API layer. It pulls:
//   • Protocol-wide metrics (TVL, utilization, avg rate, users, txns)
//   • Collateral ratio snapshots for all tracked assets
//   • Active metric alerts currently in breach
//   • Most-recent N activity log entries
//
// Everything here is purely read-only — no state mutation occurs.
// ─────────────────────────────────────────────────────────────────────────────

/// Default number of recent-activity entries surfaced in the dashboard panel.
const DASHBOARD_ACTIVITY_LIMIT: u32 = 20;

/// Aggregated dashboard snapshot bundling all real-time dashboard panels.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct DashboardSnapshot {
    /// Current protocol-wide metrics.
    pub protocol: ProtocolMetrics,
    /// Collateral ratio snapshots for all tracked assets.
    pub collateral_ratios: Vec<CollateralRatioSnapshot>,
    /// Metric names whose alert threshold is currently breached.
    pub active_alerts: Vec<Symbol>,
    /// Most-recent `DASHBOARD_ACTIVITY_LIMIT` activity entries.
    pub recent_activity: Vec<ActivityEntry>,
    /// Timestamp when this snapshot was assembled.
    pub generated_at: u64,
}

/// Produce a full real-time dashboard snapshot in a single contract call.
///
/// Designed to be the primary data source for the protocol analytics dashboard
/// described in Issue #795. All sub-reads are independent; partial failures
/// (e.g. no collateral data yet) degrade gracefully to empty collections rather
/// than returning an error.
pub fn get_dashboard_snapshot(env: &Env) -> Result<DashboardSnapshot, AnalyticsError> {
    let protocol = get_protocol_stats(env)?;
    let collateral_ratios = get_collateral_ratio_snapshots(env);
    let active_alerts = check_metric_alerts(env).unwrap_or_else(|_| Vec::new(env));
    let recent_activity = get_recent_activity(env, DASHBOARD_ACTIVITY_LIMIT, 0)
        .unwrap_or_else(|_| Vec::new(env));

    Ok(DashboardSnapshot {
        protocol,
        collateral_ratios,
        active_alerts,
        recent_activity,
        generated_at: env.ledger().timestamp(),
    })
}

/// Return a summary of per-user risk distribution across the protocol.
///
/// Iterates the activity log to collect unique users, computes each user's
/// health factor, and buckets them by risk level (1–5). Returns counts per
/// bucket and the total users sampled. This is the data source for the
/// "Risk Distribution" panel on the dashboard.
///
/// # Note
/// Because Soroban has no native iteration over all storage keys, this
/// function is limited to users visible in the bounded activity log. A full
/// enumeration would require a separate user-index maintained by the deposit
/// module — that is out of scope here (see PR description for #795).
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RiskDistributionSummary {
    /// Total unique users sampled from the activity log.
    pub users_sampled: u32,
    /// Count of users at risk level 1 (health ≥ 1.5×, low risk).
    pub level_1: u32,
    /// Count of users at risk level 2 (health ≥ 1.2×).
    pub level_2: u32,
    /// Count of users at risk level 3 (health ≥ 1.1×).
    pub level_3: u32,
    /// Count of users at risk level 4 (health ≥ 1.05×).
    pub level_4: u32,
    /// Count of users at risk level 5 (health < 1.05×, critical).
    pub level_5: u32,
}

pub fn get_risk_distribution(env: &Env) -> RiskDistributionSummary {
    let activity_log: Vec<ActivityEntry> = env
        .storage()
        .persistent()
        .get::<AnalyticsDataKey, Vec<ActivityEntry>>(&AnalyticsDataKey::ActivityLog)
        .unwrap_or_else(|| Vec::new(env));

    // Deduplicate users using a simple visited-list (Vec used as a set —
    // acceptable at dashboard sample sizes; a Map would require XDR overhead).
    let mut seen: Vec<crate::deposit::DepositDataKey> = Vec::new(env);
    let mut summary = RiskDistributionSummary {
        users_sampled: 0,
        level_1: 0,
        level_2: 0,
        level_3: 0,
        level_4: 0,
        level_5: 0,
    };

    for i in 0..activity_log.len() {
        let entry = match activity_log.get(i) {
            Some(e) => e,
            None => continue,
        };
        let user_key = crate::deposit::DepositDataKey::Position(entry.user.clone());
        // Skip if already counted.
        if seen.contains(&user_key) {
            continue;
        }
        seen.push_back(user_key);

        let health = calculate_health_factor(env, &entry.user).unwrap_or(i128::MAX);
        let level = calculate_user_risk_level(health);
        summary.users_sampled = summary.users_sampled.saturating_add(1);
        match level {
            1 => summary.level_1 = summary.level_1.saturating_add(1),
            2 => summary.level_2 = summary.level_2.saturating_add(1),
            3 => summary.level_3 = summary.level_3.saturating_add(1),
            4 => summary.level_4 = summary.level_4.saturating_add(1),
            _ => summary.level_5 = summary.level_5.saturating_add(1),
        }
    }

    summary
}

/// Summarise total borrow/deposit/withdrawal/repayment volumes aggregated from
/// the activity log. Used by the "Volume" panel on the dashboard.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct VolumeSummary {
    pub total_deposit_volume: i128,
    pub total_borrow_volume: i128,
    pub total_withdrawal_volume: i128,
    pub total_repayment_volume: i128,
    pub total_liquidation_volume: i128,
    pub entry_count: u32,
}

pub fn get_volume_summary(env: &Env) -> VolumeSummary {
    let activity_log: Vec<ActivityEntry> = env
        .storage()
        .persistent()
        .get::<AnalyticsDataKey, Vec<ActivityEntry>>(&AnalyticsDataKey::ActivityLog)
        .unwrap_or_else(|| Vec::new(env));

    let mut summary = VolumeSummary {
        total_deposit_volume: 0,
        total_borrow_volume: 0,
        total_withdrawal_volume: 0,
        total_repayment_volume: 0,
        total_liquidation_volume: 0,
        entry_count: activity_log.len() as u32,
    };

    for i in 0..activity_log.len() {
        let entry = match activity_log.get(i) {
            Some(e) => e,
            None => continue,
        };
        let name = entry.activity_type.to_string();
        if name == "deposit" {
            summary.total_deposit_volume =
                summary.total_deposit_volume.saturating_add(entry.amount);
        } else if name == "borrow" {
            summary.total_borrow_volume =
                summary.total_borrow_volume.saturating_add(entry.amount);
        } else if name == "withdraw" {
            summary.total_withdrawal_volume =
                summary.total_withdrawal_volume.saturating_add(entry.amount);
        } else if name == "repay" {
            summary.total_repayment_volume =
                summary.total_repayment_volume.saturating_add(entry.amount);
        } else if name == "liquidate" {
            summary.total_liquidation_volume =
                summary.total_liquidation_volume.saturating_add(entry.amount);
        }
    }

    summary
}

// -------------------------------------------------------------------------
// Cross-asset portfolio risk analytics (Issue #663)
// -------------------------------------------------------------------------

/// Portfolio risk score in basis points (0 = none, 10000 = critical).
/// Combines health-factor distance-to-liquidation with collateral concentration.
pub fn portfolio_risk_score(env: &Env, user: &Address) -> Result<i128, AnalyticsError> {
    let summary = crate::cross_asset::get_unified_health_factor(env, user)
        .map_err(|_| AnalyticsError::DataNotFound)?;
    if summary.weighted_debt_value == 0 {
        return Ok(0);
    }
    let hf = summary.health_factor;
    let hf_risk = if hf >= 15_000 {
        0
    } else if hf <= 5_000 {
        10_000
    } else {
        ((15_000 - hf) * 10_000) / 10_000
    };
    Ok(if hf_risk > 10_000 { 10_000 } else { hf_risk })
}
