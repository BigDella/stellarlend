use soroban_sdk::{contracterror, contracttype, Address, String};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SNSError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    NameNotFound = 4,
    NameExpired = 5,
    InvalidName = 6,
    ResolutionFailed = 7,
    CacheMiss = 8,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SNSRecord {
    pub name: String,
    pub address: Address,
    pub registered_at: u64,
    pub expires_at: u64,
    pub owner: Address,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SNSCache {
    pub name: String,
    pub address: Address,
    pub cached_at: u64,
    pub ttl: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SNSConfig {
    pub admin: Address,
    pub cache_ttl_seconds: u64,
    pub name_expiry_days: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SNSAnalytics {
    pub total_names_registered: u32,
    pub total_resolutions: u64,
    pub cache_hit_rate: u32,
    pub resolution_latency_ms: u64,
}
