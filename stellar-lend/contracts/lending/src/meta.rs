use soroban_sdk::{contracterror, contracttype, Address, Env, IntoVal, Symbol, Vec};

use crate::{borrow, deposit, pause::PauseType, withdraw};
use stellarlend_shared_deadline::require_deadline;

const BPS_DENOMINATOR: i128 = 10_000;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MetaTxError {
    Unauthorized = 1,
    Expired = 2,
    InvalidNonce = 3,
    DelegationMissing = 4,
    DelegationExpired = 5,
    PermissionDenied = 6,
    InvalidCapConfig = 7,
    UserSupplyCapExceeded = 8,
    UserBorrowCapExceeded = 9,
    PoolSupplyCapExceeded = 10,
    PoolBorrowCapExceeded = 11,
    ArithmeticOverflow = 12,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Action {
    Deposit = 1,
    Withdraw = 2,
    Borrow = 3,
    Repay = 4,
    DepositCollateral = 5,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Call {
    pub action: Action,
    pub asset: Address,
    pub amount: i128,
    pub collateral_asset: Option<Address>,
    pub collateral_amount: Option<i128>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapConfig {
    /// Maximum one account may supply to a pool, expressed as bps of pool_supply_cap.
    pub user_supply_cap_bps: u32,
    /// Maximum one account may borrow against supplied collateral, expressed as collateral bps.
    pub user_borrow_cap_bps: u32,
    /// Absolute pool-wide supply cap. Set to i128::MAX to disable.
    pub pool_supply_cap: i128,
    /// Absolute pool-wide borrow cap. Set to i128::MAX to disable.
    pub pool_borrow_cap: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapUtilization {
    pub asset: Address,
    pub user: Address,
    pub user_supplied: i128,
    pub user_borrowed: i128,
    pub pool_supplied: i128,
    pub pool_borrowed: i128,
    pub user_supply_cap_bps: u32,
    pub user_borrow_cap_bps: u32,
    pub pool_supply_cap: i128,
    pub pool_borrow_cap: i128,
}

#[contracttype]
#[derive(Clone)]
pub enum MetaDataKey {
    DelegationRegistry,
    Nonce(Address),
    Caps(Address),
    PoolSupply(Address),
    PoolBorrow(Address),
    UserSupply(Address, Address),
    UserBorrow(Address, Address),
}

pub fn set_delegation_registry(env: &Env, registry: Address) {
    env.storage()
        .persistent()
        .set(&MetaDataKey::DelegationRegistry, &registry);
}

pub fn get_delegation_registry(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&MetaDataKey::DelegationRegistry)
}

pub fn configure_caps(
    env: &Env,
    admin: Address,
    asset: Address,
    config: CapConfig,
) -> Result<(), MetaTxError> {
    let current_admin = borrow::get_admin(env).ok_or(MetaTxError::Unauthorized)?;
    if current_admin != admin {
        return Err(MetaTxError::Unauthorized);
    }
    admin.require_auth();
    validate_cap_config(&config)?;
    env.storage().persistent().set(&MetaDataKey::Caps(asset), &config);
    Ok(())
}

pub fn get_cap_utilization(env: &Env, asset: Address, user: Address) -> CapUtilization {
    let config = get_cap_config(env, &asset);
    CapUtilization {
        asset: asset.clone(),
        user: user.clone(),
        user_supplied: get_i128(env, MetaDataKey::UserSupply(asset.clone(), user.clone())),
        user_borrowed: get_i128(env, MetaDataKey::UserBorrow(asset.clone(), user)),
        pool_supplied: get_i128(env, MetaDataKey::PoolSupply(asset.clone())),
        pool_borrowed: get_i128(env, MetaDataKey::PoolBorrow(asset)),
        user_supply_cap_bps: config.user_supply_cap_bps,
        user_borrow_cap_bps: config.user_borrow_cap_bps,
        pool_supply_cap: config.pool_supply_cap,
        pool_borrow_cap: config.pool_borrow_cap,
    }
}

fn get_nonce(env: &Env, delegator: &Address) -> u64 {
    env.storage()
        .persistent()
        .get(&MetaDataKey::Nonce(delegator.clone()))
        .unwrap_or(0)
}

fn set_nonce(env: &Env, delegator: &Address, nonce: u64) {
    env.storage()
        .persistent()
        .set(&MetaDataKey::Nonce(delegator.clone()), &nonce);
}

fn default_cap_config() -> CapConfig {
    CapConfig {
        user_supply_cap_bps: 10_000,
        user_borrow_cap_bps: 10_000,
        pool_supply_cap: i128::MAX,
        pool_borrow_cap: i128::MAX,
    }
}

fn get_cap_config(env: &Env, asset: &Address) -> CapConfig {
    env.storage()
        .persistent()
        .get(&MetaDataKey::Caps(asset.clone()))
        .unwrap_or_else(default_cap_config)
}

fn validate_cap_config(config: &CapConfig) -> Result<(), MetaTxError> {
    if config.user_supply_cap_bps > 10_000 || config.user_borrow_cap_bps > 10_000 {
        return Err(MetaTxError::InvalidCapConfig);
    }
    if config.pool_supply_cap < 0 || config.pool_borrow_cap < 0 {
        return Err(MetaTxError::InvalidCapConfig);
    }
    Ok(())
}

fn get_i128(env: &Env, key: MetaDataKey) -> i128 {
    env.storage().persistent().get(&key).unwrap_or(0)
}

fn set_i128(env: &Env, key: MetaDataKey, value: i128) {
    env.storage().persistent().set(&key, &value);
}

fn checked_add(a: i128, b: i128) -> Result<i128, MetaTxError> {
    a.checked_add(b).ok_or(MetaTxError::ArithmeticOverflow)
}

fn cap_from_bps(base: i128, bps: u32) -> Result<i128, MetaTxError> {
    base.checked_mul(bps as i128)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR))
        .ok_or(MetaTxError::ArithmeticOverflow)
}

fn enforce_supply_cap(env: &Env, user: &Address, asset: &Address, amount: i128) -> Result<(), MetaTxError> {
    let config = get_cap_config(env, asset);
    let pool_supplied = get_i128(env, MetaDataKey::PoolSupply(asset.clone()));
    let user_supplied = get_i128(env, MetaDataKey::UserSupply(asset.clone(), user.clone()));
    let new_pool_supplied = checked_add(pool_supplied, amount)?;
    let new_user_supplied = checked_add(user_supplied, amount)?;

    if new_pool_supplied > config.pool_supply_cap {
        return Err(MetaTxError::PoolSupplyCapExceeded);
    }

    let user_cap = cap_from_bps(config.pool_supply_cap, config.user_supply_cap_bps)?;
    if new_user_supplied > user_cap {
        return Err(MetaTxError::UserSupplyCapExceeded);
    }

    Ok(())
}

fn record_supply(env: &Env, user: &Address, asset: &Address, amount: i128) -> Result<(), MetaTxError> {
    let pool_key = MetaDataKey::PoolSupply(asset.clone());
    let user_key = MetaDataKey::UserSupply(asset.clone(), user.clone());
    set_i128(env, pool_key.clone(), checked_add(get_i128(env, pool_key), amount)?);
    set_i128(env, user_key.clone(), checked_add(get_i128(env, user_key), amount)?);
    Ok(())
}

fn enforce_borrow_cap(
    env: &Env,
    user: &Address,
    asset: &Address,
    amount: i128,
    collateral_amount: i128,
) -> Result<(), MetaTxError> {
    let config = get_cap_config(env, asset);
    let pool_borrowed = get_i128(env, MetaDataKey::PoolBorrow(asset.clone()));
    let user_borrowed = get_i128(env, MetaDataKey::UserBorrow(asset.clone(), user.clone()));
    let new_pool_borrowed = checked_add(pool_borrowed, amount)?;
    let new_user_borrowed = checked_add(user_borrowed, amount)?;

    if new_pool_borrowed > config.pool_borrow_cap {
        return Err(MetaTxError::PoolBorrowCapExceeded);
    }

    let user_cap = cap_from_bps(collateral_amount, config.user_borrow_cap_bps)?;
    if new_user_borrowed > user_cap {
        return Err(MetaTxError::UserBorrowCapExceeded);
    }

    Ok(())
}

fn record_borrow(env: &Env, user: &Address, asset: &Address, amount: i128) -> Result<(), MetaTxError> {
    let pool_key = MetaDataKey::PoolBorrow(asset.clone());
    let user_key = MetaDataKey::UserBorrow(asset.clone(), user.clone());
    set_i128(env, pool_key.clone(), checked_add(get_i128(env, pool_key), amount)?);
    set_i128(env, user_key.clone(), checked_add(get_i128(env, user_key), amount)?);
    Ok(())
}

fn validate_delegation(
    env: &Env,
    registry: &Address,
    delegator: &Address,
    delegate: &Address,
    action: Action,
) -> Result<(), MetaTxError> {
    let permission = match action {
        Action::Deposit => 1u32,
        Action::Withdraw => 2u32,
        Action::Borrow => 4u32,
        Action::Repay => 8u32,
        Action::DepositCollateral => 16u32,
    };

    let valid: bool = env.invoke_contract(
        registry,
        &Symbol::new(env, "validate"),
        Vec::from_array(
            env,
            [
                delegator.clone().into_val(env),
                delegate.clone().into_val(env),
                (permission as u32).into_val(env),
            ],
        ),
    );

    if !valid {
        return Err(MetaTxError::PermissionDenied);
    }

    Ok(())
}

pub fn execute_delegated(
    env: &Env,
    delegator: Address,
    delegate: Address,
    nonce: u64,
    deadline: u64,
    calls: Vec<Call>,
) -> Result<(), MetaTxError> {
    delegate.require_auth();

    require_deadline(env, deadline, MetaTxError::Expired)?;

    let current = get_nonce(env, &delegator);
    if nonce != current {
        return Err(MetaTxError::InvalidNonce);
    }
    set_nonce(env, &delegator, current + 1);

    let registry = get_delegation_registry(env).ok_or(MetaTxError::DelegationMissing)?;

    for c in calls.iter() {
        validate_delegation(env, &registry, &delegator, &delegate, c.action)?;

        match c.action {
            Action::Deposit => {
                if crate::pause::is_paused(env, PauseType::Deposit) {
                    return Err(MetaTxError::Unauthorized);
                }
                enforce_supply_cap(env, &delegator, &c.asset, c.amount)?;
                deposit::deposit_with_auth(env, delegator.clone(), c.asset.clone(), c.amount, false)
                    .map_err(|_| MetaTxError::Unauthorized)?;
                record_supply(env, &delegator, &c.asset, c.amount)?;
            }
            Action::Withdraw => {
                if crate::pause::is_paused(env, PauseType::Withdraw) {
                    return Err(MetaTxError::Unauthorized);
                }
                withdraw::withdraw_with_auth(env, delegator.clone(), c.asset.clone(), c.amount, false)
                    .map_err(|_| MetaTxError::Unauthorized)?;
            }
            Action::Borrow => {
                if crate::pause::is_paused(env, PauseType::Borrow) {
                    return Err(MetaTxError::Unauthorized);
                }
                let ca = c.collateral_asset.clone().ok_or(MetaTxError::Unauthorized)?;
                let camt = c.collateral_amount.ok_or(MetaTxError::Unauthorized)?;
                enforce_borrow_cap(env, &delegator, &c.asset, c.amount, camt)?;
                borrow::borrow_trusted(env, delegator.clone(), c.asset.clone(), c.amount, ca, camt)
                    .map_err(|_| MetaTxError::Unauthorized)?;
                record_borrow(env, &delegator, &c.asset, c.amount)?;
            }
            Action::Repay => {
                if crate::pause::is_paused(env, PauseType::Repay) {
                    return Err(MetaTxError::Unauthorized);
                }
                borrow::repay(env, delegator.clone(), c.asset.clone(), c.amount)
                    .map_err(|_| MetaTxError::Unauthorized)?;
            }
            Action::DepositCollateral => {
                if crate::pause::is_paused(env, PauseType::Deposit) {
                    return Err(MetaTxError::Unauthorized);
                }
                enforce_supply_cap(env, &delegator, &c.asset, c.amount)?;
                borrow::deposit(env, delegator.clone(), c.asset.clone(), c.amount)
                    .map_err(|_| MetaTxError::Unauthorized)?;
                record_supply(env, &delegator, &c.asset, c.amount)?;
            }
        }
    }

    Ok(())
}
