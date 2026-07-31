use soroban_sdk::{contracttype, Address};

/// Storage keys for protocol state
/// Uses enum dispatch for type safety and clarity
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageKey {
    // Protocol configuration
    ProtocolConfig,
    Admin,
    Oracle,
    PauseState,

    // User positions - (user_address, asset_address)
    UserPosition(Address, Address),
    UserCollateral(Address),
    UserDebt(Address),

    // Asset configurations
    AssetConfig(Address),
    AssetTotalSupply(Address),
    AssetTotalDebt(Address),

    // Interest rate management
    InterestRateConfig(Address),
    AccruedInterest(Address, Address),

    // Risk management
    RiskParams,
    LiquidationParams(Address),
    HealthFactor(Address),

    // Flash loans
    FlashLoanConfig,
    FlashLoanFees(Address),

    // Balances and accounting
    Balance(Address, Address),
    Reserve(Address),

    // Event counters and tracking
    EventCounter,
    LastActivityTimestamp(Address),
}

/// Versioned storage keys for forward compatibility
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VersionedKey {
    V1(StorageKey),
}

/// Migration tracking for storage upgrades
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationMarker {
    pub version: u32,
    pub timestamp: u64,
    pub completed: bool,
}
