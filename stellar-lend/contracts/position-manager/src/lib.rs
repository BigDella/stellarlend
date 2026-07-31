#![no_std]
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PositionError {
    Unauthorized = 1,
    PositionNotFound = 2,
    PositionAlreadyOpen = 3,
    InvalidLeverage = 4,
    InsufficientCollateral = 5,
    HealthFactorTooLow = 6,
    AlreadyInitialized = 7,
    NotInitialized = 8,
    InvalidAmount = 9,
    Overflow = 10,
    MaxPositionsReached = 11,
    PositionNotActive = 12,
    DeleverageBelowMinimum = 13,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PositionMetadata {
    pub position_id: u64,
    pub owner: Address,
    pub deposit_asset: Address,
    pub borrow_asset: Address,
    pub collateral_amount: i128,
    pub borrowed_amount: i128,
    pub leverage_bps: u32,
    pub opened_at: u64,
    pub last_adjusted_at: u64,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PositionHealth {
    pub health_factor: i128,
    pub collateral_value: i128,
    pub debt_value: i128,
    pub liquidation_threshold: i128,
    pub is_liquidatable: bool,
    pub leverage_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    CurrentPositionId,
    Position(u64),
    OwnerPositions(Address),
    PositionCounter,
}

#[contract]
pub struct PositionManager;

#[contractimpl]
impl PositionManager {
    pub fn initialize(env: Env, admin: Address) -> Result<(), PositionError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(PositionError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::CurrentPositionId, &0u64);
        Ok(())
    }

    pub fn create_position(
        env: Env,
        owner: Address,
        deposit_asset: Address,
        borrow_asset: Address,
        collateral_amount: i128,
        borrowed_amount: i128,
        leverage_bps: u32,
    ) -> Result<u64, PositionError> {
        owner.require_auth();
        if leverage_bps < 10_000 || leverage_bps > 50_000 {
            return Err(PositionError::InvalidLeverage);
        }
        if collateral_amount <= 0 || borrowed_amount < 0 {
            return Err(PositionError::InvalidAmount);
        }

        let mut pos_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CurrentPositionId)
            .unwrap_or(0);
        pos_id += 1;

        let position = PositionMetadata {
            position_id: pos_id,
            owner: owner.clone(),
            deposit_asset: deposit_asset.clone(),
            borrow_asset: borrow_asset.clone(),
            collateral_amount,
            borrowed_amount,
            leverage_bps,
            opened_at: env.ledger().timestamp(),
            last_adjusted_at: env.ledger().timestamp(),
            active: true,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Position(pos_id), &position);

        let mut owner_positions: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OwnerPositions(owner.clone()))
            .unwrap_or(Vec::new(&env));
        owner_positions.push_back(pos_id);
        env.storage()
            .persistent()
            .set(&DataKey::OwnerPositions(owner), &owner_positions);

        env.storage()
            .instance()
            .set(&DataKey::CurrentPositionId, &pos_id);

        Ok(pos_id)
    }

    pub fn get_position(env: Env, position_id: u64) -> Option<PositionMetadata> {
        env.storage()
            .persistent()
            .get(&DataKey::Position(position_id))
    }

    pub fn update_position(
        env: Env,
        position_id: u64,
        collateral_amount: i128,
        borrowed_amount: i128,
        leverage_bps: u32,
    ) -> Result<PositionMetadata, PositionError> {
        let mut position: PositionMetadata = env
            .storage()
            .persistent()
            .get(&DataKey::Position(position_id))
            .ok_or(PositionError::PositionNotFound)?;

        if !position.active {
            return Err(PositionError::PositionNotActive);
        }

        if leverage_bps < 10_000 || leverage_bps > 50_000 {
            return Err(PositionError::InvalidLeverage);
        }

        position.collateral_amount = collateral_amount;
        position.borrowed_amount = borrowed_amount;
        position.leverage_bps = leverage_bps;
        position.last_adjusted_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Position(position_id), &position);

        Ok(position)
    }

    pub fn close_position(
        env: Env,
        caller: Address,
        position_id: u64,
    ) -> Result<PositionMetadata, PositionError> {
        caller.require_auth();
        let mut position: PositionMetadata = env
            .storage()
            .persistent()
            .get(&DataKey::Position(position_id))
            .ok_or(PositionError::PositionNotFound)?;

        if position.owner != caller {
            return Err(PositionError::Unauthorized);
        }
        if !position.active {
            return Err(PositionError::PositionNotActive);
        }

        position.active = false;
        position.last_adjusted_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Position(position_id), &position);

        Ok(position)
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
}
