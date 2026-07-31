#![no_std]
use soroban_sdk::{contract, contracterror, contractevent, contractimpl, contracttype, Address, Env, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum LeveragedYieldError {
    Unauthorized = 1,
    PositionNotFound = 2,
    InvalidLeverage = 3,
    LeverageOutOfRange = 4,
    InvalidAmount = 5,
    InsufficientCollateral = 6,
    HealthFactorTooLow = 7,
    AlreadyInitialized = 8,
    NotInitialized = 9,
    Overflow = 10,
    PositionNotActive = 11,
    SlippageExceeded = 12,
    MinHealthFactorNotMet = 13,
    DepositFailed = 14,
    BorrowFailed = 15,
    RepayFailed = 16,
    WithdrawFailed = 17,
    PriceUnavailable = 18,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct LeveragedPosition {
    pub position_id: u64,
    pub owner: Address,
    pub pool: Address,
    pub deposit_asset: Address,
    pub borrow_asset: Address,
    pub collateral_amount: i128,
    pub borrowed_amount: i128,
    pub leverage_bps: u32,
    pub health_factor: i128,
    pub opened_at: u64,
    pub last_harvested_at: u64,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct HealthSnapshot {
    pub health_factor: i128,
    pub collateral_value: i128,
    pub debt_value: i128,
    pub liquidation_threshold: i128,
    pub is_liquidatable: bool,
    pub leverage_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct LeverageConfig {
    pub min_leverage_bps: u32,
    pub max_leverage_bps: u32,
    pub target_ltv_bps: u32,
    pub min_health_factor: i128,
    pub auto_deleverage_threshold: i128,
    pub deleverage_target_bps: u32,
    pub rebalance_tolerance_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    Config,
    Position(u64),
    OwnerPositions(Address),
    PositionCounter,
    LiquidationFeeBps,
    MaxPositionCount,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct PositionOpenedEvent {
    pub position_id: u64,
    pub owner: Address,
    pub pool: Address,
    pub deposit_asset: Address,
    pub borrow_asset: Address,
    pub collateral_amount: i128,
    pub borrowed_amount: i128,
    pub leverage_bps: u32,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct PositionClosedEvent {
    pub position_id: u64,
    pub owner: Address,
    pub collateral_returned: i128,
    pub debt_repaid: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct LeverageAdjustedEvent {
    pub position_id: u64,
    pub owner: Address,
    pub old_leverage_bps: u32,
    pub new_leverage_bps: u32,
    pub additional_collateral: i128,
    pub additional_debt: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct DeleverageEvent {
    pub position_id: u64,
    pub owner: Address,
    pub reason: u32,
    pub debt_repaid: i128,
    pub collateral_withdrawn: i128,
    pub new_health_factor: i128,
    pub timestamp: u64,
}

const MAX_BPS: u32 = 10_000;
const MIN_HEALTH_FACTOR: i128 = 15_000;
const LIQUIDATION_THRESHOLD_BPS: u32 = 8_000;

#[contract]
pub struct LeveragedYield;

#[contractimpl]
impl LeveragedYield {
    pub fn initialize(
        env: Env,
        admin: Address,
        config: LeverageConfig,
    ) -> Result<(), LeveragedYieldError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(LeveragedYieldError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage()
            .instance()
            .set(&DataKey::PositionCounter, &0u64);
        env.storage()
            .instance()
            .set(&DataKey::LiquidationFeeBps, &500i128);
        env.storage()
            .instance()
            .set(&DataKey::MaxPositionCount, &100u32);
        Ok(())
    }

    pub fn set_config(
        env: Env,
        admin: Address,
        config: LeverageConfig,
    ) -> Result<(), LeveragedYieldError> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(LeveragedYieldError::Unauthorized)?;
        if admin != stored_admin {
            return Err(LeveragedYieldError::Unauthorized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Config, &config);
        Ok(())
    }

    pub fn get_config(env: Env) -> LeverageConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .unwrap()
    }

    pub fn open_position(
        env: Env,
        owner: Address,
        pool: Address,
        deposit_asset: Address,
        borrow_asset: Address,
        collateral_amount: i128,
        target_leverage_bps: u32,
        min_health_factor: i128,
    ) -> Result<u64, LeveragedYieldError> {
        owner.require_auth();

        let config: LeverageConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(LeveragedYieldError::NotInitialized)?;

        if target_leverage_bps < config.min_leverage_bps
            || target_leverage_bps > config.max_leverage_bps
        {
            return Err(LeveragedYieldError::LeverageOutOfRange);
        }
        if collateral_amount <= 0 {
            return Err(LeveragedYieldError::InvalidAmount);
        }

        if min_health_factor < MIN_HEALTH_FACTOR {
            return Err(LeveragedYieldError::MinHealthFactorNotMet);
        }

        let borrowed_amount = Self::calculate_borrow_amount(collateral_amount, target_leverage_bps)?;

        let mut counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PositionCounter)
            .unwrap_or(0);
        counter += 1;

        let hf = Self::compute_health_factor(
            collateral_amount,
            borrowed_amount,
            config.target_ltv_bps,
        );

        let position = LeveragedPosition {
            position_id: counter,
            owner: owner.clone(),
            pool,
            deposit_asset: deposit_asset.clone(),
            borrow_asset: borrow_asset.clone(),
            collateral_amount,
            borrowed_amount,
            leverage_bps: target_leverage_bps,
            health_factor: hf,
            opened_at: env.ledger().timestamp(),
            last_harvested_at: env.ledger().timestamp(),
            active: true,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Position(counter), &position);

        let mut owner_positions: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OwnerPositions(owner.clone()))
            .unwrap_or(Vec::new(&env));
        owner_positions.push_back(counter);
        env.storage()
            .persistent()
            .set(&DataKey::OwnerPositions(owner.clone()), &owner_positions);

        env.storage()
            .instance()
            .set(&DataKey::PositionCounter, &counter);

        PositionOpenedEvent {
            position_id: counter,
            owner,
            pool: position.pool.clone(),
            deposit_asset,
            borrow_asset,
            collateral_amount,
            borrowed_amount,
            leverage_bps: target_leverage_bps,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(counter)
    }

    pub fn close_position(
        env: Env,
        caller: Address,
        position_id: u64,
        min_collateral_out: i128,
    ) -> Result<LeveragedPosition, LeveragedYieldError> {
        caller.require_auth();

        let mut position: LeveragedPosition = env
            .storage()
            .persistent()
            .get(&DataKey::Position(position_id))
            .ok_or(LeveragedYieldError::PositionNotFound)?;

        if position.owner != caller {
            return Err(LeveragedYieldError::Unauthorized);
        }
        if !position.active {
            return Err(LeveragedYieldError::PositionNotActive);
        }

        let collateral_after_debt = position
            .collateral_amount
            .checked_sub(position.borrowed_amount)
            .ok_or(LeveragedYieldError::Overflow)?;

        if collateral_after_debt < min_collateral_out {
            return Err(LeveragedYieldError::SlippageExceeded);
        }

        position.active = false;
        position.health_factor = 0;

        let collateral_returned = collateral_after_debt;
        let debt_repaid = position.borrowed_amount;

        env.storage()
            .persistent()
            .set(&DataKey::Position(position_id), &position);

        PositionClosedEvent {
            position_id,
            owner: caller,
            collateral_returned,
            debt_repaid,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(position)
    }

    pub fn adjust_leverage(
        env: Env,
        caller: Address,
        position_id: u64,
        new_leverage_bps: u32,
        additional_collateral: i128,
        min_health_factor: i128,
    ) -> Result<LeveragedPosition, LeveragedYieldError> {
        caller.require_auth();

        let config: LeverageConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(LeveragedYieldError::NotInitialized)?;

        if new_leverage_bps < config.min_leverage_bps
            || new_leverage_bps > config.max_leverage_bps
        {
            return Err(LeveragedYieldError::LeverageOutOfRange);
        }

        let mut position: LeveragedPosition = env
            .storage()
            .persistent()
            .get(&DataKey::Position(position_id))
            .ok_or(LeveragedYieldError::PositionNotFound)?;

        if position.owner != caller {
            return Err(LeveragedYieldError::Unauthorized);
        }
        if !position.active {
            return Err(LeveragedYieldError::PositionNotActive);
        }

        let old_leverage_bps = position.leverage_bps;
        let new_total_collateral = position
            .collateral_amount
            .checked_add(additional_collateral)
            .ok_or(LeveragedYieldError::Overflow)?;

        let new_borrowed_amount =
            Self::calculate_borrow_amount(new_total_collateral, new_leverage_bps)?;

        let hf = Self::compute_health_factor(new_total_collateral, new_borrowed_amount, config.target_ltv_bps);

        if hf < min_health_factor || hf < MIN_HEALTH_FACTOR {
            return Err(LeveragedYieldError::MinHealthFactorNotMet);
        }

        position.collateral_amount = new_total_collateral;
        position.borrowed_amount = new_borrowed_amount;
        position.leverage_bps = new_leverage_bps;
        position.health_factor = hf;
        position.last_harvested_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Position(position_id), &position);

        LeverageAdjustedEvent {
            position_id,
            owner: caller,
            old_leverage_bps,
            new_leverage_bps,
            additional_collateral,
            additional_debt: new_borrowed_amount.saturating_sub(position.borrowed_amount),
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(position)
    }

    pub fn deleverage(
        env: Env,
        caller: Address,
        position_id: u64,
        target_leverage_bps: u32,
        debt_repay_amount: i128,
        min_collateral_out: i128,
    ) -> Result<LeveragedPosition, LeveragedYieldError> {
        caller.require_auth();

        let config: LeverageConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(LeveragedYieldError::NotInitialized)?;

        let mut position: LeveragedPosition = env
            .storage()
            .persistent()
            .get(&DataKey::Position(position_id))
            .ok_or(LeveragedYieldError::PositionNotFound)?;

        if position.owner != caller {
            return Err(LeveragedYieldError::Unauthorized);
        }
        if !position.active {
            return Err(LeveragedYieldError::PositionNotActive);
        }

        if target_leverage_bps < config.min_leverage_bps
            || target_leverage_bps > position.leverage_bps
        {
            return Err(LeveragedYieldError::LeverageOutOfRange);
        }

        if debt_repay_amount <= 0 || debt_repay_amount > position.borrowed_amount {
            return Err(LeveragedYieldError::InvalidAmount);
        }

        let new_borrowed_amount = position
            .borrowed_amount
            .checked_sub(debt_repay_amount)
            .ok_or(LeveragedYieldError::Overflow)?;

        let new_total_collateral =
            Self::calculate_collateral_from_borrow(new_borrowed_amount, target_leverage_bps)?;

        let collateral_to_withdraw = position
            .collateral_amount
            .checked_sub(new_total_collateral)
            .ok_or(LeveragedYieldError::Overflow)?;

        if collateral_to_withdraw < min_collateral_out {
            return Err(LeveragedYieldError::SlippageExceeded);
        }

        let hf = Self::compute_health_factor(new_total_collateral, new_borrowed_amount, config.target_ltv_bps);

        position.collateral_amount = new_total_collateral;
        position.borrowed_amount = new_borrowed_amount;
        position.leverage_bps = target_leverage_bps;
        position.health_factor = hf;
        position.last_harvested_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Position(position_id), &position);

        DeleverageEvent {
            position_id,
            owner: caller,
            reason: 1,
            debt_repaid: debt_repay_amount,
            collateral_withdrawn: collateral_to_withdraw,
            new_health_factor: hf,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(position)
    }

    pub fn auto_deleverage(
        env: Env,
        position_id: u64,
    ) -> Result<LeveragedPosition, LeveragedYieldError> {
        let mut position: LeveragedPosition = env
            .storage()
            .persistent()
            .get(&DataKey::Position(position_id))
            .ok_or(LeveragedYieldError::PositionNotFound)?;

        if !position.active {
            return Err(LeveragedYieldError::PositionNotActive);
        }

        let config: LeverageConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(LeveragedYieldError::NotInitialized)?;

        if position.health_factor >= config.auto_deleverage_threshold {
            return Err(LeveragedYieldError::HealthFactorTooLow);
        }

        let deleverage_factor_bps = 10_000u32;
        let target_borrowed = position
            .borrowed_amount
            .checked_mul(config.deleverage_target_bps as i128)
            .ok_or(LeveragedYieldError::Overflow)?
            .checked_div(deleverage_factor_bps as i128)
            .ok_or(LeveragedYieldError::Overflow)?;

        let debt_repay = position
            .borrowed_amount
            .checked_sub(target_borrowed)
            .ok_or(LeveragedYieldError::Overflow)?;

        let new_borrowed_amount = target_borrowed;
        let new_collateral = position
            .collateral_amount
            .checked_mul(new_borrowed_amount)
            .ok_or(LeveragedYieldError::Overflow)?
            .checked_div(position.borrowed_amount)
            .ok_or(LeveragedYieldError::Overflow)?;

        let hf = Self::compute_health_factor(new_collateral, new_borrowed_amount, config.target_ltv_bps);

        let collateral_withdrawn = position
            .collateral_amount
            .checked_sub(new_collateral)
            .ok_or(LeveragedYieldError::Overflow)?;

        position.collateral_amount = new_collateral;
        position.borrowed_amount = new_borrowed_amount;
        position.leverage_bps = config.deleverage_target_bps;
        position.health_factor = hf;
        position.last_harvested_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Position(position_id), &position);

        DeleverageEvent {
            position_id,
            owner: position.owner.clone(),
            reason: 2,
            debt_repaid: debt_repay,
            collateral_withdrawn,
            new_health_factor: hf,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(position)
    }

    pub fn get_health(env: Env, position_id: u64) -> Result<HealthSnapshot, LeveragedYieldError> {
        let position: LeveragedPosition = env
            .storage()
            .persistent()
            .get(&DataKey::Position(position_id))
            .ok_or(LeveragedYieldError::PositionNotFound)?;

        let config: LeverageConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(LeveragedYieldError::NotInitialized)?;

        let collateral_value = position.collateral_amount;
        let debt_value = position.borrowed_amount;
        let health_factor = Self::compute_health_factor(
            collateral_value,
            debt_value,
            config.target_ltv_bps,
        );
        let liquidation_threshold = collateral_value
            .checked_mul(LIQUIDATION_THRESHOLD_BPS as i128)
            .ok_or(LeveragedYieldError::Overflow)?
            .checked_div(MAX_BPS as i128)
            .ok_or(LeveragedYieldError::Overflow)?;

        Ok(HealthSnapshot {
            health_factor,
            collateral_value,
            debt_value,
            liquidation_threshold,
            is_liquidatable: health_factor < MIN_HEALTH_FACTOR,
            leverage_bps: position.leverage_bps,
        })
    }

    pub fn get_position(
        env: Env,
        position_id: u64,
    ) -> Option<LeveragedPosition> {
        env.storage()
            .persistent()
            .get(&DataKey::Position(position_id))
    }

    pub fn get_owner_positions(env: Env, owner: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::OwnerPositions(owner))
            .unwrap_or(Vec::new(&env))
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    pub fn check_and_deleverage(
        env: Env,
        position_id: u64,
    ) -> Result<Option<LeveragedPosition>, LeveragedYieldError> {
        let position: LeveragedPosition = env
            .storage()
            .persistent()
            .get(&DataKey::Position(position_id))
            .ok_or(LeveragedYieldError::PositionNotFound)?;

        if !position.active {
            return Ok(None);
        }

        let config: LeverageConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(LeveragedYieldError::NotInitialized)?;

        if position.health_factor < config.auto_deleverage_threshold {
            let result = Self::auto_deleverage(env, position_id)?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    // ── Internal math ──

    fn calculate_borrow_amount(
        collateral: i128,
        leverage_bps: u32,
    ) -> Result<i128, LeveragedYieldError> {
        let leverage_debt_bps = leverage_bps
            .checked_sub(MAX_BPS)
            .ok_or(LeveragedYieldError::LeverageOutOfRange)?;

        if leverage_debt_bps == 0 {
            return Ok(0);
        }

        let borrowed = collateral
            .checked_mul(leverage_debt_bps as i128)
            .ok_or(LeveragedYieldError::Overflow)?
            .checked_div(MAX_BPS as i128)
            .ok_or(LeveragedYieldError::Overflow)?;

        Ok(borrowed)
    }

    fn calculate_collateral_from_borrow(
        borrowed: i128,
        leverage_bps: u32,
    ) -> Result<i128, LeveragedYieldError> {
        if leverage_bps == 0 || leverage_bps < MAX_BPS {
            return Err(LeveragedYieldError::InvalidLeverage);
        }
        let leverage_debt_bps = leverage_bps
            .checked_sub(MAX_BPS)
            .ok_or(LeveragedYieldError::LeverageOutOfRange)?;

        if leverage_debt_bps == 0 {
            return Ok(borrowed);
        }

        let collateral = borrowed
            .checked_mul(MAX_BPS as i128)
            .ok_or(LeveragedYieldError::Overflow)?
            .checked_div(leverage_debt_bps as i128)
            .ok_or(LeveragedYieldError::Overflow)?;

        Ok(collateral)
    }

    fn compute_health_factor(
        collateral: i128,
        debt: i128,
        target_ltv_bps: u32,
    ) -> i128 {
        if debt <= 0 {
            return i128::MAX;
        }
        let effective_collateral = collateral
            .checked_mul(target_ltv_bps as i128)
            .unwrap_or(0)
            .checked_div(MAX_BPS as i128)
            .unwrap_or(0);

        if effective_collateral <= 0 || debt <= 0 {
            return if debt == 0 { i128::MAX } else { 0 };
        }

        effective_collateral
            .checked_mul(MIN_HEALTH_FACTOR)
            .unwrap_or(0)
            .checked_div(debt)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod test;
