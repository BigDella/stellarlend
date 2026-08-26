#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, Env, Vec};

pub mod hello_world_bridge;

// ── Strategy types ───────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum StrategyType {
    FixedDiscount,
    DutchAuction,
    TWAPBased,
    Hybrid,
}

// ── Strategy parameters ──────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct FixedDiscountParams {
    pub discount_bps: i128,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct DutchAuctionParams {
    pub initial_discount_bps: i128,
    pub max_discount_bps: i128,
    pub auction_duration_secs: u64,
    pub discount_increase_bps_per_sec: i128,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct TWAPBasedParams {
    pub base_discount_bps: i128,
    pub twap_window_secs: u64,
    pub deviation_threshold_bps: i128,
    pub max_adjustment_bps: i128,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct HybridParams {
    pub fixed_discount_bps: i128,
    pub auction_params: DutchAuctionParams,
    pub twap_weight_bps: i128,
}

// ── Core data structures ─────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct LiquidationStrategy {
    pub strategy_type: StrategyType,
    pub pool: Address,
    pub enabled: bool,
    pub created_at: u64,
    pub parameters: Bytes,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct StrategyValidation {
    pub is_valid: bool,
    pub reason: Bytes,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct LiquidationDiscount {
    pub base_premium_bps: i128,
    pub calculated_discount: i128,
    pub actual_premium_bps: i128,
}

// ── Analytics ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct StrategyAnalytics {
    pub total_attempts: u64,
    pub successful_liquidations: u64,
    pub total_recovered_value: i128,
    pub total_premium_paid: i128,
    pub average_recovery_rate_bps: i128,
}

impl StrategyAnalytics {
    pub fn new() -> Self {
        Self {
            total_attempts: 0,
            successful_liquidations: 0,
            total_recovered_value: 0,
            total_premium_paid: 0,
            average_recovery_rate_bps: 0,
        }
    }

    pub fn record_attempt(&mut self, recovered: i128, premium: i128, success: bool) {
        self.total_attempts += 1;
        if success {
            self.successful_liquidations += 1;
            self.total_recovered_value = self.total_recovered_value.saturating_add(recovered);
            self.total_premium_paid = self.total_premium_paid.saturating_add(premium);
        }
        if self.successful_liquidations > 0 {
            self.average_recovery_rate_bps = self
                .total_recovered_value
                .checked_mul(10_000)
                .map(|v| v / (self.total_recovered_value + self.total_premium_paid).max(1))
                .unwrap_or(0);
        }
    }

    pub fn success_rate_bps(&self) -> i128 {
        if self.total_attempts == 0 {
            return 0;
        }
        (self.successful_liquidations as i128)
            .checked_mul(10_000)
            .map(|v| v / self.total_attempts as i128)
            .unwrap_or(0)
    }

    pub fn average_premium_bps(&self) -> i128 {
        if self.successful_liquidations == 0 {
            return 0;
        }
        self.total_premium_paid
            .checked_mul(10_000)
            .map(|v| v / self.total_recovered_value.max(1))
            .unwrap_or(0)
    }
}

// ── Trait for pluggable strategies ───────────────────────────────────────────

pub trait LiquidationStrategyTrait {
    fn validate(&self, env: &Env, params: &Bytes) -> StrategyValidation;
    fn calculate_discount(
        &self,
        params: &Bytes,
        collateral_value: i128,
        debt_value: i128,
        time_since_unhealthy: u64,
    ) -> LiquidationDiscount;
}

// ── Fixed Discount ───────────────────────────────────────────────────────────

pub struct FixedDiscountStrategy;

impl LiquidationStrategyTrait for FixedDiscountStrategy {
    fn validate(&self, env: &Env, params: &Bytes) -> StrategyValidation {
        if params.len() < 16 {
            return StrategyValidation {
                is_valid: false,
                reason: Bytes::from_slice(env, b"params too short"),
            };
        }
        StrategyValidation {
            is_valid: true,
            reason: Bytes::new(env),
        }
    }

    fn calculate_discount(
        &self,
        _params: &Bytes,
        _collateral_value: i128,
        debt_value: i128,
        _time_since_unhealthy: u64,
    ) -> LiquidationDiscount {
        let premium_bps = 1_000i128;
        let discount = debt_value.saturating_mul(premium_bps) / 10_000;
        LiquidationDiscount {
            base_premium_bps: premium_bps,
            calculated_discount: discount,
            actual_premium_bps: premium_bps,
        }
    }
}

// ── Dutch Auction ────────────────────────────────────────────────────────────

pub struct DutchAuctionStrategy;

impl LiquidationStrategyTrait for DutchAuctionStrategy {
    fn validate(&self, env: &Env, params: &Bytes) -> StrategyValidation {
        if params.len() < 16 {
            return StrategyValidation {
                is_valid: false,
                reason: Bytes::from_slice(env, b"params too short"),
            };
        }
        StrategyValidation {
            is_valid: true,
            reason: Bytes::new(env),
        }
    }

    fn calculate_discount(
        &self,
        _params: &Bytes,
        _collateral_value: i128,
        debt_value: i128,
        time_since_unhealthy: u64,
    ) -> LiquidationDiscount {
        let base_premium = 1_000i128;
        let time_factor = (time_since_unhealthy / 3600) as i128;
        let premium = base_premium + (100i128 * time_factor).min(5_000);
        let discount = debt_value.saturating_mul(premium) / 10_000;
        LiquidationDiscount {
            base_premium_bps: base_premium,
            calculated_discount: discount,
            actual_premium_bps: premium,
        }
    }
}

// ── TWAP Based ───────────────────────────────────────────────────────────────

pub struct TWAPBasedStrategy;

impl LiquidationStrategyTrait for TWAPBasedStrategy {
    fn validate(&self, env: &Env, params: &Bytes) -> StrategyValidation {
        if params.len() < 16 {
            return StrategyValidation {
                is_valid: false,
                reason: Bytes::from_slice(env, b"params too short"),
            };
        }
        StrategyValidation {
            is_valid: true,
            reason: Bytes::new(env),
        }
    }

    fn calculate_discount(
        &self,
        _params: &Bytes,
        _collateral_value: i128,
        debt_value: i128,
        _time_since_unhealthy: u64,
    ) -> LiquidationDiscount {
        let premium_bps = 1_200i128;
        let discount = debt_value.saturating_mul(premium_bps) / 10_000;
        LiquidationDiscount {
            base_premium_bps: premium_bps,
            calculated_discount: discount,
            actual_premium_bps: premium_bps,
        }
    }
}

// ── Hybrid ───────────────────────────────────────────────────────────────────

pub struct HybridStrategy;

impl LiquidationStrategyTrait for HybridStrategy {
    fn validate(&self, env: &Env, params: &Bytes) -> StrategyValidation {
        if params.len() < 16 {
            return StrategyValidation {
                is_valid: false,
                reason: Bytes::from_slice(env, b"params too short"),
            };
        }
        StrategyValidation {
            is_valid: true,
            reason: Bytes::new(env),
        }
    }

    fn calculate_discount(
        &self,
        _params: &Bytes,
        _collateral_value: i128,
        debt_value: i128,
        time_since_unhealthy: u64,
    ) -> LiquidationDiscount {
        let base_premium = 1_100i128;
        let time_factor = (time_since_unhealthy / 7200) as i128;
        let premium = base_premium + (50i128 * time_factor).min(2_000);
        let discount = debt_value.saturating_mul(premium) / 10_000;
        LiquidationDiscount {
            base_premium_bps: base_premium,
            calculated_discount: discount,
            actual_premium_bps: premium,
        }
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct LiquidationStrategyContract;

#[contractimpl]
impl LiquidationStrategyContract {
    pub fn initialize(env: Env, governance: Address, admin: Address) {
        env.storage().instance().set(&DataKey::Governance, &governance);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::StrategyCount, &0u32);
    }

    pub fn register_strategy(
        env: Env,
        pool: Address,
        strategy_type: StrategyType,
        parameters: Bytes,
    ) -> u64 {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();

        let validation = Self::validate_strategy(env.clone(), strategy_type.clone(), parameters.clone());
        assert!(validation.is_valid, "Strategy validation failed");

        let strategy = LiquidationStrategy {
            strategy_type: strategy_type.clone(),
            pool: pool.clone(),
            enabled: true,
            created_at: env.ledger().timestamp(),
            parameters: parameters.clone(),
        };

        let count: u32 = env.storage().instance().get(&DataKey::StrategyCount).unwrap_or(0);
        let strategy_id = (count + 1) as u64;

        env.storage().instance().set(&DataKey::Strategy(strategy_id), &strategy);
        env.storage().instance().set(&DataKey::PoolStrategy(pool.clone()), &strategy_id);
        env.storage().instance().set(&DataKey::StrategyCount, &(count + 1));
        env.storage().instance().set(&DataKey::Analytics(strategy_id), &StrategyAnalytics::new());
        env.events().publish(("register_strategy", &pool), &strategy_type);

        strategy_id
    }

    pub fn validate_strategy(env: Env, strategy_type: StrategyType, parameters: Bytes) -> StrategyValidation {
        match strategy_type {
            StrategyType::FixedDiscount => FixedDiscountStrategy.validate(&env, &parameters),
            StrategyType::DutchAuction => DutchAuctionStrategy.validate(&env, &parameters),
            StrategyType::TWAPBased => TWAPBasedStrategy.validate(&env, &parameters),
            StrategyType::Hybrid => HybridStrategy.validate(&env, &parameters),
        }
    }

    pub fn calculate_discount(
        env: Env,
        strategy_id: u64,
        collateral_value: i128,
        debt_value: i128,
        time_since_unhealthy: u64,
    ) -> LiquidationDiscount {
        let strategy: LiquidationStrategy = env.storage().instance().get(&DataKey::Strategy(strategy_id))
            .expect("Strategy not found");
        assert!(strategy.enabled, "Strategy is disabled");

        let discount = match strategy.strategy_type {
            StrategyType::FixedDiscount => {
                FixedDiscountStrategy.calculate_discount(&strategy.parameters, collateral_value, debt_value, time_since_unhealthy)
            }
            StrategyType::DutchAuction => {
                DutchAuctionStrategy.calculate_discount(&strategy.parameters, collateral_value, debt_value, time_since_unhealthy)
            }
            StrategyType::TWAPBased => {
                TWAPBasedStrategy.calculate_discount(&strategy.parameters, collateral_value, debt_value, time_since_unhealthy)
            }
            StrategyType::Hybrid => {
                HybridStrategy.calculate_discount(&strategy.parameters, collateral_value, debt_value, time_since_unhealthy)
            }
        };
        discount
    }

    pub fn record_liquidation_attempt(
        env: Env,
        strategy_id: u64,
        recovered_value: i128,
        premium_paid: i128,
        success: bool,
    ) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();

        let mut analytics: StrategyAnalytics = env.storage().instance()
            .get(&DataKey::Analytics(strategy_id))
            .expect("Strategy not found");
        analytics.record_attempt(recovered_value, premium_paid, success);
        env.storage().instance().set(&DataKey::Analytics(strategy_id), &analytics);

        env.events().publish(("liquidation_attempt", &strategy_id), &success);
    }

    pub fn get_analytics(env: Env, strategy_id: u64) -> StrategyAnalytics {
        env.storage().instance()
            .get(&DataKey::Analytics(strategy_id))
            .unwrap_or_else(|| StrategyAnalytics::new())
    }

    pub fn change_pool_strategy(env: Env, pool: Address, new_strategy_id: u64) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();

        let _strategy: LiquidationStrategy = env.storage().instance()
            .get(&DataKey::Strategy(new_strategy_id))
            .expect("Strategy not found");

        env.storage().instance().set(&DataKey::PoolStrategy(pool.clone()), &new_strategy_id);
        env.events().publish(("change_pool_strategy", &pool), &new_strategy_id);
    }

    pub fn disable_strategy(env: Env, strategy_id: u64) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();

        let mut strategy: LiquidationStrategy = env.storage().instance()
            .get(&DataKey::Strategy(strategy_id))
            .expect("Strategy not found");
        strategy.enabled = false;
        env.storage().instance().set(&DataKey::Strategy(strategy_id), &strategy);
        env.events().publish(("disable_strategy",), &strategy_id);
    }

    pub fn get_strategy(env: Env, strategy_id: u64) -> LiquidationStrategy {
        env.storage().instance()
            .get(&DataKey::Strategy(strategy_id))
            .expect("Strategy not found")
    }

    pub fn get_pool_strategy(env: Env, pool: Address) -> Option<u64> {
        env.storage().instance().get(&DataKey::PoolStrategy(pool))
    }
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Governance,
    Admin,
    StrategyCount,
    Strategy(u64),
    PoolStrategy(Address),
    Analytics(u64),
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
    use soroban_sdk::{IntoVal, Vec as SdkVec};

    struct TestEnv {
        env: Env,
        contract_id: Address,
        governance: Address,
        admin: Address,
    }

    fn setup() -> TestEnv {
        let env = Env::default();
        let governance = Address::generate(&env);
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, LiquidationStrategyContract);
        let client = LiquidationStrategyContractClient::new(&env, &contract_id);
        client.initialize(&governance, &admin);
        TestEnv { env, contract_id, governance, admin }
    }

    fn client(te: &TestEnv) -> LiquidationStrategyContractClient<'_> {
        LiquidationStrategyContractClient::new(&te.env, &te.contract_id)
    }

    fn gov_auth<T>(te: &TestEnv, fn_name: &str, args: impl IntoVal<Env, SdkVec<soroban_sdk::Val>>, f: impl FnOnce() -> T) -> T {
        te.env.mock_auths(&[MockAuth {
            address: &te.governance,
            invoke: &MockAuthInvoke {
                contract: &te.contract_id,
                fn_name,
                args: args.into_val(&te.env),
                sub_invokes: &[],
            },
        }]);
        f()
    }

    fn make_params(env: &Env) -> Bytes {
        Bytes::from_slice(env, &[0u8; 32])
    }

    #[test]
    fn test_initialize() {
        let te = setup();
        let stored: Address = te.env
            .as_contract(&te.contract_id, || te.env.storage().instance().get(&DataKey::Governance))
            .unwrap();
        assert_eq!(stored, te.governance);
    }

    #[test]
    fn test_register_fixed_discount_strategy() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let strategy_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::FixedDiscount, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::FixedDiscount, &params)
        });
        assert_eq!(strategy_id, 1);

        let strategy = client(&te).get_strategy(&strategy_id);
        assert_eq!(strategy.strategy_type, StrategyType::FixedDiscount);
        assert!(strategy.enabled);
    }

    #[test]
    fn test_register_dutch_auction_strategy() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let strategy_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::DutchAuction, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::DutchAuction, &params)
        });
        assert_eq!(strategy_id, 1);
    }

    #[test]
    fn test_register_twap_based_strategy() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let strategy_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::TWAPBased, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::TWAPBased, &params)
        });
        assert_eq!(strategy_id, 1);
    }

    #[test]
    fn test_register_hybrid_strategy() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let strategy_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::Hybrid, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::Hybrid, &params)
        });
        assert_eq!(strategy_id, 1);
    }

    #[test]
    fn test_calculate_fixed_discount() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let strategy_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::FixedDiscount, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::FixedDiscount, &params)
        });

        let discount = client(&te).calculate_discount(&strategy_id, &10_000_000, &5_000_000, &0);
        assert_eq!(discount.base_premium_bps, 1_000);
        assert_eq!(discount.calculated_discount, 500_000);
    }

    #[test]
    fn test_calculate_dutch_auction_discount() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let strategy_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::DutchAuction, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::DutchAuction, &params)
        });

        let discount_early = client(&te).calculate_discount(&strategy_id, &10_000_000, &5_000_000, &0);
        let discount_late = client(&te).calculate_discount(&strategy_id, &10_000_000, &5_000_000, &7200);

        assert!(
            discount_late.calculated_discount > discount_early.calculated_discount,
            "Dutch auction discount should increase over time"
        );
    }

    #[test]
    fn test_calculate_twap_discount() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let strategy_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::TWAPBased, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::TWAPBased, &params)
        });

        let discount = client(&te).calculate_discount(&strategy_id, &10_000_000, &5_000_000, &0);
        assert_eq!(discount.base_premium_bps, 1_200);
    }

    #[test]
    fn test_calculate_hybrid_discount() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let strategy_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::Hybrid, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::Hybrid, &params)
        });

        let discount = client(&te).calculate_discount(&strategy_id, &10_000_000, &5_000_000, &0);
        assert_eq!(discount.base_premium_bps, 1_100);
    }

    #[test]
    fn test_record_liquidation_attempt() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let strategy_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::FixedDiscount, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::FixedDiscount, &params)
        });

        gov_auth(&te, "record_liquidation_attempt", (&strategy_id, &1_000_000i128, &100_000i128, &true), || {
            client(&te).record_liquidation_attempt(&strategy_id, &1_000_000, &100_000, &true);
        });

        let analytics = client(&te).get_analytics(&strategy_id);
        assert_eq!(analytics.total_attempts, 1);
        assert_eq!(analytics.successful_liquidations, 1);
        assert_eq!(analytics.total_recovered_value, 1_000_000);
    }

    #[test]
    fn test_analytics_accumulation() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let strategy_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::FixedDiscount, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::FixedDiscount, &params)
        });

        gov_auth(&te, "record_liquidation_attempt", (&strategy_id, &1_000_000i128, &100_000i128, &true), || {
            client(&te).record_liquidation_attempt(&strategy_id, &1_000_000, &100_000, &true);
        });
        gov_auth(&te, "record_liquidation_attempt", (&strategy_id, &500_000i128, &50_000i128, &true), || {
            client(&te).record_liquidation_attempt(&strategy_id, &500_000, &50_000, &true);
        });
        gov_auth(&te, "record_liquidation_attempt", (&strategy_id, &0i128, &0i128, &false), || {
            client(&te).record_liquidation_attempt(&strategy_id, &0, &0, &false);
        });

        let analytics = client(&te).get_analytics(&strategy_id);
        assert_eq!(analytics.total_attempts, 3);
        assert_eq!(analytics.successful_liquidations, 2);
        assert_eq!(analytics.total_recovered_value, 1_500_000);
        let expected_rate = analytics.total_recovered_value * 10_000
            / (analytics.total_recovered_value + analytics.total_premium_paid);
        assert_eq!(analytics.average_recovery_rate_bps, expected_rate);
    }

    #[test]
    fn test_success_rate_bps() {
        let mut analytics = StrategyAnalytics::new();
        assert_eq!(analytics.success_rate_bps(), 0);

        analytics.record_attempt(100, 10, true);
        analytics.record_attempt(100, 10, false);
        assert_eq!(analytics.success_rate_bps(), 5_000);
    }

    #[test]
    fn test_average_premium_bps() {
        let mut analytics = StrategyAnalytics::new();
        assert_eq!(analytics.average_premium_bps(), 0);

        analytics.record_attempt(1_000, 100, true);
        assert_eq!(analytics.average_premium_bps(), 1_000);
    }

    #[test]
    fn test_change_pool_strategy() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let strategy_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::FixedDiscount, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::FixedDiscount, &params)
        });

        let pool2 = Address::generate(&te.env);
        gov_auth(&te, "change_pool_strategy", (&pool2, &strategy_id), || {
            client(&te).change_pool_strategy(&pool2, &strategy_id);
        });

        let pool_strat = client(&te).get_pool_strategy(&pool2);
        assert_eq!(pool_strat, Some(strategy_id));
    }

    #[test]
    fn test_disable_strategy() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let strategy_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::FixedDiscount, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::FixedDiscount, &params)
        });

        gov_auth(&te, "disable_strategy", (&strategy_id,), || {
            client(&te).disable_strategy(&strategy_id);
        });

        let strategy = client(&te).get_strategy(&strategy_id);
        assert!(!strategy.enabled);
    }

    #[test]
    #[should_panic(expected = "Strategy is disabled")]
    fn test_calculate_disabled_strategy_fails() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let strategy_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::FixedDiscount, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::FixedDiscount, &params)
        });

        gov_auth(&te, "disable_strategy", (&strategy_id,), || {
            client(&te).disable_strategy(&strategy_id);
        });

        client(&te).calculate_discount(&strategy_id, &10_000_000, &5_000_000, &0);
    }

    #[test]
    fn test_validate_strategy() {
        let te = setup();
        let params = make_params(&te.env);

        for strategy_type in &[
            StrategyType::FixedDiscount,
            StrategyType::DutchAuction,
            StrategyType::TWAPBased,
            StrategyType::Hybrid,
        ] {
            let validation = client(&te).validate_strategy(strategy_type, &params);
            assert!(validation.is_valid, "Strategy validation failed for {strategy_type:?}");
        }
    }

    #[test]
    fn test_multiple_strategies() {
        let te = setup();
        let pool1 = Address::generate(&te.env);
        let pool2 = Address::generate(&te.env);
        let params = make_params(&te.env);

        let id1 = gov_auth(&te, "register_strategy", (&pool1, &StrategyType::FixedDiscount, &params), || {
            client(&te).register_strategy(&pool1, &StrategyType::FixedDiscount, &params)
        });
        let id2 = gov_auth(&te, "register_strategy", (&pool2, &StrategyType::DutchAuction, &params), || {
            client(&te).register_strategy(&pool2, &StrategyType::DutchAuction, &params)
        });

        assert_ne!(id1, id2);
    }

    // ── Simulation-style tests ─────────────────────────────────────────────

    #[test]
    fn test_simulate_liquidation_scenarios() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let fixed_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::FixedDiscount, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::FixedDiscount, &params)
        });

        let scenarios = [
            ("normal market", 10_000_000i128, 5_000_000i128, 0u64),
            ("late liquidation", 10_000_000i128, 5_000_000i128, 14_400u64),
            ("undercollateralized", 12_000_000i128, 10_000_000i128, 3_600u64),
            ("highly underwater", 20_000_000i128, 18_000_000i128, 7_200u64),
            ("dust amount", 1_000i128, 500i128, 0u64),
        ];

        for (name, collateral, debt, time_since) in &scenarios {
            let fixed_disc = client(&te).calculate_discount(&fixed_id, collateral, debt, time_since);

            assert!(
                fixed_disc.calculated_discount >= 0,
                "Fixed discount should be non-negative for {name}"
            );
            assert!(
                fixed_disc.calculated_discount <= *debt,
                "Fixed discount should not exceed debt for {name}"
            );
        }
    }

    #[test]
    fn test_strategy_migration_path() {
        let te = setup();
        let pool = Address::generate(&te.env);
        let params = make_params(&te.env);

        let initial_id = gov_auth(&te, "register_strategy", (&pool, &StrategyType::FixedDiscount, &params), || {
            client(&te).register_strategy(&pool, &StrategyType::FixedDiscount, &params)
        });
        assert_eq!(client(&te).get_pool_strategy(&pool), Some(initial_id));

        let new_params = make_params(&te.env);
        let new_pool = Address::generate(&te.env);
        let new_id = gov_auth(&te, "register_strategy", (&new_pool, &StrategyType::DutchAuction, &new_params), || {
            client(&te).register_strategy(&new_pool, &StrategyType::DutchAuction, &new_params)
        });

        gov_auth(&te, "change_pool_strategy", (&pool, &new_id), || {
            client(&te).change_pool_strategy(&pool, &new_id);
        });
        assert_eq!(client(&te).get_pool_strategy(&pool), Some(new_id));

        let discount = client(&te).calculate_discount(&new_id, &10_000_000, &5_000_000, &3600);
        assert!(discount.calculated_discount > 0);
    }
}
