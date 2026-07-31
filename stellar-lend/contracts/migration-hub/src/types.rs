use soroban_sdk::{contracterror, contracttype, Address, String, Val, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MigrationError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    InvalidProtocol = 4,
    MigrationFailed = 5,
    RateLimitExceeded = 6,
    BridgeError = 7,
    DeadlineExceeded = 8,
    InsufficientFunds = 9,
    InvalidPercentage = 10,
    RollbackFailed = 11,
    DestinationPoolInactive = 12,
    InterestCalculationFailed = 13,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolType {
    StellarOther,
    CrossChainBridge,
    AaveMock,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MigrationStatus {
    Pending,
    Completed,
    Failed,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationRecord {
    pub user: Address,
    pub protocol: ProtocolType,
    pub asset: Address,
    pub amount: i128,
    pub status: MigrationStatus,
    pub timestamp: u64,
    pub source_pool: Address,
    pub destination_pool: Address,
    pub interest_at_migration: i128,
    pub is_partial: bool,
    pub source_position_id: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationPreview {
    pub estimated_gas: u64,
    pub estimated_slippage_bps: u32,
    pub interest_impact: i128,
    pub expected_output: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialMigrationConfig {
    pub percentage: u32, // 0-10000 (0-100% in basis points)
    pub min_amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyRollback {
    pub migration_id: u64,
    pub reason: String,
    pub rollback_timestamp: u64,
    pub success: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Config,
    Migration(u64),
    UserMigrations(Address),
    NextMigrationId,
    Analytics,
    Admin,
    Rollback(u64),
    PoolStatus(Address),
    InterestSnapshot(u64),
    BulkMigrationConfig,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationConfig {
    pub lending_contract: Address,
    pub bridge_contract: Address,
    pub rate_limit_per_ledger: u32,
    pub migration_deadline: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationAnalytics {
    pub total_migrated_value: i128,
    pub total_users: u32,
    pub successful_migrations: u32,
    pub failed_migrations: u32,
}
