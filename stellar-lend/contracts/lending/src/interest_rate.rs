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
#[derive(Clone, Debug, PartialEq)]
pub struct InterestRateConfig {
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

    let mut rate = cfg.base_rate_bps;

    if util <= cfg.kink_utilization_bps {
        if cfg.kink_utilization_bps > 0 {
            let inc = util
                .checked_mul(cfg.slope_bps)
                .ok_or(InterestRateError::Overflow)?
                .checked_div(cfg.kink_utilization_bps)
                .ok_or(InterestRateError::DivisionByZero)?;
            rate = rate.checked_add(inc).ok_or(InterestRateError::Overflow)?;
        }
    } else {
        let rate_at_kink = cfg
            .base_rate_bps
            .checked_add(cfg.slope_bps)
            .ok_or(InterestRateError::Overflow)?;

        let util_above = util
            .checked_sub(cfg.kink_utilization_bps)
            .ok_or(InterestRateError::Overflow)?;

        let max_above = BPS_SCALE
            .checked_sub(cfg.kink_utilization_bps)
            .ok_or(InterestRateError::Overflow)?;

        if max_above > 0 {
            let addl = util_above
                .checked_mul(cfg.jump_slope_bps)
                .ok_or(InterestRateError::Overflow)?
                .checked_div(max_above)
                .ok_or(InterestRateError::DivisionByZero)?;

            rate = rate_at_kink
                .checked_add(addl)
                .ok_or(InterestRateError::Overflow)?;
        } else {
            rate = rate_at_kink;
        }
    }

    Ok(rate.max(cfg.rate_floor_bps).min(cfg.rate_ceiling_bps))
}

pub fn supply_rate_bps(env: &Env) -> Result<i128, InterestRateError> {
    let cfg = get_config(env);
    let borrow = borrow_rate_bps(env)?;
    let supply = borrow
        .checked_sub(cfg.spread_bps)
        .ok_or(InterestRateError::Overflow)?;

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

    Ok((prev, next))
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
