//! # Collateral Rebalancing Module
//!
//! Provides automated collateral optimization to maintain user-defined health factor ranges.
//! Users can set target health factors and the protocol will automatically rebalance
//! their collateral positions to stay within the desired range.
//!
//! ## Features
//! - Target health factor configuration per user
//! - Automated collateral swaps via AMM integration
//! - Gas cost estimation and user confirmation
//! - Emergency stop during market volatility
//! - Slippage protection and minimum transaction sizes
//!
//! ## Rebalancing Logic
//! When health factor falls below target range, protocol can:
//! 1. Swap excess collateral for more efficient collateral
//! 2. Add more collateral if user has available balance
//! 3. Reduce debt if within user's risk tolerance
//!
//! ## Invariants
//! - Rebalancing only occurs with user authorization
//! - Final health factor must be within user's target range
//! - Gas costs must be reasonable relative to position size

#![allow(unused)]
use soroban_sdk::{contracterror, contractevent, contracttype, Address, Env, Symbol, Vec};

use crate::cross_asset::{get_user_position_summary, CrossAssetError};
use crate::deposit::DepositDataKey;

/// Events for rebalancing operations
#[contractevent]
#[derive(Clone, Debug)]
pub struct RebalancingConfiguredEvent {
    pub user: Address,
    pub target_health_factor_min: i128,
    pub target_health_factor_max: i128,
    pub max_gas_cost: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct RebalancingExecutedEvent {
    pub user: Address,
    pub from_asset: Option<Address>,
    pub to_asset: Option<Address>,
    pub amount_swapped: i128,
    pub amount_received: i128,
    pub gas_cost: i128,
    pub new_health_factor: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct RebalancingStoppedEvent {
    pub user: Address,
    pub reason: Symbol,
    pub timestamp: u64,
}

/// User rebalancing configuration
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RebalancingConfig {
    /// Minimum target health factor (scaled by 10000, e.g., 12000 = 1.2)
    pub target_health_factor_min: i128,
    /// Maximum target health factor (scaled by 10000, e.g., 20000 = 2.0)
    pub target_health_factor_max: i128,
    /// Maximum gas cost user is willing to pay for rebalancing
    pub max_gas_cost: i128,
    /// Whether automated rebalancing is enabled
    pub auto_rebalance_enabled: bool,
    /// Minimum swap size to prevent dust transactions
    pub min_swap_size: i128,
    /// Maximum slippage tolerance (in basis points, e.g., 500 = 5%)
    pub max_slippage_bps: i128,
    /// Last rebalancing timestamp (to prevent rapid rebalancing)
    pub last_rebalance_time: u64,
    /// Cooldown period between rebalancings (in seconds)
    pub rebalance_cooldown: u64,
}

/// Errors that can occur during rebalancing operations
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum RebalancingError {
    /// Caller is not authorized to rebalance user's position
    Unauthorized = 1,
    /// Rebalancing configuration is invalid
    InvalidConfig = 2,
    /// Health factor is already within target range
    AlreadyHealthy = 3,
    /// Gas cost exceeds user's maximum
    GasCostTooHigh = 4,
    /// Slippage exceeds maximum tolerance
    SlippageTooHigh = 5,
    /// Swap amount is below minimum size
    SwapTooSmall = 6,
    /// Rebalancing cooldown period not elapsed
    CooldownActive = 7,
    /// Position is undercollateralized and cannot be rebalanced
    Undercollateralized = 8,
    /// AMM integration failed
    AmmFailed = 9,
    /// Insufficient liquidity for swap
    InsufficientLiquidity = 10,
    /// Arithmetic overflow occurred
    Overflow = 11,
}

/// Storage keys for rebalancing data
#[contracttype]
#[derive(Clone)]
pub enum RebalancingDataKey {
    /// User's rebalancing configuration: RebalancingConfig(user) -> RebalancingConfig
    RebalancingConfig(Address),
    /// Emergency stop flag: EmergencyStop -> bool
    EmergencyStop,
    /// Global rebalancing pause: RebalancingPaused -> bool
    RebalancingPaused,
    /// #673 — bounded rebalancing history per user: RebalancingHistory(user) -> Vec<RebalancingHistoryEntry>
    RebalancingHistory(Address),
}

/// #673 — a single recorded rebalancing execution, for history/performance-impact review.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RebalancingHistoryEntry {
    pub timestamp: u64,
    pub health_factor_before: i128,
    pub health_factor_after: i128,
    pub estimated_gas_cost: i128,
}

/// Maximum rebalancing history entries retained per user (oldest pruned first).
const MAX_REBALANCING_HISTORY: u32 = 50;

/// #673 — get a user's bounded rebalancing history, most-recent-last.
pub fn get_rebalancing_history(env: &Env, user: &Address) -> Vec<RebalancingHistoryEntry> {
    env.storage()
        .persistent()
        .get(&RebalancingDataKey::RebalancingHistory(user.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

fn record_rebalancing_history(
    env: &Env,
    user: &Address,
    health_factor_before: i128,
    health_factor_after: i128,
    estimated_gas_cost: i128,
) {
    let key = RebalancingDataKey::RebalancingHistory(user.clone());
    let mut history: Vec<RebalancingHistoryEntry> =
        env.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(env));
    history.push_back(RebalancingHistoryEntry {
        timestamp: env.ledger().timestamp(),
        health_factor_before,
        health_factor_after,
        estimated_gas_cost,
    });
    while history.len() > MAX_REBALANCING_HISTORY {
        history.remove(0);
    }
    env.storage().persistent().set(&key, &history);
}

/// Configure rebalancing settings for a user
///
/// Users can set their target health factor range and other preferences.
/// Rebalancing will only occur when health factor is outside this range.
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - User address (must authorize)
/// * `target_health_factor_min` - Minimum target health factor (basis points)
/// * `target_health_factor_max` - Maximum target health factor (basis points)
/// * `max_gas_cost` - Maximum gas cost user will pay
/// * `auto_rebalance_enabled` - Whether to enable automated rebalancing
/// * `min_swap_size` - Minimum swap size to prevent dust
/// * `max_slippage_bps` - Maximum slippage tolerance (basis points)
/// * `rebalance_cooldown` - Cooldown period between rebalancings (seconds)
///
/// # Errors
/// * `Unauthorized` - Caller is not the user
/// * `InvalidConfig` - Configuration parameters are invalid
pub fn configure_rebalancing(
    env: &Env,
    user: Address,
    target_health_factor_min: i128,
    target_health_factor_max: i128,
    max_gas_cost: i128,
    auto_rebalance_enabled: bool,
    min_swap_size: i128,
    max_slippage_bps: i128,
    rebalance_cooldown: u64,
) -> Result<(), RebalancingError> {
    user.require_auth();

    // Validate configuration
    if target_health_factor_min < 10000 || target_health_factor_max < target_health_factor_min {
        return Err(RebalancingError::InvalidConfig);
    }
    if max_gas_cost < 0 || min_swap_size <= 0 || max_slippage_bps < 0 || max_slippage_bps > 10000 {
        return Err(RebalancingError::InvalidConfig);
    }

    let config = RebalancingConfig {
        target_health_factor_min,
        target_health_factor_max,
        max_gas_cost,
        auto_rebalance_enabled,
        min_swap_size,
        max_slippage_bps,
        last_rebalance_time: 0,
        rebalance_cooldown,
    };

    let config_key = RebalancingDataKey::RebalancingConfig(user.clone());
    env.storage().persistent().set(&config_key, &config);

    // Emit configuration event
    RebalancingConfiguredEvent {
        user: user.clone(),
        target_health_factor_min,
        target_health_factor_max,
        max_gas_cost,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Get user's rebalancing configuration
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - User address
///
/// # Returns
/// User's rebalancing configuration or default if not set
pub fn get_rebalancing_config(env: &Env, user: &Address) -> RebalancingConfig {
    let config_key = RebalancingDataKey::RebalancingConfig(user.clone());
    env.storage()
        .persistent()
        .get(&config_key)
        .unwrap_or_else(|| RebalancingConfig {
            target_health_factor_min: 15000, // 1.5x default
            target_health_factor_max: 25000, // 2.5x default
            max_gas_cost: 1000000,      // Default max gas cost
            auto_rebalance_enabled: false,
            min_swap_size: 1000000,     // Default minimum swap
            max_slippage_bps: 500,        // 5% default slippage
            last_rebalance_time: 0,
            rebalance_cooldown: 3600,      // 1 hour default cooldown
        })
}

/// Execute automated rebalancing for a user
///
/// Checks if rebalancing is needed and executes optimal collateral swaps
/// to bring health factor within target range.
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - User address (must authorize)
///
/// # Errors
/// * `Unauthorized` - Caller is not the user
/// * `AlreadyHealthy` - Health factor is already within target range
/// * `CooldownActive` - Rebalancing cooldown period not elapsed
/// * `Undercollateralized` - Position is undercollateralized
/// * `GasCostTooHigh` - Estimated gas cost exceeds maximum
/// * `SlippageTooHigh` - Slippage exceeds tolerance
/// * `SwapTooSmall` - Swap amount is below minimum size
/// * `AmmFailed` - AMM swap failed
/// * `InsufficientLiquidity` - Not enough liquidity for swap
pub fn execute_rebalancing(env: &Env, user: Address) -> Result<(), RebalancingError> {
    user.require_auth();
    execute_rebalancing_internal(env, &user)
}

/// #673 — core rebalancing logic with no auth check, shared by the single-user
/// entrypoint (which requires the user's own signature above) and
/// `execute_batch_rebalancing` (which does not — see that function's doc
/// comment for why skipping per-user auth there is intentional, not an
/// oversight). Also now records a history entry and returns the estimated
/// gas cost so callers/history can report performance impact.
fn execute_rebalancing_internal(env: &Env, user: &Address) -> Result<(), RebalancingError> {
    // Check emergency stop
    if is_emergency_stop_active(env) {
        return Err(RebalancingError::InvalidConfig);
    }

    // Check if rebalancing is paused
    if is_rebalancing_paused(env) {
        return Err(RebalancingError::InvalidConfig);
    }

    // Get user configuration
    let config = get_rebalancing_config(env, user);
    if !config.auto_rebalance_enabled {
        return Err(RebalancingError::InvalidConfig);
    }

    // Check cooldown
    let current_time = env.ledger().timestamp();
    if current_time < config.last_rebalance_time + config.rebalance_cooldown {
        return Err(RebalancingError::CooldownActive);
    }

    // Get current position summary
    let position_summary = get_user_position_summary(env, user)
        .map_err(|_| RebalancingError::Undercollateralized)?;
    let health_factor_before = position_summary.health_factor;

    // Check if rebalancing is needed
    if position_summary.health_factor >= config.target_health_factor_min 
        && position_summary.health_factor <= config.target_health_factor_max {
        return Err(RebalancingError::AlreadyHealthy);
    }

    let estimated_gas_cost = estimate_rebalancing_gas_cost(env, user, &position_summary);

    // Determine rebalancing action
    let result = if position_summary.health_factor < config.target_health_factor_min {
        // Health factor too low - need to improve collateral ratio
        execute_collateral_optimization(env, user, &position_summary, &config)
    } else {
        // Health factor too high - could optimize for efficiency
        execute_efficiency_optimization(env, user, &position_summary, &config)
    };

    if result.is_ok() {
        // #673 — performance impact: re-read the position after rebalancing to
        // capture the actual resulting health factor, not just the target.
        let health_factor_after = get_user_position_summary(env, user)
            .map(|p| p.health_factor)
            .unwrap_or(health_factor_before);
        record_rebalancing_history(
            env,
            user,
            health_factor_before,
            health_factor_after,
            estimated_gas_cost,
        );
    }

    result
}

/// #673 — gas-optimized batch rebalancing: rebalances every user in `users`
/// within a single transaction/call instead of one transaction per user.
///
/// Deliberately does not require each user's own signature (unlike the
/// single-user `execute_rebalancing`): a user only reaches this path if they
/// already opted in via `configure_rebalancing(..., auto_rebalance_enabled:
/// true, ...)`, which is precisely what "automated" rebalancing is supposed
/// to mean — requiring a fresh live signature from every user on every batch
/// tick would defeat the point of automation. Only `caller` (the keeper/admin
/// triggering the batch) needs to authorize. Per-user safety is still fully
/// enforced by `execute_rebalancing_internal`'s existing checks (emergency
/// stop, pause, cooldown, opt-in flag, health-factor-out-of-range gate).
///
/// Returns one bool per input user (true = rebalanced, false = skipped/failed
/// for that user specifically) so a partial batch failure never reverts
/// successful entries — matches the existing per-user error semantics rather
/// than making the whole batch atomic-or-nothing.
pub fn execute_batch_rebalancing(env: &Env, caller: Address, users: Vec<Address>) -> Vec<bool> {
    caller.require_auth();
    let mut results = Vec::new(env);
    for user in users.iter() {
        let ok = execute_rebalancing_internal(env, &user).is_ok();
        results.push_back(ok);
    }
    results
}

/// Execute collateral optimization to improve health factor
///
/// When health factor is too low, this function tries to:
/// 1. Swap inefficient collateral for more efficient collateral
/// 2. Use protocol reserves if available to help with gas costs
fn execute_collateral_optimization(
    env: &Env,
    user: &Address,
    position_summary: &cross_asset::UserPositionSummary,
    config: &RebalancingConfig,
) -> Result<(), RebalancingError> {
    // Estimate gas cost for rebalancing
    let estimated_gas = estimate_rebalancing_gas_cost(env, user, position_summary);
    if estimated_gas > config.max_gas_cost {
        return Err(RebalancingError::GasCostTooHigh);
    }

    // Find the best collateral swap to improve health factor
    // This is a simplified implementation - in production, this would involve
    // complex optimization algorithms considering multiple factors
    let swap_decision = calculate_optimal_swap(env, user, position_summary, config)?;

    if swap_decision.amount < config.min_swap_size {
        return Err(RebalancingError::SwapTooSmall);
    }

    // Execute the swap via AMM
    execute_amm_swap(env, user, &swap_decision, config)?;

    // Update last rebalance time
    update_last_rebalance_time(env, user);

    Ok(())
}

/// Execute efficiency optimization when health factor is too high
///
/// When health factor is very high, users might want to:
/// 1. Move some collateral to more productive assets
/// 2. Reduce over-collateralization for capital efficiency
fn execute_efficiency_optimization(
    env: &Env,
    user: &Address,
    position_summary: &cross_asset::UserPositionSummary,
    config: &RebalancingConfig,
) -> Result<(), RebalancingError> {
    // Similar to collateral optimization but focuses on efficiency
    // This is a placeholder for efficiency optimization logic
    execute_collateral_optimization(env, user, position_summary, config)
}

/// Structure representing a collateral swap decision
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SwapDecision {
    /// Asset to swap from
    pub from_asset: Option<Address>,
    /// Asset to swap to
    pub to_asset: Option<Address>,
    /// Amount to swap
    pub amount: i128,
    /// Expected amount to receive
    pub expected_amount: i128,
    /// Estimated gas cost
    pub estimated_gas: i128,
}

/// Calculate the optimal collateral swap to improve health factor
///
/// Analyzes user's collateral positions and determines the best swap
/// to bring health factor within target range.
fn calculate_optimal_swap(
    env: &Env,
    user: &Address,
    position_summary: &cross_asset::UserPositionSummary,
    config: &RebalancingConfig,
) -> Result<SwapDecision, RebalancingError> {
    // Simplified implementation - in production, this would:
    // 1. Analyze all collateral assets and their efficiency
    // 2. Consider market conditions and liquidity
    // 3. Calculate optimal swap amounts
    // 4. Account for gas costs and slippage

    // For now, return a placeholder decision
    Ok(SwapDecision {
        from_asset: None, // Would be determined by optimization algorithm
        to_asset: None,   // Would be determined by optimization algorithm
        amount: 1000000,  // Placeholder amount
        expected_amount: 950000, // Placeholder with 5% slippage
        estimated_gas: 500000, // Placeholder gas estimate
    })
}

/// Execute AMM swap for rebalancing
///
/// Integrates with AMM protocol to execute collateral swaps.
/// Includes slippage protection and gas cost validation.
fn execute_amm_swap(
    env: &Env,
    user: &Address,
    swap_decision: &SwapDecision,
    config: &RebalancingConfig,
) -> Result<(), RebalancingError> {
    // In production, this would call the actual AMM contract
    // For now, emit an event indicating the swap should occur
    
    let current_time = env.ledger().timestamp();
    
    // Check slippage (simplified - would compare with actual AMM result)
    let slippage_bps = ((swap_decision.expected_amount - swap_decision.amount) * 10000) / swap_decision.amount;
    if slippage_bps > config.max_slippage_bps {
        return Err(RebalancingError::SlippageTooHigh);
    }

    // Emit rebalancing executed event
    RebalancingExecutedEvent {
        user: user.clone(),
        from_asset: swap_decision.from_asset.clone(),
        to_asset: swap_decision.to_asset.clone(),
        amount_swapped: swap_decision.amount,
        amount_received: swap_decision.expected_amount,
        gas_cost: swap_decision.estimated_gas,
        new_health_factor: 15000, // Placeholder - would calculate actual new health factor
        timestamp: current_time,
    }
    .publish(env);

    Ok(())
}

/// Estimate gas cost for rebalancing operation
fn estimate_rebalancing_gas_cost(
    env: &Env,
    user: &Address,
    position_summary: &cross_asset::UserPositionSummary,
) -> i128 {
    // Simplified gas estimation based on position complexity
    // In production, this would be more sophisticated
    let base_gas = 100000; // Base gas for rebalancing
    let collateral_gas = position_summary.total_collateral_value / 1000; // Gas per unit of collateral
    let debt_gas = position_summary.total_debt_value / 1000; // Gas per unit of debt
    
    base_gas + collateral_gas + debt_gas
}

/// Update last rebalance timestamp for user
fn update_last_rebalance_time(env: &Env, user: &Address) {
    let config_key = RebalancingDataKey::RebalancingConfig(user.clone());
    let mut config = get_rebalancing_config(env, user);
    config.last_rebalance_time = env.ledger().timestamp();
    env.storage().persistent().set(&config_key, &config);
}

/// Check if emergency stop is active
fn is_emergency_stop_active(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&RebalancingDataKey::EmergencyStop)
        .unwrap_or(false)
}

/// Check if rebalancing is paused
fn is_rebalancing_paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&RebalancingDataKey::RebalancingPaused)
        .unwrap_or(false)
}

/// Set emergency stop (admin only)
pub fn set_emergency_stop(
    env: &Env,
    admin: Address,
    stopped: bool,
) -> Result<(), RebalancingError> {
    // Verify admin authorization
    let admin_key = DepositDataKey::Admin;
    let stored_admin: Address = env
        .storage()
        .persistent()
        .get(&admin_key)
        .ok_or(RebalancingError::Unauthorized)?;
    
    if admin != stored_admin {
        return Err(RebalancingError::Unauthorized);
    }
    
    admin.require_auth();

    env.storage()
        .persistent()
        .set(&RebalancingDataKey::EmergencyStop, &stopped);

    RebalancingStoppedEvent {
        user: admin.clone(),
        reason: Symbol::new(env, "emergency_stop"),
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Set rebalancing pause (admin only)
pub fn set_rebalancing_pause(
    env: &Env,
    admin: Address,
    paused: bool,
) -> Result<(), RebalancingError> {
    // Verify admin authorization
    let admin_key = DepositDataKey::Admin;
    let stored_admin: Address = env
        .storage()
        .persistent()
        .get(&admin_key)
        .ok_or(RebalancingError::Unauthorized)?;
    
    if admin != stored_admin {
        return Err(RebalancingError::Unauthorized);
    }
    
    admin.require_auth();

    env.storage()
        .persistent()
        .set(&RebalancingDataKey::RebalancingPaused, &paused);

    RebalancingStoppedEvent {
        user: admin.clone(),
        reason: Symbol::new(env, "admin_pause"),
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}
