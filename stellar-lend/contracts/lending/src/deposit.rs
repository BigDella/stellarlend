pub use crate::events::VaultDepositEvent;

/// Backward-compatible name for vault deposit events (see [`VaultDepositEvent`]).
#[allow(dead_code)]
pub type DepositEvent = VaultDepositEvent;

use crate::pause::{self, PauseType};
use soroban_sdk::{contracterror, contracttype, token, Address, Env};

/// Errors that can occur during deposit operations
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum DepositError {
    InvalidAmount = 1,
    DepositPaused = 2,
    Overflow = 3,
    AssetNotSupported = 4,
    ExceedsDepositCap = 5,
    Unauthorized = 6,
}

/// Storage keys for deposit-related data
#[contracttype]
#[derive(Clone)]
#[allow(clippy::enum_variant_names)]
pub enum DepositDataKey {
    UserCollateral(Address),
    TotalAmount,
    CapAmount,
    MinAmount,
    AssetAccountedAmount(Address),
    AssetTotalShares(Address),
    DonationQuarantinedAmount(Address),
    DonationAlert(Address),
    DonationDefenseConfig,
    DonationReport(Address),
}

/// User deposit position
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct DepositCollateral {
    pub amount: i128,
    pub asset: Address,
    pub last_deposit_time: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DonationDefenseConfig {
    pub virtual_assets: i128,
    pub virtual_shares: i128,
    pub max_unaccounted_bps: i128,
    pub min_deposit_amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DonationReport {
    pub asset: Address,
    pub accounted_balance: i128,
    pub observed_balance: i128,
    pub quarantined_balance: i128,
    pub new_unaccounted_balance: i128,
    pub virtual_share_price_bps: i128,
    pub donation_detected: bool,
    pub timestamp: u64,
}

const BPS_SCALE: i128 = 10_000;
const DEFAULT_VIRTUAL_ASSETS: i128 = 1_000;
const DEFAULT_VIRTUAL_SHARES: i128 = 1_000;
const DEFAULT_MAX_UNACCOUNTED_BPS: i128 = 100;

/// Deposit collateral into the protocol
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - The depositor's address
/// * `asset` - The collateral asset address
/// * `amount` - The amount to deposit
///
/// # Returns
/// Returns the updated collateral balance on success
pub fn deposit(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
) -> Result<i128, DepositError> {
    deposit_with_auth(env, user, asset, amount, true)
}

pub(crate) fn deposit_with_auth(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    require_auth: bool,
) -> Result<i128, DepositError> {
    if require_auth {
        user.require_auth();
    }

    if pause::is_paused(env, PauseType::Deposit) {
        return Err(DepositError::DepositPaused);
    }

    if amount <= 0 {
        return Err(DepositError::InvalidAmount);
    }

    let min_deposit = get_effective_min_deposit_amount(env);
    if amount < min_deposit {
        return Err(DepositError::InvalidAmount);
    }

    let total_deposits = get_total_deposits(env);
    let deposit_cap = get_deposit_cap(env);
    let new_total = total_deposits
        .checked_add(amount)
        .ok_or(DepositError::Overflow)?;

    if new_total > deposit_cap {
        return Err(DepositError::ExceedsDepositCap);
    }

    add_asset_accounting(env, &asset, amount)?;

    let mut position = get_deposit_position(env, &user, &asset);
    position.amount = position
        .amount
        .checked_add(amount)
        .ok_or(DepositError::Overflow)?;
    position.last_deposit_time = env.ledger().timestamp();
    position.asset = asset.clone();

    save_deposit_position(env, &user, &position);
    set_total_deposits(env, new_total);
    emit_deposit_event(env, user, asset, amount, position.amount);

    Ok(position.amount)
}

/// Initialize deposit settings
pub fn initialize_deposit_settings(
    env: &Env,
    deposit_cap: i128,
    min_deposit_amount: i128,
) -> Result<(), DepositError> {
    env.storage()
        .persistent()
        .set(&DepositDataKey::CapAmount, &deposit_cap);
    env.storage()
        .persistent()
        .set(&DepositDataKey::MinAmount, &min_deposit_amount);
    Ok(())
}

pub fn set_donation_defense_config(
    env: &Env,
    config: DonationDefenseConfig,
) -> Result<(), DepositError> {
    validate_donation_config(&config)?;
    env.storage()
        .persistent()
        .set(&DepositDataKey::DonationDefenseConfig, &config);
    Ok(())
}

pub fn get_donation_defense_config(env: &Env) -> DonationDefenseConfig {
    env.storage()
        .persistent()
        .get(&DepositDataKey::DonationDefenseConfig)
        .unwrap_or_else(default_donation_config)
}

pub fn sync_donation_balance(env: &Env, asset: &Address) -> Result<DonationReport, DepositError> {
    let token_client = token::Client::new(env, asset);
    let observed_balance = token_client.balance(&env.current_contract_address());
    sync_observed_balance(env, asset, observed_balance)
}

pub fn sync_observed_balance(
    env: &Env,
    asset: &Address,
    observed_balance: i128,
) -> Result<DonationReport, DepositError> {
    if observed_balance < 0 {
        return Err(DepositError::InvalidAmount);
    }

    let accounted_balance = get_asset_accounted_amount(env, asset);
    let quarantined_balance = get_donation_quarantined_amount(env, asset);
    let expected_balance = accounted_balance
        .checked_add(quarantined_balance)
        .ok_or(DepositError::Overflow)?;

    let new_unaccounted_balance = if observed_balance > expected_balance {
        observed_balance
            .checked_sub(expected_balance)
            .ok_or(DepositError::Overflow)?
    } else {
        0
    };

    let threshold = donation_detection_threshold(env, accounted_balance)?;
    let donation_detected = new_unaccounted_balance > threshold;
    let updated_quarantine = if new_unaccounted_balance > 0 {
        quarantined_balance
            .checked_add(new_unaccounted_balance)
            .ok_or(DepositError::Overflow)?
    } else {
        quarantined_balance
    };

    if new_unaccounted_balance > 0 {
        env.storage().persistent().set(
            &DepositDataKey::DonationQuarantinedAmount(asset.clone()),
            &updated_quarantine,
        );
    }
    if donation_detected {
        env.storage()
            .persistent()
            .set(&DepositDataKey::DonationAlert(asset.clone()), &true);
    }

    let report = DonationReport {
        asset: asset.clone(),
        accounted_balance,
        observed_balance,
        quarantined_balance: updated_quarantine,
        new_unaccounted_balance,
        virtual_share_price_bps: get_virtual_share_price_bps(env, asset)?,
        donation_detected,
        timestamp: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&DepositDataKey::DonationReport(asset.clone()), &report);
    Ok(report)
}

pub fn acknowledge_donation(env: &Env, asset: &Address) {
    env.storage()
        .persistent()
        .set(&DepositDataKey::DonationAlert(asset.clone()), &false);
}

pub fn get_donation_report(env: &Env, asset: &Address) -> Option<DonationReport> {
    env.storage()
        .persistent()
        .get(&DepositDataKey::DonationReport(asset.clone()))
}

pub fn is_donation_detected(env: &Env, asset: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DepositDataKey::DonationAlert(asset.clone()))
        .unwrap_or(false)
}

pub fn get_virtual_share_price_bps(env: &Env, asset: &Address) -> Result<i128, DepositError> {
    let config = get_donation_defense_config(env);
    let accounted_assets = get_asset_accounted_amount(env, asset)
        .checked_add(config.virtual_assets)
        .ok_or(DepositError::Overflow)?;
    let total_shares = get_asset_total_shares(env, asset)
        .checked_add(config.virtual_shares)
        .ok_or(DepositError::Overflow)?;

    if total_shares <= 0 {
        return Ok(BPS_SCALE);
    }

    accounted_assets
        .checked_mul(BPS_SCALE)
        .ok_or(DepositError::Overflow)?
        .checked_div(total_shares)
        .ok_or(DepositError::Overflow)
}

pub(crate) fn subtract_asset_accounting(
    env: &Env,
    asset: &Address,
    amount: i128,
) -> Result<(), DepositError> {
    if amount < 0 {
        return Err(DepositError::InvalidAmount);
    }

    let accounted = get_asset_accounted_amount(env, asset);
    let shares = get_asset_total_shares(env, asset);
    env.storage().persistent().set(
        &DepositDataKey::AssetAccountedAmount(asset.clone()),
        &accounted.checked_sub(amount).unwrap_or(0),
    );
    env.storage().persistent().set(
        &DepositDataKey::AssetTotalShares(asset.clone()),
        &shares.checked_sub(amount).unwrap_or(0),
    );
    Ok(())
}

pub fn get_user_collateral(env: &Env, user: &Address, asset: &Address) -> DepositCollateral {
    get_deposit_position(env, user, asset)
}

fn add_asset_accounting(env: &Env, asset: &Address, amount: i128) -> Result<(), DepositError> {
    let accounted = get_asset_accounted_amount(env, asset)
        .checked_add(amount)
        .ok_or(DepositError::Overflow)?;
    let shares = get_asset_total_shares(env, asset)
        .checked_add(amount)
        .ok_or(DepositError::Overflow)?;

    env.storage().persistent().set(
        &DepositDataKey::AssetAccountedAmount(asset.clone()),
        &accounted,
    );
    env.storage()
        .persistent()
        .set(&DepositDataKey::AssetTotalShares(asset.clone()), &shares);
    Ok(())
}

fn get_asset_accounted_amount(env: &Env, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DepositDataKey::AssetAccountedAmount(asset.clone()))
        .unwrap_or(0)
}

fn get_asset_total_shares(env: &Env, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DepositDataKey::AssetTotalShares(asset.clone()))
        .unwrap_or(0)
}

fn get_donation_quarantined_amount(env: &Env, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DepositDataKey::DonationQuarantinedAmount(asset.clone()))
        .unwrap_or(0)
}

fn default_donation_config() -> DonationDefenseConfig {
    DonationDefenseConfig {
        virtual_assets: DEFAULT_VIRTUAL_ASSETS,
        virtual_shares: DEFAULT_VIRTUAL_SHARES,
        max_unaccounted_bps: DEFAULT_MAX_UNACCOUNTED_BPS,
        min_deposit_amount: 0,
    }
}

fn validate_donation_config(config: &DonationDefenseConfig) -> Result<(), DepositError> {
    if config.virtual_assets < 0
        || config.virtual_shares <= 0
        || config.max_unaccounted_bps < 0
        || config.max_unaccounted_bps > BPS_SCALE
        || config.min_deposit_amount < 0
    {
        return Err(DepositError::InvalidAmount);
    }
    Ok(())
}

fn get_effective_min_deposit_amount(env: &Env) -> i128 {
    let base_min = get_min_deposit_amount(env);
    let donation_min = get_donation_defense_config(env).min_deposit_amount;
    if donation_min > base_min {
        donation_min
    } else {
        base_min
    }
}

fn donation_detection_threshold(env: &Env, accounted_balance: i128) -> Result<i128, DepositError> {
    let config = get_donation_defense_config(env);
    accounted_balance
        .checked_mul(config.max_unaccounted_bps)
        .ok_or(DepositError::Overflow)?
        .checked_div(BPS_SCALE)
        .ok_or(DepositError::Overflow)
}

fn get_deposit_position(env: &Env, user: &Address, asset: &Address) -> DepositCollateral {
    env.storage()
        .persistent()
        .get(&DepositDataKey::UserCollateral(user.clone()))
        .unwrap_or(DepositCollateral {
            amount: 0,
            asset: asset.clone(),
            last_deposit_time: env.ledger().timestamp(),
        })
}

fn save_deposit_position(env: &Env, user: &Address, position: &DepositCollateral) {
    env.storage()
        .persistent()
        .set(&DepositDataKey::UserCollateral(user.clone()), position);
}

fn get_total_deposits(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&DepositDataKey::TotalAmount)
        .unwrap_or(0)
}

fn set_total_deposits(env: &Env, amount: i128) {
    env.storage()
        .persistent()
        .set(&DepositDataKey::TotalAmount, &amount);
}

fn get_deposit_cap(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&DepositDataKey::CapAmount)
        .unwrap_or(i128::MAX)
}

fn get_min_deposit_amount(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&DepositDataKey::MinAmount)
        .unwrap_or(0)
}

fn emit_deposit_event(env: &Env, user: Address, asset: Address, amount: i128, new_balance: i128) {
    VaultDepositEvent {
        user,
        asset,
        amount,
        new_balance,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}
