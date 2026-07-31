use soroban_sdk::{contracterror, contracttype, Address, Env};

use crate::deposit::{DepositCollateral, DepositDataKey};
use crate::dust::is_dust_amount;

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
    DustAmount = 7,
}

/// Storage keys for withdraw-related data
#[contracttype]
#[derive(Clone)]
pub enum WithdrawDataKey {
    Paused,
    MinWithdrawAmount,
}

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
    if require_auth {
        user.require_auth();
    }

    if is_paused(env) || crate::pause::is_paused(env, crate::pause::PauseType::Withdraw) {
        return Err(WithdrawError::WithdrawPaused);
    }

    if amount <= 0 {
        return Err(WithdrawError::InvalidAmount);
    }

    let min_withdraw = get_min_withdraw_amount(env);
    if amount < min_withdraw {
        return Err(WithdrawError::InvalidAmount);
    }

    let position = get_collateral_position(env, &user, &asset);

    if position.amount < amount {
        return Err(WithdrawError::InsufficientCollateral);
    }

    let new_amount = position
        .amount
        .checked_sub(amount)
        .ok_or(WithdrawError::Overflow)?;
    if is_dust_amount(new_amount, min_withdraw) {
        return Err(WithdrawError::DustAmount);
    }

    validate_collateral_ratio_after_withdraw(env, &user, new_amount)?;

    let updated_position = DepositCollateral {
        amount: new_amount,
        asset: asset.clone(),
        last_deposit_time: position.last_deposit_time,
    };

    save_collateral_position(env, &user, &updated_position);

    let total_deposits = get_total_deposits(env);
    let new_total = total_deposits.checked_sub(amount).unwrap_or(0);
    set_total_deposits(env, new_total);

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
    if min_withdraw_amount <= 0 {
        return Err(WithdrawError::InvalidAmount);
    }

    env.storage()
        .persistent()
        .set(&WithdrawDataKey::MinWithdrawAmount, &min_withdraw_amount);
    env.storage()
        .persistent()
        .set(&WithdrawDataKey::Paused, &false);
    Ok(())
}

pub fn sweep_deposit_dust(env: &Env, user: Address, asset: Address) -> Result<i128, WithdrawError> {
    user.require_auth();

    if is_paused(env) || crate::pause::is_paused(env, crate::pause::PauseType::Withdraw) {
        return Err(WithdrawError::WithdrawPaused);
    }

    let min_withdraw = get_min_withdraw_amount(env);
    let position = get_collateral_position(env, &user, &asset);
    if !is_dust_amount(position.amount, min_withdraw) {
        return Err(WithdrawError::DustAmount);
    }

    let updated_position = DepositCollateral {
        amount: 0,
        asset: asset.clone(),
        last_deposit_time: position.last_deposit_time,
    };
    save_collateral_position(env, &user, &updated_position);

    let total_deposits = get_total_deposits(env);
    let new_total = total_deposits
        .checked_sub(position.amount)
        .ok_or(WithdrawError::Overflow)?;
    set_total_deposits(env, new_total);

    WithdrawEvent {
        user,
        asset,
        amount: position.amount,
        remaining_balance: 0,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(position.amount)
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

fn is_paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&WithdrawDataKey::Paused)
        .unwrap_or(false)
}
