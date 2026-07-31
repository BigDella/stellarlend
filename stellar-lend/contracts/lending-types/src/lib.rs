#![no_std]

use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    pub collateral_amount: i128,
    pub debt_amount: i128,
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetConfig {
    pub collateral_factor: i128,
    pub liquidation_threshold: i128,
    pub reserve_factor: i128,
    pub max_supply: i128,
    pub max_borrow: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserPosition {
    pub user: Address,
    pub collateral_value: i128,
    pub debt_value: i128,
    pub health_factor: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolConfig {
    pub admin: Address,
    pub oracle: Option<Address>,
    pub debt_ceiling: i128,
    pub min_borrow_amount: i128,
    pub liquidation_threshold_bps: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PauseState {
    Active,
    Paused,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationType {
    Deposit,
    Withdraw,
    Borrow,
    Repay,
    Liquidation,
}

pub trait LendingError {
    fn to_u32(&self) -> u32;
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum CommonError {
    Unauthorized = 1,
    InvalidAmount = 2,
    InsufficientBalance = 3,
    ExceedsLimit = 4,
    ProtocolPaused = 5,
    OracleNotSet = 6,
    InvalidConfiguration = 7,
    ReentrancyDetected = 8,
}

impl LendingError for CommonError {
    fn to_u32(&self) -> u32 {
        *self as u32
    }
}

pub fn current_timestamp(env: &Env) -> u64 {
    env.ledger().timestamp()
}

pub fn calculate_health_factor(
    collateral_value: i128,
    debt_value: i128,
    threshold_bps: i128,
) -> i128 {
    if debt_value == 0 {
        return i128::MAX;
    }

    collateral_value
        .checked_mul(threshold_bps)
        .and_then(|v| v.checked_div(debt_value))
        .unwrap_or(0)
}

pub fn is_healthy(health_factor: i128) -> bool {
    health_factor >= 10_000
}

pub const BPS_DIVISOR: i128 = 10_000;
pub const MAX_BPS: i128 = 10_000;
