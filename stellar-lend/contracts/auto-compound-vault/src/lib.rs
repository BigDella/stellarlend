#![no_std]
use soroban_sdk::{contract, contracterror, contractevent, contractimpl, contracttype, token::StellarAssetClient, Address, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VaultError {
    Unauthorized = 1,
    AlreadyInitialized = 2,
    NotInitialized = 3,
    InvalidAmount = 4,
    InsufficientBalance = 5,
    InsufficientShares = 6,
    SlippageExceeded = 7,
    Overflow = 8,
    HarvestAlreadyCalled = 9,
    MinHarvestIntervalNotMet = 10,
    PerformanceFeeExceedsMax = 11,
    ManagementFeeExceedsMax = 12,
    DepositPaused = 13,
    WithdrawPaused = 14,
    VaultNotActive = 15,
    ShareMintFailed = 16,
    ShareBurnFailed = 17,
    NoRewardsToHarvest = 18,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct VaultConfig {
    pub performance_fee_bps: u32,
    pub management_fee_bps: u32,
    pub harvest_interval_secs: u64,
    pub slippage_tolerance_bps: u32,
    pub deposit_paused: bool,
    pub withdraw_paused: bool,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct VaultSnapshot {
    pub total_assets: i128,
    pub total_shares: i128,
    pub share_price: i128,
    pub last_harvested_at: u64,
    pub accrued_management_fees: i128,
    pub accrued_performance_fees: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    Config,
    ShareToken,
    UnderlyingAsset,
    RewardAsset,
    TotalAssets,
    TotalShares,
    LastHarvestedAt,
    AccruedManagementFees,
    AccruedPerformanceFees,
    VaultSnapshot,
    UserDeposits(Address),
    UserLastDepositAt(Address),
    HarvestCaller,
    MinHarvestInterval,
    MaxPerformanceFeeBps,
    MaxManagementFeeBps,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct DepositEvent {
    pub user: Address,
    pub asset_amount: i128,
    pub shares_minted: i128,
    pub share_price: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct WithdrawEvent {
    pub user: Address,
    pub shares_burned: i128,
    pub asset_amount: i128,
    pub share_price: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct HarvestEvent {
    pub rewards_claimed: i128,
    pub rewards_reinvested: i128,
    pub performance_fee: i128,
    pub new_total_assets: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct FeeCollectedEvent {
    pub fee_type: u32,
    pub amount: i128,
    pub timestamp: u64,
}

const MAX_BPS: u32 = 10_000;
const SHARE_PRECISION: i128 = 1_000_000_000;

#[contract]
pub struct AutoCompoundVault;

#[contractimpl]
impl AutoCompoundVault {
    pub fn initialize(
        env: Env,
        admin: Address,
        share_token: Address,
        underlying_asset: Address,
        reward_asset: Address,
        config: VaultConfig,
    ) -> Result<(), VaultError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(VaultError::AlreadyInitialized);
        }
        admin.require_auth();

        if config.performance_fee_bps > 2_000 {
            return Err(VaultError::PerformanceFeeExceedsMax);
        }
        if config.management_fee_bps > 500 {
            return Err(VaultError::ManagementFeeExceedsMax);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::ShareToken, &share_token);
        env.storage()
            .instance()
            .set(&DataKey::UnderlyingAsset, &underlying_asset);
        env.storage()
            .instance()
            .set(&DataKey::RewardAsset, &reward_asset);
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage().instance().set(&DataKey::TotalAssets, &0i128);
        env.storage().instance().set(&DataKey::TotalShares, &0i128);
        env.storage()
            .instance()
            .set(&DataKey::LastHarvestedAt, &0u64);
        env.storage()
            .instance()
            .set(&DataKey::AccruedManagementFees, &0i128);
        env.storage()
            .instance()
            .set(&DataKey::AccruedPerformanceFees, &0i128);
        env.storage()
            .instance()
            .set(&DataKey::MinHarvestInterval, &3600u64);
        env.storage()
            .instance()
            .set(&DataKey::MaxPerformanceFeeBps, &2_000u32);
        env.storage()
            .instance()
            .set(&DataKey::MaxManagementFeeBps, &500u32);
        env.storage()
            .instance()
            .set(&DataKey::HarvestCaller, &admin);
        Ok(())
    }

    pub fn set_config(
        env: Env,
        admin: Address,
        config: VaultConfig,
    ) -> Result<(), VaultError> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(VaultError::Unauthorized)?;
        if admin != stored_admin {
            return Err(VaultError::Unauthorized);
        }
        admin.require_auth();

        if config.performance_fee_bps > 2_000 {
            return Err(VaultError::PerformanceFeeExceedsMax);
        }
        if config.management_fee_bps > 500 {
            return Err(VaultError::ManagementFeeExceedsMax);
        }

        env.storage().instance().set(&DataKey::Config, &config);
        Ok(())
    }

    pub fn get_config(env: Env) -> VaultConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .unwrap()
    }

    pub fn deposit(
        env: Env,
        user: Address,
        amount: i128,
        min_shares: i128,
    ) -> Result<i128, VaultError> {
        user.require_auth();

        let config: VaultConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(VaultError::NotInitialized)?;

        if config.deposit_paused {
            return Err(VaultError::DepositPaused);
        }
        if amount <= 0 {
            return Err(VaultError::InvalidAmount);
        }

        let total_assets: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalAssets)
            .unwrap_or(0);
        let total_shares: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalShares)
            .unwrap_or(0);

        let shares = if total_shares == 0 || total_assets == 0 {
            amount
        } else {
            amount
                .checked_mul(total_shares)
                .ok_or(VaultError::Overflow)?
                .checked_div(total_assets)
                .ok_or(VaultError::Overflow)?
        };

        if shares < min_shares {
            return Err(VaultError::SlippageExceeded);
        }

        let new_total_assets = total_assets
            .checked_add(amount)
            .ok_or(VaultError::Overflow)?;
        let new_total_shares = total_shares
            .checked_add(shares)
            .ok_or(VaultError::Overflow)?;

        let share_price = if new_total_shares > 0 {
            new_total_assets
                .checked_mul(SHARE_PRECISION)
                .ok_or(VaultError::Overflow)?
                .checked_div(new_total_shares)
                .ok_or(VaultError::Overflow)?
        } else {
            SHARE_PRECISION
        };

        env.storage()
            .instance()
            .set(&DataKey::TotalAssets, &new_total_assets);
        env.storage()
            .instance()
            .set(&DataKey::TotalShares, &new_total_shares);

        let share_client = StellarAssetClient::new(&env, &Self::get_share_token(&env));
        share_client.mint(&user, &shares);

        let mut user_deposits: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::UserDeposits(user.clone()))
            .unwrap_or(0);
        user_deposits = user_deposits
            .checked_add(amount)
            .ok_or(VaultError::Overflow)?;
        env.storage()
            .persistent()
            .set(&DataKey::UserDeposits(user.clone()), &user_deposits);
        env.storage()
            .persistent()
            .set(&DataKey::UserLastDepositAt(user.clone()), &env.ledger().timestamp());

        DepositEvent {
            user,
            asset_amount: amount,
            shares_minted: shares,
            share_price,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(shares)
    }

    pub fn withdraw(
        env: Env,
        user: Address,
        shares: i128,
        min_assets: i128,
    ) -> Result<i128, VaultError> {
        user.require_auth();

        let config: VaultConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(VaultError::NotInitialized)?;

        if config.withdraw_paused {
            return Err(VaultError::WithdrawPaused);
        }
        if shares <= 0 {
            return Err(VaultError::InvalidAmount);
        }

        let total_assets: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalAssets)
            .unwrap_or(0);
        let total_shares: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalShares)
            .unwrap_or(0);

        if shares > total_shares {
            return Err(VaultError::InsufficientShares);
        }

        let user_share_balance = Self::get_share_balance(&env, &user);
        if shares > user_share_balance {
            return Err(VaultError::InsufficientShares);
        }

        let assets = shares
            .checked_mul(total_assets)
            .ok_or(VaultError::Overflow)?
            .checked_div(total_shares)
            .ok_or(VaultError::Overflow)?;

        if assets < min_assets {
            return Err(VaultError::SlippageExceeded);
        }

        if assets > total_assets {
            return Err(VaultError::InsufficientBalance);
        }

        let new_total_assets = total_assets
            .checked_sub(assets)
            .ok_or(VaultError::Overflow)?;
        let new_total_shares = total_shares
            .checked_sub(shares)
            .ok_or(VaultError::Overflow)?;

        let share_price = if new_total_shares > 0 {
            new_total_assets
                .checked_mul(SHARE_PRECISION)
                .ok_or(VaultError::Overflow)?
                .checked_div(new_total_shares)
                .ok_or(VaultError::Overflow)?
        } else {
            SHARE_PRECISION
        };

        env.storage()
            .instance()
            .set(&DataKey::TotalAssets, &new_total_assets);
        env.storage()
            .instance()
            .set(&DataKey::TotalShares, &new_total_shares);

        let share_client = StellarAssetClient::new(&env, &Self::get_share_token(&env));
        share_client.burn(&user, &shares);

        WithdrawEvent {
            user,
            shares_burned: shares,
            asset_amount: assets,
            share_price,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(assets)
    }

    pub fn harvest(
        env: Env,
        caller: Address,
        min_rewards: i128,
    ) -> Result<i128, VaultError> {
        caller.require_auth();

        let config: VaultConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(VaultError::NotInitialized)?;

        let last_harvest: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LastHarvestedAt)
            .unwrap_or(0);
        let now = env.ledger().timestamp();

        if last_harvest > 0 && (now - last_harvest) < config.harvest_interval_secs {
            return Err(VaultError::MinHarvestIntervalNotMet);
        }

        let total_assets: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalAssets)
            .unwrap_or(0);

        let rewards_claimed = total_assets
            .checked_mul(100)
            .ok_or(VaultError::Overflow)?
            .checked_div(10_000)
            .ok_or(VaultError::Overflow)?;

        if rewards_claimed < min_rewards {
            return Err(VaultError::NoRewardsToHarvest);
        }

        let performance_fee = rewards_claimed
            .checked_mul(config.performance_fee_bps as i128)
            .ok_or(VaultError::Overflow)?
            .checked_div(MAX_BPS as i128)
            .ok_or(VaultError::Overflow)?;

        let rewards_reinvested = rewards_claimed
            .checked_sub(performance_fee)
            .ok_or(VaultError::Overflow)?;

        let new_total_assets = total_assets
            .checked_add(rewards_reinvested)
            .ok_or(VaultError::Overflow)?;

        let accrued_perf_fees: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AccruedPerformanceFees)
            .unwrap_or(0);
        let new_accrued = accrued_perf_fees
            .checked_add(performance_fee)
            .ok_or(VaultError::Overflow)?;

        env.storage()
            .instance()
            .set(&DataKey::TotalAssets, &new_total_assets);
        env.storage()
            .instance()
            .set(&DataKey::LastHarvestedAt, &now);
        env.storage()
            .instance()
            .set(&DataKey::AccruedPerformanceFees, &new_accrued);

        HarvestEvent {
            rewards_claimed,
            rewards_reinvested,
            performance_fee,
            new_total_assets,
            timestamp: now,
        }
        .publish(&env);

        Ok(rewards_reinvested)
    }

    pub fn preview_deposit(env: Env, amount: i128) -> Result<i128, VaultError> {
        if amount <= 0 {
            return Err(VaultError::InvalidAmount);
        }

        let total_assets: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalAssets)
            .unwrap_or(0);
        let total_shares: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalShares)
            .unwrap_or(0);

        if total_shares == 0 || total_assets == 0 {
            return Ok(amount);
        }

        let shares = amount
            .checked_mul(total_shares)
            .ok_or(VaultError::Overflow)?
            .checked_div(total_assets)
            .ok_or(VaultError::Overflow)?;

        Ok(shares)
    }

    pub fn preview_withdraw(env: Env, shares: i128) -> Result<i128, VaultError> {
        if shares <= 0 {
            return Err(VaultError::InvalidAmount);
        }

        let total_assets: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalAssets)
            .unwrap_or(0);
        let total_shares: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalShares)
            .unwrap_or(0);

        if total_shares == 0 {
            return Ok(0);
        }

        let assets = shares
            .checked_mul(total_assets)
            .ok_or(VaultError::Overflow)?
            .checked_div(total_shares)
            .ok_or(VaultError::Overflow)?;

        Ok(assets)
    }

    pub fn get_vault_snapshot(env: Env) -> VaultSnapshot {
        let total_assets: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalAssets)
            .unwrap_or(0);
        let total_shares: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalShares)
            .unwrap_or(0);
        let last_harvested_at: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LastHarvestedAt)
            .unwrap_or(0);
        let accrued_mgmt_fees: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AccruedManagementFees)
            .unwrap_or(0);
        let accrued_perf_fees: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AccruedPerformanceFees)
            .unwrap_or(0);

        let share_price = if total_shares > 0 {
            total_assets
                .checked_mul(SHARE_PRECISION)
                .unwrap_or(SHARE_PRECISION)
                .checked_div(total_shares)
                .unwrap_or(SHARE_PRECISION)
        } else {
            SHARE_PRECISION
        };

        VaultSnapshot {
            total_assets,
            total_shares,
            share_price,
            last_harvested_at,
            accrued_management_fees: accrued_mgmt_fees,
            accrued_performance_fees: accrued_perf_fees,
        }
    }

    pub fn get_share_price(env: Env) -> i128 {
        let total_assets: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalAssets)
            .unwrap_or(0);
        let total_shares: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalShares)
            .unwrap_or(0);

        if total_shares == 0 {
            return SHARE_PRECISION;
        }

        total_assets
            .checked_mul(SHARE_PRECISION)
            .unwrap_or(SHARE_PRECISION)
            .checked_div(total_shares)
            .unwrap_or(SHARE_PRECISION)
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    fn get_share_token(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::ShareToken)
            .unwrap()
    }

    fn get_share_balance(env: &Env, user: &Address) -> i128 {
        let share_token: Address = env
            .storage()
            .instance()
            .get(&DataKey::ShareToken)
            .unwrap();
        let client = soroban_sdk::token::Client::new(env, &share_token);
        client.balance(user)
    }
}

#[cfg(test)]
mod test;
