#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env, symbol_short};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum FeeError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    InvalidBounds = 4,
    RateChangeExceedsLimit = 5,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FeeConfig {
    pub admin: Address,
    pub base_spread_bps: u32,
    pub base_liquidation_penalty_bps: u32,
    pub base_flash_loan_fee_bps: u32,
    pub min_fee_bps: u32,
    pub max_fee_bps: u32,
    pub max_change_per_hour_bps: u32,
    pub utilization_weight_bps: u32,
    pub volatility_weight_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FeeSnapshot {
    pub spread_fee_bps: u32,
    pub liquidation_penalty_bps: u32,
    pub flash_loan_fee_bps: u32,
    pub utilization_premium_bps: u32,
    pub volatility_premium_bps: u32,
    pub computed_at_ledger: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PoolUtilization {
    pub total_deposits: i128,
    pub total_borrows: i128,
}

#[contract]
pub struct DynamicFeeEngine;

#[contractimpl]
impl DynamicFeeEngine {
    pub fn initialize(
        env: Env,
        admin: Address,
        config: FeeConfig,
    ) -> Result<(), FeeError> {
        if env.storage().instance().has(&symbol_short!("config")) {
            return Err(FeeError::AlreadyInitialized);
        }
        admin.require_auth();

        if config.min_fee_bps > config.max_fee_bps {
            return Err(FeeError::InvalidBounds);
        }

        env.storage().instance().set(&symbol_short!("config"), &config);
        Ok(())
    }

    pub fn compute_fees(
        env: Env,
        utilization: PoolUtilization,
        volatility_bps: u32,
    ) -> Result<FeeSnapshot, FeeError> {
        let config: FeeConfig = env
            .storage()
            .instance()
            .get(&symbol_short!("config"))
            .ok_or(FeeError::NotInitialized)?;

        let util_rate_bps = if utilization.total_deposits > 0 {
            ((utilization.total_borrows * 10_000) / utilization.total_deposits) as u32
        } else {
            0
        };

        let util_premium = (util_rate_bps * config.utilization_weight_bps) / 10_000;
        let vol_premium = (volatility_bps * config.volatility_weight_bps) / 10_000;

        let raw_spread = config.base_spread_bps + util_premium + vol_premium;
        let spread_fee = raw_spread.max(config.min_fee_bps).min(config.max_fee_bps);

        let raw_liq = config.base_liquidation_penalty_bps + (vol_premium / 2);
        let liq_penalty = raw_liq.max(config.min_fee_bps).min(config.max_fee_bps);

        let raw_flash = config.base_flash_loan_fee_bps + (util_premium / 2);
        let flash_fee = raw_flash.max(config.min_fee_bps).min(config.max_fee_bps);

        let prev_key = symbol_short!("lastfee");
        if let Some(prev) = env.storage().instance().get::<_, FeeSnapshot>(&prev_key) {
            let spread_change = if spread_fee > prev.spread_fee_bps {
                spread_fee - prev.spread_fee_bps
            } else {
                prev.spread_fee_bps - spread_fee
            };

            if spread_change > config.max_change_per_hour_bps {
                let capped_spread = if spread_fee > prev.spread_fee_bps {
                    prev.spread_fee_bps + config.max_change_per_hour_bps
                } else {
                    prev.spread_fee_bps.saturating_sub(config.max_change_per_hour_bps)
                };

                let snapshot = FeeSnapshot {
                    spread_fee_bps: capped_spread.max(config.min_fee_bps).min(config.max_fee_bps),
                    liquidation_penalty_bps: liq_penalty,
                    flash_loan_fee_bps: flash_fee,
                    utilization_premium_bps: util_premium,
                    volatility_premium_bps: vol_premium,
                    computed_at_ledger: env.ledger().sequence() as u64,
                };
                env.storage().instance().set(&prev_key, &snapshot);
                return Ok(snapshot);
            }
        }

        let snapshot = FeeSnapshot {
            spread_fee_bps: spread_fee,
            liquidation_penalty_bps: liq_penalty,
            flash_loan_fee_bps: flash_fee,
            utilization_premium_bps: util_premium,
            volatility_premium_bps: vol_premium,
            computed_at_ledger: env.ledger().sequence() as u64,
        };

        env.storage().instance().set(&prev_key, &snapshot);

        env.events().publish(
            (symbol_short!("fee"), symbol_short!("update")),
            (spread_fee, liq_penalty, flash_fee),
        );

        Ok(snapshot)
    }

    pub fn get_current_fees(env: Env) -> Option<FeeSnapshot> {
        env.storage().instance().get(&symbol_short!("lastfee"))
    }

    pub fn get_config(env: Env) -> Result<FeeConfig, FeeError> {
        env.storage()
            .instance()
            .get(&symbol_short!("config"))
            .ok_or(FeeError::NotInitialized)
    }

    pub fn update_config(
        env: Env,
        caller: Address,
        new_config: FeeConfig,
    ) -> Result<(), FeeError> {
        caller.require_auth();
        let config: FeeConfig = env
            .storage()
            .instance()
            .get(&symbol_short!("config"))
            .ok_or(FeeError::NotInitialized)?;

        if caller != config.admin {
            return Err(FeeError::Unauthorized);
        }
        if new_config.min_fee_bps > new_config.max_fee_bps {
            return Err(FeeError::InvalidBounds);
        }

        env.storage().instance().set(&symbol_short!("config"), &new_config);
        Ok(())
    }
}
