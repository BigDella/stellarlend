//! # Borrow Implementation (Simplified Lending)
//!
//! Core borrow logic for the simplified lending contract. Handles collateral
//! validation, debt tracking, interest calculation, and pause controls.
//!
//! ## Interest Model
//! Uses a fixed 5% APY simple interest model:
//! `interest = principal * 500bps * time_elapsed / seconds_per_year`
//!
//! ## Collateral Requirements
//! Minimum collateral ratio is 150% (15,000 basis points).

pub use crate::events::{BorrowCollateralDepositEvent, BorrowEvent, RepayEvent};

/// Backward-compatible name for collateral added to a borrow position (see [`BorrowCollateralDepositEvent`]).
pub type DepositEvent = BorrowCollateralDepositEvent;

use crate::pause::{self, PauseType};
use crate::reentrancy::{ReentrancyGuard, ReentrancyKey};
use crate::rounding;
use soroban_sdk::{contracterror, contracttype, Address, Env, IntoVal, Symbol, I256};

#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum RateType {
    Variable = 0,
    Stable = 1,
}

/// Errors that can occur during borrow operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BorrowError {
    /// Collateral amount does not meet the 150% minimum ratio
    InsufficientCollateral = 1,
    /// Total protocol debt would exceed the configured debt ceiling
    DebtCeilingReached = 2,
    /// Borrow operations are currently paused
    ProtocolPaused = 3,
    /// Borrow or collateral amount is zero or negative
    InvalidAmount = 4,
    /// Arithmetic overflow during calculation
    Overflow = 5,
    /// Caller is not authorized for this operation
    Unauthorized = 6,
    /// The requested asset is not supported for borrowing
    AssetNotSupported = 7,
    /// Borrow amount is below the configured minimum
    BelowMinimumBorrow = 8,
    /// Amount is below minimum transaction threshold (dust)
    AmountBelowMinimum = 9,
    /// No dust available to sweep
    NoDustToSweep = 10,
    /// Reentrancy detected
    ReentrancyDetected = 11,
    /// Repay amount exceeds current debt
    RepayAmountTooHigh = 12,
    /// Position is healthy and cannot be liquidated
    PositionHealthy = 13,
    /// Insufficient reserves to recover bad debt
    InsufficientReserves = 14,
}

/// Borrow on behalf of a user when authorization is provided via a trusted delegate.
///
/// This bypasses `user.require_auth()` and is intended to be called only after
/// delegation/nonce checks at a higher layer.
pub(crate) fn borrow_trusted(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    collateral_asset: Address,
    collateral_amount: i128,
) -> Result<(), BorrowError> {
    borrow_inner(
        env,
        user,
        asset,
        amount,
        collateral_asset,
        collateral_amount,
        RateType::Variable,
        BorrowAuth::TrustedCommitment,
    )
}

/// Storage keys for protocol-wide data.
#[contracttype]
#[derive(Clone)]
#[allow(clippy::enum_variant_names)]
pub enum BorrowDataKey {
    /// Protocol admin address
    ProtocolAdmin,
    /// Per-user debt position
    BorrowUserDebt(Address),
    /// Per-user variable-rate debt position
    BorrowUserVariableDebt(Address),
    /// Per-user stable-rate debt position
    BorrowUserStableDebt(Address),
    /// Per-user collateral position
    BorrowUserCollateral(Address),
    /// Aggregate protocol debt
    BorrowTotalDebt,
    /// Maximum total debt allowed
    BorrowDebtCeiling,
    /// Interest rate configuration
    BorrowInterestRate,
    /// Collateral ratio configuration
    BorrowCollateralRatio,
    /// Minimum borrow amount
    BorrowMinAmount,
    /// Oracle contract address for price feeds (optional)
    OracleAddress,
    /// Liquidation threshold in basis points (e.g. 8000 = 80%)
    LiquidationThresholdBps,
    /// Close factor in basis points (e.g. 5000 = 50%)
    CloseFactorBps,
    /// Dust amount tracking for debt positions
    DustAmount(Address),
    /// Liquidation incentive in basis points (e.g. 1000 = 10%)
    LiquidationIncentiveBps,
    /// Global interest index (for invariant testing)
    InterestIndex,
    /// Stablecoin configuration for a specific asset
    AssetStablecoinConfig(Address),
    /// Stable borrow rate state (protocol-wide)
    StableRateState,
    /// Stable rate premium in basis points
    StableRatePremiumBps,
    /// Stable rate recalculation interval in seconds
    StableRateRecalcInterval,
}

// ─── Constants ───────────────────────────────────────────────────────────────

/// Minimum transaction amount (1 unit of asset)
const MIN_TRANSACTION_AMOUNT: i128 = 1;

/// Dust threshold (same as minimum transaction amount)
const DUST_THRESHOLD: i128 = MIN_TRANSACTION_AMOUNT;

/// Dynamic stablecoin configuration.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct StablecoinConfig {
    /// Target price in oracle units (e.g. 100_000_000 for $1.00)
    pub target_price: i128,
    /// Minimum deviation before stability fee kicks in (basis points)
    pub peg_threshold_bps: i128,
    /// Fee added to interest rate when depegged (basis points)
    pub stability_fee_bps: i128,
    /// Threshold for emergency actions (basis points)
    pub emergency_threshold_bps: i128,
}

/// User debt position tracking.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct DebtPosition {
    /// Principal amount borrowed
    pub borrowed_amount: i128,
    /// Cumulative interest accrued
    pub interest_accrued: i128,
    /// Timestamp of last interest accrual
    pub last_update: u64,
    /// Address of the borrowed asset
    pub asset: Address,

    pub rate_type: RateType,
    pub stable_rate_bps: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct StableRateState {
    pub avg_rate_bps: i128,
    pub last_update: u64,
}

/// User collateral position tracking.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct BorrowCollateral {
    /// Amount of collateral deposited
    pub amount: i128,
    /// Address of the collateral asset
    pub asset: Address,
}

const COLLATERAL_RATIO_MIN: i128 = 15000; // 150% in basis points
const SECONDS_PER_YEAR: u64 = 31536000;

const DEFAULT_STABLE_PREMIUM_BPS: i128 = 100; // 1%
const DEFAULT_STABLE_RECALC_INTERVAL_SECS: u64 = 6 * 3600; // 6h
const DEFAULT_SWITCH_FEE_BPS: i128 = 10; // 0.10%

fn get_current_variable_rate_bps(env: &Env) -> Result<i128, BorrowError> {
    crate::interest_rate::borrow_rate_bps(env).map_err(|e| {
        let be: BorrowError = e.into();
        be
    })
}

pub fn set_variable_borrow_rate_bps(
    env: &Env,
    admin: &Address,
    rate_bps: i128,
) -> Result<(), BorrowError> {
    let current = get_admin(env).ok_or(BorrowError::Unauthorized)?;
    if *admin != current {
        return Err(BorrowError::Unauthorized);
    }
    if !(0..=10000).contains(&rate_bps) {
        return Err(BorrowError::InvalidAmount);
    }

    let update = crate::interest_rate::InterestRateConfigUpdate {
        model: None,
        base_rate_bps: Some(rate_bps),
        kink_utilization_bps: None,
        slope_bps: None,
        jump_slope_bps: None,
        rate_floor_bps: None,
        rate_ceiling_bps: None,
        spread_bps: None,
    };

    crate::interest_rate::update_config(env, admin, update).map_err(|e| {
        let be: BorrowError = e.into();
        be
    })?;
    Ok(())
}

fn get_stable_rate_premium_bps(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::StableRatePremiumBps)
        .unwrap_or(DEFAULT_STABLE_PREMIUM_BPS)
}

fn get_stable_rate_recalc_interval_secs(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::StableRateRecalcIntervalSecs)
        .unwrap_or(DEFAULT_STABLE_RECALC_INTERVAL_SECS)
}

fn get_rate_switch_fee_bps(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::RateSwitchFeeBps)
        .unwrap_or(DEFAULT_SWITCH_FEE_BPS)
}

fn get_stable_rate_state(env: &Env) -> StableRateState {
    if let Some(state) = env
        .storage()
        .persistent()
        .get(&BorrowDataKey::StableRateState)
    {
        return state;
    }

    let state = StableRateState {
        avg_rate_bps: get_current_variable_rate_bps(env).unwrap_or(0),
        last_update: env.ledger().timestamp(),
    };
    env.storage()
        .persistent()
        .set(&BorrowDataKey::StableRateState, &state);
    state
}

fn update_stable_rate_state_if_needed(env: &Env) -> StableRateState {
    if !env
        .storage()
        .persistent()
        .has(&BorrowDataKey::StableRateState)
    {
        let state = get_stable_rate_state(env);
        env.storage()
            .persistent()
            .set(&BorrowDataKey::StableRateState, &state);
        return state;
    }

    let mut state = get_stable_rate_state(env);
    let now = env.ledger().timestamp();
    let interval = get_stable_rate_recalc_interval_secs(env);

    if now <= state.last_update {
        return state;
    }

    if interval == 0 || now - state.last_update < interval {
        return state;
    }

    // Simple EMA smoothing toward current variable rate:
    // new_avg = (old * 9 + current) / 10
    let current = match get_current_variable_rate_bps(env) {
        Ok(v) => v,
        Err(_) => state.avg_rate_bps,
    };
    let new_avg = state
        .avg_rate_bps
        .checked_mul(9)
        .and_then(|v| v.checked_add(current))
        .and_then(|v| v.checked_div(10))
        .unwrap_or(current);

    state.avg_rate_bps = new_avg;
    state.last_update = now;

    env.storage()
        .persistent()
        .set(&BorrowDataKey::StableRateState, &state);

    state
}

fn get_current_stable_rate_bps(env: &Env) -> Result<i128, BorrowError> {
    let state = update_stable_rate_state_if_needed(env);
    let premium = get_stable_rate_premium_bps(env);
    state
        .avg_rate_bps
        .checked_add(premium)
        .ok_or(BorrowError::Overflow)
}

/// Borrow assets against deposited collateral (requires direct user authorization).
pub fn borrow(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    collateral_asset: Address,
    collateral_amount: i128,
) -> Result<(), BorrowError> {
    borrow_with_rate(
        env,
        user,
        asset,
        amount,
        collateral_asset,
        collateral_amount,
        RateType::Variable,
    )
}

pub fn borrow_with_rate(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    collateral_asset: Address,
    collateral_amount: i128,
    rate_type: RateType,
) -> Result<(), BorrowError> {
    borrow_inner(
        env,
        user,
        asset,
        amount,
        collateral_asset,
        collateral_amount,
        rate_type,
        BorrowAuth::RequireUserSignature,
    )
}

/// Borrow on behalf of a user who previously authorized a scheduled commitment (no `user.require_auth`).
pub(crate) fn borrow_from_commitment(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    collateral_asset: Address,
    collateral_amount: i128,
) -> Result<(), BorrowError> {
    borrow_inner(
        env,
        user,
        asset,
        amount,
        collateral_asset,
        collateral_amount,
        RateType::Variable,
        BorrowAuth::TrustedCommitment,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BorrowAuth {
    RequireUserSignature,
    TrustedCommitment,
}

fn borrow_inner(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    collateral_asset: Address,
    collateral_amount: i128,
    rate_type: RateType,
    auth: BorrowAuth,
) -> Result<(), BorrowError> {
    // CHECKS-EFFECTS-INTERACTIONS PATTERN
    // 1. CHECKS: Reentrancy guard, authorization, pause state, validation
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::BorrowLock, false)
        .map_err(|_| BorrowError::ReentrancyDetected)?;

    if auth == BorrowAuth::RequireUserSignature {
        user.require_auth();
    }

    if pause::is_paused(env, PauseType::Borrow) {
        return Err(BorrowError::ProtocolPaused);
    }

    if amount <= 0 || collateral_amount <= 0 {
        return Err(BorrowError::InvalidAmount);
    }

    // Gas-efficient dust check (early return)
    if amount < DUST_THRESHOLD {
        return Err(BorrowError::AmountBelowMinimum);
    }

    let min_borrow = get_min_borrow_amount(env);
    if amount < min_borrow {
        return Err(BorrowError::AmountBelowMinimum);
    }

    validate_collateral_ratio(collateral_amount, amount)?;

    let total_debt = get_total_debt(env);
    let debt_ceiling = get_debt_ceiling(env);
    let new_total = total_debt
        .checked_add(amount)
        .ok_or(BorrowError::Overflow)?;

    if new_total > debt_ceiling {
        return Err(BorrowError::DebtCeilingReached);
    }

    // 2. EFFECTS: Update state before any external interactions
    let mut debt_position = get_debt_position(env, &user, Some(&asset), rate_type);
    debt_position.rate_type = rate_type;
    let accrued_interest = calculate_interest(env, &debt_position)?;

    debt_position.borrowed_amount = debt_position
        .borrowed_amount
        .checked_add(amount)
        .ok_or(BorrowError::Overflow)?;
    debt_position.interest_accrued = debt_position
        .interest_accrued
        .checked_add(accrued_interest)
        .ok_or(BorrowError::Overflow)?;
    debt_position.last_update = env.ledger().timestamp();
    debt_position.asset = asset.clone();

    if rate_type == RateType::Stable {
        debt_position.stable_rate_bps = get_current_stable_rate_bps(env)?;
    }

    let mut collateral_position = get_collateral_position(env, &user);
    collateral_position.amount = collateral_position
        .amount
        .checked_add(collateral_amount)
        .ok_or(BorrowError::Overflow)?;
    collateral_position.asset = collateral_asset.clone();

    save_debt_position(env, &user, &debt_position);
    save_collateral_position(env, &user, &collateral_position);
    set_total_debt(env, new_total);

    // 3. INTERACTIONS: External calls (risk_monitor) and events
    crate::risk_monitor::on_utilization_changed(env, new_total, debt_ceiling);

    emit_borrow_event(env, user, asset, amount, collateral_amount);

    Ok(())
}

/// Deposit collateral
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - The user's address
/// * `asset` - The collateral asset
/// * `amount` - The amount to deposit
pub fn deposit(env: &Env, user: Address, asset: Address, amount: i128) -> Result<(), BorrowError> {
    // CHECKS-EFFECTS-INTERACTIONS PATTERN
    // 1. CHECKS: Reentrancy guard, validation
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::DepositCollateralLock, false)
        .map_err(|_| BorrowError::ReentrancyDetected)?;

    if amount <= 0 {
        return Err(BorrowError::InvalidAmount);
    }

    // Gas-efficient dust check (early return)
    if amount < DUST_THRESHOLD {
        return Err(BorrowError::AmountBelowMinimum);
    }

    // 2. EFFECTS: Update state before any external interactions
    let mut collateral_position = get_collateral_position(env, &user);

    // If it's the first deposit, set the asset
    if collateral_position.amount == 0 {
        collateral_position.asset = asset.clone();
    } else if collateral_position.asset != asset {
        return Err(BorrowError::AssetNotSupported);
    }

    collateral_position.amount = collateral_position
        .amount
        .checked_add(amount)
        .ok_or(BorrowError::Overflow)?;

    save_collateral_position(env, &user, &collateral_position);

    // 3. INTERACTIONS: Emit events
    BorrowCollateralDepositEvent {
        user,
        asset,
        amount,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Repay borrowed assets
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - The user's address
/// * `asset` - The borrowed asset
/// * `amount` - The amount to repay
pub fn repay(env: &Env, user: Address, asset: Address, amount: i128) -> Result<(), BorrowError> {
    let variable = get_debt_position(env, &user, Some(&asset), RateType::Variable);
    if variable.borrowed_amount > 0 || variable.interest_accrued > 0 {
        repay_with_rate(env, user, asset, amount, RateType::Variable)
    } else {
        repay_with_rate(env, user, asset, amount, RateType::Stable)
    }
}

pub fn repay_with_rate(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    rate_type: RateType,
) -> Result<(), BorrowError> {
    // CHECKS-EFFECTS-INTERACTIONS PATTERN
    // 1. CHECKS: Reentrancy guard, validation
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::RepayLock, false)
        .map_err(|_| BorrowError::ReentrancyDetected)?;

    if amount <= 0 {
        return Err(BorrowError::InvalidAmount);
    }

    let mut debt_position = get_debt_position(env, &user, Some(&asset), rate_type);
    debt_position.rate_type = rate_type;

    if debt_position.borrowed_amount == 0 && debt_position.interest_accrued == 0 {
        return Err(BorrowError::InvalidAmount);
    }

    if debt_position.asset != asset {
        return Err(BorrowError::AssetNotSupported);
    }

    // 2. EFFECTS: Update state before any external interactions
    // First repay interest, then principal
    let accrued_interest = calculate_interest(env, &debt_position)?;
    debt_position.interest_accrued = debt_position
        .interest_accrued
        .checked_add(accrued_interest)
        .ok_or(BorrowError::Overflow)?;
    debt_position.last_update = env.ledger().timestamp();

    let mut remaining_repayment = amount;

    // Repay interest first
    if remaining_repayment >= debt_position.interest_accrued {
        remaining_repayment -= debt_position.interest_accrued;
        debt_position.interest_accrued = 0;
    } else {
        debt_position.interest_accrued -= remaining_repayment;
        remaining_repayment = 0;
    }

    // Repay principal
    if remaining_repayment > 0 {
        if remaining_repayment > debt_position.borrowed_amount {
            return Err(BorrowError::RepayAmountTooHigh);
        }
        debt_position.borrowed_amount -= remaining_repayment;

        // Update total protocol debt
        let total_debt = get_total_debt(env);
        let new_total = total_debt
            .checked_sub(remaining_repayment)
            .ok_or(BorrowError::Overflow)?;
        set_total_debt(env, new_total);
    }

    save_debt_position(env, &user, &debt_position);

    // 3. INTERACTIONS: Emit events
    RepayEvent {
        user,
        asset,
        amount,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

pub fn switch_rate_type(
    env: &Env,
    user: Address,
    asset: Address,
    to_rate_type: RateType,
) -> Result<(), BorrowError> {
    // CHECKS-EFFECTS-INTERACTIONS PATTERN
    // 1. CHECKS: Reentrancy guard, authorization
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::BorrowLock, false)
        .map_err(|_| BorrowError::ReentrancyDetected)?;

    user.require_auth();

    let from_rate_type = if to_rate_type == RateType::Variable {
        RateType::Stable
    } else {
        RateType::Variable
    };

    let mut from_position = get_debt_position(env, &user, Some(&asset), from_rate_type);
    from_position.rate_type = from_rate_type;
    if from_position.borrowed_amount == 0 && from_position.interest_accrued == 0 {
        return Err(BorrowError::InvalidAmount);
    }

    let mut to_position = get_debt_position(env, &user, Some(&asset), to_rate_type);
    to_position.rate_type = to_rate_type;
    if to_position.borrowed_amount != 0 || to_position.interest_accrued != 0 {
        return Err(BorrowError::AssetNotSupported);
    }

    // 2. EFFECTS: Update state before any external interactions
    // Accrue interest on the source position before moving.
    let accrued_interest = calculate_interest(env, &from_position)?;
    from_position.interest_accrued = from_position
        .interest_accrued
        .checked_add(accrued_interest)
        .ok_or(BorrowError::Overflow)?;
    from_position.last_update = env.ledger().timestamp();

    // Switch fee as an added cost on the source principal.
    let fee_bps = get_rate_switch_fee_bps(env);
    let fee = from_position
        .borrowed_amount
        .checked_mul(fee_bps)
        .ok_or(BorrowError::Overflow)?
        .checked_div(10000)
        .ok_or(BorrowError::Overflow)?;
    from_position.interest_accrued = from_position
        .interest_accrued
        .checked_add(fee)
        .ok_or(BorrowError::Overflow)?;

    // Move into the target bucket.
    to_position.borrowed_amount = from_position.borrowed_amount;
    to_position.interest_accrued = from_position.interest_accrued;
    to_position.last_update = env.ledger().timestamp();
    to_position.asset = asset;

    if to_rate_type == RateType::Stable {
        to_position.stable_rate_bps = get_current_stable_rate_bps(env)?;
    }

    // Clear the source bucket.
    from_position.borrowed_amount = 0;
    from_position.interest_accrued = 0;
    from_position.last_update = env.ledger().timestamp();
    from_position.stable_rate_bps = 0;

    save_debt_position(env, &user, &from_position);
    save_debt_position(env, &user, &to_position);

    // 3. INTERACTIONS: No external calls, only state updates

    Ok(())
}

/// Validate collateral ratio meets minimum requirements
pub(crate) fn validate_collateral_ratio(collateral: i128, borrow: i128) -> Result<(), BorrowError> {
    let min_collateral = borrow
        .checked_mul(COLLATERAL_RATIO_MIN)
        .ok_or(BorrowError::Overflow)?
        .checked_div(10000)
        .ok_or(BorrowError::InvalidAmount)?;

    if collateral < min_collateral {
        return Err(BorrowError::InsufficientCollateral);
    }

    Ok(())
}

pub(crate) fn calculate_interest(env: &Env, position: &DebtPosition) -> Result<i128, BorrowError> {
    if position.borrowed_amount == 0 {
        return Ok(0);
    }

    let current_time = env.ledger().timestamp();
    let time_elapsed = current_time.saturating_sub(position.last_update);

    let borrowed_256 = I256::from_i128(env, position.borrowed_amount);
    let rate_bps = match position.rate_type {
        RateType::Variable => get_current_variable_rate_bps(env)?,
        RateType::Stable => {
            if position.stable_rate_bps > 0 {
                position.stable_rate_bps
            } else {
                get_current_stable_rate_bps(env)?
            }
        }
    };
    let rate_256 = I256::from_i128(env, rate_bps);
    let time_256 = I256::from_i128(env, time_elapsed as i128);

    // Use depositor-friendly rounding (round down) for interest calculation
    // This ensures borrowers pay less interest due to rounding
    let mut interest_256 = borrowed_256
        .mul(&rate_256)
        .mul(&time_256)
        .div(&I256::from_i128(env, 10000))
        .div(&I256::from_i128(env, SECONDS_PER_YEAR as i128));

    // Stability fee logic
    if let Some(config) = get_stablecoin_config(env, &position.asset) {
        if let Some(oracle) = get_oracle(env) {
            let price = get_asset_price(env, &oracle, &position.asset);
            let deviation = config.target_price.saturating_sub(price);
            let deviation_bps = if config.target_price > 0 {
                deviation
                    .saturating_mul(10000)
                    .saturating_div(config.target_price)
            } else {
                0
            };

            if deviation_bps > config.peg_threshold_bps {
                // Use depositor-friendly rounding (round down) for stability fee
                let stability_fee_256 = borrowed_256
                    .mul(&I256::from_i128(env, config.stability_fee_bps))
                    .mul(&time_256)
                    .div(&I256::from_i128(env, 10000))
                    .div(&I256::from_i128(env, SECONDS_PER_YEAR as i128));

                interest_256 = interest_256.add(&stability_fee_256);

                crate::events::PegDeviationEvent {
                    asset: position.asset.clone(),
                    price,
                    target_price: config.target_price,
                    deviation_bps,
                    timestamp: env.ledger().timestamp(),
                }
                .publish(env);

                crate::events::StabilityFeeAppliedEvent {
                    asset: position.asset.clone(),
                    fee_bps: config.stability_fee_bps,
                    timestamp: env.ledger().timestamp(),
                }
                .publish(env);
            }
        }
    }

    interest_256.to_i128().ok_or(BorrowError::Overflow)
}

fn get_asset_price(env: &Env, oracle: &Address, asset: &Address) -> i128 {
    env.invoke_contract(
        oracle,
        &Symbol::new(env, "price"),
        (asset.clone(),).into_val(env),
    )
}

fn get_debt_position(
    env: &Env,
    user: &Address,
    default_asset: Option<&Address>,
    rate_type: RateType,
) -> DebtPosition {
    let key = match rate_type {
        RateType::Variable => BorrowDataKey::BorrowUserVariableDebt(user.clone()),
        RateType::Stable => BorrowDataKey::BorrowUserStableDebt(user.clone()),
    };

    // Backward compat: if new key doesn't exist, fall back to legacy BorrowUserDebt
    // for variable-rate position.
    if !env.storage().persistent().has(&key) && rate_type == RateType::Variable {
        if let Some(legacy) = env
            .storage()
            .persistent()
            .get::<BorrowDataKey, DebtPosition>(&BorrowDataKey::BorrowUserDebt(user.clone()))
        {
            if legacy.rate_type == RateType::Variable {
                return DebtPosition {
                    borrowed_amount: legacy.borrowed_amount,
                    interest_accrued: legacy.interest_accrued,
                    last_update: legacy.last_update,
                    asset: legacy.asset,
                    rate_type: RateType::Variable,
                    stable_rate_bps: 0,
                };
            }
        }
    }

    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or(DebtPosition {
            borrowed_amount: 0,
            interest_accrued: 0,
            last_update: env.ledger().timestamp(),
            asset: default_asset.cloned().unwrap_or_else(|| user.clone()),
            rate_type,
            stable_rate_bps: 0,
        })
}

fn save_debt_position(env: &Env, user: &Address, position: &DebtPosition) {
    let key = match position.rate_type {
        RateType::Variable => BorrowDataKey::BorrowUserVariableDebt(user.clone()),
        RateType::Stable => BorrowDataKey::BorrowUserStableDebt(user.clone()),
    };

    env.storage().persistent().set(&key, position);
    update_compat_user_debt(env, user);
}

fn update_compat_user_debt(env: &Env, user: &Address) {
    let v = env
        .storage()
        .persistent()
        .get::<BorrowDataKey, DebtPosition>(&BorrowDataKey::BorrowUserVariableDebt(user.clone()));
    let s = env
        .storage()
        .persistent()
        .get::<BorrowDataKey, DebtPosition>(&BorrowDataKey::BorrowUserStableDebt(user.clone()));

    let chosen = if let Some(pos) = v {
        if pos.borrowed_amount > 0 || pos.interest_accrued > 0 {
            Some(pos)
        } else {
            s
        }
    } else {
        s
    };

    if let Some(pos) = chosen {
        env.storage()
            .persistent()
            .set(&BorrowDataKey::BorrowUserDebt(user.clone()), &pos);
    } else {
        // Ensure legacy key is cleared to an empty variable position
        env.storage().persistent().set(
            &BorrowDataKey::BorrowUserDebt(user.clone()),
            &DebtPosition {
                borrowed_amount: 0,
                interest_accrued: 0,
                last_update: env.ledger().timestamp(),
                asset: user.clone(),
                rate_type: RateType::Variable,
                stable_rate_bps: 0,
            },
        );
    }
}

fn get_collateral_position(env: &Env, user: &Address) -> BorrowCollateral {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::BorrowUserCollateral(user.clone()))
        .unwrap_or(BorrowCollateral {
            amount: 0,
            asset: user.clone(),
        })
}

fn save_collateral_position(env: &Env, user: &Address, position: &BorrowCollateral) {
    env.storage()
        .persistent()
        .set(&BorrowDataKey::BorrowUserCollateral(user.clone()), position);
}

pub(crate) fn get_total_debt(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::BorrowTotalDebt)
        .unwrap_or(0)
}

fn set_total_debt(env: &Env, amount: i128) {
    env.storage()
        .persistent()
        .set(&BorrowDataKey::BorrowTotalDebt, &amount);
}

pub(crate) fn get_debt_ceiling(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::BorrowDebtCeiling)
        .unwrap_or(i128::MAX)
}

pub(crate) fn get_min_borrow_amount(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::BorrowMinAmount)
        .unwrap_or(1000)
}

fn emit_borrow_event(env: &Env, user: Address, asset: Address, amount: i128, collateral: i128) {
    BorrowEvent {
        user,
        asset,
        amount,
        collateral,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}

pub fn initialize_borrow_settings(
    env: &Env,
    debt_ceiling: i128,
    min_borrow_amount: i128,
) -> Result<(), BorrowError> {
    // Note: ProtocolAdmin check should be performed by the caller (lib.rs)
    env.storage()
        .persistent()
        .set(&BorrowDataKey::BorrowDebtCeiling, &debt_ceiling);
    env.storage()
        .persistent()
        .set(&BorrowDataKey::BorrowMinAmount, &min_borrow_amount);
    crate::interest_rate::set_default_if_missing(env);
    if !env
        .storage()
        .persistent()
        .has(&BorrowDataKey::StableRatePremiumBps)
    {
        env.storage().persistent().set(
            &BorrowDataKey::StableRatePremiumBps,
            &DEFAULT_STABLE_PREMIUM_BPS,
        );
    }
    if !env
        .storage()
        .persistent()
        .has(&BorrowDataKey::StableRateRecalcIntervalSecs)
    {
        env.storage().persistent().set(
            &BorrowDataKey::StableRateRecalcIntervalSecs,
            &DEFAULT_STABLE_RECALC_INTERVAL_SECS,
        );
    }
    if !env
        .storage()
        .persistent()
        .has(&BorrowDataKey::RateSwitchFeeBps)
    {
        env.storage()
            .persistent()
            .set(&BorrowDataKey::RateSwitchFeeBps, &DEFAULT_SWITCH_FEE_BPS);
    }
    Ok(())
}

pub fn get_user_debt(env: &Env, user: &Address) -> DebtPosition {
    let variable = get_debt_position(env, user, None, RateType::Variable);
    let stable = get_debt_position(env, user, None, RateType::Stable);

    let mut position = if variable.borrowed_amount > 0 || variable.interest_accrued > 0 {
        variable
    } else {
        stable
    };

    match calculate_interest(env, &position) {
        Ok(accrued) => {
            position.interest_accrued = position.interest_accrued.saturating_add(accrued);
        }
        Err(BorrowError::Overflow) => {
            // Read-only view: saturate rather than under-reporting interest.
            position.interest_accrued = i128::MAX;
        }
        Err(_) => {}
    }
    position
}

pub fn get_user_debt_with_rate(env: &Env, user: &Address, rate_type: RateType) -> DebtPosition {
    let mut position = get_debt_position(env, user, None, rate_type);
    match calculate_interest(env, &position) {
        Ok(accrued) => {
            position.interest_accrued = position.interest_accrued.saturating_add(accrued);
        }
        Err(BorrowError::Overflow) => {
            position.interest_accrued = i128::MAX;
        }
        Err(_) => {}
    }
    position
}

pub fn get_user_collateral(env: &Env, user: &Address) -> BorrowCollateral {
    get_collateral_position(env, user)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage()
        .persistent()
        .set(&BorrowDataKey::ProtocolAdmin, admin);
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::ProtocolAdmin)
}

/// Returns the oracle address if configured. Used by views for collateral/debt valuation.
pub fn get_oracle(env: &Env) -> Option<Address> {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::OracleAddress)
}

/// Returns liquidation threshold in basis points (e.g. 8000 = 80%). Default 8000 if not set.
pub fn get_liquidation_threshold_bps(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::LiquidationThresholdBps)
        .unwrap_or(8000)
}

/// Returns close factor in basis points (e.g. 5000 = 50%). Default 5000 if not set.
pub fn get_close_factor_bps(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::CloseFactorBps)
        .unwrap_or(5000)
}

/// Returns liquidation incentive in basis points (e.g. 1000 = 10%). Default 1000 if not set.
pub fn get_liquidation_incentive_bps(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::LiquidationIncentiveBps)
        .unwrap_or(1000)
}

/// Sweep dust amounts from user's debt position
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - The user's address
/// * `asset` - The asset address
///
/// # Returns
/// Returns the dust amount swept on success
pub fn sweep_dust(env: &Env, user: Address, asset: Address) -> Result<i128, BorrowError> {
    // CHECKS-EFFECTS-INTERACTIONS PATTERN
    // 1. CHECKS: Reentrancy guard, authorization
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::BorrowLock, false)
        .map_err(|_| BorrowError::ReentrancyDetected)?;

    user.require_auth();

    // Get dust amount
    let dust_amount = get_dust_amount(env, &user);
    if dust_amount < DUST_THRESHOLD {
        return Err(BorrowError::NoDustToSweep);
    }

    let mut debt_position = get_debt_position(env, &user, Some(&asset), RateType::Variable);

    // 2. EFFECTS: Update state before any external interactions
    // Remove dust from position
    debt_position.borrowed_amount = debt_position
        .borrowed_amount
        .checked_sub(dust_amount)
        .ok_or(BorrowError::Overflow)?;
    save_debt_position(env, &user, &debt_position);

    // Clear dust tracking
    clear_dust(env, &user);

    // Update total debt
    let total_debt = get_total_debt(env);
    let new_total = total_debt
        .checked_sub(dust_amount)
        .ok_or(BorrowError::Overflow)?;
    set_total_debt(env, new_total);

    // 3. INTERACTIONS: Transfer dust to user (if applicable)
    // Note: For debt positions, dust is typically written off rather than transferred
    // since it represents debt, not assets held by the protocol

    Ok(dust_amount)
}

/// Get dust amount for a user's position
fn get_dust_amount(env: &Env, user: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::DustAmount(user.clone()))
        .unwrap_or(0)
}

/// Set dust amount for a user's position
fn set_dust_amount(env: &Env, user: &Address, dust: i128) {
    env.storage()
        .persistent()
        .set(&BorrowDataKey::DustAmount(user.clone()), &dust);
}

/// Clear dust tracking for a user's position
fn clear_dust(env: &Env, user: &Address) {
    env.storage()
        .persistent()
        .remove(&BorrowDataKey::DustAmount(user.clone()));
}

/// Track dust accumulation during operations
pub fn track_dust(env: &Env, user: &Address, dust: i128) {
    if dust > 0 && dust < DUST_THRESHOLD {
        let current_dust = get_dust_amount(env, user);
        let new_dust = current_dust.checked_add(dust).unwrap_or(current_dust);
        set_dust_amount(env, user, new_dust);
    }
}

/// Set oracle address for price feeds (admin only). Caller must be admin and authorize.
pub fn set_oracle(env: &Env, admin: &Address, oracle: Address) -> Result<(), BorrowError> {
    let current = get_admin(env).ok_or(BorrowError::Unauthorized)?;
    if *admin != current {
        return Err(BorrowError::Unauthorized);
    }
    admin.require_auth();
    env.storage()
        .persistent()
        .set(&BorrowDataKey::OracleAddress, &oracle);
    Ok(())
}

/// Set liquidation threshold in basis points (admin only). E.g. 8000 = 80%.
pub fn set_liquidation_threshold_bps(
    env: &Env,
    admin: &Address,
    bps: i128,
) -> Result<(), BorrowError> {
    let current = get_admin(env).ok_or(BorrowError::Unauthorized)?;
    if *admin != current {
        return Err(BorrowError::Unauthorized);
    }
    admin.require_auth();
    if bps <= 0 || bps > 10000 {
        return Err(BorrowError::InvalidAmount);
    }
    env.storage()
        .persistent()
        .set(&BorrowDataKey::LiquidationThresholdBps, &bps);
    Ok(())
}

/// Set close factor in basis points (admin only). E.g. 5000 = 50%.
pub fn set_close_factor_bps(env: &Env, admin: &Address, bps: i128) -> Result<(), BorrowError> {
    let current = get_admin(env).ok_or(BorrowError::Unauthorized)?;
    if *admin != current {
        return Err(BorrowError::Unauthorized);
    }
    admin.require_auth();
    if bps <= 0 || bps > 10000 {
        return Err(BorrowError::InvalidAmount);
    }
    env.storage()
        .persistent()
        .set(&BorrowDataKey::CloseFactorBps, &bps);
    Ok(())
}

/// Set liquidation incentive in basis points (admin only). E.g. 1000 = 10%.
pub fn set_liquidation_incentive_bps(
    env: &Env,
    admin: &Address,
    bps: i128,
) -> Result<(), BorrowError> {
    let current = get_admin(env).ok_or(BorrowError::Unauthorized)?;
    if *admin != current {
        return Err(BorrowError::Unauthorized);
    }
    admin.require_auth();
    if !(0..=10000).contains(&bps) {
        return Err(BorrowError::InvalidAmount);
    }
    env.storage()
        .persistent()
        .set(&BorrowDataKey::LiquidationIncentiveBps, &bps);
    Ok(())
}

/// Get current interest index (for invariant testing)
pub fn get_interest_index(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::InterestIndex)
        .unwrap_or(1_000_000_000) // Default to 1.0 with 9 decimals
}

/// Set interest index (internal use)
pub fn set_interest_index(env: &Env, index: i128) {
    env.storage()
        .persistent()
        .set(&BorrowDataKey::InterestIndex, &index);
}

pub fn set_stablecoin_config(
    env: &Env,
    admin: &Address,
    asset: Address,
    config: StablecoinConfig,
) -> Result<(), BorrowError> {
    let current = get_admin(env).ok_or(BorrowError::Unauthorized)?;
    if *admin != current {
        return Err(BorrowError::Unauthorized);
    }
    admin.require_auth();
    env.storage()
        .persistent()
        .set(&BorrowDataKey::AssetStablecoinConfig(asset), &config);
    Ok(())
}

pub fn get_stablecoin_config(env: &Env, asset: &Address) -> Option<StablecoinConfig> {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::AssetStablecoinConfig(asset.clone()))
}
