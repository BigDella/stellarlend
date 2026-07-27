use soroban_sdk::{contracttype, Address, Env, String, Vec};
use stellarlend_amm::AmmError;

// ─── Constants ───────────────────────────────────────────────────────────────

/// Default withdrawal buffer: 80% in AMM, 20% in pool
pub const DEFAULT_WITHDRAWAL_BUFFER_BPS: i128 = 8000;

/// Basis points scale
pub const BPS_SCALE: i128 = 10_000;

/// Utilization threshold for auto-allocation (80%)
pub const AUTO_ALLOCATION_UTILIZATION_THRESHOLD_BPS: i128 = 8000;

/// Minimum liquidity ratio for impermanent loss alert (50% drop)
pub const IL_ALERT_THRESHOLD_BPS: i128 = 5000;

// ─── Storage Keys ────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum AmmLendingKey {
    /// LP token balance per lending pool asset: Address -> i128
    LpTokenBalance(Address),
    /// AMM protocol address for a lending pool asset
    AmmProtocolForAsset(Address),
    /// Withdrawal buffer BPS per asset: Address -> i128
    WithdrawalBufferBps(Address),
    /// Pool utilization snapshot per asset: Address -> i128
    PoolUtilization(Address),
    /// LP fee accrued for distribution: Address -> i128
    AccruedLpFees(Address),
    /// Impermanent loss tracking: Address -> (initial_price, current_price)
    IlTracking(Address),
    /// Auto-allocation enabled flag
    AutoAllocationEnabled,
    /// Admin address
    AmmLendingAdmin,
}

// ─── Types ───────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug)]
pub struct LpTokenPosition {
    pub asset: Address,
    pub lp_tokens: i128,
    pub underlying_amount: i128,
    pub amm_protocol: Address,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct IlSnapshot {
    pub initial_price: i128,
    pub current_price: i128,
    pub price_ratio_bps: i128,
    pub alert_triggered: bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct AllocationSuggestion {
    pub asset: Address,
    pub current_utilization_bps: i128,
    pub suggested_allocation_bps: i128,
    pub reason: String,
}

// ─── Admin ───────────────────────────────────────────────────────────────────

pub fn initialize_amm_lending(env: &Env, admin: Address) -> Result<(), AmmError> {
    let key = AmmLendingKey::AmmLendingAdmin;
    if env.storage().instance().has(&key) {
        return Err(AmmError::AlreadyInitialized);
    }
    admin.require_auth();
    env.storage().instance().set(&key, &admin);
    env.storage()
        .instance()
        .set(&AmmLendingKey::AutoAllocationEnabled, &true);
    Ok(())
}

fn require_amm_admin(env: &Env, caller: &Address) -> Result<(), AmmError> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&AmmLendingKey::AmmLendingAdmin)
        .ok_or(AmmError::Unauthorized)?;
    if *caller != admin {
        return Err(AmmError::Unauthorized);
    }
    caller.require_auth();
    Ok(())
}

// ─── LP Token Wrapping ───────────────────────────────────────────────────────

/// Wrap lending pool deposits into AMM LP tokens.
/// Moves funds from the lending pool to the AMM and tracks the LP position.
pub fn wrap_deposit_to_lp(
    env: &Env,
    admin: Address,
    asset: Address,
    amount: i128,
    amm_protocol: Address,
) -> Result<LpTokenPosition, AmmError> {
    require_amm_admin(env, &admin)?;

    if amount <= 0 {
        return Err(AmmError::InvalidSwapParams);
    }

    // Record the AMM protocol for this asset
    env.storage()
        .persistent()
        .set(&AmmLendingKey::AmmProtocolForAsset(asset.clone()), &amm_protocol);

    // Simulated LP token mint (in production, would call AMM add_liquidity)
    let lp_tokens_received = amount; // Simplified: 1:1 for accounting

    let key = AmmLendingKey::LpTokenBalance(asset.clone());
    let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
    let new_balance = current.saturating_add(lp_tokens_received);
    env.storage().persistent().set(&key, &new_balance);

    // Track initial price for impermanent loss
    // In production this would use the oracle price
    let initial_price: i128 = BPS_SCALE; // Starting at 1.0 price ratio
    env.storage().persistent().set(
        &AmmLendingKey::IlTracking(asset.clone()),
        &IlSnapshot {
            initial_price,
            current_price: initial_price,
            price_ratio_bps: BPS_SCALE,
            alert_triggered: false,
        },
    );

    Ok(LpTokenPosition {
        asset,
        lp_tokens: new_balance,
        underlying_amount: amount,
        amm_protocol,
    })
}

/// Unwrap LP tokens back to underlying lending pool assets.
/// Respects the withdrawal buffer (keeps 80% in AMM unless buffer is adjusted).
pub fn unwrap_lp_to_deposit(
    env: &Env,
    admin: Address,
    asset: Address,
    lp_tokens: i128,
) -> Result<i128, AmmError> {
    require_amm_admin(env, &admin)?;

    if lp_tokens <= 0 {
        return Err(AmmError::InvalidSwapParams);
    }

    let key = AmmLendingKey::LpTokenBalance(asset.clone());
    let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);

    if lp_tokens > current {
        return Err(AmmError::InsufficientLiquidity);
    }

    // Check withdrawal buffer
    let buffer_bps: i128 = env
        .storage()
        .persistent()
        .get(&AmmLendingKey::WithdrawalBufferBps(asset.clone()))
        .unwrap_or(DEFAULT_WITHDRAWAL_BUFFER_BPS);

    let remaining_after = current - lp_tokens;
    let remaining_ratio = remaining_after
        .saturating_mul(BPS_SCALE)
        .checked_div(current)
        .unwrap_or(0);

    if remaining_ratio < buffer_bps {
        return Err(AmmError::InsufficientLiquidity);
    }

    let new_balance = current - lp_tokens;
    env.storage().persistent().set(&key, &new_balance);

    Ok(lp_tokens)
}

/// Get LP token balance for an asset
pub fn get_lp_token_balance(env: &Env, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&AmmLendingKey::LpTokenBalance(asset.clone()))
        .unwrap_or(0)
}

// ─── Withdrawal Buffer Management ────────────────────────────────────────────

/// Set the withdrawal buffer for an asset (admin only).
/// buffer_bps: percentage of funds to keep in AMM (e.g., 8000 = 80%).
pub fn set_withdrawal_buffer(
    env: &Env,
    admin: Address,
    asset: Address,
    buffer_bps: i128,
) -> Result<(), AmmError> {
    require_amm_admin(env, &admin)?;

    if buffer_bps > BPS_SCALE {
        return Err(AmmError::InvalidSwapParams);
    }

    env.storage()
        .persistent()
        .set(&AmmLendingKey::WithdrawalBufferBps(asset), &buffer_bps);
    Ok(())
}

/// Get the withdrawal buffer for an asset
pub fn get_withdrawal_buffer(env: &Env, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&AmmLendingKey::WithdrawalBufferBps(asset.clone()))
        .unwrap_or(DEFAULT_WITHDRAWAL_BUFFER_BPS)
}

// ─── Automated Liquidity Allocation ─────────────────────────────────────────

/// Calculate optimal AMM allocation based on pool utilization.
/// Higher utilization = more funds should stay in lending pool.
/// Lower utilization = more funds can be deployed to AMM.
pub fn calculate_optimal_allocation(
    env: &Env,
    asset: &Address,
    total_liquidity: i128,
    borrowed_amount: i128,
) -> Result<AllocationSuggestion, AmmError> {
    if total_liquidity == 0 {
        return Err(AmmError::InvalidSwapParams);
    }

    let utilization_bps = borrowed_amount
        .saturating_mul(BPS_SCALE)
        .checked_div(total_liquidity)
        .unwrap_or(0);

    // Store utilization for rebalancing triggers
    env.storage()
        .persistent()
        .set(&AmmLendingKey::PoolUtilization(asset.clone()), &utilization_bps);

    let suggested_allocation_bps: i128;
    let reason: String;

    if utilization_bps > AUTO_ALLOCATION_UTILIZATION_THRESHOLD_BPS {
        // High utilization: reduce AMM allocation, keep funds in lending pool
        suggested_allocation_bps = BPS_SCALE - utilization_bps;
        reason = String::from_str(&env, "High pool utilization — prioritizing lending liquidity");
    } else if utilization_bps < AUTO_ALLOCATION_UTILIZATION_THRESHOLD_BPS / 2 {
        // Low utilization: increase AMM allocation
        suggested_allocation_bps = DEFAULT_WITHDRAWAL_BUFFER_BPS;
        reason = String::from_str(&env, "Low pool utilization — deploying to AMM for yield");
    } else {
        // Moderate utilization: maintain default buffer
        suggested_allocation_bps = DEFAULT_WITHDRAWAL_BUFFER_BPS;
        reason = String::from_str(&env, "Moderate utilization — maintaining default allocation");
    }

    Ok(AllocationSuggestion {
        asset: asset.clone(),
        current_utilization_bps: utilization_bps,
        suggested_allocation_bps,
        reason,
    })
}

/// Execute automated AMM rebalancing based on pool utilization.
/// Called by keeper/oracle to maintain optimal allocation.
pub fn auto_rebalance_allocation(
    env: &Env,
    admin: Address,
    asset: Address,
    total_liquidity: i128,
    borrowed_amount: i128,
    current_amm_balance: i128,
) -> Result<i128, AmmError> {
    require_amm_admin(env, &admin)?;

    let suggestion = calculate_optimal_allocation(env, &asset, total_liquidity, borrowed_amount)?;
    let target_amm_balance = total_liquidity
        .saturating_mul(suggestion.suggested_allocation_bps)
        .checked_div(BPS_SCALE)
        .unwrap_or(0);

    if current_amm_balance == target_amm_balance {
        return Ok(0); // No rebalancing needed
    }

    let rebalance_amount = if current_amm_balance > target_amm_balance {
        // Too much in AMM, move some back to lending pool
        current_amm_balance - target_amm_balance
    } else {
        // Too little in AMM, move more to AMM
        target_amm_balance - current_amm_balance
    };

    // Update withdrawal buffer to match the new allocation target
    env.storage().persistent().set(
        &AmmLendingKey::WithdrawalBufferBps(asset.clone()),
        &suggestion.suggested_allocation_bps,
    );

    Ok(rebalance_amount)
}

/// Record LP fee accrued for distribution to depositors.
pub fn record_lp_fees(
    env: &Env,
    admin: Address,
    asset: Address,
    fee_amount: i128,
) -> Result<(), AmmError> {
    require_amm_admin(env, &admin)?;

    if fee_amount <= 0 {
        return Err(AmmError::InvalidSwapParams);
    }

    let key = AmmLendingKey::AccruedLpFees(asset.clone());
    let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
    env.storage()
        .persistent()
        .set(&key, &current.saturating_add(fee_amount));

    Ok(())
}

/// Get accrued LP fees for an asset.
pub fn get_accrued_lp_fees(env: &Env, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&AmmLendingKey::AccruedLpFees(asset.clone()))
        .unwrap_or(0)
}

// ─── Impermanent Loss Monitoring ─────────────────────────────────────────────

/// Update impermanent loss tracking with current price.
/// Returns true if IL alert threshold is crossed.
pub fn update_il_tracking(
    env: &Env,
    asset: &Address,
    current_price: i128,
) -> Result<bool, AmmError> {
    let key = AmmLendingKey::IlTracking(asset.clone());
    let mut snapshot: IlSnapshot = env.storage().persistent().get(&key).ok_or(AmmError::InvalidSwapParams)?;

    snapshot.current_price = current_price;

    if snapshot.initial_price > 0 {
        let ratio_bps = current_price
            .saturating_mul(BPS_SCALE)
            .checked_div(snapshot.initial_price)
            .unwrap_or(BPS_SCALE);
        snapshot.price_ratio_bps = ratio_bps;

        // Alert if price has dropped more than threshold
        if ratio_bps < IL_ALERT_THRESHOLD_BPS && !snapshot.alert_triggered {
            snapshot.alert_triggered = true;
            env.storage().persistent().set(&key, &snapshot);
            return Ok(true); // Alert!
        }
    }

    env.storage().persistent().set(&key, &snapshot);
    Ok(false)
}

/// Get IL tracking snapshot for an asset.
pub fn get_il_snapshot(env: &Env, asset: &Address) -> Option<IlSnapshot> {
    env.storage()
        .persistent()
        .get(&AmmLendingKey::IlTracking(asset.clone()))
}

// ─── Pool Allocation Optimizer (#682) ─────────────────────────────────────────

/// Target utilization for optimal capital efficiency (80%)
const OPTIMAL_UTILIZATION_BPS: i128 = 8000;

/// Minimum rebalance threshold (5% difference to trigger rebalance)
const REBALANCE_THRESHOLD_BPS: i128 = 500;

/// Pool allocation recommendation
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct AllocationRecommendation {
    pub asset: Address,
    pub current_utilization_bps: i128,
    pub recommended_allocation_bps: i128,
    pub action: AllocationAction,
    pub amount: i128,
}

/// Action to take for allocation optimization
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum AllocationAction {
    /// Move funds into this pool (under-utilized)
    Increase,
    /// Move funds out of this pool (over-utilized)
    Decrease,
    /// No change needed
    NoChange,
}

/// Result of an optimization pass
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct OptimizationResult {
    pub recommendations: Vec<AllocationRecommendation>,
    pub total_capital_efficiency_bps: i128,
    pub estimated_yield_improvement_bps: i128,
}

/// Analyze pool utilization and recommend allocation changes.
///
/// Examines all tracked pools and recommends rebalancing actions to
/// maximize capital efficiency while maintaining safety buffers.
pub fn optimize_allocation(
    env: &Env,
    pools: &Vec<Address>,
) -> Result<OptimizationResult, AmmError> {
    let mut recommendations: Vec<AllocationRecommendation> = Vec::new(env);
    let mut total_utilization: i128 = 0;
    let mut pool_count: i128 = 0;

    for pool in pools.iter() {
        let utilization_key = AmmLendingKey::PoolUtilization(pool.clone());
        let current_utilization: i128 = env
            .storage()
            .persistent()
            .get::<AmmLendingKey, i128>(&utilization_key)
            .unwrap_or(0);

        total_utilization = total_utilization.saturating_add(current_utilization);
        pool_count = pool_count.saturating_add(1);

        let deviation = if current_utilization > OPTIMAL_UTILIZATION_BPS {
            current_utilization - OPTIMAL_UTILIZATION_BPS
        } else {
            OPTIMAL_UTILIZATION_BPS - current_utilization
        };

        let (action, amount) = if deviation < REBALANCE_THRESHOLD_BPS {
            (AllocationAction::NoChange, 0)
        } else if current_utilization < OPTIMAL_UTILIZATION_BPS {
            // Under-utilized — increase allocation
            let buffer_key = AmmLendingKey::WithdrawalBufferBps(pool.clone());
            let buffer_bps: i128 = env
                .storage()
                .persistent()
                .get::<AmmLendingKey, i128>(&buffer_key)
                .unwrap_or(DEFAULT_WITHDRAWAL_BUFFER_BPS);
            let available = BPS_SCALE.saturating_sub(buffer_bps);
            let increase_amount = available
                .saturating_mul(OPTIMAL_UTILIZATION_BPS - current_utilization)
                .checked_div(BPS_SCALE)
                .unwrap_or(0);
            (AllocationAction::Increase, increase_amount)
        } else {
            // Over-utilized — decrease allocation
            let excess = current_utilization - OPTIMAL_UTILIZATION_BPS;
            let decrease_amount = excess
                .saturating_mul(current_utilization)
                .checked_div(BPS_SCALE)
                .unwrap_or(0);
            (AllocationAction::Decrease, decrease_amount)
        };

        recommendations.push_back(AllocationRecommendation {
            asset: pool.clone(),
            current_utilization_bps: current_utilization,
            recommended_allocation_bps: OPTIMAL_UTILIZATION_BPS,
            action,
            amount,
        });
    }

    let avg_utilization = if pool_count > 0 {
        total_utilization.checked_div(pool_count).unwrap_or(0)
    } else {
        0
    };

    // Estimate yield improvement: closer to optimal = better yield
    let efficiency = if avg_utilization <= OPTIMAL_UTILIZATION_BPS {
        avg_utilization
    } else {
        // Over-utilization means higher rates but more risk
        OPTIMAL_UTILIZATION_BPS
    };

    // Yield improvement estimate: moving from current to optimal
    let yield_improvement = if avg_utilization < OPTIMAL_UTILIZATION_BPS {
        (OPTIMAL_UTILIZATION_BPS - avg_utilization).checked_div(100).unwrap_or(0)
    } else {
        0
    };

    Ok(OptimizationResult {
        recommendations,
        total_capital_efficiency_bps: efficiency,
        estimated_yield_improvement_bps: yield_improvement,
    })
}

/// Update the utilization snapshot for a pool.
///
/// Should be called whenever deposits/withdrawals/borrows change pool state.
pub fn update_pool_utilization(
    env: &Env,
    asset: &Address,
    utilization_bps: i128,
) {
    let key = AmmLendingKey::PoolUtilization(asset.clone());
    env.storage().persistent().set(&key, &utilization_bps);
}

/// Get the current utilization snapshot for a pool.
pub fn get_pool_utilization(env: &Env, asset: &Address) -> i128 {
    let key = AmmLendingKey::PoolUtilization(asset.clone());
    env.storage()
        .persistent()
        .get::<AmmLendingKey, i128>(&key)
        .unwrap_or(0)
}
