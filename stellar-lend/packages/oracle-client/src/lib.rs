#![no_std]

use soroban_sdk::{contracttype, Address, Env, Vec};

pub mod fallback;

pub const PRICE_ORACLE_INTERFACE_VERSION: u32 = 1;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OraclePrice {
    pub price: i128,
    pub updated_at: u64,
    pub source: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleClientConfig {
    pub max_staleness_seconds: u64,
    pub cache_ttl_seconds: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedOraclePrice {
    pub price: OraclePrice,
    pub cached_at: u64,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum OracleClientError {
    UnsupportedVersion = 1,
    InvalidPrice = 2,
    StalePrice = 3,
    PrimaryUnavailable = 4,
    SecondaryUnavailable = 5,
    CacheMiss = 6,
    CacheStale = 7,
    NoPriceAvailable = 8,
}

pub trait PriceOracle {
    fn interface_version(&self) -> u32;
    fn price(&self, env: &Env, asset: &Address) -> Result<OraclePrice, OracleClientError>;
}

pub fn validate_version(version: u32) -> Result<(), OracleClientError> {
    if version == PRICE_ORACLE_INTERFACE_VERSION {
        Ok(())
    } else {
        Err(OracleClientError::UnsupportedVersion)
    }
}

pub fn validate_price(
    env: &Env,
    price: &OraclePrice,
    max_staleness_seconds: u64,
) -> Result<(), OracleClientError> {
    if price.price <= 0 {
        return Err(OracleClientError::InvalidPrice);
    }

    let now = env.ledger().timestamp();
    if price.updated_at > now || now.saturating_sub(price.updated_at) > max_staleness_seconds {
        return Err(OracleClientError::StalePrice);
    }

    Ok(())
}

pub trait PriceCache {
    fn cached_price(&self, env: &Env, asset: &Address) -> Option<CachedOraclePrice>;
    fn cache_price(&self, env: &Env, asset: &Address, price: &OraclePrice);
}

pub struct FallbackOracleClient<P, S, C> {
    pub primary: P,
    pub secondary: Option<S>,
    pub cache: C,
    pub config: OracleClientConfig,
}

impl<P, S, C> FallbackOracleClient<P, S, C>
where
    P: PriceOracle,
    S: PriceOracle,
    C: PriceCache,
{
    pub fn get_price(&self, env: &Env, asset: &Address) -> Result<OraclePrice, OracleClientError> {
        validate_version(self.primary.interface_version())?;
        if let Ok(price) = self.primary.price(env, asset) {
            validate_price(env, &price, self.config.max_staleness_seconds)?;
            self.cache.cache_price(env, asset, &price);
            return Ok(price);
        }

        if let Some(secondary) = &self.secondary {
            validate_version(secondary.interface_version())?;
            if let Ok(price) = secondary.price(env, asset) {
                validate_price(env, &price, self.config.max_staleness_seconds)?;
                self.cache.cache_price(env, asset, &price);
                return Ok(price);
            }
        }

        if let Some(cached) = self.cache.cached_price(env, asset) {
            let now = env.ledger().timestamp();
            if now >= cached.cached_at
                && now - cached.cached_at <= self.config.cache_ttl_seconds
                && validate_price(env, &cached.price, self.config.max_staleness_seconds).is_ok()
            {
                return Ok(cached.price);
            }
        }

        Err(OracleClientError::NoPriceAvailable)
    }
}

pub fn first_valid_price(
    env: &Env,
    prices: &Vec<OraclePrice>,
    max_staleness_seconds: u64,
) -> Result<OraclePrice, OracleClientError> {
    for i in 0..prices.len() {
        let price = prices.get(i).ok_or(OracleClientError::NoPriceAvailable)?;
        if validate_price(env, &price, max_staleness_seconds).is_ok() {
            return Ok(price);
        }
    }

    Err(OracleClientError::NoPriceAvailable)
}
