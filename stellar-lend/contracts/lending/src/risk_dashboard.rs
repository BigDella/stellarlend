use soroban_sdk::{contracttype, Address, Env, Map, Vec};

#[contracttype]
#[derive(Clone, Debug)]
pub struct PoolHealthMetrics {
    pub pool_id: Address,
    pub utilization_rate: i128,
    pub total_supplied: i128,
    pub total_borrowed: i128,
    pub available_liquidity: i128,
    pub average_ltv: i128,
    pub concentration_risk: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct LiquidationRiskEntry {
    pub user: Address,
    pub pool_id: Address,
    pub health_factor: i128,
    pub collateral_value: i128,
    pub debt_value: i128,
    pub liquidation_threshold: i128,
    pub risk_level: RiskLevel,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RiskLevel {
    Safe,
    Warning,
    Danger,
    Critical,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct OracleHealthStatus {
    pub asset: Address,
    pub last_update_timestamp: u64,
    pub price: i128,
    pub staleness_seconds: u64,
    pub deviation_from_twap: i128,
    pub is_healthy: bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ProtocolSafetyScore {
    pub overall_score: i128,
    pub liquidity_score: i128,
    pub solvency_score: i128,
    pub oracle_health_score: i128,
    pub concentration_score: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct AlertConfig {
    pub health_factor_threshold: i128,
    pub utilization_threshold: i128,
    pub concentration_threshold: i128,
    pub oracle_staleness_threshold: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct RiskMetricTrend {
    pub metric_name: soroban_sdk::String,
    pub current_value: i128,
    pub previous_value: i128,
    pub change_percentage: i128,
    pub timestamp: u64,
}

pub fn calculate_pool_health(
    env: &Env,
    pool_id: &Address,
    total_supplied: i128,
    total_borrowed: i128,
) -> PoolHealthMetrics {
    let utilization_rate = if total_supplied > 0 {
        (total_borrowed * 10_000) / total_supplied
    } else {
        0
    };

    let available_liquidity = total_supplied - total_borrowed;
    
    let concentration_risk = calculate_concentration_risk(env, pool_id);
    let average_ltv = calculate_average_ltv(env, pool_id);

    PoolHealthMetrics {
        pool_id: pool_id.clone(),
        utilization_rate,
        total_supplied,
        total_borrowed,
        available_liquidity,
        average_ltv,
        concentration_risk,
    }
}

pub fn calculate_liquidation_risk(
    user: &Address,
    pool_id: &Address,
    health_factor: i128,
    collateral_value: i128,
    debt_value: i128,
    liquidation_threshold: i128,
) -> LiquidationRiskEntry {
    let risk_level = determine_risk_level(health_factor);

    LiquidationRiskEntry {
        user: user.clone(),
        pool_id: pool_id.clone(),
        health_factor,
        collateral_value,
        debt_value,
        liquidation_threshold,
        risk_level,
    }
}

pub fn determine_risk_level(health_factor: i128) -> RiskLevel {
    if health_factor >= 15_000 {
        RiskLevel::Safe
    } else if health_factor >= 12_000 {
        RiskLevel::Warning
    } else if health_factor >= 10_000 {
        RiskLevel::Danger
    } else {
        RiskLevel::Critical
    }
}

pub fn check_oracle_health(
    env: &Env,
    asset: &Address,
    price: i128,
    last_update: u64,
    staleness_threshold: u64,
) -> OracleHealthStatus {
    let current_time = env.ledger().timestamp();
    let staleness_seconds = current_time.saturating_sub(last_update);
    
    let twap_deviation = calculate_twap_deviation(env, asset, price);
    
    let is_healthy = staleness_seconds <= staleness_threshold && 
                     twap_deviation.abs() <= 1_000;

    OracleHealthStatus {
        asset: asset.clone(),
        last_update_timestamp: last_update,
        price,
        staleness_seconds,
        deviation_from_twap: twap_deviation,
        is_healthy,
    }
}

pub fn calculate_protocol_safety_score(
    env: &Env,
    liquidity_score: i128,
    solvency_score: i128,
    oracle_score: i128,
    concentration_score: i128,
) -> ProtocolSafetyScore {
    let overall_score = (liquidity_score * 25 + 
                         solvency_score * 35 + 
                         oracle_score * 20 + 
                         concentration_score * 20) / 100;

    ProtocolSafetyScore {
        overall_score,
        liquidity_score,
        solvency_score,
        oracle_health_score: oracle_score,
        concentration_score,
        timestamp: env.ledger().timestamp(),
    }
}

pub fn get_liquidation_heatmap(
    env: &Env,
    pools: &Vec<Address>,
) -> Map<Address, Vec<LiquidationRiskEntry>> {
    let mut heatmap = Map::new(env);
    
    for pool in pools.iter() {
        let risks = get_pool_liquidation_risks(env, &pool);
        heatmap.set(pool, risks);
    }
    
    heatmap
}

pub fn calculate_metric_trend(
    env: &Env,
    metric_name: soroban_sdk::String,
    current_value: i128,
    previous_value: i128,
) -> RiskMetricTrend {
    let change_percentage = if previous_value != 0 {
        ((current_value - previous_value) * 10_000) / previous_value
    } else {
        0
    };

    RiskMetricTrend {
        metric_name,
        current_value,
        previous_value,
        change_percentage,
        timestamp: env.ledger().timestamp(),
    }
}

fn calculate_concentration_risk(_env: &Env, _pool_id: &Address) -> i128 {
    5_000
}

fn calculate_average_ltv(_env: &Env, _pool_id: &Address) -> i128 {
    6_000
}

fn calculate_twap_deviation(_env: &Env, _asset: &Address, _current_price: i128) -> i128 {
    200
}

fn get_pool_liquidation_risks(_env: &Env, _pool: &Address) -> Vec<LiquidationRiskEntry> {
    Vec::new(_env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_determine_risk_level() {
        assert_eq!(determine_risk_level(20_000), RiskLevel::Safe);
        assert_eq!(determine_risk_level(13_000), RiskLevel::Warning);
        assert_eq!(determine_risk_level(11_000), RiskLevel::Danger);
        assert_eq!(determine_risk_level(9_000), RiskLevel::Critical);
    }

    #[test]
    fn test_pool_health_calculation() {
        let env = Env::default();
        let pool_id = Address::generate(&env);
        
        let health = calculate_pool_health(&env, &pool_id, 1_000_000, 800_000);
        
        assert_eq!(health.utilization_rate, 8_000);
        assert_eq!(health.available_liquidity, 200_000);
    }

    #[test]
    fn test_protocol_safety_score() {
        let env = Env::default();
        
        let score = calculate_protocol_safety_score(&env, 8_000, 9_000, 8_500, 7_000);
        
        assert!(score.overall_score > 7_000);
        assert!(score.overall_score < 9_000);
    }

    #[test]
    fn test_metric_trend() {
        let env = Env::default();
        let name = soroban_sdk::String::from_str(&env, "utilization");
        
        let trend = calculate_metric_trend(&env, name, 8_000, 7_000);
        
        assert_eq!(trend.change_percentage, 1_428);
    }
}
