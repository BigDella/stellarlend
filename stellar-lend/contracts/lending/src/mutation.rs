use soroban_sdk::{Address, Env, Bytes, Val, Vec};

use crate::borrow;
use crate::deposit;
use crate::flash_loan;
use crate::pause;
use crate::reentrancy::{ReentrancyGuard, ReentrancyKey};
use crate::token_receiver;
use crate::withdraw;
use crate::interest;
use crate::insurance;
use crate::lazy;
use crate::storage;
use crate::views::UserPositionSummary;
use crate::interest_rate;
use crate::risk_monitor;

pub use crate::borrow::{BorrowError, DebtPosition, BorrowCollateral};
pub use crate::deposit::{DepositError, DepositCollateral};
pub use crate::withdraw::WithdrawError;
pub use crate::flash_loan::{FlashLoanError, ManipulationConfig as FlashManipulationConfig};
pub use crate::pause::PauseType;
pub use crate::interest::InterestCacheError;
use crate::borrow::get_admin as get_borrow_admin;
use crate::interest_rate as ir;
use crate::liquidation::{LiquidationError, LiquidationPlan, PositionSnapshot};
use crate::storage::PoolConfig;
use crate::lazy::LazyError;
use crate::insurance::InsuranceError;

pub fn mutate_initialize(
    env: &Env,
    admin: Address,
    debt_ceiling: i128,
    min_borrow_amount: i128,
) -> Result<(), BorrowError> {
    let _guard =
        ReentrancyGuard::new_constructor(env).map_err(|_| BorrowError::ReentrancyDetected)?;
    if get_borrow_admin(env).is_some() {
        return Err(BorrowError::Unauthorized);
    }
    borrow::set_admin(env, &admin);
    borrow::initialize_borrow_settings(env, debt_ceiling, min_borrow_amount)?;
    Ok(())
}

pub fn mutate_borrow(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    collateral_asset: Address,
    collateral_amount: i128,
) -> Result<(), BorrowError> {
    borrow::borrow(env, user, asset, amount, collateral_asset, collateral_amount)
}

pub fn mutate_repay(env: &Env, user: Address, asset: Address, amount: i128) -> Result<(), BorrowError> {
    user.require_auth();
    if pause::is_paused(env, PauseType::Repay) {
        return Err(BorrowError::ProtocolPaused);
    }
    borrow::repay(env, user, asset, amount)
}

pub fn mutate_deposit(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
) -> Result<i128, DepositError> {
    if pause::is_paused(env, PauseType::Deposit) {
        return Err(DepositError::DepositPaused);
    }
    deposit::deposit(env, user, asset, amount)
}

pub fn mutate_deposit_collateral(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
) -> Result<(), BorrowError> {
    user.require_auth();
    if pause::is_paused(env, PauseType::Deposit) {
        return Err(BorrowError::ProtocolPaused);
    }
    borrow::deposit(env, user, asset, amount)
}

pub fn mutate_withdraw(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
) -> Result<i128, WithdrawError> {
    if pause::is_paused(env, PauseType::Withdraw) {
        return Err(WithdrawError::WithdrawPaused);
    }
    withdraw::withdraw(env, user, asset, amount)
}

pub fn mutate_set_pause(
    env: &Env,
    admin: Address,
    pause_type: PauseType,
    paused: bool,
) -> Result<(), BorrowError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| BorrowError::ReentrancyDetected)?;
    let current_admin = get_borrow_admin(env).ok_or(BorrowError::Unauthorized)?;
    if admin != current_admin {
        return Err(BorrowError::Unauthorized);
    }
    admin.require_auth();
    pause::set_pause(env, admin, pause_type, paused);
    Ok(())
}

pub fn mutate_set_oracle(env: &Env, admin: Address, oracle: Address) -> Result<(), BorrowError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| BorrowError::ReentrancyDetected)?;
    borrow::set_oracle(env, &admin, oracle)
}

pub fn mutate_set_liquidation_threshold_bps(
    env: &Env,
    admin: Address,
    bps: i128,
) -> Result<(), BorrowError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| BorrowError::ReentrancyDetected)?;
    borrow::set_liquidation_threshold_bps(env, &admin, bps)
}

pub fn mutate_flash_loan(
    env: &Env,
    receiver: Address,
    asset: Address,
    amount: i128,
    spot_price: i128,
    params: Bytes,
) -> Result<(), FlashLoanError> {
    flash_loan::flash_loan(env, receiver, asset, amount, spot_price, params)
}

pub fn mutate_flash_record_price(env: &Env, asset: Address, price: i128) {
    flash_loan::record_price_sample(env, &asset, price);
}

pub fn mutate_set_flash_manipulation_config(
    env: &Env,
    admin: Address,
    config: FlashManipulationConfig,
) -> Result<(), FlashLoanError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| FlashLoanError::Reentrancy)?;
    let current_admin = get_borrow_admin(env).ok_or(FlashLoanError::Unauthorized)?;
    if admin != current_admin {
        return Err(FlashLoanError::Unauthorized);
    }
    admin.require_auth();
    flash_loan::set_manipulation_config(env, config)
}

pub fn mutate_set_flash_loan_fee_bps(env: &Env, fee_bps: i128) -> Result<(), FlashLoanError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| FlashLoanError::Reentrancy)?;
    let current_admin = get_borrow_admin(env).ok_or(FlashLoanError::Unauthorized)?;
    current_admin.require_auth();
    flash_loan::set_flash_loan_fee_bps(env, fee_bps)
}

pub fn mutate_accrue_interest(env: &Env) -> Result<i128, InterestCacheError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| InterestCacheError::Overflow)?;
    Ok(interest::accrue(env)?.cumulative_index)
}

pub fn mutate_invalidate_interest_cache(
    env: &Env,
    admin: Address,
) -> Result<(), InterestCacheError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| InterestCacheError::Overflow)?;
    if get_borrow_admin(env).as_ref() == Some(&admin) {
        admin.require_auth();
    }
    interest::invalidate(env).map(|_| ())
}

pub fn mutate_plan_liquidation(
    env: &Env,
    snapshot: PositionSnapshot,
    requested_repay_value: i128,
    max_oracle_age_secs: u64,
    est_gas_cost: i128,
) -> Result<LiquidationPlan, LiquidationError> {
    let _guard = ReentrancyGuard::new_read_only(env);
    crate::liquidation::plan_liquidation(
        &snapshot,
        requested_repay_value,
        env.ledger().timestamp(),
        max_oracle_age_secs,
        est_gas_cost,
    )
}

pub fn mutate_migrate_packed_config(
    env: &Env,
    admin: Address,
) -> Result<PoolConfig, crate::storage::PackError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| crate::storage::PackError::BpsFieldOverflow)?;
    if get_borrow_admin(env).as_ref() == Some(&admin) {
        admin.require_auth();
    }
    storage::migrate_from_legacy(env)
}

pub fn mutate_migrate_lazy_fields(env: &Env, admin: Address) -> Result<u32, LazyError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| LazyError::InvalidValue)?;
    if get_borrow_admin(env).as_ref() == Some(&admin) {
        admin.require_auth();
    }
    Ok(lazy::migrate_initialize_all(env))
}

pub fn mutate_insurance_initialize(env: &Env, admin: Address) -> Result<(), InsuranceError> {
    let _guard =
        ReentrancyGuard::new_constructor(env).map_err(|_| InsuranceError::Unauthorized)?;
    insurance::initialize(env, &admin)
}

pub fn mutate_insurance_fund_pool(env: &Env, amount: i128) -> Result<(), InsuranceError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| InsuranceError::Unauthorized)?;
    insurance::fund_pool(env, amount)
}

pub fn mutate_insurance_collect_premium(
    env: &Env,
    payer: Address,
    asset: Address,
    coverage_amount: i128,
) -> Result<i128, InsuranceError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| InsuranceError::Unauthorized)?;
    insurance::collect_premium(env, payer, asset, coverage_amount)
}

pub fn mutate_insurance_submit_claim(
    env: &Env,
    claimant: Address,
    asset: Address,
    amount: i128,
) -> Result<u64, InsuranceError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| InsuranceError::Unauthorized)?;
    insurance::submit_claim(env, claimant, asset, amount)
}

pub fn mutate_insurance_evaluate_claim(
    env: &Env,
    admin: Address,
    claim_id: u64,
    approve: bool,
) -> Result<(), InsuranceError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| InsuranceError::Unauthorized)?;
    insurance::evaluate_claim(env, admin, claim_id, approve)
}

pub fn mutate_insurance_cancel_claim(
    env: &Env,
    claimant: Address,
    claim_id: u64,
) -> Result<(), InsuranceError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| InsuranceError::Unauthorized)?;
    insurance::cancel_claim(env, claimant, claim_id)
}

pub fn mutate_insurance_set_coverage_limit(
    env: &Env,
    admin: Address,
    asset: Address,
    limit_bps: i128,
) -> Result<(), InsuranceError> {
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::GlobalLock, false)
        .map_err(|_| InsuranceError::Unauthorized)?;
    insurance::set_coverage_limit(env, admin, asset, limit_bps)
}

pub fn mutate_sweep_deposit_dust(
    env: &Env,
    user: Address,
    asset: Address,
) -> Result<i128, DepositError> {
    deposit::sweep_dust(env, user, asset)
}

pub fn mutate_sweep_borrow_dust(
    env: &Env,
    user: Address,
    asset: Address,
) -> Result<i128, BorrowError> {
    borrow::sweep_dust(env, user, asset)
}

pub fn mutate_sweep_withdraw_dust(
    env: &Env,
    user: Address,
    asset: Address,
) -> Result<i128, WithdrawError> {
    withdraw::sweep_dust(env, user, asset)
}
