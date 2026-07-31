use soroban_sdk::{contracttype, Address};

/// Unified user position across all assets in the protocol
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserPosition {
    pub user: Address,
    pub collateral_value: i128,
    pub debt_value: i128,
    pub health_factor: i128,
    pub last_updated: u64,
}

/// Core position structure for tracking collateral and debt
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    pub collateral_amount: i128,
    pub debt_amount: i128,
    pub last_updated: u64,
}

/// Per-asset configuration stored in the protocol
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetConfig {
    pub collateral_factor: i128,
    pub liquidation_threshold: i128,
    pub reserve_factor: i128,
    pub max_supply: i128,
    pub max_borrow: i128,
}

/// Global protocol configuration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolConfig {
    pub admin: Address,
    pub oracle: Option<Address>,
    pub debt_ceiling: i128,
    pub min_borrow_amount: i128,
    pub liquidation_threshold_bps: i128,
}

/// Protocol pause state
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PauseState {
    Active,
    Paused,
}

/// Supported operation types
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationType {
    Deposit,
    Withdraw,
    Borrow,
    Repay,
    Liquidation,
    FlashLoan,
}

/// Interest rate configuration for dynamic rate model
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterestRateConfig {
    pub base_rate_bps: i128,
    pub kink_utilization_bps: i128,
    pub slope_bps: i128,
    pub jump_slope_bps: i128,
    pub rate_floor_bps: i128,
    pub rate_ceiling_bps: i128,
    pub spread_bps: i128,
    pub last_update: u64,
}

/// Risk parameters for liquidation and health checks
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskParams {
    pub close_factor_bps: i128,
    pub liquidation_incentive_bps: i128,
    pub collateral_ratio_bps: i128,
}

/// Flash loan configuration and tracking
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlashLoanConfig {
    pub enabled: bool,
    pub fee_bps: i128,
    pub max_loan_amount: i128,
}

/// User's asset position (collateral or debt)
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetPosition {
    pub asset: Address,
    pub amount: i128,
    pub last_interest_update: u64,
}

/// Liquidation configuration per asset
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiquidationConfig {
    pub enabled: bool,
    pub incentive_bps: i128,
    pub close_factor_bps: i128,
}
