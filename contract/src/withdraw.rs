use crate::reentrancy_guard::{GuardKey, NonReentrant};  // ← ADD
use crate::pause::{self, PauseType};
use soroban_sdk::{contracterror, contracttype, Address, Env, token};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum WithdrawError {
    InvalidAmount = 1,
    WithdrawPaused = 2,
    InsufficientBalance = 3,
    Reentrancy = 4,  // ← ADD
    Overflow = 5,
}

#[contracttype]
#[derive(Clone)]
pub enum WithdrawDataKey {
    UserBalance(Address),
    MinWithdrawal,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct WithdrawPosition {
    pub amount: i128,
    pub last_withdraw_time: u64,
    /// Expiration timestamp (0 = never expire)
    pub expires_at: u64,
}

const WITHDRAW_POSITION_TTL: u64 = 30 * 24 * 3600; // 30 days

pub fn withdraw(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
) -> Result<i128, WithdrawError> {
    // 🛡️ REENTRANCY GUARD
    let _guard = NonReentrant::new(env.clone(), GuardKey::WithdrawGuard)
        .map_err(|_| WithdrawError::Reentrancy)?;

    // ✓ CHECK
    user.require_auth();

    if pause::is_paused(env, PauseType::Withdraw) {
        return Err(WithdrawError::WithdrawPaused);
    }

    if amount <= 0 {
        return Err(WithdrawError::InvalidAmount);
    }

    let mut position = get_withdraw_position(env, &user);
    if position.amount < amount {
        return Err(WithdrawError::InsufficientBalance);
    }

    // ✓ EFFECT - Update state FIRST
    position.amount = position.amount.checked_sub(amount)
        .ok_or(WithdrawError::Overflow)?;
    position.last_withdraw_time = env.ledger().timestamp();
    position.expires_at = env.ledger().timestamp().saturating_add(WITHDRAW_POSITION_TTL);

    save_withdraw_position(env, &user, &position);

    // ✓ INTERACTION - External call LAST
    let token_client = token::Client::new(env, &asset);
    token_client.transfer(&env.current_contract_address(), &user, &amount);

    Ok(position.amount)
}

fn get_withdraw_position(env: &Env, user: &Address) -> WithdrawPosition {
    let now = env.ledger().timestamp();
    if let Some(stored) = env
        .storage()
        .persistent()
        .get::<WithdrawDataKey, WithdrawPosition>(&WithdrawDataKey::UserBalance(user.clone()))
    {
        if stored.expires_at != 0 && stored.expires_at <= now {
            let default = WithdrawPosition {
                amount: 0,
                last_withdraw_time: now,
                expires_at: 0,
            };
            env.storage()
                .persistent()
                .set(&WithdrawDataKey::UserBalance(user.clone()), &default);
            return default;
        }
        stored
    } else {
        WithdrawPosition {
            amount: 0,
            last_withdraw_time: now,
            expires_at: 0,
        }
    }
}

fn save_withdraw_position(env: &Env, user: &Address, position: &WithdrawPosition) {
    env.storage()
        .persistent()
        .set(&WithdrawDataKey::UserBalance(user.clone()), position);
}

pub fn initialize_withdraw_settings(
    env: &Env,
    min_withdrawal: i128,
) -> Result<(), WithdrawError> {
    // Keep global min withdrawal in instance storage for fast access
    env.storage()
        .instance()
        .set(&WithdrawDataKey::MinWithdrawal, &min_withdrawal);
    Ok(())
}

pub fn set_withdraw_paused(env: &Env, paused: bool) -> Result<(), WithdrawError> {
    if paused {
        env.storage()
            .instance()
            .set(&PauseType::Withdraw, &true);
    }
    Ok(())
}

/// Cleanup helper: remove expired withdraw position for a user
pub fn cleanup_expired_withdraw_position(env: &Env, user: Address) {
    let now = env.ledger().timestamp();
    if let Some(stored) = env
        .storage()
        .persistent()
        .get::<WithdrawDataKey, WithdrawPosition>(&WithdrawDataKey::UserBalance(user.clone()))
    {
        if stored.expires_at != 0 && stored.expires_at <= now {
            let default = WithdrawPosition {
                amount: 0,
                last_withdraw_time: now,
                expires_at: 0,
            };
            env.storage()
                .persistent()
                .set(&WithdrawDataKey::UserBalance(user), &default);
        }
    }
}