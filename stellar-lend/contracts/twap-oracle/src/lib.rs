#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Symbol};

const BPS_DENOM: i128 = 10_000;
const DEFAULT_WINDOW_SECS: u64 = 1800;
const DEFAULT_MAX_DEVIATION_BPS: i128 = 500;
const DEFAULT_MIN_SAMPLES: u32 = 3;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TwapOracleError {
    Unauthorized = 1,
    InvalidConfig = 2,
    InsufficientSamples = 3,
    PriceManipulationDetected = 4,
    Overflow = 5,
    AlreadyInitialized = 6,
    NotInitialized = 7,
    InvalidPrice = 8,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TwapConfig {
    pub admin: Address,
    pub window_secs: u64,
    pub max_deviation_bps: i128,
    pub min_samples: u32,
    pub initialized: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PriceObservation {
    pub price: i128,
    pub timestamp: u64,
    pub block: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TwapAccumulator {
    pub price_sum: i128,
    pub total_time: u64,
    pub twap: i128,
    pub sample_count: u32,
    pub last_update: u64,
    pub last_price: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TwapResult {
    pub twap: i128,
    pub spot_price: i128,
    pub deviation_bps: i128,
    pub manipulation_detected: bool,
    pub used_fallback: bool,
    pub sample_count: u32,
}

#[contracttype]
#[derive(Clone)]
pub enum TwapKey {
    Config,
    Accumulator(Address),
    Observation(Address, u32),
    ObservationCount(Address),
}

#[contract]
pub struct TwapOracle;

#[contractimpl]
impl TwapOracle {
    pub fn initialize(env: Env, admin: Address) -> Result<(), TwapOracleError> {
        if env.storage().instance().has(&TwapKey::Config) {
            return Err(TwapOracleError::AlreadyInitialized);
        }

        let config = TwapConfig {
            admin,
            window_secs: DEFAULT_WINDOW_SECS,
            max_deviation_bps: DEFAULT_MAX_DEVIATION_BPS,
            min_samples: DEFAULT_MIN_SAMPLES,
            initialized: true,
        };

        env.storage().instance().set(&TwapKey::Config, &config);
        Ok(())
    }

    pub fn set_config(
        env: Env,
        admin: Address,
        window_secs: u64,
        max_deviation_bps: i128,
        min_samples: u32,
    ) -> Result<(), TwapOracleError> {
        admin.require_auth();

        let config: TwapConfig = env
            .storage()
            .instance()
            .get(&TwapKey::Config)
            .ok_or(TwapOracleError::NotInitialized)?;

        if admin != config.admin {
            return Err(TwapOracleError::Unauthorized);
        }

        if window_secs == 0 || max_deviation_bps <= 0 || max_deviation_bps > BPS_DENOM || min_samples == 0 {
            return Err(TwapOracleError::InvalidConfig);
        }

        let updated = TwapConfig {
            admin: config.admin,
            window_secs,
            max_deviation_bps,
            min_samples,
            initialized: true,
        };

        env.storage().instance().set(&TwapKey::Config, &updated);
        Ok(())
    }

    pub fn get_config(env: Env) -> Option<TwapConfig> {
        env.storage().instance().get(&TwapKey::Config)
    }

    pub fn record_price(env: Env, asset: Address, price: i128) -> Result<(), TwapOracleError> {
        let config: TwapConfig = env
            .storage()
            .instance()
            .get(&TwapKey::Config)
            .ok_or(TwapOracleError::NotInitialized)?;

        config.admin.require_auth();

        if price <= 0 {
            return Err(TwapOracleError::InvalidPrice);
        }

        Self::update_accumulator(&env, &asset, price, &config);

        Ok(())
    }

    pub fn get_twap(env: Env, asset: Address) -> TwapResult {
        let config: TwapConfig = match env.storage().instance().get(&TwapKey::Config) {
            Some(c) => c,
            None => {
                return TwapResult {
                    twap: 0,
                    spot_price: 0,
                    deviation_bps: 0,
                    manipulation_detected: false,
                    used_fallback: true,
                    sample_count: 0,
                }
            }
        };

        let acc: TwapAccumulator = match env
            .storage()
            .persistent()
            .get(&TwapKey::Accumulator(asset.clone()))
        {
            Some(a) => a,
            None => {
                return TwapResult {
                    twap: 0,
                    spot_price: 0,
                    deviation_bps: 0,
                    manipulation_detected: false,
                    used_fallback: true,
                    sample_count: 0,
                }
            }
        };

        let spot_price = acc.last_price;
        let sample_count = acc.sample_count;
        let twap = acc.twap;

        let insufficient_samples = sample_count < config.min_samples;

        let (deviation_bps, manipulation_detected) = if insufficient_samples || twap <= 0 || spot_price <= 0 {
            (0, false)
        } else {
            let diff = if spot_price > twap {
                spot_price - twap
            } else {
                twap - spot_price
            };
            let deviation = diff
                .checked_mul(BPS_DENOM)
                .unwrap_or(0)
                .checked_div(twap)
                .unwrap_or(0);
            (deviation as i128, deviation > config.max_deviation_bps)
        };

        TwapResult {
            twap,
            spot_price,
            deviation_bps,
            manipulation_detected,
            used_fallback: insufficient_samples || twap <= 0,
            sample_count,
        }
    }

    pub fn get_liquidation_price(env: Env, asset: Address) -> TwapResult {
        let result = Self::get_twap(env, asset);

        if result.used_fallback || result.manipulation_detected {
            TwapResult {
                twap: result.spot_price,
                ..result
            }
        } else {
            result
        }
    }

    pub fn check_deviation(
        env: Env,
        asset: Address,
        spot_price: i128,
    ) -> Result<TwapResult, TwapOracleError> {
        let config: TwapConfig = env
            .storage()
            .instance()
            .get(&TwapKey::Config)
            .ok_or(TwapOracleError::NotInitialized)?;

        let acc: TwapAccumulator = env
            .storage()
            .persistent()
            .get(&TwapKey::Accumulator(asset.clone()))
            .ok_or(TwapOracleError::InsufficientSamples)?;

        if acc.sample_count < config.min_samples {
            return Err(TwapOracleError::InsufficientSamples);
        }

        if acc.twap <= 0 || spot_price <= 0 {
            return Err(TwapOracleError::InvalidPrice);
        }

        let diff = if spot_price > acc.twap {
            spot_price - acc.twap
        } else {
            acc.twap - spot_price
        };
        let deviation_bps = diff
            .checked_mul(BPS_DENOM)
            .ok_or(TwapOracleError::Overflow)?
            .checked_div(acc.twap)
            .ok_or(TwapOracleError::Overflow)?;

        let manipulation_detected = deviation_bps > config.max_deviation_bps;

        if manipulation_detected {
            return Err(TwapOracleError::PriceManipulationDetected);
        }

        Ok(TwapResult {
            twap: acc.twap,
            spot_price,
            deviation_bps,
            manipulation_detected: false,
            used_fallback: false,
            sample_count: acc.sample_count,
        })
    }

    fn update_accumulator(env: &Env, asset: &Address, price: i128, config: &TwapConfig) {
        let key = TwapKey::Accumulator(asset.clone());
        let now = env.ledger().timestamp();

        let mut acc: TwapAccumulator = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(TwapAccumulator {
                price_sum: 0,
                total_time: 0,
                twap: price,
                sample_count: 0,
                last_update: 0,
                last_price: price,
            });

        if acc.last_update == 0 {
            acc.price_sum = price;
            acc.total_time = 1;
            acc.twap = price;
            acc.sample_count = 1;
            acc.last_update = now;
            acc.last_price = price;
        } else {
            let elapsed = now.saturating_sub(acc.last_update);

            if elapsed >= config.window_secs {
                acc.price_sum = price;
                acc.total_time = 1;
                acc.twap = price;
                acc.sample_count = 1;
            } else {
                acc.price_sum = acc.price_sum.saturating_add(price);
                acc.sample_count = acc.sample_count.saturating_add(1);
                acc.total_time = acc.total_time.saturating_add(elapsed);
                if acc.sample_count > 0 {
                    acc.twap = acc.price_sum / acc.sample_count as i128;
                }
            }
            acc.last_update = now;
            acc.last_price = price;
        }

        env.storage().persistent().set(&key, &acc);

        let obs_count_key = TwapKey::ObservationCount(asset.clone());
        let count: u32 = env.storage().persistent().get(&obs_count_key).unwrap_or(0);
        let obs = PriceObservation {
            price,
            timestamp: now,
            block: env.ledger().sequence(),
        };
        env.storage()
            .persistent()
            .set(&TwapKey::Observation(asset.clone(), count), &obs);
        env.storage()
            .persistent()
            .set(&obs_count_key, &(count.wrapping_add(1)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup_env() -> (Env, Address) {
        let env = Env::default();
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, TwapOracle);
        let client = TwapOracleClient::new(&env, &contract_id);
        client.initialize(&admin);
        (env, admin)
    }

    #[test]
    fn test_initialize() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, TwapOracle);
        let client = TwapOracleClient::new(&env, &contract_id);
        client.initialize(&admin);

        let config = client.get_config().unwrap();
        assert_eq!(config.admin, admin);
        assert_eq!(config.window_secs, DEFAULT_WINDOW_SECS);
        assert_eq!(config.max_deviation_bps, DEFAULT_MAX_DEVIATION_BPS);
        assert_eq!(config.min_samples, DEFAULT_MIN_SAMPLES);
    }

    #[test]
    fn test_double_initialize_fails() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, TwapOracle);
        let client = TwapOracleClient::new(&env, &contract_id);
        client.initialize(&admin);

        let result = client.try_initialize(&admin);
        assert!(result.is_err());
    }

    #[test]
    fn test_record_and_get_twap() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let asset = Address::generate(&env);
        let contract_id = env.register_contract(None, TwapOracle);
        let client = TwapOracleClient::new(&env, &contract_id);
        client.initialize(&admin);

        client.record_price(&asset, &1000i128);
        client.record_price(&asset, &1100i128);
        client.record_price(&asset, &1050i128);

        let result = client.get_twap(&asset);
        assert!(result.twap > 0);
        assert!(!result.used_fallback);
        assert_eq!(result.sample_count, 3);
    }

    #[test]
    fn test_insufficient_samples_fallback() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let asset = Address::generate(&env);
        let contract_id = env.register_contract(None, TwapOracle);
        let client = TwapOracleClient::new(&env, &contract_id);
        client.initialize(&admin);

        client.record_price(&asset, &1000i128);

        let result = client.get_twap(&asset);
        assert!(result.used_fallback);
        assert_eq!(result.sample_count, 1);
    }

    #[test]
    fn test_deviation_check_passes() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let asset = Address::generate(&env);
        let contract_id = env.register_contract(None, TwapOracle);
        let client = TwapOracleClient::new(&env, &contract_id);
        client.initialize(&admin);

        client.record_price(&asset, &1000i128);
        client.record_price(&asset, &1000i128);
        client.record_price(&asset, &1000i128);

        let result = client.check_deviation(&asset, &1010i128);
        assert!(result.is_ok());
        let ok = result.unwrap();
        assert!(!ok.manipulation_detected);
    }

    #[test]
    fn test_deviation_check_fails() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let asset = Address::generate(&env);
        let contract_id = env.register_contract(None, TwapOracle);
        let client = TwapOracleClient::new(&env, &contract_id);
        client.initialize(&admin);

        client.record_price(&asset, &1000i128);
        client.record_price(&asset, &1000i128);
        client.record_price(&asset, &1000i128);

        let result = client.check_deviation(&asset, &2000i128);
        assert!(result.is_err());
    }

    #[test]
    fn test_liquidation_price_uses_twap() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let asset = Address::generate(&env);
        let contract_id = env.register_contract(None, TwapOracle);
        let client = TwapOracleClient::new(&env, &contract_id);
        client.initialize(&admin);

        client.record_price(&asset, &1000i128);
        client.record_price(&asset, &1000i128);
        client.record_price(&asset, &1000i128);

        let result = client.get_liquidation_price(&asset);
        assert_eq!(result.twap, 1000);
        assert_eq!(result.spot_price, 1000);
        assert!(!result.manipulation_detected);
    }

    #[test]
    fn test_set_config() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, TwapOracle);
        let client = TwapOracleClient::new(&env, &contract_id);
        client.initialize(&admin);

        client.set_config(&admin, &3600u64, &1000i128, &5u32);

        let config = client.get_config().unwrap();
        assert_eq!(config.window_secs, 3600);
        assert_eq!(config.max_deviation_bps, 1000);
        assert_eq!(config.min_samples, 5);
    }

    #[test]
    fn test_get_twap_no_data() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let asset = Address::generate(&env);
        let contract_id = env.register_contract(None, TwapOracle);
        let client = TwapOracleClient::new(&env, &contract_id);
        client.initialize(&admin);

        let result = client.get_twap(&asset);
        assert!(result.used_fallback);
        assert_eq!(result.sample_count, 0);
    }

    #[test]
    fn test_window_reset() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let asset = Address::generate(&env);
        let contract_id = env.register_contract(None, TwapOracle);
        let client = TwapOracleClient::new(&env, &contract_id);
        client.initialize(&admin);

        env.ledger().set_timestamp(1000);
        client.record_price(&asset, &1000i128);
        client.record_price(&asset, &1000i128);
        client.record_price(&asset, &1000i128);

        env.ledger().set_timestamp(1000 + DEFAULT_WINDOW_SECS + 1);
        client.record_price(&asset, &2000i128);

        let result = client.get_twap(&asset);
        assert_eq!(result.twap, 2000);
        assert_eq!(result.sample_count, 1);
    }
}
