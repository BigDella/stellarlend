//! # Interest Rate Module
//!
//! Implements a kink-based (piecewise linear) interest rate model for the lending protocol.
//!
//! ## Rate Model
//! The borrow rate is determined by protocol utilization (borrows / deposits):
//! - **Below kink** (default 80%): `rate = base_rate + (utilization / kink) * multiplier`
//! - **Above kink**: `rate = base_rate + multiplier + ((util - kink) / (1 - kink)) * jump_multiplier`
//!
//! The supply rate is derived as: `supply_rate = borrow_rate - spread`
//!
//! ## Configuration (defaults)
//! - Base rate: 1% APY
//! - Kink utilization: 80%
//! - Multiplier: 20% (slope below kink)
//! - Jump multiplier: 100% (slope above kink)
//! - Rate floor: 0.5%, Rate ceiling: 100%
//! - Spread: 2%
//!
//! ## Emergency Adjustment
//! Admin can apply a positive or negative emergency adjustment to the calculated rate,
//! bounded to ±100%.

#![allow(unused)]
use soroban_sdk::{contracterror, contracttype, Address, Env, IntoVal};

use crate::deposit::{DepositDataKey, ProtocolAnalytics};

/// Errors that can occur during interest rate operations
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum InterestRateError {
    /// Unauthorized access - caller is not admin
    Unauthorized = 1,
    /// Invalid parameter value
    InvalidParameter = 2,
    /// Parameter change exceeds maximum allowed change
    ParameterChangeTooLarge = 3,
    /// Overflow occurred during calculation
    Overflow = 4,
    /// Division by zero (e.g., no deposits)
    DivisionByZero = 5,
    /// Contract has already been initialized
    AlreadyInitialized = 6,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum InterestRateModelKind {
    Linear = 0,
    Kink = 1,
    Jump = 2,
    Exponential = 3,
}

/// Storage keys for interest rate data
#[contracttype]
#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum InterestRateDataKey {
    /// Kink-based interest rate model parameters
    /// Value type: InterestRateConfig
    InterestRateConfig,
    /// Module admin address authorized for rate adjustments
    /// Value type: Address
    Admin,
    /// Placeholder for emergency rate adjustment status
    EmergencyRateAdjustment,
    /// Global compound interest index (borrow + supply)
    /// Value type: LendingIndex
    LendingIndex,
}

/// Index scale: 1e12 represents 1.0 (to maintain precision for compound growth)
pub const INDEX_SCALE: i128 = 1_000_000_000_000;

/// Global lending index for compound interest accrual.
/// Borrow index starts at INDEX_SCALE (= 1.0) and grows each time interest is accrued.
/// User interest = principal * (current_index - user_index) / user_index
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct LendingIndex {
    /// Current borrow index (scaled by INDEX_SCALE)
    pub borrow_index: i128,
    /// Current supply index (scaled by INDEX_SCALE)
    pub supply_index: i128,
    /// Timestamp of the last index update
    pub last_update: u64,
}

/// Interest rate configuration parameters
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct InterestRateConfig {
    /// Active interest rate model implementation
    pub model: InterestRateModelKind,
    /// Base interest rate (in basis points, e.g., 100 = 1% per year)
    /// This is the minimum rate when utilization is 0%
    pub base_rate_bps: i128,
    /// Kink utilization (in basis points, e.g., 8000 = 80%)
    /// Below this utilization, rate increases slowly. Above it, rate increases faster.
    pub kink_utilization_bps: i128,
    /// Multiplier for rate below kink (in basis points, e.g., 2000 = 20x)
    /// Rate below kink = base_rate + (utilization / kink_utilization) * multiplier
    pub multiplier_bps: i128,
    /// Jump multiplier for rate above kink (in basis points, e.g., 10000 = 100x)
    /// Rate above kink = base_rate + multiplier + (utilization - kink_utilization) / (10000 - kink_utilization) * jump_multiplier
    pub jump_multiplier_bps: i128,
    /// Minimum interest rate floor (in basis points)
    pub rate_floor_bps: i128,
    /// Maximum interest rate ceiling (in basis points)
    pub rate_ceiling_bps: i128,
    /// Spread between borrow and supply rates (in basis points, e.g., 200 = 2%)
    /// Supply rate = borrow rate - spread
    pub spread_bps: i128,
    /// Emergency rate adjustment (in basis points, added/subtracted from calculated rate)
    /// Can be positive or negative
    pub emergency_adjustment_bps: i128,
    /// Last update timestamp
    pub last_update: u64,
}

/// Constants for validation
const BASIS_POINTS_SCALE: i128 = 10_000; // 100% = 10,000 basis points
const SECONDS_PER_YEAR: u64 = 365 * 86400; // 31,536,000 seconds

/// Default interest rate configuration
fn get_default_config() -> InterestRateConfig {
    InterestRateConfig {
        model: InterestRateModelKind::Kink,
        base_rate_bps: 100,          // 1% base rate
        kink_utilization_bps: 8000,  // 80% kink
        multiplier_bps: 2000,        // 20% multiplier below kink
        jump_multiplier_bps: 10000,  // 100% jump multiplier above kink
        rate_floor_bps: 50,          // 0.5% minimum rate
        rate_ceiling_bps: 10000,     // 100% maximum rate
        spread_bps: 200,             // 2% spread
        emergency_adjustment_bps: 0, // No emergency adjustment
        last_update: 0,
    }
}

fn checked_mul_div(lhs: i128, rhs: i128, divisor: i128) -> Result<i128, InterestRateError> {
    if divisor == 0 {
        return Err(InterestRateError::DivisionByZero);
    }
    lhs.checked_mul(rhs)
        .ok_or(InterestRateError::Overflow)?
        .checked_div(divisor)
        .ok_or(InterestRateError::DivisionByZero)
}

fn linear_rate(utilization: i128, config: &InterestRateConfig) -> Result<i128, InterestRateError> {
    let increase = checked_mul_div(utilization, config.multiplier_bps, BASIS_POINTS_SCALE)?;
    config
        .base_rate_bps
        .checked_add(increase)
        .ok_or(InterestRateError::Overflow)
}

fn kink_rate(utilization: i128, config: &InterestRateConfig) -> Result<i128, InterestRateError> {
    if utilization <= config.kink_utilization_bps {
        if config.kink_utilization_bps == 0 {
            return Ok(config.base_rate_bps);
        }
        let increase = checked_mul_div(
            utilization,
            config.multiplier_bps,
            config.kink_utilization_bps,
        )?;
        return config
            .base_rate_bps
            .checked_add(increase)
            .ok_or(InterestRateError::Overflow);
    }

    let rate_at_kink = config
        .base_rate_bps
        .checked_add(config.multiplier_bps)
        .ok_or(InterestRateError::Overflow)?;
    let utilization_above_kink = utilization
        .checked_sub(config.kink_utilization_bps)
        .ok_or(InterestRateError::Overflow)?;
    let max_utilization_above_kink = BASIS_POINTS_SCALE
        .checked_sub(config.kink_utilization_bps)
        .ok_or(InterestRateError::Overflow)?;
    let additional_rate = checked_mul_div(
        utilization_above_kink,
        config.jump_multiplier_bps,
        max_utilization_above_kink,
    )?;

    rate_at_kink
        .checked_add(additional_rate)
        .ok_or(InterestRateError::Overflow)
}

fn jump_rate(utilization: i128, config: &InterestRateConfig) -> Result<i128, InterestRateError> {
    let mut rate = linear_rate(utilization, config)?;
    if utilization > config.kink_utilization_bps {
        let utilization_above_kink = utilization
            .checked_sub(config.kink_utilization_bps)
            .ok_or(InterestRateError::Overflow)?;
        let max_utilization_above_kink = BASIS_POINTS_SCALE
            .checked_sub(config.kink_utilization_bps)
            .ok_or(InterestRateError::Overflow)?;
        let jump = checked_mul_div(
            utilization_above_kink,
            config.jump_multiplier_bps,
            max_utilization_above_kink,
        )?;
        rate = rate.checked_add(jump).ok_or(InterestRateError::Overflow)?;
    }
    Ok(rate)
}

fn exponential_rate(
    utilization: i128,
    config: &InterestRateConfig,
) -> Result<i128, InterestRateError> {
    let utilization_squared = checked_mul_div(utilization, utilization, BASIS_POINTS_SCALE)?;
    let utilization_cubed = checked_mul_div(utilization_squared, utilization, BASIS_POINTS_SCALE)?;
    let quadratic = checked_mul_div(
        utilization_squared,
        config.multiplier_bps,
        BASIS_POINTS_SCALE,
    )?;
    let cubic = checked_mul_div(
        utilization_cubed,
        config.jump_multiplier_bps,
        BASIS_POINTS_SCALE,
    )?;

    config
        .base_rate_bps
        .checked_add(quadratic)
        .ok_or(InterestRateError::Overflow)?
        .checked_add(cubic)
        .ok_or(InterestRateError::Overflow)
}

pub fn calculate_model_borrow_rate(
    model: InterestRateModelKind,
    utilization: i128,
    config: &InterestRateConfig,
) -> Result<i128, InterestRateError> {
    match model {
        InterestRateModelKind::Linear => linear_rate(utilization, config),
        InterestRateModelKind::Kink => kink_rate(utilization, config),
        InterestRateModelKind::Jump => jump_rate(utilization, config),
        InterestRateModelKind::Exponential => exponential_rate(utilization, config),
    }
}

/// Get interest rate configuration
pub fn get_interest_rate_config(env: &Env) -> Option<InterestRateConfig> {
    let config_key = InterestRateDataKey::InterestRateConfig;
    env.storage()
        .persistent()
        .get::<InterestRateDataKey, InterestRateConfig>(&config_key)
}

/// Initialize interest rate configuration
pub fn initialize_interest_rate_config(env: &Env, admin: Address) -> Result<(), InterestRateError> {
    let config_key = InterestRateDataKey::InterestRateConfig;

    // Guard against double initialization
    if env
        .storage()
        .persistent()
        .has::<InterestRateDataKey>(&config_key)
    {
        return Err(InterestRateError::AlreadyInitialized);
    }

    let config = get_default_config();
    env.storage().persistent().set(&config_key, &config);

    Ok(())
}

/// Calculate protocol utilization
/// Utilization = total_borrows / total_deposits (in basis points)
/// Returns utilization in basis points (0-10000)
pub fn calculate_utilization(env: &Env) -> Result<i128, InterestRateError> {
    let analytics_key = DepositDataKey::ProtocolAnalytics;
    let analytics = env
        .storage()
        .persistent()
        .get::<DepositDataKey, ProtocolAnalytics>(&analytics_key)
        .unwrap_or(ProtocolAnalytics {
            total_deposits: 0,
            total_borrows: 0,
            total_value_locked: 0,
        });

    if analytics.total_deposits == 0 {
        return Ok(0); // No deposits means 0% utilization
    }

    // Calculate utilization: (borrows * 10000) / deposits
    let utilization = analytics
        .total_borrows
        .checked_mul(BASIS_POINTS_SCALE)
        .ok_or(InterestRateError::Overflow)?
        .checked_div(analytics.total_deposits)
        .ok_or(InterestRateError::DivisionByZero)?;

    // Cap at 100%
    Ok(utilization.min(BASIS_POINTS_SCALE))
}

/// Calculate borrow interest rate based on utilization
/// Uses a piecewise linear model with a kink
///
/// Below kink: rate = base_rate + (utilization / kink_utilization) * multiplier
/// Above kink: rate = base_rate + multiplier + ((utilization - kink) / (10000 - kink)) * jump_multiplier
pub fn calculate_borrow_rate(env: &Env) -> Result<i128, InterestRateError> {
    let config = get_interest_rate_config(env).ok_or(InterestRateError::InvalidParameter)?;
    let utilization = calculate_utilization(env)?;

    let mut rate = calculate_model_borrow_rate(config.model, utilization, &config)?;

    // Apply emergency adjustment
    rate = rate
        .checked_add(config.emergency_adjustment_bps)
        .ok_or(InterestRateError::Overflow)?;

    // Apply rate limits
    rate = rate.max(config.rate_floor_bps).min(config.rate_ceiling_bps);

    Ok(rate)
}

/// Calculate supply interest rate
/// Supply rate = borrow rate - spread
pub fn calculate_supply_rate(env: &Env) -> Result<i128, InterestRateError> {
    let config = get_interest_rate_config(env).ok_or(InterestRateError::InvalidParameter)?;
    let borrow_rate = calculate_borrow_rate(env)?;

    // Supply rate = borrow rate - spread
    let supply_rate = borrow_rate
        .checked_sub(config.spread_bps)
        .ok_or(InterestRateError::Overflow)?;

    // Ensure supply rate doesn't go below floor
    Ok(supply_rate.max(config.rate_floor_bps))
}

/// Calculate accrued interest using dynamic rate
///
/// # Arguments
/// * `principal` - The principal amount
/// * `last_accrual_time` - Last time interest was accrued
/// * `current_time` - Current timestamp
/// * `rate_bps` - Interest rate in basis points (annual)
///
/// # Returns
/// Accrued interest amount
pub fn calculate_accrued_interest(
    principal: i128,
    last_accrual_time: u64,
    current_time: u64,
    rate_bps: i128,
) -> Result<i128, InterestRateError> {
    if principal == 0 {
        return Ok(0);
    }

    if current_time <= last_accrual_time {
        return Ok(0);
    }

    // Calculate time elapsed in seconds
    let time_elapsed = current_time
        .checked_sub(last_accrual_time)
        .ok_or(InterestRateError::Overflow)?;

    // Calculate interest: principal * (rate / 10000) * (time_elapsed / seconds_per_year)
    // To avoid precision loss: principal * rate * time_elapsed / (10000 * seconds_per_year)
    let denominator = BASIS_POINTS_SCALE
        .checked_mul(SECONDS_PER_YEAR as i128)
        .ok_or(InterestRateError::Overflow)?;

    let numerator = principal
        .checked_mul(rate_bps)
        .ok_or(InterestRateError::Overflow)?
        .checked_mul(time_elapsed as i128)
        .ok_or(InterestRateError::Overflow)?;

    let interest = numerator
        .checked_div(denominator)
        .ok_or(InterestRateError::DivisionByZero)?;

    Ok(interest)
}

/// Update interest rate configuration parameters
///
/// # Arguments
/// * `env` - The Soroban environment
/// * `caller` - The caller address (must be admin)
/// * `base_rate_bps` - New base rate (None to keep current)
/// * `kink_utilization_bps` - New kink utilization (None to keep current)
/// * `multiplier_bps` - New multiplier (None to keep current)
/// * `jump_multiplier_bps` - New jump multiplier (None to keep current)
/// * `rate_floor_bps` - New rate floor (None to keep current)
/// * `rate_ceiling_bps` - New rate ceiling (None to keep current)
/// * `spread_bps` - New spread (None to keep current)
#[allow(clippy::too_many_arguments)]
pub fn update_interest_rate_config(
    env: &Env,
    caller: Address,
    base_rate_bps: Option<i128>,
    kink_utilization_bps: Option<i128>,
    multiplier_bps: Option<i128>,
    jump_multiplier_bps: Option<i128>,
    rate_floor_bps: Option<i128>,
    rate_ceiling_bps: Option<i128>,
    spread_bps: Option<i128>,
) -> Result<(), InterestRateError> {
    // Check authorization
    crate::admin::require_admin(env, &caller).map_err(|_| InterestRateError::Unauthorized)?;

    let config_key = InterestRateDataKey::InterestRateConfig;
    let mut config = get_interest_rate_config(env).ok_or(InterestRateError::InvalidParameter)?;

    // Update parameters with validation
    if let Some(rate) = base_rate_bps {
        if !(0..=BASIS_POINTS_SCALE).contains(&rate) {
            return Err(InterestRateError::InvalidParameter);
        }
        config.base_rate_bps = rate;
    }

    if let Some(kink) = kink_utilization_bps {
        if kink <= 0 || kink >= BASIS_POINTS_SCALE {
            return Err(InterestRateError::InvalidParameter);
        }
        config.kink_utilization_bps = kink;
    }

    if let Some(mult) = multiplier_bps {
        if mult < 0 {
            return Err(InterestRateError::InvalidParameter);
        }
        config.multiplier_bps = mult;
    }

    if let Some(jump) = jump_multiplier_bps {
        if jump < 0 {
            return Err(InterestRateError::InvalidParameter);
        }
        config.jump_multiplier_bps = jump;
    }

    if let Some(floor) = rate_floor_bps {
        if !(0..=BASIS_POINTS_SCALE).contains(&floor) {
            return Err(InterestRateError::InvalidParameter);
        }
        if floor > config.rate_ceiling_bps {
            return Err(InterestRateError::InvalidParameter);
        }
        config.rate_floor_bps = floor;
    }

    if let Some(ceiling) = rate_ceiling_bps {
        if !(0..=BASIS_POINTS_SCALE).contains(&ceiling) {
            return Err(InterestRateError::InvalidParameter);
        }
        if ceiling < config.rate_floor_bps {
            return Err(InterestRateError::InvalidParameter);
        }
        config.rate_ceiling_bps = ceiling;
    }

    if let Some(spread) = spread_bps {
        if !(0..=BASIS_POINTS_SCALE).contains(&spread) {
            return Err(InterestRateError::InvalidParameter);
        }
        config.spread_bps = spread;
    }

    config.last_update = env.ledger().timestamp();
    env.storage().persistent().set(&config_key, &config);

    Ok(())
}

pub fn switch_interest_rate_model(
    env: &Env,
    caller: Address,
    model: InterestRateModelKind,
) -> Result<(), InterestRateError> {
    crate::admin::require_admin(env, &caller).map_err(|_| InterestRateError::Unauthorized)?;

    let config_key = InterestRateDataKey::InterestRateConfig;
    let mut config = get_interest_rate_config(env).ok_or(InterestRateError::InvalidParameter)?;
    config.model = model;
    config.last_update = env.ledger().timestamp();
    env.storage().persistent().set(&config_key, &config);

    Ok(())
}

/// Set emergency rate adjustment
///
/// # Arguments
/// * `env` - The Soroban environment
/// * `caller` - The caller address (must be admin)
/// * `adjustment_bps` - Emergency adjustment in basis points (can be negative)
pub fn set_emergency_rate_adjustment(
    env: &Env,
    caller: Address,
    adjustment_bps: i128,
) -> Result<(), InterestRateError> {
    // Check authorization
    crate::admin::require_admin(env, &caller).map_err(|_| InterestRateError::Unauthorized)?;

    // Validate adjustment is within reasonable bounds
    if adjustment_bps.abs() > BASIS_POINTS_SCALE {
        return Err(InterestRateError::InvalidParameter);
    }

    let config_key = InterestRateDataKey::InterestRateConfig;
    let mut config = get_interest_rate_config(env).ok_or(InterestRateError::InvalidParameter)?;

    config.emergency_adjustment_bps = adjustment_bps;
    config.last_update = env.ledger().timestamp();

    env.storage().persistent().set(&config_key, &config);

    Ok(())
}

/// Get current borrow rate (in basis points)
pub fn get_current_borrow_rate(env: &Env) -> Result<i128, InterestRateError> {
    calculate_borrow_rate(env)
}

/// Get current supply rate (in basis points)
pub fn get_current_supply_rate(env: &Env) -> Result<i128, InterestRateError> {
    calculate_supply_rate(env)
}

/// Get current utilization (in basis points)
pub fn get_current_utilization(env: &Env) -> Result<i128, InterestRateError> {
    calculate_utilization(env)
}

// -------------------------------------------------------------------------
// Compound Interest Index
// -------------------------------------------------------------------------

/// Return the stored global lending index, or a fresh default if not yet initialised.
pub fn get_lending_index(env: &Env) -> LendingIndex {
    env.storage()
        .persistent()
        .get::<InterestRateDataKey, LendingIndex>(&InterestRateDataKey::LendingIndex)
        .unwrap_or(LendingIndex {
            borrow_index: INDEX_SCALE,
            supply_index: INDEX_SCALE,
            last_update: env.ledger().timestamp(),
        })
}

/// Update the global borrow/supply indices based on time elapsed since the last update.
///
/// The indices grow continuously:
///   new_index = old_index + old_index * rate_bps * time_elapsed / (BASIS_POINTS * SECONDS_PER_YEAR)
///
/// Idempotent within the same ledger second (returns stored value unchanged).
pub fn update_lending_index(env: &Env) -> Result<LendingIndex, InterestRateError> {
    let mut idx = get_lending_index(env);
    let current_time = env.ledger().timestamp();

    if current_time <= idx.last_update {
        return Ok(idx);
    }

    let time_elapsed = current_time - idx.last_update;
    let borrow_rate = calculate_borrow_rate(env).unwrap_or(0);
    let supply_rate = calculate_supply_rate(env).unwrap_or(0);

    let denom = BASIS_POINTS_SCALE
        .checked_mul(SECONDS_PER_YEAR as i128)
        .ok_or(InterestRateError::Overflow)?;

    // borrow_index growth
    let borrow_growth = idx
        .borrow_index
        .checked_mul(borrow_rate)
        .ok_or(InterestRateError::Overflow)?
        .checked_mul(time_elapsed as i128)
        .ok_or(InterestRateError::Overflow)?
        .checked_div(denom)
        .unwrap_or(0);

    // supply_index growth
    let supply_growth = idx
        .supply_index
        .checked_mul(supply_rate)
        .ok_or(InterestRateError::Overflow)?
        .checked_mul(time_elapsed as i128)
        .ok_or(InterestRateError::Overflow)?
        .checked_div(denom)
        .unwrap_or(0);

    idx.borrow_index = idx
        .borrow_index
        .checked_add(borrow_growth)
        .ok_or(InterestRateError::Overflow)?;
    idx.supply_index = idx
        .supply_index
        .checked_add(supply_growth)
        .ok_or(InterestRateError::Overflow)?;
    idx.last_update = current_time;

    env.storage()
        .persistent()
        .set(&InterestRateDataKey::LendingIndex, &idx);

    Ok(idx)
}

/// Return the current borrow index (scaled by INDEX_SCALE).
pub fn get_borrow_index(env: &Env) -> i128 {
    get_lending_index(env).borrow_index
}

/// Return the current supply index (scaled by INDEX_SCALE).
pub fn get_supply_index(env: &Env) -> i128 {
    get_lending_index(env).supply_index
}

/// Compute the compound interest accrued on `principal` as the index moved
/// from `user_index` to `current_index`.
///
/// Returns 0 when `current_index <= user_index` or `user_index == 0`.
pub fn compute_index_interest(
    principal: i128,
    user_index: i128,
    current_index: i128,
) -> Result<i128, InterestRateError> {
    if principal == 0 || user_index == 0 || current_index <= user_index {
        return Ok(0);
    }
    let delta = current_index
        .checked_sub(user_index)
        .ok_or(InterestRateError::Overflow)?;
    let interest = principal
        .checked_mul(delta)
        .ok_or(InterestRateError::Overflow)?
        .checked_div(user_index)
        .ok_or(InterestRateError::DivisionByZero)?;
    Ok(interest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(model: InterestRateModelKind) -> InterestRateConfig {
        InterestRateConfig {
            model,
            base_rate_bps: 100,
            kink_utilization_bps: 8000,
            multiplier_bps: 2000,
            jump_multiplier_bps: 10_000,
            rate_floor_bps: 0,
            rate_ceiling_bps: 10_000,
            spread_bps: 200,
            emergency_adjustment_bps: 0,
            last_update: 0,
        }
    }

    #[test]
    fn model_formulas_cover_all_implementations() {
        let util = 9000;
        assert_eq!(
            calculate_model_borrow_rate(
                InterestRateModelKind::Linear,
                util,
                &cfg(InterestRateModelKind::Linear)
            )
            .unwrap(),
            1900
        );
        assert_eq!(
            calculate_model_borrow_rate(
                InterestRateModelKind::Kink,
                util,
                &cfg(InterestRateModelKind::Kink)
            )
            .unwrap(),
            7100
        );
        assert_eq!(
            calculate_model_borrow_rate(
                InterestRateModelKind::Jump,
                util,
                &cfg(InterestRateModelKind::Jump)
            )
            .unwrap(),
            6900
        );
        assert_eq!(
            calculate_model_borrow_rate(
                InterestRateModelKind::Exponential,
                util,
                &cfg(InterestRateModelKind::Exponential)
            )
            .unwrap(),
            9010
        );
    }
}
