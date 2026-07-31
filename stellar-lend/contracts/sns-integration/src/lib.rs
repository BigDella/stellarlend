#![no_std]

use soroban_sdk::{contract, contractimpl, log, symbol_short, Address, Env, Map, String, Symbol};

mod types;

#[cfg(test)]
mod test;

use types::{SNSAnalytics, SNSCache, SNSConfig, SNSError, SNSRecord};

const KEY_NAMES: Symbol = symbol_short!("names");
const KEY_CACHE: Symbol = symbol_short!("cache");
const KEY_CONFIG: Symbol = symbol_short!("config");
const KEY_ANALYTICS: Symbol = symbol_short!("analytics");

#[contract]
pub struct SNSIntegration;

#[contractimpl]
impl SNSIntegration {
    pub fn initialize(
        env: Env,
        admin: Address,
        cache_ttl_seconds: u64,
        name_expiry_days: u64,
    ) -> Result<(), SNSError> {
        if env.storage().instance().has(&KEY_CONFIG) {
            return Err(SNSError::AlreadyInitialized);
        }
        admin.require_auth();

        let config = SNSConfig {
            admin,
            cache_ttl_seconds,
            name_expiry_days,
        };
        env.storage().instance().set(&KEY_CONFIG, &config);

        let analytics = SNSAnalytics {
            total_names_registered: 0,
            total_resolutions: 0,
            cache_hit_rate: 0,
            resolution_latency_ms: 0,
        };
        env.storage().instance().set(&KEY_ANALYTICS, &analytics);

        Ok(())
    }

    pub fn register_name(
        env: Env,
        name: String,
        address: Address,
    ) -> Result<(), SNSError> {
        address.require_auth();
        Self::require_initialized(&env)?;

        if name.len() == 0 {
            return Err(SNSError::InvalidName);
        }

        let config: SNSConfig = env
            .storage()
            .instance()
            .get(&KEY_CONFIG)
            .ok_or(SNSError::NotInitialized)?;

        let expires_at = env.ledger().timestamp() + (config.name_expiry_days * 86400);

        let record = SNSRecord {
            name: name.clone(),
            address: address.clone(),
            registered_at: env.ledger().timestamp(),
            expires_at,
            owner: address.clone(),
        };

        let mut names: Map<String, SNSRecord> = env
            .storage()
            .persistent()
            .get(&KEY_NAMES)
            .unwrap_or_else(|| Map::new(&env));
        names.set(name.clone(), record);
        env.storage().persistent().set(&KEY_NAMES, &names);

        // Invalidate cache
        let mut cache: Map<String, SNSCache> = env
            .storage()
            .persistent()
            .get(&KEY_CACHE)
            .unwrap_or_else(|| Map::new(&env));
        cache.remove(name.clone());
        env.storage().persistent().set(&KEY_CACHE, &cache);

        // Update analytics
        let mut analytics: SNSAnalytics = env
            .storage()
            .instance()
            .get(&KEY_ANALYTICS)
            .unwrap();
        analytics.total_names_registered += 1;
        env.storage().instance().set(&KEY_ANALYTICS, &analytics);

        log!(&env, "SNS name registered: {} -> {}", name, address);

        Ok(())
    }

    pub fn resolve_name(env: Env, name: String) -> Result<Address, SNSError> {
        Self::require_initialized(&env)?;

        // Check cache first
        if let Some(cached) = Self::get_from_cache(&env, &name) {
            let mut analytics: SNSAnalytics = env
                .storage()
                .instance()
                .get(&KEY_ANALYTICS)
                .unwrap();
            analytics.total_resolutions += 1;
            if analytics.cache_hit_rate < 100 {
                analytics.cache_hit_rate += 1;
            }
            env.storage().instance().set(&KEY_ANALYTICS, &analytics);
            return Ok(cached.address);
        }

        // Fetch from names map
        let names: Map<String, SNSRecord> = env
            .storage()
            .persistent()
            .get(&KEY_NAMES)
            .ok_or(SNSError::NameNotFound)?;

        let record: SNSRecord = names.get(name.clone()).ok_or(SNSError::NameNotFound)?;

        if record.expires_at < env.ledger().timestamp() {
            return Err(SNSError::NameExpired);
        }

        // Cache the resolution
        let config: SNSConfig = env
            .storage()
            .instance()
            .get(&KEY_CONFIG)
            .ok_or(SNSError::NotInitialized)?;

        let cache_entry = SNSCache {
            name: name.clone(),
            address: record.address.clone(),
            cached_at: env.ledger().timestamp(),
            ttl: config.cache_ttl_seconds,
        };

        let mut cache: Map<String, SNSCache> = env
            .storage()
            .persistent()
            .get(&KEY_CACHE)
            .unwrap_or_else(|| Map::new(&env));
        cache.set(name.clone(), cache_entry);
        env.storage().persistent().set(&KEY_CACHE, &cache);

        // Update analytics
        let mut analytics: SNSAnalytics = env
            .storage()
            .instance()
            .get(&KEY_ANALYTICS)
            .unwrap();
        analytics.total_resolutions += 1;
        env.storage().instance().set(&KEY_ANALYTICS, &analytics);

        log!(&env, "SNS name resolved: {} -> {}", name, record.address);

        Ok(record.address)
    }

    pub fn resolve_names_batch(
        env: Env,
        names: soroban_sdk::Vec<String>,
    ) -> soroban_sdk::Vec<Result<Address, SNSError>> {
        let mut results = soroban_sdk::Vec::new();
        for name in names.iter() {
            let result = Self::resolve_name(env.clone(), name);
            results.push_back(result);
        }
        results
    }

    pub fn validate_name(env: Env, name: String) -> Result<Address, SNSError> {
        Self::resolve_name(env, name)
    }

    pub fn get_analytics(env: Env) -> Result<SNSAnalytics, SNSError> {
        Self::require_initialized(&env)?;
        env.storage()
            .instance()
            .get(&KEY_ANALYTICS)
            .ok_or(SNSError::NotInitialized)
    }

    pub fn is_name_expired(env: Env, name: String) -> Result<bool, SNSError> {
        let names: Map<String, SNSRecord> = env
            .storage()
            .persistent()
            .get(&KEY_NAMES)
            .ok_or(SNSError::NameNotFound)?;

        let record: SNSRecord = names.get(name).ok_or(SNSError::NameNotFound)?;
        Ok(record.expires_at < env.ledger().timestamp())
    }

    pub fn renew_name(env: Env, name: String) -> Result<(), SNSError> {
        let mut names: Map<String, SNSRecord> = env
            .storage()
            .persistent()
            .get(&KEY_NAMES)
            .ok_or(SNSError::NameNotFound)?;

        let mut record: SNSRecord = names.get(name.clone()).ok_or(SNSError::NameNotFound)?;

        record.owner.require_auth();

        let config: SNSConfig = env
            .storage()
            .instance()
            .get(&KEY_CONFIG)
            .ok_or(SNSError::NotInitialized)?;

        record.expires_at = env.ledger().timestamp() + (config.name_expiry_days * 86400);
        names.set(name.clone(), record);
        env.storage().persistent().set(&KEY_NAMES, &names);

        log!(&env, "SNS name renewed: {}", name);

        Ok(())
    }

    pub fn get_record(env: Env, name: String) -> Result<SNSRecord, SNSError> {
        let names: Map<String, SNSRecord> = env
            .storage()
            .persistent()
            .get(&KEY_NAMES)
            .ok_or(SNSError::NameNotFound)?;

        names.get(name).ok_or(SNSError::NameNotFound)
    }

    fn get_from_cache(env: &Env, name: &String) -> Option<SNSCache> {
        let cache: Map<String, SNSCache> = env
            .storage()
            .persistent()
            .get(&KEY_CACHE)?;

        let entry = cache.get(name.clone())?;
        if env.ledger().timestamp() < entry.cached_at + entry.ttl {
            return Some(entry);
        }

        // Expired: remove
        let mut cache_mut = cache;
        cache_mut.remove(name.clone());
        env.storage().persistent().set(&KEY_CACHE, &cache_mut);
        None
    }

    fn require_initialized(env: &Env) -> Result<(), SNSError> {
        if !env.storage().instance().has(&KEY_CONFIG) {
            return Err(SNSError::NotInitialized);
        }
        Ok(())
    }
}
