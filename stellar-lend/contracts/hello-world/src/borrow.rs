//! # Borrow Module
//!
//! Handles asset borrowing operations for the lending protocol.
//!
//! Users can borrow assets against their deposited collateral, subject to:
//! - Minimum collateral ratio requirements (150% default)
//! - Maximum borrow limits based on collateral value
//! - Pause switch checks
//!
//! ## Interest Accrual
//! Interest is accrued on existing debt before any new borrow using the dynamic
//! rate from the `interest_rate` module. The rate is based on protocol utilization
//! following a kink-based piecewise linear model.
//!
//! ## Invariants
//! - A user must have collateral deposited before borrowing.
//! - The collateral ratio must remain at or above the minimum after the borrow.
//! - Borrow amount must not exceed the maximum borrowable given current collateral.

#![allow(unused)]
use soroban_sdk::{contracterror, Address, Env, IntoVal, Map, Symbol, Val, Vec};

use crate::deposit::{
    add_activity_log, emit_analytics_updated_event, emit_position_updated_event,
    emit_user_activity_tracked_event, update_protocol_analytics, update_user_analytics, Activity,
    AssetParams, DepositDataKey, Position, ProtocolAnalytics, UserAnalytics,
};
use crate::events::{emit_borrow, BorrowEvent};

/// Errors that can occur during borrow operations
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BorrowError {
    /// Borrow amount must be greater than zero
    InvalidAmount = 1,
    /// Asset address is invalid
    InvalidAsset = 2,
    /// Insufficient collateral to borrow
    InsufficientCollateral = 3,
    /// Borrow operations are currently paused
    BorrowPaused = 4,
    /// Borrow would violate minimum collateral ratio
    InsufficientCollateralRatio = 5,
    /// Overflow occurred during calculation
    Overflow = 6,
    /// Reentrancy detected
    Reentrancy = 7,
    /// Maximum borrow limit exceeded
    MaxBorrowExceeded = 8,
    /// Asset is not enabled for borrowing
    AssetNotEnabled = 9,
}

// Minimum collateral ratio (in basis points, e.g., 15000 = 150%)
// This is the minimum ratio required: collateral_value / debt_value >= 1.5
// Minimum collateral ratio is now managed by the risk_params module
// const MIN_COLLATERAL_RATIO_BPS: i128 = 15000; // 150% (Legacy)

/// Annual interest rate in basis points (e.g., 500 = 5% per year)
/// This is a simple constant rate model - in production, this would be more sophisticated
// Interest rate is now calculated dynamically based on utilization
// See interest_rate module for details
/// Calculate interest accrued since last accrual time
/// Uses simple interest: interest = principal * rate * time
/// Calculate accrued interest using dynamic interest rate
/// Uses the current borrow rate based on protocol utilization
fn calculate_accrued_interest(
    env: &Env,
    principal: i128,
    last_accrual_time: u64,
    current_time: u64,
) -> Result<i128, BorrowError> {
    if principal == 0 {
        return Ok(0);
    }

    if current_time <= last_accrual_time {
        return Ok(0);
    }

    // Get current borrow rate (in basis points)
    let rate_bps =
        crate::interest_rate::calculate_borrow_rate(env).map_err(|_| BorrowError::Overflow)?;

    // Calculate interest using the dynamic rate
    crate::interest_rate::calculate_accrued_interest(
        principal,
        last_accrual_time,
        current_time,
        rate_bps,
    )
    .map_err(|_| BorrowError::Overflow)
}

/// Accrue compound interest on a position using the global borrow index.
///
/// The index model ensures all users (active and passive) see fair interest
/// distribution without timestamp-manipulation risk.
fn accrue_interest(env: &Env, user: &Address, position: &mut Position) -> Result<(), BorrowError> {
    let current_time = env.ledger().timestamp();

    if position.debt == 0 {
        position.borrow_interest = 0;
        position.last_accrual_time = current_time;
        // Sync user's index even with no debt so subsequent borrows start fresh.
        let current_index = crate::interest_rate::update_lending_index(env)
            .map_err(|_| BorrowError::Overflow)?
            .borrow_index;
        env.storage().persistent().set(
            &DepositDataKey::UserBorrowIndex(user.clone()),
            &current_index,
        );
        return Ok(());
    }

    // Update the global index to the current ledger time (atomic with this tx).
    let current_index = crate::interest_rate::update_lending_index(env)
        .map_err(|_| BorrowError::Overflow)?
        .borrow_index;

    // Retrieve the user's recorded index from their last interaction.
    let user_index = env
        .storage()
        .persistent()
        .get::<DepositDataKey, i128>(&DepositDataKey::UserBorrowIndex(user.clone()))
        .unwrap_or(current_index); // Default: user borrowed at current index (no prior debt).

    // Compound interest = principal * (current_index - user_index) / user_index
    let new_interest =
        crate::interest_rate::compute_index_interest(position.debt, user_index, current_index)
            .map_err(|_| BorrowError::Overflow)?;

    position.borrow_interest = position
        .borrow_interest
        .checked_add(new_interest)
        .ok_or(BorrowError::Overflow)?;

    position.last_accrual_time = current_time;

    // Store the updated index so the next accrual starts from here.
    env.storage().persistent().set(
        &DepositDataKey::UserBorrowIndex(user.clone()),
        &current_index,
    );

    Ok(())
}

/// Calculate collateral ratio
/// Returns (collateral_value * collateral_factor) / (debt + interest) in basis points
/// Returns None if debt is zero (infinite ratio)
fn calculate_collateral_ratio(
    collateral: i128,
    debt: i128,
    interest: i128,
    collateral_factor: i128,
) -> Option<i128> {
    let total_debt = debt.checked_add(interest)?;
    if total_debt == 0 {
        return None; // No debt means infinite ratio
    }

    // collateral_value = collateral * collateral_factor / 10000 (basis points)
    let collateral_value = collateral
        .checked_mul(collateral_factor)?
        .checked_div(10000)?;

    // ratio = (collateral_value * 10000) / total_debt (in basis points)
    collateral_value.checked_mul(10000)?.checked_div(total_debt)
}

/// Calculate maximum borrowable amount based on collateral
/// Returns the maximum amount that can be borrowed while maintaining minimum collateral ratio
fn calculate_max_borrowable(
    collateral: i128,
    current_debt: i128,
    current_interest: i128,
    collateral_factor: i128,
    min_collateral_ratio: i128,
) -> Result<i128, BorrowError> {
    // Calculate collateral value
    let collateral_value = collateral
        .checked_mul(collateral_factor)
        .ok_or(BorrowError::Overflow)?
        .checked_div(10000)
        .ok_or(BorrowError::Overflow)?;

    // Calculate current total debt
    let current_total_debt = current_debt
        .checked_add(current_interest)
        .ok_or(BorrowError::Overflow)?;

    // Maximum debt allowed = collateral_value / (MIN_COLLATERAL_RATIO_BPS / 10000)
    // = collateral_value * 10000 / MIN_COLLATERAL_RATIO_BPS
    let max_debt = collateral_value
        .checked_mul(10000)
        .ok_or(BorrowError::Overflow)?
        .checked_div(min_collateral_ratio)
        .ok_or(BorrowError::Overflow)?;

    // Maximum borrowable = max_debt - current_total_debt
    if max_debt > current_total_debt {
        max_debt
            .checked_sub(current_total_debt)
            .ok_or(BorrowError::Overflow)
    } else {
        Ok(0) // Already at or above max debt
    }
}

/// Validate that borrow would maintain minimum collateral ratio.
///
/// For multi-asset users (those with entries in `UserAssetList`), the collateral
/// value is computed as the oracle-weighted sum across all deposited assets.
/// For single-asset / legacy users the aggregate `CollateralBalance` is used.
fn validate_collateral_ratio_after_borrow(
    env: &Env,
    user: &Address,
    borrow_amount: i128,
    collateral_factor: i128,
) -> Result<(), BorrowError> {
    // Get user position
    let position_key = DepositDataKey::Position(user.clone());
    let position = env
        .storage()
        .persistent()
        .get::<DepositDataKey, Position>(&position_key)
        .ok_or(BorrowError::InsufficientCollateral)?;

    // Determine effective collateral value:
    // - Multi-asset path: oracle-priced sum across all deposited assets
    // - Legacy path: raw aggregate CollateralBalance (collateral_factor applied below)
    let (effective_collateral, apply_factor) =
        if crate::multi_collateral::has_multi_asset_collateral(env, user) {
            let total = crate::multi_collateral::calculate_total_collateral_value(env, user)
                .map_err(|_| BorrowError::Overflow)?;
            // total already has collateral factors applied per asset
            (total, false)
        } else {
            let collateral_key = DepositDataKey::CollateralBalance(user.clone());
            let bal = env
                .storage()
                .persistent()
                .get::<DepositDataKey, i128>(&collateral_key)
                .unwrap_or(0);
            (bal, true)
        };

    if effective_collateral == 0 {
        return Err(BorrowError::InsufficientCollateral);
    }

    // Calculate new debt after borrow
    let new_debt = position
        .debt
        .checked_add(borrow_amount)
        .ok_or(BorrowError::Overflow)?;
    let total_debt = new_debt
        .checked_add(position.borrow_interest)
        .ok_or(BorrowError::Overflow)?;

    if total_debt == 0 {
        return Ok(());
    }

    // Apply collateral factor for legacy single-asset path
    let collateral_value = if apply_factor {
        effective_collateral
            .checked_mul(collateral_factor)
            .ok_or(BorrowError::Overflow)?
            .checked_div(10000)
            .ok_or(BorrowError::Overflow)?
    } else {
        effective_collateral
    };

    // ratio = collateral_value * 10000 / total_debt (in basis points)
    let ratio = collateral_value
        .checked_mul(10000)
        .ok_or(BorrowError::Overflow)?
        .checked_div(total_debt)
        .ok_or(BorrowError::Overflow)?;

    let min_ratio = crate::risk_params::get_min_collateral_ratio(env).unwrap_or(15000);
    if ratio < min_ratio {
        return Err(BorrowError::InsufficientCollateralRatio);
    }

    Ok(())
}

/// Borrow assets from the protocol
pub fn borrow_asset(
    env: &Env,
    user: Address,
    asset: Option<Address>,
    amount: i128,
) -> Result<i128, BorrowError> {
    // Validate amount
    if amount <= 0 {
        return Err(BorrowError::InvalidAmount);
    }

    // Check for reentrancy
    let _guard =
        crate::reentrancy::ReentrancyGuard::new(env).map_err(|_| BorrowError::Reentrancy)?;

    // Check if borrows are paused
    let pause_switches_key = DepositDataKey::PauseSwitches;
    if let Some(pause_map) = env
        .storage()
        .persistent()
        .get::<DepositDataKey, Map<Symbol, bool>>(&pause_switches_key)
    {
        if let Some(paused) = pause_map.get(Symbol::new(env, "pause_borrow")) {
            if paused {
                return Err(BorrowError::BorrowPaused);
            }
        }
    }

    // Get current timestamp
    let timestamp = env.ledger().timestamp();

    // Validate asset if provided
    if let Some(ref asset_addr) = asset {
        // Validate asset address - ensure it's not the contract itself
        if asset_addr == &env.current_contract_address() {
            return Err(BorrowError::InvalidAsset);
        }

        // Check asset parameters
        let asset_params_key = DepositDataKey::AssetParams(asset_addr.clone());
        if let Some(params) = env
            .storage()
            .persistent()
            .get::<DepositDataKey, AssetParams>(&asset_params_key)
        {
            if !params.deposit_enabled {
                return Err(BorrowError::AssetNotEnabled);
            }
        }
    }

    // Get user position
    let position_key = DepositDataKey::Position(user.clone());
    #[allow(clippy::unnecessary_lazy_evaluations)]
    let mut position = env
        .storage()
        .persistent()
        .get::<DepositDataKey, Position>(&position_key)
        .unwrap_or_else(|| Position {
            collateral: 0,
            debt: 0,
            borrow_interest: 0,
            last_accrual_time: timestamp,
        });

    // Accrue compound interest on existing debt before borrowing
    accrue_interest(env, &user, &mut position)?;

    // Get effective collateral for borrowing capacity:
    // Multi-asset users: oracle-priced aggregate (collateral factors already applied)
    // Legacy users: raw aggregate CollateralBalance
    let (current_collateral, use_raw_factor) =
        if crate::multi_collateral::has_multi_asset_collateral(env, &user) {
            let total = crate::multi_collateral::calculate_total_collateral_value(env, &user)
                .map_err(|_| BorrowError::Overflow)?;
            (total, false)
        } else {
            let collateral_key = DepositDataKey::CollateralBalance(user.clone());
            let bal = env
                .storage()
                .persistent()
                .get::<DepositDataKey, i128>(&collateral_key)
                .unwrap_or(0);
            (bal, true)
        };

    // Check if user has collateral
    if current_collateral == 0 {
        return Err(BorrowError::InsufficientCollateral);
    }

    // Get asset parameters for collateral factor
    let collateral_factor = if let Some(asset_addr) = asset.as_ref() {
        let asset_params_key = DepositDataKey::AssetParams(asset_addr.clone());
        if let Some(params) = env
            .storage()
            .persistent()
            .get::<DepositDataKey, AssetParams>(&asset_params_key)
        {
            params.collateral_factor
        } else {
            10000
        }
    } else {
        10000
    };

    // Get borrow fee bps if provided
    let borrow_fee_bps = if let Some(asset_addr) = asset.as_ref() {
        let asset_params_key = DepositDataKey::AssetParams(asset_addr.clone());
        if let Some(params) = env
            .storage()
            .persistent()
            .get::<DepositDataKey, AssetParams>(&asset_params_key)
        {
            params.borrow_fee_bps
        } else {
            0
        }
    } else {
        0
    };

    // Get minimum collateral ratio from risk params
    let min_ratio = crate::risk_params::get_min_collateral_ratio(env).unwrap_or(15000);

    // For multi-asset users collateral value is already oracle-weighted;
    // pass 10000 (identity) so calculate_max_borrowable skips the factor step.
    let effective_factor = if use_raw_factor {
        collateral_factor
    } else {
        10000
    };

    // Calculate maximum borrowable amount
    let max_borrowable = calculate_max_borrowable(
        current_collateral,
        position.debt,
        position.borrow_interest,
        effective_factor,
        min_ratio,
    )?;

    // Check if borrow amount exceeds maximum
    if amount > max_borrowable {
        return Err(BorrowError::MaxBorrowExceeded);
    }

    // Validate collateral ratio after borrow
    validate_collateral_ratio_after_borrow(env, &user, amount, collateral_factor)?;

    // Calculate new debt
    let new_debt = position
        .debt
        .checked_add(amount)
        .ok_or(BorrowError::Overflow)?;

    // Calculate borrow fee
    let fee_amount = amount
        .checked_mul(borrow_fee_bps)
        .ok_or(BorrowError::Overflow)?
        .checked_div(10000)
        .ok_or(BorrowError::Overflow)?;

    // Amount user actually receives
    let receive_amount = amount
        .checked_sub(fee_amount)
        .ok_or(BorrowError::Overflow)?;

    if receive_amount <= 0 {
        return Err(BorrowError::InvalidAmount);
    }

    // Update position
    position.debt = new_debt;
    position.last_accrual_time = timestamp;
    env.storage().persistent().set(&position_key, &position);

    // Handle asset transfer - contract sends tokens to user
    if let Some(ref asset_addr) = asset {
        // Skip actual token transfers in unit tests to avoid Storage error with non-existent contracts
        #[cfg(not(test))]
        {
            let token_client = soroban_sdk::token::Client::new(env, asset_addr);

            // Check contract balance
            let contract_balance = token_client.balance(&env.current_contract_address());
            if contract_balance < amount {
                return Err(BorrowError::InsufficientCollateral);
            }

            token_client.transfer(&env.current_contract_address(), &user, &receive_amount);
        }

        // Credit fee to protocol reserve
        if fee_amount > 0 {
            let reserve_key = DepositDataKey::ProtocolReserve(asset.clone());
            let current_reserve = env
                .storage()
                .persistent()
                .get::<DepositDataKey, i128>(&reserve_key)
                .unwrap_or(0);
            env.storage().persistent().set(
                &reserve_key,
                &(current_reserve
                    .checked_add(fee_amount)
                    .ok_or(BorrowError::Overflow)?),
            );
        }
    }

    // Update user analytics
    update_user_analytics_borrow(env, &user, amount, timestamp)?;

    // Update protocol analytics
    update_protocol_analytics_borrow(env, amount)?;

    // Add to activity log
    add_activity_log(
        env,
        &user,
        Symbol::new(env, "borrow"),
        amount,
        asset.clone(),
        timestamp,
    )
    .map_err(|e| match e {
        crate::deposit::DepositError::Overflow => BorrowError::Overflow,
        _ => BorrowError::Overflow,
    })?;

    // Emit borrow event
    emit_borrow(
        env,
        BorrowEvent {
            user: user.clone(),
            asset: asset.clone(),
            amount,
            timestamp,
        },
    );

    // Emit position updated event
    emit_position_updated_event(env, &user, &position);
    emit_analytics_updated_event(env, &user, "borrow", amount, timestamp);
    emit_user_activity_tracked_event(env, &user, Symbol::new(env, "borrow"), amount, timestamp);

    // Return total debt
    let total_debt = position
        .debt
        .checked_add(position.borrow_interest)
        .ok_or(BorrowError::Overflow)?;
    Ok(total_debt)
}

/// Update user analytics after borrow
fn update_user_analytics_borrow(
    env: &Env,
    user: &Address,
    amount: i128,
    timestamp: u64,
) -> Result<(), BorrowError> {
    let analytics_key = DepositDataKey::UserAnalytics(user.clone());
    #[allow(clippy::unnecessary_lazy_evaluations)]
    let mut analytics = env
        .storage()
        .persistent()
        .get::<DepositDataKey, UserAnalytics>(&analytics_key)
        .unwrap_or_else(|| UserAnalytics {
            total_deposits: 0,
            total_borrows: 0,
            total_withdrawals: 0,
            total_repayments: 0,
            collateral_value: 0,
            debt_value: 0,
            collateralization_ratio: 0,
            activity_score: 0,
            transaction_count: 0,
            first_interaction: timestamp,
            last_activity: timestamp,
            risk_level: 0,
            loyalty_tier: 0,
        });

    analytics.total_borrows = analytics
        .total_borrows
        .checked_add(amount)
        .ok_or(BorrowError::Overflow)?;
    analytics.debt_value = analytics
        .debt_value
        .checked_add(amount)
        .ok_or(BorrowError::Overflow)?;

    if analytics.debt_value > 0 && analytics.collateral_value > 0 {
        analytics.collateralization_ratio = analytics
            .collateral_value
            .checked_mul(10000)
            .and_then(|v| v.checked_div(analytics.debt_value))
            .unwrap_or(0);
    } else {
        analytics.collateralization_ratio = 0;
    }

    analytics.transaction_count = analytics.transaction_count.saturating_add(1);
    analytics.last_activity = timestamp;

    env.storage().persistent().set(&analytics_key, &analytics);
    Ok(())
}

/// Update protocol analytics after borrow
fn update_protocol_analytics_borrow(env: &Env, amount: i128) -> Result<(), BorrowError> {
    let analytics_key = DepositDataKey::ProtocolAnalytics;
    let mut analytics = env
        .storage()
        .persistent()
        .get::<DepositDataKey, ProtocolAnalytics>(&analytics_key)
        .unwrap_or(ProtocolAnalytics {
            total_deposits: 0,
            total_borrows: 0,
            total_value_locked: 0,
        });

    analytics.total_borrows = analytics
        .total_borrows
        .checked_add(amount)
        .ok_or(BorrowError::Overflow)?;
    env.storage().persistent().set(&analytics_key, &analytics);
    Ok(())
}
