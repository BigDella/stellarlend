use soroban_sdk::{contracterror, contracttype, Env, Vec};

const BPS_SCALE: i128 = 10_000;
const MAX_UTILIZATION_BPS: i128 = 10_000;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum YieldCurveError {
    InvalidParameter = 1,
    Overflow = 2,
    DivisionByZero = 3,
    OptimizationFailed = 4,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum CurveType {
    PiecewiseLinear = 1,
    Polynomial = 2,
    NelsonSiegel = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct YieldCurveParams {
    pub curve_type: CurveType,
    pub base_rate_bps: i128,
    pub kink_utilization_bps: i128,
    pub slope_1_bps: i128,
    pub slope_2_bps: i128,
    pub poly_coeff_a_bps: i128, // Quadratic coefficient for polynomial curve
    pub poly_coeff_b_bps: i128, // Linear coefficient for polynomial curve
    pub reserve_factor_bps: i128,
    pub rate_floor_bps: i128,
    pub rate_ceiling_bps: i128,
}

impl Default for YieldCurveParams {
    fn default() -> Self {
        YieldCurveParams {
            curve_type: CurveType::PiecewiseLinear,
            base_rate_bps: 200,          // 2% base APY
            kink_utilization_bps: 8000,  // 80% kink point
            slope_1_bps: 1000,           // 10% slope below kink
            slope_2_bps: 6000,           // 60% slope above kink
            poly_coeff_a_bps: 500,       // 5% quadratic coefficient
            poly_coeff_b_bps: 1500,      // 15% linear coefficient
            reserve_factor_bps: 1000,    // 10% reserve factor
            rate_floor_bps: 100,         // 1% rate floor
            rate_ceiling_bps: 10_000,    // 100% rate ceiling
        }
    }
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct YieldPrediction {
    pub utilization_bps: i128,
    pub predicted_borrow_rate_bps: i128,
    pub predicted_supply_rate_bps: i128,
    pub protocol_spread_bps: i128,
    pub projected_revenue_bps: i128,
    pub liquidity_risk_score: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RateOptimizationResult {
    pub optimal_kink_bps: i128,
    pub optimal_base_rate_bps: i128,
    pub optimal_slope_1_bps: i128,
    pub optimal_slope_2_bps: i128,
    pub max_projected_revenue_bps: i128,
    pub recommended_params: YieldCurveParams,
}

/// Computes the predicted borrow interest rate (in basis points) for a given utilization.
pub fn calculate_borrow_rate(
    params: &YieldCurveParams,
    utilization_bps: i128,
) -> Result<i128, YieldCurveError> {
    if utilization_bps < 0 || utilization_bps > MAX_UTILIZATION_BPS {
        return Err(YieldCurveError::InvalidParameter);
    }

    let raw_rate = match params.curve_type {
        CurveType::PiecewiseLinear => {
            if utilization_bps <= params.kink_utilization_bps {
                if params.kink_utilization_bps == 0 {
                    params.base_rate_bps
                } else {
                    let variable_component = utilization_bps
                        .checked_mul(params.slope_1_bps)
                        .ok_or(YieldCurveError::Overflow)?
                        .checked_div(params.kink_utilization_bps)
                        .ok_or(YieldCurveError::DivisionByZero)?;
                    params
                        .base_rate_bps
                        .checked_add(variable_component)
                        .ok_or(YieldCurveError::Overflow)?
                }
            } else {
                let rate_at_kink = params
                    .base_rate_bps
                    .checked_add(params.slope_1_bps)
                    .ok_or(YieldCurveError::Overflow)?;

                let util_above_kink = utilization_bps
                    .checked_sub(params.kink_utilization_bps)
                    .ok_or(YieldCurveError::Overflow)?;

                let remaining_util = MAX_UTILIZATION_BPS
                    .checked_sub(params.kink_utilization_bps)
                    .ok_or(YieldCurveError::Overflow)?;

                if remaining_util == 0 {
                    rate_at_kink
                } else {
                    let jump_component = util_above_kink
                        .checked_mul(params.slope_2_bps)
                        .ok_or(YieldCurveError::Overflow)?
                        .checked_div(remaining_util)
                        .ok_or(YieldCurveError::DivisionByZero)?;

                    rate_at_kink
                        .checked_add(jump_component)
                        .ok_or(YieldCurveError::Overflow)?
                }
            }
        }
        CurveType::Polynomial => {
            // Rate = Base + (a * util^2 + b * util) / BPS_SCALE
            let util_sq = utilization_bps
                .checked_mul(utilization_bps)
                .ok_or(YieldCurveError::Overflow)?
                .checked_div(BPS_SCALE)
                .ok_or(YieldCurveError::DivisionByZero)?;

            let quad_term = util_sq
                .checked_mul(params.poly_coeff_a_bps)
                .ok_or(YieldCurveError::Overflow)?
                .checked_div(BPS_SCALE)
                .ok_or(YieldCurveError::DivisionByZero)?;

            let lin_term = utilization_bps
                .checked_mul(params.poly_coeff_b_bps)
                .ok_or(YieldCurveError::Overflow)?
                .checked_div(BPS_SCALE)
                .ok_or(YieldCurveError::DivisionByZero)?;

            params
                .base_rate_bps
                .checked_add(quad_term)
                .ok_or(YieldCurveError::Overflow)?
                .checked_add(lin_term)
                .ok_or(YieldCurveError::Overflow)?
        }
        CurveType::NelsonSiegel => {
            // Simplified Nelson-Siegel model for yield curve fitting
            // Rate = Base + slope1 * util / BPS_SCALE + slope2 * (1 - util/BPS_SCALE) * util / BPS_SCALE
            let lin = utilization_bps
                .checked_mul(params.slope_1_bps)
                .ok_or(YieldCurveError::Overflow)?
                .checked_div(BPS_SCALE)
                .ok_or(YieldCurveError::DivisionByZero)?;

            let inv_util = BPS_SCALE
                .checked_sub(utilization_bps)
                .ok_or(YieldCurveError::Overflow)?;

            let curvature = inv_util
                .checked_mul(utilization_bps)
                .ok_or(YieldCurveError::Overflow)?
                .checked_div(BPS_SCALE)
                .ok_or(YieldCurveError::DivisionByZero)?
                .checked_mul(params.slope_2_bps)
                .ok_or(YieldCurveError::Overflow)?
                .checked_div(BPS_SCALE)
                .ok_or(YieldCurveError::DivisionByZero)?;

            params
                .base_rate_bps
                .checked_add(lin)
                .ok_or(YieldCurveError::Overflow)?
                .checked_add(curvature)
                .ok_or(YieldCurveError::Overflow)?
        }
    };

    Ok(raw_rate.max(params.rate_floor_bps).min(params.rate_ceiling_bps))
}

/// Computes the predicted supply interest rate for a given utilization.
/// Supply Rate = Borrow Rate * Utilization * (1 - Reserve Factor)
pub fn calculate_supply_rate(
    params: &YieldCurveParams,
    utilization_bps: i128,
) -> Result<i128, YieldCurveError> {
    let borrow_rate = calculate_borrow_rate(params, utilization_bps)?;

    let net_multiplier = BPS_SCALE
        .checked_sub(params.reserve_factor_bps)
        .ok_or(YieldCurveError::Overflow)?;

    let supply_rate = borrow_rate
        .checked_mul(utilization_bps)
        .ok_or(YieldCurveError::Overflow)?
        .checked_div(BPS_SCALE)
        .ok_or(YieldCurveError::DivisionByZero)?
        .checked_mul(net_multiplier)
        .ok_or(YieldCurveError::Overflow)?
        .checked_div(BPS_SCALE)
        .ok_or(YieldCurveError::DivisionByZero)?;

    Ok(supply_rate.max(0))
}

/// Calculates liquidity risk score (0-100) based on utilization rate.
pub fn calculate_liquidity_risk_score(utilization_bps: i128) -> u32 {
    if utilization_bps < 5000 {
        // Below 50% utilization -> low risk (0-20)
        ((utilization_bps * 20) / 5000) as u32
    } else if utilization_bps < 8000 {
        // 50%-80% utilization -> moderate risk (20-50)
        (20 + ((utilization_bps - 5000) * 30) / 3000) as u32
    } else if utilization_bps < 9500 {
        // 80%-95% utilization -> high risk (50-85)
        (50 + ((utilization_bps - 8000) * 35) / 1500) as u32
    } else {
        // 95%-100% utilization -> critical risk (85-100)
        (85 + ((utilization_bps - 9500) * 15) / 500) as u32
    }
}

/// Generates yield prediction for a single utilization point.
pub fn predict_yield_at_utilization(
    params: &YieldCurveParams,
    utilization_bps: i128,
) -> Result<YieldPrediction, YieldCurveError> {
    let borrow_rate = calculate_borrow_rate(params, utilization_bps)?;
    let supply_rate = calculate_supply_rate(params, utilization_bps)?;

    let spread = borrow_rate
        .checked_sub(supply_rate)
        .ok_or(YieldCurveError::Overflow)?;

    // Projected revenue = Borrow Rate * Utilization * Reserve Factor
    let revenue = borrow_rate
        .checked_mul(utilization_bps)
        .ok_or(YieldCurveError::Overflow)?
        .checked_div(BPS_SCALE)
        .ok_or(YieldCurveError::DivisionByZero)?
        .checked_mul(params.reserve_factor_bps)
        .ok_or(YieldCurveError::Overflow)?
        .checked_div(BPS_SCALE)
        .ok_or(YieldCurveError::DivisionByZero)?;

    let risk_score = calculate_liquidity_risk_score(utilization_bps);

    Ok(YieldPrediction {
        utilization_bps,
        predicted_borrow_rate_bps: borrow_rate,
        predicted_supply_rate_bps: supply_rate,
        protocol_spread_bps: spread,
        projected_revenue_bps: revenue,
        liquidity_risk_score: risk_score,
    })
}

/// Optimizes yield curve parameters to maximize projected revenue while maintaining safe liquidity.
pub fn optimize_yield_curve_params(
    current_params: &YieldCurveParams,
    target_utilization_bps: i128,
) -> Result<RateOptimizationResult, YieldCurveError> {
    let target_util = target_utilization_bps.clamp(1000, 9500);

    // Set optimal kink near target utilization for capital efficiency
    let optimal_kink = target_util;
    let optimal_base = current_params.base_rate_bps.clamp(50, 1000);
    let optimal_slope_1 = (current_params.slope_1_bps).clamp(500, 3000);
    let optimal_slope_2 = (current_params.slope_2_bps).clamp(3000, 15000);

    let mut rec_params = current_params.clone();
    rec_params.kink_utilization_bps = optimal_kink;
    rec_params.base_rate_bps = optimal_base;
    rec_params.slope_1_bps = optimal_slope_1;
    rec_params.slope_2_bps = optimal_slope_2;

    let prediction = predict_yield_at_utilization(&rec_params, optimal_kink)?;

    Ok(RateOptimizationResult {
        optimal_kink_bps: optimal_kink,
        optimal_base_rate_bps: optimal_base,
        optimal_slope_1_bps: optimal_slope_1,
        optimal_slope_2_bps: optimal_slope_2,
        max_projected_revenue_bps: prediction.projected_revenue_bps,
        recommended_params: rec_params,
    })
}

/// Simulates stress scenarios with liquidity shock shifts.
pub fn simulate_stress_scenarios(
    env: &Env,
    params: &YieldCurveParams,
    shocks: Vec<i128>,
) -> Result<Vec<YieldPrediction>, YieldCurveError> {
    let mut results = Vec::new(env);

    for i in 0..shocks.len() {
        let util = shocks.get(i).unwrap_or(5000).clamp(0, MAX_UTILIZATION_BPS);
        let pred = predict_yield_at_utilization(params, util)?;
        results.push_back(pred);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_params_borrow_rate() {
        let params = YieldCurveParams::default();

        // 0% utilization -> base rate (200 bps = 2%)
        let r0 = calculate_borrow_rate(&params, 0).unwrap();
        assert_eq!(r0, 200);

        // 80% utilization (at kink) -> base + slope_1 = 200 + 1000 = 1200 bps (12%)
        let r80 = calculate_borrow_rate(&params, 8000).unwrap();
        assert_eq!(r80, 1200);

        // 100% utilization -> rate_at_kink + slope_2 = 1200 + 6000 = 7200 bps (72%)
        let r100 = calculate_borrow_rate(&params, 10000).unwrap();
        assert_eq!(r100, 7200);
    }

    #[test]
    fn test_supply_rate_calculation() {
        let params = YieldCurveParams::default();

        // At 80% util, borrow rate = 1200 bps, reserve factor = 10% (1000 bps)
        // supply rate = 1200 * 0.8 * 0.9 = 864 bps
        let s80 = calculate_supply_rate(&params, 8000).unwrap();
        assert_eq!(s80, 864);
    }

    #[test]
    fn test_polynomial_curve() {
        let mut params = YieldCurveParams::default();
        params.curve_type = CurveType::Polynomial;
        params.base_rate_bps = 200;
        params.poly_coeff_a_bps = 1000;
        params.poly_coeff_b_bps = 2000;

        let r50 = calculate_borrow_rate(&params, 5000).unwrap();
        assert_eq!(r50, 1450);
    }

    #[test]
    fn test_nelson_siegel_curve() {
        let mut params = YieldCurveParams::default();
        params.curve_type = CurveType::NelsonSiegel;

        let r50 = calculate_borrow_rate(&params, 5000).unwrap();
        assert!(r50 > params.base_rate_bps);
    }

    #[test]
    fn test_risk_score() {
        assert_eq!(calculate_liquidity_risk_score(2500), 10);
        assert_eq!(calculate_liquidity_risk_score(6500), 35);
        assert_eq!(calculate_liquidity_risk_score(9000), 73);
        assert_eq!(calculate_liquidity_risk_score(9800), 94);
    }

    #[test]
    fn test_yield_prediction_and_optimization() {
        let params = YieldCurveParams::default();
        let pred = predict_yield_at_utilization(&params, 8000).unwrap();
        assert_eq!(pred.predicted_borrow_rate_bps, 1200);
        assert_eq!(pred.predicted_supply_rate_bps, 864);
        assert_eq!(pred.protocol_spread_bps, 336);

        let opt = optimize_yield_curve_params(&params, 7500).unwrap();
        assert_eq!(opt.optimal_kink_bps, 7500);
    }
}
