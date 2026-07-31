use soroban_sdk::{contracttype, Address, Env, Symbol};

use crate::admin::require_admin;
use crate::interest_rate;
use crate::reserve::{ReserveDataKey, ReserveError, BASIS_POINTS_SCALE, DEFAULT_RESERVE_FACTOR_BPS, MAX_RESERVE_FACTOR_BPS};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReserveFactorCurve {
    pub base_bps: i128,
    pub slope_bps: i128,
    pub kink_utilization_bps: i128,
    pub jump_slope_bps: i128,
    pub min_bps: i128,
    pub max_bps: i128,
    pub enabled: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReserveFactorPreview {
    pub utilization_bps: i128,
    pub reserve_factor_bps: i128,
    pub static_reserve_factor_bps: i128,
    pub revenue_delta_bps: i128,
}

pub fn default_curve() -> ReserveFactorCurve {
    ReserveFactorCurve {
        base_bps: DEFAULT_RESERVE_FACTOR_BPS / 2,
        slope_bps: 700,
        kink_utilization_bps: 8_000,
        jump_slope_bps: 1_800,
        min_bps: 250,
        max_bps: MAX_RESERVE_FACTOR_BPS,
        enabled: true,
    }
}

pub fn validate_curve(curve: &ReserveFactorCurve) -> Result<(), ReserveError> {
    if curve.base_bps < 0
        || curve.slope_bps < 0
        || curve.jump_slope_bps < 0
        || curve.min_bps < 0
        || curve.max_bps > MAX_RESERVE_FACTOR_BPS
        || curve.min_bps > curve.max_bps
        || curve.kink_utilization_bps <= 0
        || curve.kink_utilization_bps >= BASIS_POINTS_SCALE
    {
        return Err(ReserveError::InvalidReserveFactor);
    }

    Ok(())
}

pub fn set_reserve_factor_curve(
    env: &Env,
    caller: Address,
    asset: Option<Address>,
    curve: ReserveFactorCurve,
) -> Result<(), ReserveError> {
    caller.require_auth();
    require_admin(env, &caller).map_err(|_| ReserveError::Unauthorized)?;
    validate_curve(&curve)?;

    env.storage()
        .persistent()
        .set(&ReserveDataKey::ReserveFactorCurve(asset.clone()), &curve);

    let topics = (Symbol::new(env, "reserve_curve_set"), caller);
    env.events().publish(topics, (asset, curve));

    Ok(())
}

pub fn set_reserve_factor_bounds(
    env: &Env,
    caller: Address,
    asset: Option<Address>,
    min_bps: i128,
    max_bps: i128,
) -> Result<(), ReserveError> {
    caller.require_auth();
    require_admin(env, &caller).map_err(|_| ReserveError::Unauthorized)?;

    let mut curve = get_reserve_factor_curve(env, asset.clone());
    curve.min_bps = min_bps;
    curve.max_bps = max_bps;
    validate_curve(&curve)?;

    env.storage()
        .persistent()
        .set(&ReserveDataKey::ReserveFactorCurve(asset.clone()), &curve);

    let topics = (Symbol::new(env, "reserve_bounds_set"), caller);
    env.events().publish(topics, (asset, min_bps, max_bps));

    Ok(())
}

pub fn get_reserve_factor_curve(env: &Env, asset: Option<Address>) -> ReserveFactorCurve {
    env.storage()
        .persistent()
        .get(&ReserveDataKey::ReserveFactorCurve(asset))
        .unwrap_or_else(default_curve)
}

pub fn calculate_dynamic_reserve_factor(
    utilization_bps: i128,
    curve: &ReserveFactorCurve,
) -> Result<i128, ReserveError> {
    validate_curve(curve)?;
    if !curve.enabled {
        return Ok(curve.base_bps.clamp(curve.min_bps, curve.max_bps));
    }

    let utilization = utilization_bps.clamp(0, BASIS_POINTS_SCALE);
    let mut factor = curve.base_bps;

    if utilization <= curve.kink_utilization_bps {
        factor = factor
            .checked_add(
                utilization
                    .checked_mul(curve.slope_bps)
                    .ok_or(ReserveError::Overflow)?
                    .checked_div(curve.kink_utilization_bps)
                    .ok_or(ReserveError::Overflow)?,
            )
            .ok_or(ReserveError::Overflow)?;
    } else {
        let above_kink = utilization
            .checked_sub(curve.kink_utilization_bps)
            .ok_or(ReserveError::Overflow)?;
        let denominator = BASIS_POINTS_SCALE
            .checked_sub(curve.kink_utilization_bps)
            .ok_or(ReserveError::Overflow)?;

        factor = factor
            .checked_add(curve.slope_bps)
            .ok_or(ReserveError::Overflow)?
            .checked_add(
                above_kink
                    .checked_mul(curve.jump_slope_bps)
                    .ok_or(ReserveError::Overflow)?
                    .checked_div(denominator)
                    .ok_or(ReserveError::Overflow)?,
            )
            .ok_or(ReserveError::Overflow)?;
    }

    Ok(factor.clamp(curve.min_bps, curve.max_bps))
}

pub fn get_dynamic_reserve_factor(
    env: &Env,
    asset: Option<Address>,
) -> Result<i128, ReserveError> {
    let utilization = interest_rate::calculate_utilization(env).map_err(|_| ReserveError::Overflow)?;
    let curve = get_reserve_factor_curve(env, asset);
    calculate_dynamic_reserve_factor(utilization, &curve)
}

pub fn preview_reserve_factor(
    env: &Env,
    asset: Option<Address>,
    utilization_bps: Option<i128>,
) -> Result<ReserveFactorPreview, ReserveError> {
    let utilization = utilization_bps
        .unwrap_or(interest_rate::calculate_utilization(env).map_err(|_| ReserveError::Overflow)?)
        .clamp(0, BASIS_POINTS_SCALE);
    let curve = get_reserve_factor_curve(env, asset.clone());
    let dynamic = calculate_dynamic_reserve_factor(utilization, &curve)?;
    let static_factor = crate::reserve::get_static_reserve_factor(env, asset);

    Ok(ReserveFactorPreview {
        utilization_bps: utilization,
        reserve_factor_bps: dynamic,
        static_reserve_factor_bps: static_factor,
        revenue_delta_bps: dynamic - static_factor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve() -> ReserveFactorCurve {
        ReserveFactorCurve {
            base_bps: 500,
            slope_bps: 700,
            kink_utilization_bps: 8_000,
            jump_slope_bps: 1_800,
            min_bps: 250,
            max_bps: 3_000,
            enabled: true,
        }
    }

    #[test]
    fn curve_scales_before_kink() {
        let factor = calculate_dynamic_reserve_factor(4_000, &curve()).unwrap();
        assert_eq!(factor, 850);
    }

    #[test]
    fn curve_is_continuous_at_kink() {
        let factor = calculate_dynamic_reserve_factor(8_000, &curve()).unwrap();
        assert_eq!(factor, 1_200);
    }

    #[test]
    fn curve_applies_jump_slope_after_kink() {
        let factor = calculate_dynamic_reserve_factor(9_000, &curve()).unwrap();
        assert_eq!(factor, 2_100);
    }

    #[test]
    fn curve_respects_governance_bounds() {
        let mut bounded = curve();
        bounded.max_bps = 1_500;
        assert_eq!(
            calculate_dynamic_reserve_factor(10_000, &bounded).unwrap(),
            1_500
        );

        bounded.min_bps = 900;
        assert_eq!(
            calculate_dynamic_reserve_factor(0, &bounded).unwrap(),
            900
        );
    }
}
