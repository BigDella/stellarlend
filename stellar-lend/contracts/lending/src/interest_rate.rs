use soroban_sdk::{contracterror, contracttype, Address, Env};

use crate::borrow::{get_admin, get_debt_ceiling, get_total_debt, BorrowDataKey, BorrowError};

const BPS_SCALE: i128 = 10_000;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum InterestRateError {
    Unauthorized = 1,
    InvalidParameter = 2,
    Overflow = 3,
    DivisionByZero = 4,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum InterestRateModelKind {
    Linear = 0,
    Kink = 1,
    Jump = 2,
    Exponential = 3,
}

impl InterestRateModelKind {
    pub fn from_code(code: u32) -> Result<Self, InterestRateError> {
        match code {
            0 => Ok(Self::Linear),
            1 => Ok(Self::Kink),
            2 => Ok(Self::Jump),
            3 => Ok(Self::Exponential),
            _ => Err(InterestRateError::InvalidParameter),
        }
    }
}

pub trait InterestRateModel {
    fn calculate(
        utilization_bps: i128,
        cfg: &InterestRateConfig,
    ) -> Result<i128, InterestRateError>;
}

pub struct LinearRateModel;
pub struct KinkRateModel;
pub struct JumpRateModel;
pub struct ExponentialRateModel;

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct InterestRateConfig {
    pub model: InterestRateModelKind,
    pub base_rate_bps: i128,
    pub kink_utilization_bps: i128,
    pub slope_bps: i128,
    pub jump_slope_bps: i128,
    pub rate_floor_bps: i128,
    pub rate_ceiling_bps: i128,
    pub spread_bps: i128,
    pub last_update: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct InterestRateConfigUpdate {
    pub model: Option<u32>,
    pub base_rate_bps: Option<i128>,
    pub kink_utilization_bps: Option<i128>,
    pub slope_bps: Option<i128>,
    pub jump_slope_bps: Option<i128>,
    pub rate_floor_bps: Option<i128>,
    pub rate_ceiling_bps: Option<i128>,
    pub spread_bps: Option<i128>,
}

fn default_config(env: &Env) -> InterestRateConfig {
    InterestRateConfig {
        model: InterestRateModelKind::Kink,
        base_rate_bps: 100,
        kink_utilization_bps: 8000,
        slope_bps: 2000,
        jump_slope_bps: 10_000,
        rate_floor_bps: 0,
        rate_ceiling_bps: 10_000,
        spread_bps: 200,
        last_update: env.ledger().timestamp(),
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

fn rate_at_kink(cfg: &InterestRateConfig) -> Result<i128, InterestRateError> {
    cfg.base_rate_bps
        .checked_add(cfg.slope_bps)
        .ok_or(InterestRateError::Overflow)
}

impl InterestRateModel for LinearRateModel {
    fn calculate(
        utilization_bps: i128,
        cfg: &InterestRateConfig,
    ) -> Result<i128, InterestRateError> {
        let inc = checked_mul_div(utilization_bps, cfg.slope_bps, BPS_SCALE)?;
        cfg.base_rate_bps
            .checked_add(inc)
            .ok_or(InterestRateError::Overflow)
    }
}

impl InterestRateModel for KinkRateModel {
    fn calculate(
        utilization_bps: i128,
        cfg: &InterestRateConfig,
    ) -> Result<i128, InterestRateError> {
        if utilization_bps <= cfg.kink_utilization_bps {
            if cfg.kink_utilization_bps == 0 {
                return Ok(cfg.base_rate_bps);
            }
            let inc = checked_mul_div(utilization_bps, cfg.slope_bps, cfg.kink_utilization_bps)?;
            return cfg
                .base_rate_bps
                .checked_add(inc)
                .ok_or(InterestRateError::Overflow);
        }

        let util_above = utilization_bps
            .checked_sub(cfg.kink_utilization_bps)
            .ok_or(InterestRateError::Overflow)?;
        let max_above = BPS_SCALE
            .checked_sub(cfg.kink_utilization_bps)
            .ok_or(InterestRateError::Overflow)?;
        let addl = checked_mul_div(util_above, cfg.jump_slope_bps, max_above)?;

        rate_at_kink(cfg)?
            .checked_add(addl)
            .ok_or(InterestRateError::Overflow)
    }
}

impl InterestRateModel for JumpRateModel {
    fn calculate(
        utilization_bps: i128,
        cfg: &InterestRateConfig,
    ) -> Result<i128, InterestRateError> {
        let linear = checked_mul_div(utilization_bps, cfg.slope_bps, BPS_SCALE)?;
        let mut rate = cfg
            .base_rate_bps
            .checked_add(linear)
            .ok_or(InterestRateError::Overflow)?;

        if utilization_bps > cfg.kink_utilization_bps {
            let util_above = utilization_bps
                .checked_sub(cfg.kink_utilization_bps)
                .ok_or(InterestRateError::Overflow)?;
            let max_above = BPS_SCALE
                .checked_sub(cfg.kink_utilization_bps)
                .ok_or(InterestRateError::Overflow)?;
            let jump = checked_mul_div(util_above, cfg.jump_slope_bps, max_above)?;
            rate = rate.checked_add(jump).ok_or(InterestRateError::Overflow)?;
        }

        Ok(rate)
    }
}

impl InterestRateModel for ExponentialRateModel {
    fn calculate(
        utilization_bps: i128,
        cfg: &InterestRateConfig,
    ) -> Result<i128, InterestRateError> {
        let util_squared = checked_mul_div(utilization_bps, utilization_bps, BPS_SCALE)?;
        let util_cubed = checked_mul_div(util_squared, utilization_bps, BPS_SCALE)?;
        let quadratic = checked_mul_div(util_squared, cfg.slope_bps, BPS_SCALE)?;
        let cubic = checked_mul_div(util_cubed, cfg.jump_slope_bps, BPS_SCALE)?;

        cfg.base_rate_bps
            .checked_add(quadratic)
            .ok_or(InterestRateError::Overflow)?
            .checked_add(cubic)
            .ok_or(InterestRateError::Overflow)
    }
}

pub fn calculate_model_rate_bps(
    model: InterestRateModelKind,
    utilization_bps: i128,
    cfg: &InterestRateConfig,
) -> Result<i128, InterestRateError> {
    match model {
        InterestRateModelKind::Linear => LinearRateModel::calculate(utilization_bps, cfg),
        InterestRateModelKind::Kink => KinkRateModel::calculate(utilization_bps, cfg),
        InterestRateModelKind::Jump => JumpRateModel::calculate(utilization_bps, cfg),
        InterestRateModelKind::Exponential => ExponentialRateModel::calculate(utilization_bps, cfg),
    }
}

fn clamp_rate(rate: i128, cfg: &InterestRateConfig) -> i128 {
    rate.max(cfg.rate_floor_bps).min(cfg.rate_ceiling_bps)
}

pub fn get_config(env: &Env) -> InterestRateConfig {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::BorrowInterestRate)
        .unwrap_or_else(|| default_config(env))
}

pub fn set_default_if_missing(env: &Env) {
    if env
        .storage()
        .persistent()
        .has::<BorrowDataKey>(&BorrowDataKey::BorrowInterestRate)
    {
        return;
    }

    let cfg = default_config(env);
    env.storage()
        .persistent()
        .set(&BorrowDataKey::BorrowInterestRate, &cfg);
}

pub fn utilization_bps(env: &Env) -> Result<i128, InterestRateError> {
    let ceiling = get_debt_ceiling(env);
    if ceiling <= 0 {
        return Ok(0);
    }

    let debt = get_total_debt(env);
    if debt <= 0 {
        return Ok(0);
    }

    let util = debt
        .checked_mul(BPS_SCALE)
        .ok_or(InterestRateError::Overflow)?
        .checked_div(ceiling)
        .ok_or(InterestRateError::DivisionByZero)?;

    Ok(util.min(BPS_SCALE).max(0))
}

pub fn borrow_rate_bps(env: &Env) -> Result<i128, InterestRateError> {
    let cfg = get_config(env);
    let util = utilization_bps(env)?;
    Ok(clamp_rate(
        calculate_model_rate_bps(cfg.model, util, &cfg)?,
        &cfg,
    ))
}

pub fn supply_rate_bps(env: &Env) -> Result<i128, InterestRateError> {
    let cfg = get_config(env);
    let borrow = borrow_rate_bps(env)?;
    let supply = if borrow <= cfg.spread_bps {
        0
    } else {
        borrow
            .checked_sub(cfg.spread_bps)
            .ok_or(InterestRateError::Overflow)?
    };

    Ok(supply.max(cfg.rate_floor_bps))
}

pub fn update_config(
    env: &Env,
    caller: &Address,
    update: InterestRateConfigUpdate,
) -> Result<(InterestRateConfig, InterestRateConfig), InterestRateError> {
    caller.require_auth();
    let Some(admin) = get_admin(env) else {
        return Err(InterestRateError::Unauthorized);
    };
    if *caller != admin {
        return Err(InterestRateError::Unauthorized);
    }

    let prev = get_config(env);
    let mut next = prev.clone();

    if let Some(model) = update.model {
        next.model = InterestRateModelKind::from_code(model)?;
    }

    if let Some(v) = update.base_rate_bps {
        if v < 0 || v > BPS_SCALE {
            return Err(InterestRateError::InvalidParameter);
        }
        next.base_rate_bps = v;
    }

    if let Some(v) = update.kink_utilization_bps {
        if v <= 0 || v >= BPS_SCALE {
            return Err(InterestRateError::InvalidParameter);
        }
        next.kink_utilization_bps = v;
    }

    if let Some(v) = update.slope_bps {
        if v < 0 {
            return Err(InterestRateError::InvalidParameter);
        }
        next.slope_bps = v;
    }

    if let Some(v) = update.jump_slope_bps {
        if v < 0 {
            return Err(InterestRateError::InvalidParameter);
        }
        next.jump_slope_bps = v;
    }

    if let Some(v) = update.rate_floor_bps {
        if v < 0 || v > BPS_SCALE {
            return Err(InterestRateError::InvalidParameter);
        }
        next.rate_floor_bps = v;
    }

    if let Some(v) = update.rate_ceiling_bps {
        if v < 0 || v > BPS_SCALE {
            return Err(InterestRateError::InvalidParameter);
        }
        next.rate_ceiling_bps = v;
    }

    if next.rate_floor_bps > next.rate_ceiling_bps {
        return Err(InterestRateError::InvalidParameter);
    }

    if let Some(v) = update.spread_bps {
        if v < 0 || v > BPS_SCALE {
            return Err(InterestRateError::InvalidParameter);
        }
        next.spread_bps = v;
    }

    next.last_update = env.ledger().timestamp();

    env.storage()
        .persistent()
        .set(&BorrowDataKey::BorrowInterestRate, &next);

    crate::events::InterestRateModelUpdatedEvent {
        caller: caller.clone(),
        previous: prev.clone(),
        updated: next.clone(),
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok((prev, next))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(model: InterestRateModelKind) -> InterestRateConfig {
        InterestRateConfig {
            model,
            base_rate_bps: 100,
            kink_utilization_bps: 8000,
            slope_bps: 2000,
            jump_slope_bps: 10_000,
            rate_floor_bps: 0,
            rate_ceiling_bps: 10_000,
            spread_bps: 200,
            last_update: 0,
        }
    }

    #[test]
    fn model_rate_formulas_are_distinct_and_bounded() {
        let util = 9000;
        assert_eq!(
            calculate_model_rate_bps(
                InterestRateModelKind::Linear,
                util,
                &cfg(InterestRateModelKind::Linear)
            )
            .unwrap(),
            1900
        );
        assert_eq!(
            calculate_model_rate_bps(
                InterestRateModelKind::Kink,
                util,
                &cfg(InterestRateModelKind::Kink)
            )
            .unwrap(),
            7100
        );
        assert_eq!(
            calculate_model_rate_bps(
                InterestRateModelKind::Jump,
                util,
                &cfg(InterestRateModelKind::Jump)
            )
            .unwrap(),
            6900
        );
        assert_eq!(
            calculate_model_rate_bps(
                InterestRateModelKind::Exponential,
                util,
                &cfg(InterestRateModelKind::Exponential)
            )
            .unwrap(),
            9010
        );
    }
}

impl From<InterestRateError> for BorrowError {
    fn from(value: InterestRateError) -> Self {
        match value {
            InterestRateError::Unauthorized => BorrowError::Unauthorized,
            InterestRateError::InvalidParameter => BorrowError::InvalidAmount,
            InterestRateError::Overflow => BorrowError::Overflow,
            InterestRateError::DivisionByZero => BorrowError::Overflow,
        }
    }
}
