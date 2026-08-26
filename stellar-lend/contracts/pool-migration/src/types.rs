use soroban_sdk::{contracterror, contracttype, Address};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MigrationError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    SourcePoolNotFound = 4,
    DestinationPoolNotFound = 5,
    SourcePoolFrozen = 6,
    DestinationPoolFrozen = 7,
    InsufficientBalance = 8,
    InsufficientDestinationLiquidity = 9,
    HealthFactorViolation = 10,
    SlippageExceeded = 11,
    MigrationTooSmall = 12,
    MigrationTooLarge = 13,
    CooldownNotElapsed = 14,
    DeadlineExceeded = 15,
    SamePool = 16,
    InvalidPercentage = 17,
    BatchSizeExceeded = 18,
    MigrationNotFound = 19,
    AlreadyCompleted = 20,
    SafetyCheckFailed = 21,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MigrationStatus {
    Pending,
    Completed,
    Failed,
    RolledBack,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolMigrationConfig {
    pub admin: Address,
    pub lending_contract: Address,
    pub min_migration_amount: i128,
    pub max_migration_amount: i128,
    pub cooldown_secs: u64,
    pub max_slippage_bps: u32,
    pub deadline: u64,
    pub max_batch_size: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolMigrationRecord {
    pub id: u64,
    pub user: Address,
    pub source_pool: Address,
    pub destination_pool: Address,
    pub asset: Address,
    pub amount: i128,
    pub shares_burned: i128,
    pub shares_minted: i128,
    pub status: MigrationStatus,
    pub timestamp: u64,
    pub gas_used: u64,
    pub slippage_bps: u32,
    pub interest_accrued: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationPreview {
    pub source_shares_to_burn: i128,
    pub destination_shares_to_mint: i128,
    pub estimated_output: i128,
    pub estimated_slippage_bps: u32,
    pub interest_impact: i128,
    pub destination_liquidity: i128,
    pub safety_passed: bool,
    pub warnings: soroban_sdk::Vec<soroban_sdk::String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationAnalytics {
    pub total_migrations: u64,
    pub successful_migrations: u64,
    pub failed_migrations: u64,
    pub total_volume_migrated: i128,
    pub total_gas_consumed: u64,
    pub average_slippage_bps: u32,
    pub unique_users: u32,
    pub largest_migration: i128,
    pub most_active_source_pool: Option<Address>,
    pub most_active_destination_pool: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolMigrationStats {
    pub pool: Address,
    pub total_outflow: i128,
    pub total_inflow: i128,
    pub migration_count: u64,
    pub unique_migrators: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafetyCheckResult {
    pub passed: bool,
    pub source_pool_active: bool,
    pub destination_pool_active: bool,
    pub destination_has_liquidity: bool,
    pub health_factor_ok: bool,
    pub within_limits: bool,
    pub cooldown_elapsed: bool,
    pub failure_code: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Config,
    Admin,
    Migration(u64),
    NextMigrationId,
    Analytics,
    UserLastMigration(Address),
    UserMigrationCount(Address),
    PoolStats(Address),
    Paused,
}
