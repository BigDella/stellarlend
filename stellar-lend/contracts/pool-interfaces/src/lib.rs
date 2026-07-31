#![no_std]
use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PoolInfo {
    pub pool_address: Address,
    pub asset: Address,
    pub apy_bps: u32,
    pub utilization_bps: u32,
    pub total_liquidity: i128,
    pub available_liquidity: i128,
    pub collateral_factor_bps: u32,
    pub risk_score: u32,
    pub active: bool,
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PoolAllocation {
    pub pool: Address,
    pub asset: Address,
    pub weight_bps: u32,
    pub amount: i128,
    pub expected_apy_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct UserPosition {
    pub pool: Address,
    pub asset: Address,
    pub deposited: i128,
    pub shares: i128,
    pub rewards_claimed: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum RiskProfile {
    Conservative,
    Moderate,
    Aggressive,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RouterConfig {
    pub min_apy_differential_bps: u32,
    pub max_pools: u32,
    pub rebalance_cooldown_secs: u64,
    pub slippage_tolerance_bps: u32,
    pub default_risk_profile: RiskProfile,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum PoolInterfaceError {
    PoolNotSupported = 1,
    InsufficientLiquidity = 2,
    RateUnavailable = 3,
    PoolPaused = 4,
    InvalidAmount = 5,
    SlippageExceeded = 6,
    Unauthorized = 7,
}

pub trait PoolInterface {
    fn get_pool_info(env: soroban_sdk::Env, pool: Address) -> PoolInfo;
    fn deposit(env: soroban_sdk::Env, pool: Address, user: Address, amount: i128) -> Result<i128, PoolInterfaceError>;
    fn withdraw(env: soroban_sdk::Env, pool: Address, user: Address, amount: i128) -> Result<i128, PoolInterfaceError>;
    fn get_apy(env: soroban_sdk::Env, pool: Address) -> u32;
    fn get_utilization(env: soroban_sdk::Env, pool: Address) -> u32;
    fn get_available_liquidity(env: soroban_sdk::Env, pool: Address) -> i128;
}
