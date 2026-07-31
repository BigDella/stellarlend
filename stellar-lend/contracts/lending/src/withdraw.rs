use soroban_sdk::{contracterror, contracttype, Address, Env};

use crate::deposit::{DepositCollateral, DepositDataKey};
use crate::reentrancy::{ReentrancyGuard, ReentrancyKey};
use crate::rounding;

pub use crate::events::WithdrawEvent;

/// Errors that can occur during withdraw operations
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum WithdrawError {
    InvalidAmount = 1,
    WithdrawPaused = 2,
    Overflow = 3,
    InsufficientCollateral = 4,
    InsufficientCollateralRatio = 5,
    Unauthorized = 6,
    ReentrancyDetected = 7,
    AmountBelowMinimum = 8,
    NoDustToSweep = 9,
}

/// Storage keys for withdraw-related data
#[contracttype]
#[derive(Clone)]
pub enum WithdrawDataKey {
    Paused,
    MinWithdrawAmount,
    DustAmount(Address),
}

// ─── Constants ───────────────────────────────────────────────────────────────

/// Minimum transaction amount (1 unit of asset)
const MIN_TRANSACTION_AMOUNT: i128 = 1;

/// Dust threshold (same as minimum transaction amount)
const DUST_THRESHOLD: i128 = MIN_TRANSACTION_AMOUNT;

/// Minimum collateral ratio in basis points (150%)
const MIN_COLLATERAL_RATIO_BPS: i128 = 15000;

/// Withdraw collateral from the protocol
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - The withdrawer's address
/// * `asset` - The collateral asset address
/// * `amount` - The amount to withdraw
///
/// # Returns
/// Returns the remaining collateral balance on success
pub fn withdraw(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
) -> Result<i128, WithdrawError> {
    withdraw_with_auth(env, user, asset, amount, true)
}

pub(crate) fn withdraw_with_auth(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    require_auth: bool,
) -> Result<i128, WithdrawError> {
    // CHECKS-EFFECTS-INTERACTIONS PATTERN
    // 1. CHECKS: Reentrancy guard, authorization, pause state, validation
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::WithdrawLock, false)
        .map_err(|_| WithdrawError::ReentrancyDetected)?;

    if require_auth {
        user.require_auth();
    }

    if is_paused(env) || crate::pause::is_paused(env, crate::pause::PauseType::Withdraw) {
        return Err(WithdrawError::WithdrawPaused);
    }

    if amount <= 0 {
        return Err(WithdrawError::InvalidAmount);
    }

    // Gas-efficient dust check (early return)
    if amount < DUST_THRESHOLD {
        return Err(WithdrawError::AmountBelowMinimum);
    }

    let min_withdraw = get_min_withdraw_amount(env);
    if amount < min_withdraw {
        return Err(WithdrawError::AmountBelowMinimum);
    }

    let position = get_collateral_position(env, &user, &asset);

    if position.amount < amount {
        return Err(WithdrawError::InsufficientCollateral);
    }

    let new_amount = position
        .amount
        .checked_sub(amount)
        .ok_or(WithdrawError::Overflow)?;

    validate_collateral_ratio_after_withdraw(env, &user, new_amount)?;

    // 2. EFFECTS: Update state before any external interactions
    let updated_position = DepositCollateral {
        amount: new_amount,
        asset: asset.clone(),
        last_deposit_time: position.last_deposit_time,
    };

    save_collateral_position(env, &user, &updated_position);

    let total_deposits = get_total_deposits(env);
    let new_total = total_deposits.checked_sub(amount).unwrap_or(0);
    set_total_deposits(env, new_total);

    // 3. INTERACTIONS: Emit events (no external calls in withdraw)
    WithdrawEvent {
        user,
        asset,
        amount,
        remaining_balance: new_amount,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(new_amount)
}

/// Validate collateral ratio remains above minimum after withdrawal
fn validate_collateral_ratio_after_withdraw(
    env: &Env,
    user: &Address,
    remaining_collateral: i128,
) -> Result<(), WithdrawError> {
    use crate::borrow::{BorrowDataKey, DebtPosition};

    let debt_position: Option<DebtPosition> = env
        .storage()
        .persistent()
        .get(&BorrowDataKey::BorrowUserDebt(user.clone()));

    if let Some(debt) = debt_position {
        let total_debt = debt
            .borrowed_amount
            .checked_add(debt.interest_accrued)
            .ok_or(WithdrawError::Overflow)?;

        if total_debt > 0 {
            let min_collateral = total_debt
                .checked_mul(MIN_COLLATERAL_RATIO_BPS)
                .ok_or(WithdrawError::Overflow)?
                .checked_div(10000)
                .ok_or(WithdrawError::Overflow)?;

            if remaining_collateral < min_collateral {
                return Err(WithdrawError::InsufficientCollateralRatio);
            }
        }
    }

    Ok(())
}

/// Initialize withdraw settings
pub fn initialize_withdraw_settings(
    env: &Env,
    min_withdraw_amount: i128,
) -> Result<(), WithdrawError> {
    env.storage()
        .persistent()
        .set(&WithdrawDataKey::MinWithdrawAmount, &min_withdraw_amount);
    env.storage()
        .persistent()
        .set(&WithdrawDataKey::Paused, &false);
    Ok(())
}

/// Set withdraw pause state
pub fn set_withdraw_paused(env: &Env, paused: bool) -> Result<(), WithdrawError> {
    env.storage()
        .persistent()
        .set(&WithdrawDataKey::Paused, &paused);
    Ok(())
}

fn get_collateral_position(env: &Env, user: &Address, asset: &Address) -> DepositCollateral {
    env.storage()
        .persistent()
        .get(&DepositDataKey::UserCollateral(user.clone()))
        .unwrap_or(DepositCollateral {
            amount: 0,
            asset: asset.clone(),
            last_deposit_time: env.ledger().timestamp(),
        })
}

fn save_collateral_position(env: &Env, user: &Address, position: &DepositCollateral) {
    env.storage()
        .persistent()
        .set(&DepositDataKey::UserCollateral(user.clone()), position);
}

fn get_total_deposits(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&DepositDataKey::TotalAmount)
        .unwrap_or(0)
}

fn set_total_deposits(env: &Env, amount: i128) {
    env.storage()
        .persistent()
        .set(&DepositDataKey::TotalAmount, &amount);
}

fn get_min_withdraw_amount(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&WithdrawDataKey::MinWithdrawAmount)
        .unwrap_or(0)
}

/// Sweep dust amounts from user's withdraw position
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - The user's address
/// * `asset` - The asset address
///
/// # Returns
/// Returns the dust amount swept on success
pub fn sweep_dust(env: &Env, user: Address, asset: Address) -> Result<i128, WithdrawError> {
    // CHECKS-EFFECTS-INTERACTIONS PATTERN
    // 1. CHECKS: Reentrancy guard, authorization
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::WithdrawLock, false)
        .map_err(|_| WithdrawError::ReentrancyDetected)?;

    user.require_auth();

    // Get dust amount
    let dust_amount = get_dust_amount(env, &user, &asset);
    if dust_amount < DUST_THRESHOLD {
        return Err(WithdrawError::NoDustToSweep);
    }

    let position = get_collateral_position(env, &user, &asset);

    // 2. EFFECTS: Update state before any external interactions
    // Remove dust from position
    let new_amount = position
        .amount
        .checked_sub(dust_amount)
        .ok_or(WithdrawError::Overflow)?;

    let updated_position = DepositCollateral {
        amount: new_amount,
        asset: asset.clone(),
        last_deposit_time: position.last_deposit_time,
    };

    save_collateral_position(env, &user, &updated_position);

    // Clear dust tracking
    clear_dust(env, &user, &asset);

    // Update total deposits
    let total_deposits = get_total_deposits(env);
    let new_total = total_deposits
        .checked_sub(dust_amount)
        .ok_or(WithdrawError::Overflow)?;
    set_total_deposits(env, new_total);

    // 3. INTERACTIONS: Transfer dust to user
    let token_client = crate::token::Client::new(env, &asset);
    token_client.transfer(&env.current_contract_address(), &user, &dust_amount);

    Ok(dust_amount)
}

/// Get dust amount for a user's position
fn get_dust_amount(env: &Env, user: &Address, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&WithdrawDataKey::DustAmount(user.clone()))
        .unwrap_or(0)
}

/// Set dust amount for a user's position
fn set_dust_amount(env: &Env, user: &Address, asset: &Address, dust: i128) {
    env.storage()
        .persistent()
        .set(&WithdrawDataKey::DustAmount(user.clone()), &dust);
}

/// Clear dust tracking for a user's position
fn clear_dust(env: &Env, user: &Address, asset: &Address) {
    env.storage()
        .persistent()
        .remove(&WithdrawDataKey::DustAmount(user.clone()));
}

/// Track dust accumulation during operations
pub fn track_dust(env: &Env, user: &Address, asset: &Address, dust: i128) {
    if dust > 0 && dust < DUST_THRESHOLD {
        let current_dust = get_dust_amount(env, user, asset);
        let new_dust = current_dust.checked_add(dust).unwrap_or(current_dust);
        set_dust_amount(env, user, asset, new_dust);
    }
}

fn is_paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&WithdrawDataKey::Paused)
        .unwrap_or(false)
}
