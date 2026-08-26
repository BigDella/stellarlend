#![no_std]
#![allow(deprecated)]

use soroban_sdk::token::{Client as TokenClient, StellarAssetClient};
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Symbol};

const BPS: i128 = 10_000;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum StablecoinError {
    Unauthorized = 1,
    AlreadyInitialized = 2,
    NotInitialized = 3,
    InvalidAmount = 4,
    InvalidParameter = 5,
    Overflow = 6,
    Shutdown = 7,
    InsufficientCollateral = 8,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    StablecoinToken,
    CollateralToken,
    ReserveRatioBps,
    Shutdown,
    TotalCollateral,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct FractionalReserveConfig {
    /// Minimum collateral backing ratio (basis points).
    /// Example: 2_000 => 20% reserve backing.
    pub reserve_ratio_bps: i128,
}

fn require_init(env: &Env) -> Result<(), StablecoinError> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(StablecoinError::NotInitialized);
    }
    Ok(())
}

fn require_admin(env: &Env, caller: &Address) -> Result<(), StablecoinError> {
    require_init(env)?;
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if &admin != caller {
        return Err(StablecoinError::Unauthorized);
    }
    Ok(())
}

fn is_shutdown(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Shutdown)
        .unwrap_or(false)
}

fn stablecoin_token(env: &Env) -> Result<Address, StablecoinError> {
    require_init(env)?;
    env.storage()
        .instance()
        .get(&DataKey::StablecoinToken)
        .ok_or(StablecoinError::NotInitialized)
}

fn collateral_token(env: &Env) -> Result<Address, StablecoinError> {
    require_init(env)?;
    env.storage()
        .instance()
        .get(&DataKey::CollateralToken)
        .ok_or(StablecoinError::NotInitialized)
}

fn reserve_ratio_bps(env: &Env) -> Result<i128, StablecoinError> {
    require_init(env)?;
    Ok(env
        .storage()
        .instance()
        .get(&DataKey::ReserveRatioBps)
        .unwrap_or(BPS))
}

fn add_total_collateral(env: &Env, amount: i128) -> Result<(), StablecoinError> {
    let current: i128 = env
        .storage()
        .instance()
        .get(&DataKey::TotalCollateral)
        .unwrap_or(0);
    let next = current
        .checked_add(amount)
        .ok_or(StablecoinError::Overflow)?;
    env.storage()
        .instance()
        .set(&DataKey::TotalCollateral, &next);
    Ok(())
}

fn sub_total_collateral(env: &Env, amount: i128) -> Result<(), StablecoinError> {
    let current: i128 = env
        .storage()
        .instance()
        .get(&DataKey::TotalCollateral)
        .unwrap_or(0);
    if amount > current {
        return Err(StablecoinError::InsufficientCollateral);
    }
    env.storage()
        .instance()
        .set(&DataKey::TotalCollateral, &(current - amount));
    Ok(())
}

#[contract]
pub struct StablecoinContract;

#[contractimpl]
impl StablecoinContract {
    pub fn initialize(
        env: Env,
        admin: Address,
        stablecoin_token: Address,
        collateral_token: Address,
        config: FractionalReserveConfig,
    ) -> Result<(), StablecoinError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(StablecoinError::AlreadyInitialized);
        }

        admin.require_auth();

        if !(0..=BPS).contains(&config.reserve_ratio_bps) {
            return Err(StablecoinError::InvalidParameter);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::StablecoinToken, &stablecoin_token);
        env.storage()
            .instance()
            .set(&DataKey::CollateralToken, &collateral_token);
        env.storage()
            .instance()
            .set(&DataKey::ReserveRatioBps, &config.reserve_ratio_bps);
        env.storage().instance().set(&DataKey::Shutdown, &false);
        env.storage()
            .instance()
            .set(&DataKey::TotalCollateral, &0i128);

        env.events().publish(
            (Symbol::new(&env, "stablecoin_initialized"),),
            (
                admin,
                stablecoin_token,
                collateral_token,
                config.reserve_ratio_bps,
            ),
        );

        Ok(())
    }

    pub fn set_shutdown(env: Env, caller: Address, shutdown: bool) -> Result<(), StablecoinError> {
        caller.require_auth();
        require_admin(&env, &caller)?;
        env.storage().instance().set(&DataKey::Shutdown, &shutdown);
        env.events()
            .publish((Symbol::new(&env, "shutdown_set"), caller), shutdown);
        Ok(())
    }

    pub fn set_reserve_ratio(
        env: Env,
        caller: Address,
        reserve_ratio_bps: i128,
    ) -> Result<(), StablecoinError> {
        caller.require_auth();
        require_admin(&env, &caller)?;
        if !(0..=BPS).contains(&reserve_ratio_bps) {
            return Err(StablecoinError::InvalidParameter);
        }
        env.storage()
            .instance()
            .set(&DataKey::ReserveRatioBps, &reserve_ratio_bps);
        env.events().publish(
            (Symbol::new(&env, "reserve_ratio_set"), caller),
            reserve_ratio_bps,
        );
        Ok(())
    }

    /// Deposit collateral tokens into the contract.
    pub fn deposit_collateral(
        env: Env,
        user: Address,
        amount: i128,
    ) -> Result<(), StablecoinError> {
        require_init(&env)?;
        user.require_auth();
        if is_shutdown(&env) {
            return Err(StablecoinError::Shutdown);
        }
        if amount <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        let collateral = collateral_token(&env)?;
        TokenClient::new(&env, &collateral).transfer(
            &user,
            env.current_contract_address(),
            &amount,
        );
        add_total_collateral(&env, amount)?;

        env.events()
            .publish((Symbol::new(&env, "collateral_deposited"), user), amount);
        Ok(())
    }

    /// Mint stablecoin against deposited collateral using the configured fractional reserve ratio.
    ///
    /// This implementation is intentionally conservative: users may mint up to
    /// `collateral_amount * reserve_ratio_bps / 10_000`.
    pub fn mint_from_collateral(
        env: Env,
        user: Address,
        collateral_amount: i128,
    ) -> Result<i128, StablecoinError> {
        require_init(&env)?;
        user.require_auth();
        if is_shutdown(&env) {
            return Err(StablecoinError::Shutdown);
        }
        if collateral_amount <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        let rr = reserve_ratio_bps(&env)?;
        let mint_amount = collateral_amount
            .checked_mul(rr)
            .ok_or(StablecoinError::Overflow)?
            / BPS;

        if mint_amount <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        let stable = stablecoin_token(&env)?;
        StellarAssetClient::new(&env, &stable).mint(&user, &mint_amount);

        env.events().publish(
            (Symbol::new(&env, "stablecoin_minted"), user),
            (collateral_amount, mint_amount),
        );

        Ok(mint_amount)
    }

    /// Burn stablecoin and redeem proportional collateral.
    pub fn burn_and_redeem(
        env: Env,
        user: Address,
        burn_amount: i128,
    ) -> Result<i128, StablecoinError> {
        require_init(&env)?;
        user.require_auth();
        if is_shutdown(&env) {
            return Err(StablecoinError::Shutdown);
        }
        if burn_amount <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        // Simple redemption rule: 1:1 collateral payout, bounded by actual collateral held.
        let collateral_out = burn_amount;
        sub_total_collateral(&env, collateral_out)?;

        let stable = stablecoin_token(&env)?;
        TokenClient::new(&env, &stable).burn(&user, &burn_amount);

        let collateral = collateral_token(&env)?;
        TokenClient::new(&env, &collateral).transfer(
            &env.current_contract_address(),
            &user,
            &collateral_out,
        );

        env.events().publish(
            (Symbol::new(&env, "stablecoin_redeemed"), user),
            (burn_amount, collateral_out),
        );

        Ok(collateral_out)
    }

    pub fn get_config(env: Env) -> Result<FractionalReserveConfig, StablecoinError> {
        Ok(FractionalReserveConfig {
            reserve_ratio_bps: reserve_ratio_bps(&env)?,
        })
    }

    pub fn get_total_collateral(env: Env) -> Result<i128, StablecoinError> {
        require_init(&env)?;
        Ok(env
            .storage()
            .instance()
            .get(&DataKey::TotalCollateral)
            .unwrap_or(0))
    }
}
