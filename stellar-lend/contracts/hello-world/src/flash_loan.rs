//! # Flash Loan Module
//!
//! Provides uncollateralized flash loan functionality for the lending protocol.
//!
//! Flash loans allow users to borrow assets without collateral, provided the loan
//! (principal + fee) is repaid within the same transaction via a callback contract.
//!
//! ## Fee Structure
//! - Default fee: 9 basis points (0.09%) of the borrowed amount.
//! - Fee is configurable by the admin.
//!
//! ## Reentrancy Protection
//! An active flash loan is recorded per (user, asset) pair. A second flash loan
//! for the same pair is rejected until the first is repaid, preventing reentrancy.
//! This implementation uses a RAII guard to ensure the guard is always cleared,
//! but only after all verification steps are completed.
//!
//! ## Invariants
//! - The borrowed amount must be within configured min/max limits.
//! - The contract must have sufficient liquidity to fund the loan.
//! - Repayment must cover principal + fee in full.

#![allow(unused)]
use crate::events::{
    emit_flash_loan_initiated, emit_flash_loan_repaid, FlashLoanInitiatedEvent,
    FlashLoanRepaidEvent,
};
use soroban_sdk::{contracterror, contracttype, Address, Env, IntoVal, Map, Symbol, Val, Vec};

use crate::deposit::DepositDataKey;

/// Errors that can occur during flash loan operations
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FlashLoanError {
    /// Flash loan amount must be greater than zero
    InvalidAmount = 1,
    /// Asset address is invalid
    InvalidAsset = 2,
    /// Insufficient liquidity for flash loan
    InsufficientLiquidity = 3,
    /// Flash loan operations are currently paused
    FlashLoanPaused = 4,
    /// Flash loan not repaid within transaction
    NotRepaid = 5,
    /// Insufficient repayment amount
    InsufficientRepayment = 6,
    /// Overflow occurred during calculation
    Overflow = 7,
    /// Reentrancy detected
    Reentrancy = 8,
    /// Invalid callback
    InvalidCallback = 9,
    /// Callback execution failed
    CallbackFailed = 10,
    /// Borrow exceeds pool-relative liquidity cap
    ExceedsLiquidityCap = 11,
    /// Price impact of flash loan is too high
    ExcessivePriceImpact = 12,
    /// A concurrent flash loan is already active for this asset
    ConcurrentLoan = 13,
    /// TWAP deviation indicates price manipulation
    PriceManipulationDetected = 14,
    /// Flash loan crossed into a later ledger sequence before completion.
    Expired = 15,
    /// Combo liquidation would not cover principal, fee, and a minimum profit.
    Unprofitable = 16,
    /// Multi-asset flash loan was submitted with no legs.
    EmptyLegs = 17,
    /// Multi-asset flash loan exceeded the maximum number of legs.
    TooManyLegs = 18,
}

/// Storage keys for flash loan-related data
#[contracttype]
#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum FlashLoanDataKey {
    /// Global flash loan configuration (fee, limits)
    FlashLoanConfig,
    /// Active flash loan record for repayment tracking
    /// Key: (User Address, Asset Address)
    ActiveFlashLoan(Address, Address),
    /// Reentrancy guard lock key for flash loans
    /// Key: (User Address, Asset Address)
    FlashLoanGuard(Address, Address),
    /// Pause switches for flash loan operations
    PauseSwitches,
    /// Attack prevention config
    ManipulationConfig,
    /// Per-asset TWAP accumulator
    TwapAccumulator(Address),
    /// Per-asset concurrent loan sentinel
    AssetLoanActive(Address),
}

/// Attack-prevention configuration for flash loans.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct FlashManipulationConfig {
    /// Maximum fraction of pool liquidity borrowable in one flash loan (bps).
    pub max_borrow_liquidity_bps: i128,
    /// Maximum price impact per flash loan (bps).
    pub max_price_impact_bps: i128,
    /// Maximum TWAP-vs-spot deviation before the loan is blocked (bps).
    pub max_twap_deviation_bps: i128,
    /// Minimum TWAP samples required before the deviation check is enforced.
    pub min_twap_samples: u32,
    /// TWAP window in ledger seconds.
    pub twap_window_secs: u64,
}

/// Per-asset TWAP accumulator state.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TwapState {
    pub price_sum: i128,
    pub sample_count: u32,
    pub last_update: u64,
    pub twap: i128,
}

const BPS_DENOM: i128 = 10_000;

/// Configuration for flash loan operations
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct FlashLoanConfig {
    /// Flash loan fee in basis points (e.g., 9 = 0.09%)
    pub fee_bps: i128,
    /// Maximum allowed flash loan amount for any single asset
    pub max_amount: i128,
    /// Minimum allowed flash loan amount
    pub min_amount: i128,
}

/// Record of an active flash loan
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct FlashLoanRecord {
    /// Amount borrowed
    pub amount: i128,
    /// Fee to be repaid
    pub fee: i128,
    /// Timestamp when loan was initiated
    pub timestamp: u64,
    /// Ledger sequence when the flash loan began.
    pub sequence_number: u32,
    /// Callback contract address
    pub callback: Address,
}

const DEFAULT_FLASH_LOAN_FEE_BPS: i128 = 9;
const DEFAULT_MAX_FLASH_LOAN_AMOUNT: i128 = 1_000_000_000_000; // Example: 1M tokens
const DEFAULT_MIN_FLASH_LOAN_AMOUNT: i128 = 100; // Example: 100 tokens
/// Maximum assets in a single multi-asset flash loan (gas bound).
pub const MAX_FLASH_LOAN_LEGS: u32 = 5;
/// Default 7-decimal unit price used when an oracle quote is unavailable.
const DEFAULT_ASSET_PRICE: i128 = 10_000_000;

/// Default flash loan operational configuration.
fn get_default_config() -> FlashLoanConfig {
    FlashLoanConfig {
        fee_bps: DEFAULT_FLASH_LOAN_FEE_BPS,
        max_amount: DEFAULT_MAX_FLASH_LOAN_AMOUNT,
        min_amount: DEFAULT_MIN_FLASH_LOAN_AMOUNT,
    }
}

fn default_manipulation_config() -> FlashManipulationConfig {
    FlashManipulationConfig {
        max_borrow_liquidity_bps: 5_000,
        max_price_impact_bps: 100,
        max_twap_deviation_bps: 200,
        min_twap_samples: 3,
        twap_window_secs: 300,
    }
}

fn get_manipulation_config(env: &Env) -> FlashManipulationConfig {
    env.storage()
        .persistent()
        .get(&FlashLoanDataKey::ManipulationConfig)
        .unwrap_or_else(default_manipulation_config)
}

/// Update the attack-prevention configuration (admin only).
pub fn set_manipulation_config(
    env: &Env,
    caller: Address,
    config: FlashManipulationConfig,
) -> Result<(), FlashLoanError> {
    crate::admin::require_admin(env, &caller).map_err(|_| FlashLoanError::InvalidCallback)?;
    env.storage()
        .persistent()
        .set(&FlashLoanDataKey::ManipulationConfig, &config);
    Ok(())
}

/// Record a spot price into the per-asset TWAP accumulator.
pub fn record_price_sample(env: &Env, asset: &Address, spot_price: i128) {
    if spot_price <= 0 {
        return;
    }
    let key = FlashLoanDataKey::TwapAccumulator(asset.clone());
    let mut acc: TwapState = env.storage().persistent().get(&key).unwrap_or(TwapState {
        price_sum: spot_price,
        sample_count: 0,
        last_update: 0,
        twap: spot_price,
    });
    let cfg = get_manipulation_config(env);
    let now = env.ledger().timestamp();
    if now.saturating_sub(acc.last_update) > cfg.twap_window_secs {
        acc.price_sum = spot_price;
        acc.sample_count = 1;
        acc.twap = spot_price;
    } else {
        acc.price_sum = acc.price_sum.saturating_add(spot_price);
        acc.sample_count = acc.sample_count.saturating_add(1);
        acc.twap = acc
            .price_sum
            .checked_div(acc.sample_count as i128)
            .unwrap_or(acc.twap);
    }
    acc.last_update = now;
    env.storage().persistent().set(&key, &acc);
}

fn check_twap_deviation(
    env: &Env,
    asset: &Address,
    spot_price: i128,
) -> Result<(), FlashLoanError> {
    if spot_price <= 0 {
        return Ok(());
    }
    let cfg = get_manipulation_config(env);
    let acc: Option<TwapState> = env
        .storage()
        .persistent()
        .get(&FlashLoanDataKey::TwapAccumulator(asset.clone()));
    let acc = match acc {
        Some(a) if a.sample_count >= cfg.min_twap_samples => a,
        _ => return Ok(()),
    };
    if acc.twap <= 0 {
        return Ok(());
    }
    let diff = (spot_price - acc.twap).abs();
    let deviation_bps = diff
        .checked_mul(BPS_DENOM)
        .ok_or(FlashLoanError::Overflow)?
        .checked_div(acc.twap)
        .ok_or(FlashLoanError::Overflow)?;
    if deviation_bps > cfg.max_twap_deviation_bps {
        return Err(FlashLoanError::PriceManipulationDetected);
    }
    Ok(())
}

fn check_liquidity_cap(env: &Env, pool_balance: i128, amount: i128) -> Result<(), FlashLoanError> {
    let cfg = get_manipulation_config(env);
    let max_borrow = pool_balance
        .checked_mul(cfg.max_borrow_liquidity_bps)
        .ok_or(FlashLoanError::Overflow)?
        .checked_div(BPS_DENOM)
        .ok_or(FlashLoanError::Overflow)?;
    if amount > max_borrow {
        return Err(FlashLoanError::ExceedsLiquidityCap);
    }
    Ok(())
}

fn check_price_impact(env: &Env, pool_balance: i128, amount: i128) -> Result<(), FlashLoanError> {
    if pool_balance <= 0 {
        return Err(FlashLoanError::ExcessivePriceImpact);
    }
    let cfg = get_manipulation_config(env);
    let denom = pool_balance.saturating_add(amount);
    let impact_bps = amount
        .checked_mul(BPS_DENOM)
        .ok_or(FlashLoanError::Overflow)?
        .checked_div(denom)
        .ok_or(FlashLoanError::Overflow)?;
    if impact_bps > cfg.max_price_impact_bps {
        return Err(FlashLoanError::ExcessivePriceImpact);
    }
    Ok(())
}

fn acquire_asset_guard(env: &Env, asset: &Address) -> Result<(), FlashLoanError> {
    let key = FlashLoanDataKey::AssetLoanActive(asset.clone());
    if env
        .storage()
        .instance()
        .get::<_, bool>(&key)
        .unwrap_or(false)
    {
        return Err(FlashLoanError::ConcurrentLoan);
    }
    env.storage().instance().set(&key, &true);
    Ok(())
}

fn release_asset_guard(env: &Env, asset: &Address) {
    let key = FlashLoanDataKey::AssetLoanActive(asset.clone());
    env.storage().instance().set(&key, &false);
}

/// Get flash loan configuration
fn get_flash_loan_config(env: &Env) -> FlashLoanConfig {
    let config_key = FlashLoanDataKey::FlashLoanConfig;
    env.storage()
        .persistent()
        .get::<FlashLoanDataKey, FlashLoanConfig>(&config_key)
        .unwrap_or_else(get_default_config)
}

/// Calculate flash loan fee
fn calculate_flash_loan_fee(env: &Env, amount: i128) -> Result<i128, FlashLoanError> {
    let config = get_flash_loan_config(env);

    // Fee = amount * fee_bps / 10000
    amount
        .checked_mul(config.fee_bps)
        .ok_or(FlashLoanError::Overflow)?
        .checked_div(10000)
        .ok_or(FlashLoanError::Overflow)
}

/// Check if flash loan is active
fn is_flash_loan_active(env: &Env, user: &Address, asset: &Address) -> bool {
    let loan_key: soroban_sdk::Val =
        FlashLoanDataKey::ActiveFlashLoan(user.clone(), asset.clone()).into_val(env);
    env.storage().temporary().has(&loan_key)
}

/// Record flash loan details
fn record_flash_loan(
    env: &Env,
    user: &Address,
    asset: &Address,
    amount: i128,
    fee: i128,
    callback: &Address,
) {
    let loan_key: soroban_sdk::Val =
        FlashLoanDataKey::ActiveFlashLoan(user.clone(), asset.clone()).into_val(env);
    let record = FlashLoanRecord {
        amount,
        fee,
        timestamp: env.ledger().timestamp(),
        sequence_number: env.ledger().sequence_number(),
        callback: callback.clone(),
    };
    env.storage().temporary().set(&loan_key, &record);
}

/// Clear flash loan record
fn clear_flash_loan(env: &Env, user: &Address, asset: &Address) {
    let loan_key: soroban_sdk::Val =
        FlashLoanDataKey::ActiveFlashLoan(user.clone(), asset.clone()).into_val(env);
    env.storage().temporary().remove(&loan_key);
}

/// Execute flash loan
///
/// Refactored to use a RAII guard and unified callback pattern.
/// The reentrancy guard is cleared only after successful repayment verification.
pub fn execute_flash_loan(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    callback: Address,
) -> Result<i128, FlashLoanError> {
    // 1. Validation
    if amount <= 0 {
        return Err(FlashLoanError::InvalidAmount);
    }

    let config = get_flash_loan_config(env);
    if amount < config.min_amount || amount > config.max_amount {
        return Err(FlashLoanError::InvalidAmount);
    }

    let pause_map_key = FlashLoanDataKey::PauseSwitches;
    if let Some(pause_map) = env
        .storage()
        .persistent()
        .get::<FlashLoanDataKey, Map<Symbol, bool>>(&pause_map_key)
    {
        if pause_map
            .get(Symbol::new(env, "pause_flash_loan"))
            .unwrap_or(false)
        {
            return Err(FlashLoanError::FlashLoanPaused);
        }
    }

    // 2. Preparation
    let fee = calculate_flash_loan_fee(env, amount)?;
    let total_required = amount.checked_add(fee).ok_or(FlashLoanError::Overflow)?;
    let start_sequence = env.ledger().sequence_number();

    let token_client = soroban_sdk::token::Client::new(env, &asset);
    let initial_balance = token_client.balance(&env.current_contract_address());
    if initial_balance < amount {
        return Err(FlashLoanError::InsufficientLiquidity);
    }

    // 2b. Attack-prevention checks.
    // Pool-relative liquidity cap.
    check_liquidity_cap(env, initial_balance, amount)?;
    // Price impact guard (constant-product approximation).
    check_price_impact(env, initial_balance, amount)?;
    // Per-asset concurrent loan guard — blocks sandwich attacks.
    acquire_asset_guard(env, &asset)?;

    // 3. Initiate Guards (RAII)
    // The granular guard automatically clears when execute_flash_loan finishes.

    // Granular guard prevents re-entry into flash loan for same user/asset
    // Note: We intentionally do NOT use a global guard here because the callback
    // MUST be allowed to call back into the protocol (e.g., to repay the loan).
    let lock_key: soroban_sdk::Val =
        FlashLoanDataKey::FlashLoanGuard(user.clone(), asset.clone()).into_val(env);
    let _granular_guard = crate::reentrancy::ReentrancyGuard::new_with_key(env, lock_key)
        .map_err(|_| FlashLoanError::Reentrancy)?;

    // Record the loan details for repay_flash_loan helper
    record_flash_loan(env, &user, &asset, amount, fee, &callback);

    // 4. Transfer funds to user
    token_client.transfer(&env.current_contract_address(), &callback, &amount);

    emit_flash_loan_initiated(
        env,
        FlashLoanInitiatedEvent {
            user: user.clone(),
            asset: asset.clone(),
            amount,
            fee,
            callback: callback.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );

    // 5. Invoke Callback
    let callback_symbol = Symbol::new(env, "on_flash_loan");
    let _: soroban_sdk::Val = env.invoke_contract(
        &callback,
        &callback_symbol,
        (user.clone(), asset.clone(), amount, fee).into_val(env),
    );

    if env.ledger().sequence_number() != start_sequence {
        return Err(FlashLoanError::Expired);
    }

    // 6. Repayment via Transfer From
    // Soroban blocks re-entry from the callback, so the callback cannot call `repay_flash_loan`.
    // Instead, the callback must authorize the lending contract to pull the funds
    // (principal + fee), and we execute the pull here after the callback returns.
    token_client.transfer_from(
        &env.current_contract_address(),
        &callback,
        &env.current_contract_address(),
        &total_required,
    );

    // 7. Credit fee to reserve
    if fee > 0 {
        let reserve_key = DepositDataKey::ProtocolReserve(Some(asset.clone()));
        let current_reserve = env
            .storage()
            .persistent()
            .get::<DepositDataKey, i128>(&reserve_key)
            .unwrap_or(0);
        let new_reserve = current_reserve
            .checked_add(fee)
            .ok_or(FlashLoanError::Overflow)?;
        env.storage().persistent().set(&reserve_key, &new_reserve);
    }

    // Explicitly clear the record if successfully finished (optional, but cleaner)
    // The guards will still drop and do their job.
    clear_flash_loan(env, &user, &asset);

    // Release the per-asset concurrent loan guard.
    release_asset_guard(env, &asset);

    Ok(total_required)
}

/// Repay flash loan (Helper)
///
/// Can be called by the user/callback to repay the loan.
/// Does NOT clear the guard; clearing is handled by the execute_flash_loan guard.
pub fn repay_flash_loan(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
) -> Result<(), FlashLoanError> {
    let loan_key: soroban_sdk::Val =
        FlashLoanDataKey::ActiveFlashLoan(user.clone(), asset.clone()).into_val(env);
    let record = env
        .storage()
        .temporary()
        .get::<Val, FlashLoanRecord>(&loan_key)
        .ok_or(FlashLoanError::NotRepaid)?;

    if env.ledger().sequence_number() != record.sequence_number {
        return Err(FlashLoanError::Expired);
    }

    let total_required = record
        .amount
        .checked_add(record.fee)
        .ok_or(FlashLoanError::Overflow)?;
    if amount < total_required {
        return Err(FlashLoanError::InsufficientRepayment);
    }

    let token_client = soroban_sdk::token::Client::new(env, &asset);

    // Transfer funds from user back to contract
    token_client.transfer_from(
        &env.current_contract_address(),
        &user,
        &env.current_contract_address(),
        &total_required,
    );

    emit_flash_loan_repaid(
        env,
        FlashLoanRepaidEvent {
            user: user.clone(),
            asset: asset.clone(),
            amount: record.amount,
            fee: record.fee,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}

/// Set flash loan configuration (Admin only)
pub fn set_flash_loan_config(
    env: &Env,
    caller: Address,
    new_config: FlashLoanConfig,
) -> Result<(), FlashLoanError> {
    crate::admin::require_admin(env, &caller).map_err(|_| FlashLoanError::InvalidCallback)?;

    if !(0..=10000).contains(&new_config.fee_bps) {
        return Err(FlashLoanError::InvalidAmount);
    }

    let config_key = FlashLoanDataKey::FlashLoanConfig;
    env.storage().persistent().set(&config_key, &new_config);
    Ok(())
}

/// Set flash loan fee separately (Admin only)
pub fn set_flash_loan_fee(env: &Env, caller: Address, fee_bps: i128) -> Result<(), FlashLoanError> {
    crate::admin::require_admin(env, &caller).map_err(|_| FlashLoanError::InvalidCallback)?;

    if !(0..=10000).contains(&fee_bps) {
        return Err(FlashLoanError::InvalidAmount);
    }

    let mut config = get_flash_loan_config(env);
    config.fee_bps = fee_bps;

    let config_key = FlashLoanDataKey::FlashLoanConfig;
    env.storage().persistent().set(&config_key, &config);
    Ok(())
}

// ========================================================================
// Flash loan + liquidation combo (Issue #661)
// ========================================================================

/// One asset/amount pair in a multi-asset flash loan.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct FlashLoanLeg {
    pub asset: Address,
    pub amount: i128,
}

/// Read-only profit simulation for a flash-loan-funded liquidation.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct FlashLoanLiquidationSim {
    pub debt_amount: i128,
    pub flash_fee: i128,
    pub collateral_seized: i128,
    pub incentive_amount: i128,
    pub estimated_profit: i128,
    pub profitable: bool,
    pub gas_units_estimate: u64,
}

/// Result of an executed flash-loan liquidation combo.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct FlashLoanLiquidationResult {
    pub debt_repaid: i128,
    pub collateral_seized: i128,
    pub flash_fee: i128,
    pub profit: i128,
}

fn resolve_asset_price(env: &Env, asset: &Option<Address>) -> i128 {
    let Some(addr) = asset else {
        return DEFAULT_ASSET_PRICE;
    };
    crate::oracle::get_price(env, addr).unwrap_or(DEFAULT_ASSET_PRICE)
}

fn estimate_combo_gas(legs: u32) -> u64 {
    // Base combo path avoids the callback invoke; each extra asset adds reads/writes.
    80_000u64.saturating_add(u64::from(legs.saturating_mul(12_000)))
}

/// Pre-execution profit simulation for a flash loan + liquidation combo.
///
/// Does not mutate protocol state. Returns `Unprofitable` only as a flag on
/// the result (never as an error) so callers can inspect the numbers.
pub fn simulate_flash_loan_liquidation(
    env: &Env,
    debt_asset: Option<Address>,
    collateral_asset: Option<Address>,
    debt_amount: i128,
) -> Result<FlashLoanLiquidationSim, FlashLoanError> {
    if debt_amount <= 0 {
        return Err(FlashLoanError::InvalidAmount);
    }

    let config = get_flash_loan_config(env);
    if debt_amount < config.min_amount || debt_amount > config.max_amount {
        return Err(FlashLoanError::InvalidAmount);
    }

    let flash_fee = calculate_flash_loan_fee(env, debt_amount)?;
    let incentive_bps = crate::risk_params::get_liquidation_incentive(env).unwrap_or(1_000);
    let incentive_amount = debt_amount
        .checked_mul(incentive_bps)
        .ok_or(FlashLoanError::Overflow)?
        .checked_div(BPS_DENOM)
        .ok_or(FlashLoanError::Overflow)?;

    let debt_price = resolve_asset_price(env, &debt_asset);
    let coll_price = resolve_asset_price(env, &collateral_asset);

    // seized_value = debt_value * (1 + incentive)
    let debt_value = debt_amount
        .checked_mul(debt_price)
        .ok_or(FlashLoanError::Overflow)?
        .checked_div(DEFAULT_ASSET_PRICE)
        .ok_or(FlashLoanError::Overflow)?;
    let seized_value = debt_value
        .checked_add(
            debt_value
                .checked_mul(incentive_bps)
                .ok_or(FlashLoanError::Overflow)?
                .checked_div(BPS_DENOM)
                .ok_or(FlashLoanError::Overflow)?,
        )
        .ok_or(FlashLoanError::Overflow)?;
    let collateral_seized = if coll_price > 0 {
        seized_value
            .checked_mul(DEFAULT_ASSET_PRICE)
            .ok_or(FlashLoanError::Overflow)?
            .checked_div(coll_price)
            .ok_or(FlashLoanError::Overflow)?
    } else {
        return Err(FlashLoanError::InvalidAsset);
    };

    let fee_value = flash_fee
        .checked_mul(debt_price)
        .ok_or(FlashLoanError::Overflow)?
        .checked_div(DEFAULT_ASSET_PRICE)
        .ok_or(FlashLoanError::Overflow)?;
    let estimated_profit = seized_value
        .checked_sub(debt_value)
        .ok_or(FlashLoanError::Overflow)?
        .checked_sub(fee_value)
        .unwrap_or(0);

    Ok(FlashLoanLiquidationSim {
        debt_amount,
        flash_fee,
        collateral_seized,
        incentive_amount,
        estimated_profit,
        profitable: estimated_profit > 0,
        gas_units_estimate: estimate_combo_gas(1),
    })
}

/// Atomic flash loan + liquidation. Rolls back (returns error) when the combo
/// is unprofitable. Skips the callback invoke for a gas-optimized path.
pub fn execute_flash_loan_liquidation(
    env: &Env,
    liquidator: Address,
    borrower: Address,
    debt_asset: Option<Address>,
    collateral_asset: Option<Address>,
    debt_amount: i128,
) -> Result<FlashLoanLiquidationResult, FlashLoanError> {
    liquidator.require_auth();

    let sim = simulate_flash_loan_liquidation(env, debt_asset.clone(), collateral_asset.clone(), debt_amount)?;
    if !sim.profitable {
        return Err(FlashLoanError::Unprofitable);
    }

    let Some(debt_addr) = debt_asset.clone() else {
        return Err(FlashLoanError::InvalidAsset);
    };

    let pause_map_key = FlashLoanDataKey::PauseSwitches;
    if let Some(pause_map) = env
        .storage()
        .persistent()
        .get::<FlashLoanDataKey, Map<Symbol, bool>>(&pause_map_key)
    {
        if pause_map
            .get(Symbol::new(env, "pause_flash_loan"))
            .unwrap_or(false)
        {
            return Err(FlashLoanError::FlashLoanPaused);
        }
    }

    let token_client = soroban_sdk::token::Client::new(env, &debt_addr);
    let pool = env.current_contract_address();
    let initial_balance = token_client.balance(&pool);
    if initial_balance < debt_amount {
        return Err(FlashLoanError::InsufficientLiquidity);
    }
    check_liquidity_cap(env, initial_balance, debt_amount)?;
    check_price_impact(env, initial_balance, debt_amount)?;
    acquire_asset_guard(env, &debt_addr)?;

    let start_sequence = env.ledger().sequence_number();

    // Fund the liquidator from pool liquidity (the flash leg).
    token_client.transfer(&pool, &liquidator, &debt_amount);

    emit_flash_loan_initiated(
        env,
        FlashLoanInitiatedEvent {
            user: liquidator.clone(),
            asset: debt_addr.clone(),
            amount: debt_amount,
            fee: sim.flash_fee,
            callback: pool.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );

    let (_debt_repaid, collateral_seized, _incentive) = crate::liquidate::liquidate(
        env,
        liquidator.clone(),
        borrower.clone(),
        debt_asset,
        collateral_asset,
        debt_amount,
    )
    .map_err(|_| FlashLoanError::CallbackFailed)?;

    if env.ledger().sequence_number() != start_sequence {
        return Err(FlashLoanError::Expired);
    }

    let total_required = debt_amount
        .checked_add(sim.flash_fee)
        .ok_or(FlashLoanError::Overflow)?;

    token_client.transfer_from(&pool, &liquidator, &pool, &total_required);

    if sim.flash_fee > 0 {
        let reserve_key = DepositDataKey::ProtocolReserve(Some(debt_addr.clone()));
        let current_reserve = env
            .storage()
            .persistent()
            .get::<DepositDataKey, i128>(&reserve_key)
            .unwrap_or(0);
        let new_reserve = current_reserve
            .checked_add(sim.flash_fee)
            .ok_or(FlashLoanError::Overflow)?;
        env.storage().persistent().set(&reserve_key, &new_reserve);
    }

    emit_flash_loan_repaid(
        env,
        FlashLoanRepaidEvent {
            user: liquidator.clone(),
            asset: debt_addr.clone(),
            amount: debt_amount,
            fee: sim.flash_fee,
            timestamp: env.ledger().timestamp(),
        },
    );

    crate::events::emit_flash_loan_liquidation_combo(
        env,
        crate::events::FlashLoanLiquidationComboEvent {
            liquidator: liquidator.clone(),
            borrower,
            debt_asset: debt_addr.clone(),
            debt_amount,
            collateral_seized,
            flash_fee: sim.flash_fee,
            profit: sim.estimated_profit,
            timestamp: env.ledger().timestamp(),
        },
    );

    release_asset_guard(env, &debt_addr);

    Ok(FlashLoanLiquidationResult {
        debt_repaid: debt_amount,
        collateral_seized,
        flash_fee: sim.flash_fee,
        profit: sim.estimated_profit,
    })
}

/// Multi-asset flash loan: borrow several assets, invoke a single callback,
/// then pull principal+fee for every leg. Any unpaid leg reverts the tx.
pub fn execute_multi_asset_flash_loan(
    env: &Env,
    user: Address,
    legs: Vec<FlashLoanLeg>,
    callback: Address,
) -> Result<i128, FlashLoanError> {
    user.require_auth();

    if legs.is_empty() {
        return Err(FlashLoanError::EmptyLegs);
    }
    if legs.len() > MAX_FLASH_LOAN_LEGS {
        return Err(FlashLoanError::TooManyLegs);
    }

    let pause_map_key = FlashLoanDataKey::PauseSwitches;
    if let Some(pause_map) = env
        .storage()
        .persistent()
        .get::<FlashLoanDataKey, Map<Symbol, bool>>(&pause_map_key)
    {
        if pause_map
            .get(Symbol::new(env, "pause_flash_loan"))
            .unwrap_or(false)
        {
            return Err(FlashLoanError::FlashLoanPaused);
        }
    }

    let config = get_flash_loan_config(env);
    let start_sequence = env.ledger().sequence_number();
    let mut total_fees: i128 = 0;
    let pool = env.current_contract_address();

    // Validate and fund every leg before the callback so a mid-loop failure
    // reverts the whole transaction.
    for i in 0..legs.len() {
        let leg = legs.get(i).unwrap();
        if leg.amount <= 0 || leg.amount < config.min_amount || leg.amount > config.max_amount {
            return Err(FlashLoanError::InvalidAmount);
        }
        let fee = calculate_flash_loan_fee(env, leg.amount)?;
        total_fees = total_fees
            .checked_add(fee)
            .ok_or(FlashLoanError::Overflow)?;

        let token_client = soroban_sdk::token::Client::new(env, &leg.asset);
        let initial_balance = token_client.balance(&pool);
        if initial_balance < leg.amount {
            return Err(FlashLoanError::InsufficientLiquidity);
        }
        check_liquidity_cap(env, initial_balance, leg.amount)?;
        check_price_impact(env, initial_balance, leg.amount)?;
        acquire_asset_guard(env, &leg.asset)?;
        record_flash_loan(env, &user, &leg.asset, leg.amount, fee, &callback);
        token_client.transfer(&pool, &callback, &leg.amount);

        emit_flash_loan_initiated(
            env,
            FlashLoanInitiatedEvent {
                user: user.clone(),
                asset: leg.asset.clone(),
                amount: leg.amount,
                fee,
                callback: callback.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    let callback_symbol = Symbol::new(env, "on_flash_loan_multi");
    let _: soroban_sdk::Val = env.invoke_contract(
        &callback,
        &callback_symbol,
        (user.clone(), legs.clone(), total_fees).into_val(env),
    );

    if env.ledger().sequence_number() != start_sequence {
        return Err(FlashLoanError::Expired);
    }

    for i in 0..legs.len() {
        let leg = legs.get(i).unwrap();
        let fee = calculate_flash_loan_fee(env, leg.amount)?;
        let total_required = leg
            .amount
            .checked_add(fee)
            .ok_or(FlashLoanError::Overflow)?;
        let token_client = soroban_sdk::token::Client::new(env, &leg.asset);
        token_client.transfer_from(&pool, &callback, &pool, &total_required);

        if fee > 0 {
            let reserve_key = DepositDataKey::ProtocolReserve(Some(leg.asset.clone()));
            let current_reserve = env
                .storage()
                .persistent()
                .get::<DepositDataKey, i128>(&reserve_key)
                .unwrap_or(0);
            let new_reserve = current_reserve
                .checked_add(fee)
                .ok_or(FlashLoanError::Overflow)?;
            env.storage().persistent().set(&reserve_key, &new_reserve);
        }

        emit_flash_loan_repaid(
            env,
            FlashLoanRepaidEvent {
                user: user.clone(),
                asset: leg.asset.clone(),
                amount: leg.amount,
                fee,
                timestamp: env.ledger().timestamp(),
            },
        );
        clear_flash_loan(env, &user, &leg.asset);
        release_asset_guard(env, &leg.asset);
    }

    Ok(total_fees)
}
