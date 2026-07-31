use soroban_sdk::{Address, Env};

use crate::{
    validate_price, OracleClientError, OracleClientConfig, OraclePrice, PriceCache, PriceOracle,
};

pub fn resolve_with_fallback<P, S, C>(
    env: &Env,
    asset: &Address,
    primary: &P,
    secondary: Option<&S>,
    cache: &C,
    config: &OracleClientConfig,
) -> Result<OraclePrice, OracleClientError>
where
    P: PriceOracle,
    S: PriceOracle,
    C: PriceCache,
{
    if let Ok(price) = primary.price(env, asset) {
        validate_price(env, &price, config.max_staleness_seconds)?;
        cache.cache_price(env, asset, &price);
        return Ok(price);
    }

    if let Some(secondary) = secondary {
        if let Ok(price) = secondary.price(env, asset) {
            validate_price(env, &price, config.max_staleness_seconds)?;
            cache.cache_price(env, asset, &price);
            return Ok(price);
        }
    }

    if let Some(cached) = cache.cached_price(env, asset) {
        let now = env.ledger().timestamp();
        if now >= cached.cached_at
            && now - cached.cached_at <= config.cache_ttl_seconds
            && validate_price(env, &cached.price, config.max_staleness_seconds).is_ok()
        {
            return Ok(cached.price);
        }
    }

    Err(OracleClientError::NoPriceAvailable)
}
