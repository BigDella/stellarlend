pub use crate::events::VaultDepositEvent;

/// Backward-compatible name for vault deposit events (see [`VaultDepositEvent`]).
#[allow(dead_code)]
pub type DepositEvent = VaultDepositEvent;

use crate::pause::{self, PauseType};
use crate::reentrancy::{ReentrancyError, ReentrancyGuard, ReentrancyKey};
use crate::rounding;
use soroban_sdk::{contracterror, contracttype, Address, Env};

/// Errors that can occur during deposit operations
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum DepositError {
    InvalidAmount = 1,
    DepositPaused = 2,
    Overflow = 3,
    AssetNotSupported = 4,
    ExceedsDepositCap = 5,
    Unauthorized = 6,
    ReentrancyDetected = 7,
    AmountBelowMinimum = 8,
    NoDustToSweep = 9,
}

/// Storage keys for deposit-related data
#[contracttype]
#[derive(Clone)]
#[allow(clippy::enum_variant_names)]
pub enum DepositDataKey {
    UserCollateral(Address),
    TotalAmount,
    CapAmount,
    MinAmount,
    DustAmount(Address),
}

// ─── Constants ───────────────────────────────────────────────────────────────

/// Minimum transaction amount (1 unit of asset)
const MIN_TRANSACTION_AMOUNT: i128 = 1;

/// Dust threshold (same as minimum transaction amount)
const DUST_THRESHOLD: i128 = MIN_TRANSACTION_AMOUNT;

/// User deposit position
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct DepositCollateral {
    pub amount: i128,
    pub asset: Address,
    pub last_deposit_time: u64,
}

/// Deposit collateral into the protocol
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - The depositor's address
/// * `asset` - The collateral asset address
/// * `amount` - The amount to deposit
///
/// # Returns
/// Returns the updated collateral balance on success
pub fn deposit(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
) -> Result<i128, DepositError> {
    deposit_with_auth(env, user, asset, amount, true)
}

pub(crate) fn deposit_with_auth(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    require_auth: bool,
) -> Result<i128, DepositError> {
    // CHECKS-EFFECTS-INTERACTIONS PATTERN
    // 1. CHECKS: Reentrancy guard, authorization, pause state, validation
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::DepositLock, false)
        .map_err(|_| DepositError::ReentrancyDetected)?;

    if require_auth {
        user.require_auth();
    }

    if pause::is_paused(env, PauseType::Deposit) {
        return Err(DepositError::DepositPaused);
    }

    if amount <= 0 {
        return Err(DepositError::InvalidAmount);
    }

    // Gas-efficient dust check (early return)
    if amount < DUST_THRESHOLD {
        return Err(DepositError::AmountBelowMinimum);
    }

    let min_deposit = get_min_deposit_amount(env);
    if amount < min_deposit {
        return Err(DepositError::AmountBelowMinimum);
    }

    let total_deposits = get_total_deposits(env);
    let deposit_cap = get_deposit_cap(env);
    let new_total = total_deposits
        .checked_add(amount)
        .ok_or(DepositError::Overflow)?;

    if new_total > deposit_cap {
        return Err(DepositError::ExceedsDepositCap);
    }

    // 2. EFFECTS: Update state before any external interactions
    let mut position = get_deposit_position(env, &user, &asset);
    position.amount = position
        .amount
        .checked_add(amount)
        .ok_or(DepositError::Overflow)?;
    position.last_deposit_time = env.ledger().timestamp();
    position.asset = asset.clone();

    save_deposit_position(env, &user, &position);
    set_total_deposits(env, new_total);

    // 3. INTERACTIONS: Emit events (no external calls in deposit)
    emit_deposit_event(env, user, asset, amount, position.amount);

    Ok(position.amount)
}

/// Initialize deposit settings
pub fn initialize_deposit_settings(
    env: &Env,
    deposit_cap: i128,
    min_deposit_amount: i128,
) -> Result<(), DepositError> {
    env.storage()
        .persistent()
        .set(&DepositDataKey::CapAmount, &deposit_cap);
    env.storage()
        .persistent()
        .set(&DepositDataKey::MinAmount, &min_deposit_amount);
    Ok(())
}

pub fn get_user_collateral(env: &Env, user: &Address, asset: &Address) -> DepositCollateral {
    get_deposit_position(env, user, asset)
}

fn get_deposit_position(env: &Env, user: &Address, asset: &Address) -> DepositCollateral {
    env.storage()
        .persistent()
        .get(&DepositDataKey::UserCollateral(user.clone()))
        .unwrap_or(DepositCollateral {
            amount: 0,
            asset: asset.clone(),
            last_deposit_time: env.ledger().timestamp(),
        })
}

fn save_deposit_position(env: &Env, user: &Address, position: &DepositCollateral) {
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

fn get_deposit_cap(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&DepositDataKey::CapAmount)
        .unwrap_or(i128::MAX)
}

fn get_min_deposit_amount(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&DepositDataKey::MinAmount)
        .unwrap_or(0)
}

/// Sweep dust amounts from user's deposit position
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - The user's address
/// * `asset` - The asset address
///
/// # Returns
/// Returns the dust amount swept on success
pub fn sweep_dust(env: &Env, user: Address, asset: Address) -> Result<i128, DepositError> {
    // CHECKS-EFFECTS-INTERACTIONS PATTERN
    // 1. CHECKS: Reentrancy guard, authorization
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::DepositLock, false)
        .map_err(|_| DepositError::ReentrancyDetected)?;

    user.require_auth();

    // Get dust amount
    let dust_amount = get_dust_amount(env, &user, &asset);
    if dust_amount < DUST_THRESHOLD {
        return Err(DepositError::NoDustToSweep);
    }

    let mut position = get_deposit_position(env, &user, &asset);

    // 2. EFFECTS: Update state before any external interactions
    // Remove dust from position
    position.amount = position
        .amount
        .checked_sub(dust_amount)
        .ok_or(DepositError::Overflow)?;
    save_deposit_position(env, &user, &position);

    // Clear dust tracking
    clear_dust(env, &user, &asset);

    // Update total deposits
    let total_deposits = get_total_deposits(env);
    let new_total = total_deposits
        .checked_sub(dust_amount)
        .ok_or(DepositError::Overflow)?;
    set_total_deposits(env, new_total);

    // 3. INTERACTIONS: Transfer dust to user
    let token_client = token::Client::new(env, &asset);
    token_client.transfer(&env.current_contract_address(), &user, &dust_amount);

    Ok(dust_amount)
}

/// Get dust amount for a user's position
fn get_dust_amount(env: &Env, user: &Address, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DepositDataKey::DustAmount(user.clone()))
        .unwrap_or(0)
}

/// Set dust amount for a user's position
fn set_dust_amount(env: &Env, user: &Address, dust: i128) {
    env.storage()
        .persistent()
        .set(&DepositDataKey::DustAmount(user.clone()), &dust);
}

/// Clear dust tracking for a user's position
fn clear_dust(env: &Env, user: &Address, asset: &Address) {
    env.storage()
        .persistent()
        .remove(&DepositDataKey::DustAmount(user.clone()));
}

/// Track dust accumulation during operations
pub fn track_dust(env: &Env, user: &Address, asset: &Address, dust: i128) {
    if dust > 0 && dust < DUST_THRESHOLD {
        let current_dust = get_dust_amount(env, user, asset);
        let new_dust = current_dust.checked_add(dust).unwrap_or(current_dust);
        set_dust_amount(env, user, new_dust);
    }
}

fn emit_deposit_event(env: &Env, user: Address, asset: Address, amount: i128, new_balance: i128) {
    VaultDepositEvent {
        user,
        asset,
        amount,
        new_balance,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}
