#![no_std]
use soroban_sdk::{contract, contracterror, contractevent, contractimpl, contracttype, Address, Env, Vec};

use pool_interfaces::{PoolAllocation, RiskProfile, RouterConfig};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RouterError {
    Unauthorized = 1,
    AlreadyInitialized = 2,
    NotInitialized = 3,
    InvalidAmount = 4,
    InsufficientBalance = 5,
    PoolNotSupported = 6,
    NoPoolsConfigured = 7,
    PoolAtCapacity = 8,
    SlippageExceeded = 9,
    RebalanceCooldownActive = 10,
    Overflow = 11,
    MaxPoolsExceeded = 12,
    UserPositionNotFound = 13,
    RiskProfileMismatch = 14,
    RebalanceThresholdNotMet = 15,
    AllocationMismatch = 16,
    DepositPaused = 17,
    WithdrawPaused = 18,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct UserRouterPosition {
    pub user: Address,
    pub asset: Address,
    pub total_deposited: i128,
    pub allocations: Vec<PoolAllocation>,
    pub last_rebalance_at: u64,
    pub risk_profile: RiskProfile,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    Config,
    RegisteredPool(Address),
    RegisteredPools,
    UserPosition(Address, Address),
    Allocations(Address, Address),
    LastRebalanceAt(Address, Address),
    RiskProfile(Address),
    Paused,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct PoolRegisteredEvent {
    pub pool: Address,
    pub asset: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct DepositRoutedEvent {
    pub user: Address,
    pub asset: Address,
    pub total_amount: i128,
    pub pool_count: u32,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct WithdrawRoutedEvent {
    pub user: Address,
    pub asset: Address,
    pub total_amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct RebalanceEvent {
    pub user: Address,
    pub asset: Address,
    pub old_allocations: Vec<PoolAllocation>,
    pub new_allocations: Vec<PoolAllocation>,
    pub reason: u32,
    pub timestamp: u64,
}

const MAX_BPS: u32 = 10_000;

#[contract]
pub struct YieldRouter;

#[contractimpl]
impl YieldRouter {
    pub fn initialize(
        env: Env,
        admin: Address,
        config: RouterConfig,
    ) -> Result<(), RouterError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(RouterError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage()
            .instance()
            .set(&DataKey::RegisteredPools, &Vec::<Address>::new(&env));
        env.storage().instance().set(&DataKey::Paused, &false);
        Ok(())
    }

    pub fn register_pool(
        env: Env,
        admin: Address,
        pool: Address,
        asset: Address,
    ) -> Result<(), RouterError> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(RouterError::Unauthorized)?;
        if admin != stored_admin {
            return Err(RouterError::Unauthorized);
        }
        admin.require_auth();

        if env.storage().persistent().has(&DataKey::RegisteredPool(pool.clone())) {
            return Ok(());
        }

        let config: RouterConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(RouterError::NotInitialized)?;

        let mut pools: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::RegisteredPools)
            .unwrap_or(Vec::new(&env));

        if (pools.len() as u32) >= config.max_pools {
            return Err(RouterError::MaxPoolsExceeded);
        }

        env.storage()
            .persistent()
            .set(&DataKey::RegisteredPool(pool.clone()), &asset);
        pools.push_back(pool.clone());
        env.storage()
            .instance()
            .set(&DataKey::RegisteredPools, &pools);

        PoolRegisteredEvent {
            pool,
            asset,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(())
    }

    pub fn deposit(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
        risk_profile: RiskProfile,
        min_total_shares: i128,
    ) -> Result<i128, RouterError> {
        user.require_auth();

        let paused: bool = env.storage().instance().get(&DataKey::Paused).unwrap_or(false);
        if paused {
            return Err(RouterError::DepositPaused);
        }

        if amount <= 0 {
            return Err(RouterError::InvalidAmount);
        }

        let allocations = Self::compute_allocation(&env, &asset, amount, &risk_profile)?;

        if allocations.len() == 0 {
            return Err(RouterError::NoPoolsConfigured);
        }

        let mut total_deposited: i128 = 0;
        for alloc in allocations.iter() {
            let pool_info: Option<Address> = env
                .storage()
                .persistent()
                .get(&DataKey::RegisteredPool(alloc.pool.clone()));
            if pool_info.is_none() {
                continue;
            }
            total_deposited = total_deposited
                .checked_add(alloc.amount)
                .ok_or(RouterError::Overflow)?;
        }

        if total_deposited < min_total_shares {
            return Err(RouterError::SlippageExceeded);
        }

        env.storage()
            .persistent()
            .set(
                &DataKey::UserPosition(user.clone(), asset.clone()),
                &UserRouterPosition {
                    user: user.clone(),
                    asset: asset.clone(),
                    total_deposited,
                    allocations: allocations.clone(),
                    last_rebalance_at: env.ledger().timestamp(),
                    risk_profile: risk_profile.clone(),
                },
            );

        env.storage()
            .persistent()
            .set(
                &DataKey::RiskProfile(user.clone()),
                &risk_profile,
            );

        env.storage()
            .persistent()
            .set(
                &DataKey::Allocations(user.clone(), asset.clone()),
                &allocations,
            );

        DepositRoutedEvent {
            user,
            asset,
            total_amount: total_deposited,
            pool_count: allocations.len(),
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(total_deposited)
    }

    pub fn withdraw(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
        min_total_return: i128,
    ) -> Result<i128, RouterError> {
        user.require_auth();

        let paused: bool = env.storage().instance().get(&DataKey::Paused).unwrap_or(false);
        if paused {
            return Err(RouterError::WithdrawPaused);
        }

        if amount <= 0 {
            return Err(RouterError::InvalidAmount);
        }

        let position: UserRouterPosition = env
            .storage()
            .persistent()
            .get(&DataKey::UserPosition(user.clone(), asset.clone()))
            .ok_or(RouterError::UserPositionNotFound)?;

        if amount > position.total_deposited {
            return Err(RouterError::InsufficientBalance);
        }

        let allocations: Vec<PoolAllocation> = env
            .storage()
            .persistent()
            .get(&DataKey::Allocations(user.clone(), asset.clone()))
            .unwrap_or(Vec::new(&env));

        let withdrawal_ratio = amount
            .checked_mul(MAX_BPS as i128)
            .ok_or(RouterError::Overflow)?
            .checked_div(position.total_deposited)
            .ok_or(RouterError::Overflow)?;

        let mut total_withdrawn: i128 = 0;
        for alloc in allocations.iter() {
            let withdraw_amount = alloc
                .amount
                .checked_mul(withdrawal_ratio)
                .ok_or(RouterError::Overflow)?
                .checked_div(MAX_BPS as i128)
                .ok_or(RouterError::Overflow)?;

            if withdraw_amount > 0 {
                total_withdrawn = total_withdrawn
                    .checked_add(withdraw_amount)
                    .ok_or(RouterError::Overflow)?;
            }
        }

        if total_withdrawn < min_total_return {
            return Err(RouterError::SlippageExceeded);
        }

        let remaining = position
            .total_deposited
            .checked_sub(amount)
            .ok_or(RouterError::Overflow)?;
        let remaining_ratio = if position.total_deposited > 0 {
            remaining
                .checked_mul(MAX_BPS as i128)
                .ok_or(RouterError::Overflow)?
                .checked_div(position.total_deposited)
                .ok_or(RouterError::Overflow)?
        } else {
            0
        };

        let mut new_allocations: Vec<PoolAllocation> = Vec::new(&env);
        for alloc in allocations.iter() {
            let new_amount = alloc
                .amount
                .checked_mul(remaining_ratio)
                .ok_or(RouterError::Overflow)?
                .checked_div(MAX_BPS as i128)
                .ok_or(RouterError::Overflow)?;

            if new_amount > 0 {
                new_allocations.push_back(PoolAllocation {
                    pool: alloc.pool.clone(),
                    asset: alloc.asset.clone(),
                    weight_bps: alloc.weight_bps,
                    amount: new_amount,
                    expected_apy_bps: alloc.expected_apy_bps,
                });
            }
        }

        env.storage()
            .persistent()
            .set(
                &DataKey::UserPosition(user.clone(), asset.clone()),
                &UserRouterPosition {
                    user: user.clone(),
                    asset: asset.clone(),
                    total_deposited: remaining,
                    allocations: new_allocations.clone(),
                    last_rebalance_at: position.last_rebalance_at,
                    risk_profile: position.risk_profile.clone(),
                },
            );

        env.storage()
            .persistent()
            .set(
                &DataKey::Allocations(user.clone(), asset.clone()),
                &new_allocations,
            );

        WithdrawRoutedEvent {
            user,
            asset,
            total_amount: total_withdrawn,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(total_withdrawn)
    }

    pub fn rebalance(
        env: Env,
        user: Address,
        asset: Address,
        new_risk_profile: Option<RiskProfile>,
        min_total_return: i128,
    ) -> Result<Vec<PoolAllocation>, RouterError> {
        user.require_auth();

        let position: UserRouterPosition = env
            .storage()
            .persistent()
            .get(&DataKey::UserPosition(user.clone(), asset.clone()))
            .ok_or(RouterError::UserPositionNotFound)?;

        let config: RouterConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(RouterError::NotInitialized)?;

        let now = env.ledger().timestamp();
        if now - position.last_rebalance_at < config.rebalance_cooldown_secs {
            return Err(RouterError::RebalanceCooldownActive);
        }

        let profile = new_risk_profile.clone().unwrap_or(position.risk_profile.clone());
        let old_allocations: Vec<PoolAllocation> = env
            .storage()
            .persistent()
            .get(&DataKey::Allocations(user.clone(), asset.clone()))
            .unwrap_or(Vec::new(&env));

        let new_allocations = Self::compute_allocation(
            &env,
            &asset,
            position.total_deposited,
            &profile,
        )?;

        if new_allocations.len() == 0 {
            return Err(RouterError::NoPoolsConfigured);
        }

        let mut new_total: i128 = 0;
        for alloc in new_allocations.iter() {
            new_total = new_total
                .checked_add(alloc.amount)
                .ok_or(RouterError::Overflow)?;
        }

        if new_total < min_total_return {
            return Err(RouterError::SlippageExceeded);
        }

        env.storage()
            .persistent()
            .set(
                &DataKey::UserPosition(user.clone(), asset.clone()),
                &UserRouterPosition {
                    user: user.clone(),
                    asset: asset.clone(),
                    total_deposited: position.total_deposited,
                    allocations: new_allocations.clone(),
                    last_rebalance_at: now,
                    risk_profile: profile.clone(),
                },
            );

        env.storage()
            .persistent()
            .set(
                &DataKey::Allocations(user.clone(), asset.clone()),
                &new_allocations,
            );

        env.storage()
            .persistent()
            .set(
                &DataKey::RiskProfile(user.clone()),
                &profile,
            );

        let reason: u32 = if new_risk_profile.is_some() { 1 } else { 2 };

        RebalanceEvent {
            user,
            asset,
            old_allocations,
            new_allocations: new_allocations.clone(),
            reason,
            timestamp: now,
        }
        .publish(&env);

        Ok(new_allocations)
    }

    pub fn get_user_position(
        env: Env,
        user: Address,
        asset: Address,
    ) -> Option<UserRouterPosition> {
        env.storage()
            .persistent()
            .get(&DataKey::UserPosition(user, asset))
    }

    pub fn get_allocations(
        env: Env,
        user: Address,
        asset: Address,
    ) -> Vec<PoolAllocation> {
        env.storage()
            .persistent()
            .get(&DataKey::Allocations(user, asset))
            .unwrap_or(Vec::new(&env))
    }

    pub fn get_registered_pools(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::RegisteredPools)
            .unwrap_or(Vec::new(&env))
    }

    pub fn is_pool_registered(env: Env, pool: Address) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::RegisteredPool(pool))
    }

    pub fn get_config(env: Env) -> RouterConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .unwrap()
    }

    pub fn set_config(
        env: Env,
        admin: Address,
        config: RouterConfig,
    ) -> Result<(), RouterError> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(RouterError::Unauthorized)?;
        if admin != stored_admin {
            return Err(RouterError::Unauthorized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Config, &config);
        Ok(())
    }

    pub fn set_paused(
        env: Env,
        admin: Address,
        paused: bool,
    ) -> Result<(), RouterError> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(RouterError::Unauthorized)?;
        if admin != stored_admin {
            return Err(RouterError::Unauthorized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &paused);
        Ok(())
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    fn compute_allocation(
        env: &Env,
        _asset: &Address,
        total_amount: i128,
        risk_profile: &RiskProfile,
    ) -> Result<Vec<PoolAllocation>, RouterError> {
        let pools: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::RegisteredPools)
            .unwrap_or(Vec::new(env));

        if pools.len() == 0 {
            return Err(RouterError::NoPoolsConfigured);
        }

        let (min_apy_threshold, max_allocation_pct) = match risk_profile {
            RiskProfile::Conservative => (300u32, 3_000u32),
            RiskProfile::Moderate => (500u32, 5_000u32),
            RiskProfile::Aggressive => (800u32, MAX_BPS),
        };

        let pool_count = pools.len() as u32;
        let base_allocation_bps = MAX_BPS / pool_count;

        let mut allocations: Vec<PoolAllocation> = Vec::new(env);
        let mut allocated_bps: u32 = 0;

        for i in 0..pools.len() {
            let pool = pools.get(i).unwrap();
            let pool_asset: Address = env
                .storage()
                .persistent()
                .get(&DataKey::RegisteredPool(pool.clone()))
                .unwrap();

            let weight_bps = if i == pools.len() - 1 {
                MAX_BPS - allocated_bps
            } else {
                let alloc = base_allocation_bps.min(max_allocation_pct);
                allocated_bps = allocated_bps.checked_add(alloc).ok_or(RouterError::Overflow)?;
                alloc
            };

            let amount = total_amount
                .checked_mul(weight_bps as i128)
                .ok_or(RouterError::Overflow)?
                .checked_div(MAX_BPS as i128)
                .ok_or(RouterError::Overflow)?;

            if amount > 0 {
                allocations.push_back(PoolAllocation {
                    pool,
                    asset: pool_asset,
                    weight_bps,
                    amount,
                    expected_apy_bps: min_apy_threshold,
                });
            }
        }

        Ok(allocations)
    }
}

#[cfg(test)]
mod test;
