#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, String};

const BPS_DIVISOR: i128 = 10_000;
const MAX_SCORE: u32 = 1000;
const LETTER_GRADES: [&str; 10] = ["A+", "A", "A-", "B+", "B", "B-", "C+", "C", "C-", "D"];

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum RiskScoringError {
    Unauthorized = 1,
    NotInitialized = 2,
    AlreadyInitialized = 3,
    InvalidPool = 4,
    InvalidInput = 5,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskScore {
    pub pool: Address,
    pub asset_volatility_score: u32,
    pub oracle_deviation_score: u32,
    pub pool_utilization_score: u32,
    pub liquidation_history_score: u32,
    pub overall_score: u32,
    pub letter_grade: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskFactorBreakdown {
    pub asset_volatility_bps: u32,
    pub oracle_deviation_bps: u32,
    pub pool_utilization_bps: u32,
    pub liquidation_history_bps: u32,
    pub weights: RiskWeights,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskWeights {
    pub asset_volatility_weight: u32,
    pub oracle_deviation_weight: u32,
    pub pool_utilization_weight: u32,
    pub liquidation_history_weight: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    Initialized,
    PoolRiskScore(Address),
    PoolHistory(Address),
    RiskWeights,
    HistoryCount,
}

fn score_to_letter_grade(score: u32) -> String {
    let idx = if score >= 950 { 0 }
    else if score >= 900 { 1 }
    else if score >= 850 { 2 }
    else if score >= 800 { 3 }
    else if score >= 750 { 4 }
    else if score >= 700 { 5 }
    else if score >= 650 { 6 }
    else if score >= 600 { 7 }
    else if score >= 550 { 8 }
    else { 9 };
    String::from_slice(&[], LETTER_GRADES[idx].as_bytes())
}

fn compute_risk_score(
    asset_volatility_bps: u32,
    oracle_deviation_bps: u32,
    pool_utilization_bps: u32,
    liquidation_history_bps: u32,
    weights: &RiskWeights,
) -> u32 {
    let vol_score = if asset_volatility_bps < 1000 { 250 }
    else if asset_volatility_bps < 2500 { 200 }
    else if asset_volatility_bps < 5000 { 150 }
    else { 100 };

    let oracle_score = if oracle_deviation_bps < 50 { 250 }
    else if oracle_deviation_bps < 100 { 200 }
    else if oracle_deviation_bps < 300 { 150 }
    else { 100 };

    let util_score = if pool_utilization_bps < 6000 { 250 }
    else if pool_utilization_bps < 8000 { 200 }
    else if pool_utilization_bps < 9500 { 150 }
    else { 100 };

    let liq_score = if liquidation_history_bps < 100 { 250 }
    else if liquidation_history_bps < 500 { 200 }
    else if liquidation_history_bps < 2000 { 150 }
    else { 100 };

    let weighted = (vol_score * weights.asset_volatility_weight
        + oracle_score * weights.oracle_deviation_weight
        + util_score * weights.pool_utilization_weight
        + liq_score * weights.liquidation_history_weight)
        / BPS_DIVISOR as u32;

    core::cmp::min(weighted, MAX_SCORE)
}

#[contract]
pub struct RiskScoringContract;

#[contractimpl]
impl RiskScoringContract {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(
            &DataKey::RiskWeights,
            &RiskWeights {
                asset_volatility_weight: 3000,
                oracle_deviation_weight: 2500,
                pool_utilization_weight: 2500,
                liquidation_history_weight: 2000,
            },
        );
    }

    pub fn set_risk_weights(env: Env, weights: RiskWeights) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        admin.require_auth();

        let total = weights.asset_volatility_weight
            + weights.oracle_deviation_weight
            + weights.pool_utilization_weight
            + weights.liquidation_history_weight;
        assert_eq!(total, BPS_DIVISOR as u32, "weights must sum to 10000");

        env.storage().instance().set(&DataKey::RiskWeights, &weights);
    }

    pub fn get_risk_weights(env: Env) -> RiskWeights {
        env.storage().instance().get(&DataKey::RiskWeights).expect("not initialized")
    }

    pub fn calculate_score(
        env: Env,
        pool: Address,
        asset_volatility_bps: u32,
        oracle_deviation_bps: u32,
        pool_utilization_bps: u32,
        liquidation_history_bps: u32,
    ) -> RiskScore {
        let weights: RiskWeights = env.storage().instance().get(&DataKey::RiskWeights).expect("not initialized");
        let overall = compute_risk_score(
            asset_volatility_bps,
            oracle_deviation_bps,
            pool_utilization_bps,
            liquidation_history_bps,
            &weights,
        );

        RiskScore {
            pool,
            asset_volatility_score: if asset_volatility_bps < 1000 { 250 } else if asset_volatility_bps < 2500 { 200 } else if asset_volatility_bps < 5000 { 150 } else { 100 },
            oracle_deviation_score: if oracle_deviation_bps < 50 { 250 } else if oracle_deviation_bps < 100 { 200 } else if oracle_deviation_bps < 300 { 150 } else { 100 },
            pool_utilization_score: if pool_utilization_bps < 6000 { 250 } else if pool_utilization_bps < 8000 { 200 } else if pool_utilization_bps < 9500 { 150 } else { 100 },
            liquidation_history_score: if liquidation_history_bps < 100 { 250 } else if liquidation_history_bps < 500 { 200 } else if liquidation_history_bps < 2000 { 150 } else { 100 },
            overall_score: overall,
            letter_grade: score_to_letter_grade(overall),
            timestamp: env.ledger().timestamp(),
        }
    }

    pub fn record_pool_risk_score(
        env: Env,
        pool: Address,
        score: RiskScore,
    ) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        admin.require_auth();
        env.storage().persistent().set(&DataKey::PoolRiskScore(pool), &score);
    }

    pub fn get_pool_risk_score(env: Env, pool: Address) -> Option<RiskScore> {
        env.storage().persistent().get(&DataKey::PoolRiskScore(pool))
    }

    pub fn get_default_weights() -> RiskWeights {
        RiskWeights {
            asset_volatility_weight: 3000,
            oracle_deviation_weight: 2500,
            pool_utilization_weight: 2500,
            liquidation_history_weight: 2000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
    use soroban_sdk::IntoVal;

    struct TestEnv {
        env: Env,
        contract_id: Address,
        admin: Address,
        pool: Address,
    }

    impl TestEnv {
        fn new() -> Self {
            let env = Env::default();
            let admin = Address::generate(&env);
            let pool = Address::generate(&env);
            let contract_id = env.register(RiskScoringContract, ());
            let client = RiskScoringContractClient::new(&env, &contract_id);
            client.initialize(&admin);
            TestEnv { env, contract_id, admin, pool }
        }

        fn client(&self) -> RiskScoringContractClient<'_> {
            RiskScoringContractClient::new(&self.env, &self.contract_id)
        }
    }

    #[test]
    fn test_initialize() {
        let t = TestEnv::new();
        let weights = t.client().get_risk_weights();
        assert_eq!(weights.asset_volatility_weight, 3000);
        assert_eq!(weights.oracle_deviation_weight, 2500);
    }

    #[test]
    fn test_calculate_score_high_risk() {
        let t = TestEnv::new();
        let score = t.client().calculate_score(
            &t.pool,
            &8000,  // high volatility
            &500,   // high oracle deviation
            &9800,  // near full utilization
            &5000,  // many liquidations
        );
        assert!(score.overall_score < 500);
        assert_eq!(score.letter_grade, String::from_slice(&t.env, b"D"));
    }

    #[test]
    fn test_calculate_score_low_risk() {
        let t = TestEnv::new();
        let score = t.client().calculate_score(
            &t.pool,
            &500,   // low volatility
            &20,    // low oracle deviation
            &4000,  // low utilization
            &50,    // few liquidations
        );
        assert!(score.overall_score >= 900);
        assert_eq!(score.letter_grade, String::from_slice(&t.env, b"A+"));
    }

    #[test]
    fn test_set_risk_weights() {
        let t = TestEnv::new();
        let new_weights = RiskWeights {
            asset_volatility_weight: 4000,
            oracle_deviation_weight: 2000,
            pool_utilization_weight: 2000,
            liquidation_history_weight: 2000,
        };
        t.env.mock_auths(&[MockAuth {
            address: &t.admin,
            invoke: &MockAuthInvoke {
                contract: &t.contract_id,
                fn_name: "set_risk_weights",
                args: (&new_weights,).into_val(&t.env),
                sub_invokes: &[],
            },
        }]);
        t.client().set_risk_weights(&new_weights);
        let stored = t.client().get_risk_weights();
        assert_eq!(stored.asset_volatility_weight, 4000);
    }

    #[test]
    fn test_record_and_get_pool_score() {
        let t = TestEnv::new();
        let score = t.client().calculate_score(
            &t.pool, &500, &20, &4000, &50
        );
        t.env.mock_auths(&[MockAuth {
            address: &t.admin,
            invoke: &MockAuthInvoke {
                contract: &t.contract_id,
                fn_name: "record_pool_risk_score",
                args: (&t.pool, &score).into_val(&t.env),
                sub_invokes: &[],
            },
        }]);
        t.client().record_pool_risk_score(&t.pool, &score);
        let stored = t.client().get_pool_risk_score(&t.pool).unwrap();
        assert_eq!(stored.overall_score, score.overall_score);
        assert_eq!(stored.letter_grade, score.letter_grade);
    }

    #[test]
    fn test_default_weights_sum_to_10000() {
        let w = RiskScoringContract::get_default_weights();
        assert_eq!(
            w.asset_volatility_weight + w.oracle_deviation_weight
                + w.pool_utilization_weight + w.liquidation_history_weight,
            BPS_DIVISOR as u32
        );
    }
}