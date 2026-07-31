use soroban_sdk::{contracterror, Address, Env, Vec};
use crate::{
    errors::LendingError,
    deposit::DepositDataKey,
    types::{EmergencyTrigger, EmergencyState, EmergencyWithdrawal},
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EmergencyWithdrawalError {
    NotActive = 1,
    AlreadyActive = 2,
    WindowNotOpen = 3,
    NotAuthorized = 4,
    InsufficientBalance = 5,
    ExceedsWithdrawalCap = 6,
    InvalidParameter = 7,
    AlreadyWithdrawn = 8,
}

const EMERGENCY_WINDOW_DELAY: u64 = 48 * 3600; // 48 hours
const EMERGENCY_WINDOW_DURATION: u64 = 7 * 24 * 3600; // 7 days
const DEFAULT_WITHDRAWAL_CAP_BPS: i128 = 3000; // 30%

pub fn initialize_emergency_withdrawal(env: &Env) {
    let default_state = EmergencyState {
        is_active: false,
        trigger: EmergencyTrigger::AdminEmergency,
        started_at: 0,
        window_opens_at: 0,
        window_closes_at: 0,
        withdrawal_cap_bps: DEFAULT_WITHDRAWAL_CAP_BPS,
        total_withdrawn_this_window: 0,
        bad_debt: 0,
    };
    env.storage().persistent().set(&DepositDataKey::EmergencyState, &default_state);
}

pub fn trigger_emergency(
    env: &Env,
    caller: Address,
    trigger: EmergencyTrigger,
    withdrawal_cap_bps: Option<i128>,
    bad_debt: Option<i128>,
) -> Result<(), EmergencyWithdrawalError> {
    // Check authorization: admin or governance
    crate::risk_management::require_admin(env, &caller).map_err(|_| EmergencyWithdrawalError::NotAuthorized)?;

    let mut state = get_emergency_state(env);
    if state.is_active {
        return Err(EmergencyWithdrawalError::AlreadyActive);
    }

    let now = env.ledger().timestamp();
    state.is_active = true;
    state.trigger = trigger;
    state.started_at = now;
    state.window_opens_at = now + EMERGENCY_WINDOW_DELAY;
    state.window_closes_at = now + EMERGENCY_WINDOW_DELAY + EMERGENCY_WINDOW_DURATION;
    state.withdrawal_cap_bps = withdrawal_cap_bps.unwrap_or(DEFAULT_WITHDRAWAL_CAP_BPS);
    state.bad_debt = bad_debt.unwrap_or(0);
    state.total_withdrawn_this_window = 0;

    save_emergency_state(env, &state);

    // Emit event
    crate::events::emit_emergency_triggered(env, state.clone());

    Ok(())
}

pub fn cancel_emergency(
    env: &Env,
    caller: Address,
) -> Result<(), EmergencyWithdrawalError> {
    crate::risk_management::require_admin(env, &caller).map_err(|_| EmergencyWithdrawalError::NotAuthorized)?;

    let mut state = get_emergency_state(env);
    if !state.is_active {
        return Err(EmergencyWithdrawalError::NotActive);
    }

    // Can only cancel before window opens
    let now = env.ledger().timestamp();
    if now >= state.window_opens_at {
        return Err(EmergencyWithdrawalError::WindowNotOpen);
    }

    state.is_active = false;
    save_emergency_state(env, &state);

    crate::events::emit_emergency_cancelled(env);

    Ok(())
}

pub fn get_emergency_state(env: &Env) -> EmergencyState {
    env.storage()
        .persistent()
        .get(&DepositDataKey::EmergencyState)
        .unwrap_or(EmergencyState {
            is_active: false,
            trigger: EmergencyTrigger::AdminEmergency,
            started_at: 0,
            window_opens_at: 0,
            window_closes_at: 0,
            withdrawal_cap_bps: DEFAULT_WITHDRAWAL_CAP_BPS,
            total_withdrawn_this_window: 0,
            bad_debt: 0,
        })
}

fn save_emergency_state(env: &Env, state: &EmergencyState) {
    env.storage()
        .persistent()
        .set(&DepositDataKey::EmergencyState, state);
}

pub fn emergency_withdraw(
    env: &Env,
    user: Address,
    asset: Option<Address>,
    amount: i128,
) -> Result<i128, LendingError> {
    user.require_auth();

    let state = get_emergency_state(env);
    if !state.is_active {
        return Err(EmergencyWithdrawalError::NotActive.into());
    }

    let now = env.ledger().timestamp();
    if now < state.window_opens_at || now > state.window_closes_at {
        return Err(EmergencyWithdrawalError::WindowNotOpen.into());
    }

    if amount <= 0 {
        return Err(EmergencyWithdrawalError::InvalidParameter.into());
    }

    // Get user's deposit balance
    let user_balance = crate::deposit::get_user_balance(env, &user, asset.clone());
    if user_balance < amount {
        return Err(EmergencyWithdrawalError::InsufficientBalance.into());
    }

    // Calculate loss share (if any bad debt)
    let total_supply = crate::deposit::get_total_supply(env, asset.clone());
    let loss_share_bps = if total_supply > 0 && state.bad_debt > 0 {
        (state.bad_debt * 10000) / total_supply
    } else {
        0
    };

    let amount_after_loss = amount - (amount * loss_share_bps / 10000);

    // Check withdrawal cap
    let tvl = crate::deposit::get_total_supply(env, asset.clone());
    let cap_amount = (tvl * state.withdrawal_cap_bps) / 10000;
    if state.total_withdrawn_this_window + amount > cap_amount {
        return Err(EmergencyWithdrawalError::ExceedsWithdrawalCap.into());
    }

    // Update user balance
    crate::deposit::update_user_balance(env, &user, asset.clone(), user_balance - amount)?;

    // Update emergency state
    let mut new_state = state.clone();
    new_state.total_withdrawn_this_window += amount;
    save_emergency_state(env, &new_state);

    // Transfer tokens to user
    let token_address = asset.clone().unwrap_or_else(|| env.current_contract_address());
    let token_client = soroban_sdk::token::Client::new(env, &token_address);
    token_client.transfer(&env.current_contract_address(), &user, &amount_after_loss);

    // Record withdrawal
    let withdrawal = EmergencyWithdrawal {
        user: user.clone(),
        asset: asset.clone(),
        amount: amount_after_loss,
        withdrawn_at: now,
        loss_share_bps,
    };
    record_emergency_withdrawal(env, withdrawal.clone());

    // Emit event
    crate::events::emit_emergency_withdrawal(env, withdrawal);

    Ok(amount_after_loss)
}

fn record_emergency_withdrawal(env: &Env, withdrawal: EmergencyWithdrawal) {
    let key = DepositDataKey::EmergencyWithdrawal(withdrawal.user.clone(), withdrawal.withdrawn_at);
    env.storage().persistent().set(&key, &withdrawal);
}

pub fn get_user_emergency_withdrawals(env: &Env, user: Address) -> Vec<EmergencyWithdrawal> {
    // For simplicity, return empty vec for now - in real implementation, we'd track all withdrawals
    Vec::new(env)
}
