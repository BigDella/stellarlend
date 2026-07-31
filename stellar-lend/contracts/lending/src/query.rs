use soroban_sdk::{contracttype, Address, Env, Symbol, Vec, I256};

use crate::borrow::{
    get_close_factor_bps, get_liquidation_incentive_bps, get_liquidation_threshold_bps, get_oracle,
    get_stablecoin_config, get_user_collateral, get_user_debt, BorrowCollateral, DebtPosition,
};
use crate::views::{
    get_collateral_balance, get_collateral_value, get_debt_balance, get_debt_value,
    get_health_factor, get_user_position, UserPositionSummary, HEALTH_FACTOR_SCALE,
    HEALTH_FACTOR_NO_DEBT, compute_health_factor, collateral_value, debt_value,
};
use crate::reentrancy::ReentrancyGuard;
use crate::interest;
use crate::storage::PoolConfig;
use crate::lazy::{LazyField, get as lazy_get};
use crate::insurance;
use crate::borrow::get_admin as get_borrow_admin;

// Re-export all view functions grouped by category

// ── Position queries ─────────────────────────────────────────────────────────

pub fn query_user_debt(env: &Env, user: &Address) -> DebtPosition {
    get_user_debt(env, user)
}

pub fn query_user_collateral(env: &Env, user: &Address) -> BorrowCollateral {
    get_user_collateral(env, user)
}

pub fn query_collateral_balance(env: &Env, user: &Address) -> i128 {
    let _guard = ReentrancyGuard::new_read_only(env);
    get_collateral_balance(env, user)
}

pub fn query_debt_balance(env: &Env, user: &Address) -> i128 {
    let _guard = ReentrancyGuard::new_read_only(env);
    get_debt_balance(env, user)
}

pub fn query_collateral_value(env: &Env, user: &Address) -> i128 {
    let _guard = ReentrancyGuard::new_read_only(env);
    get_collateral_value(env, user)
}

pub fn query_debt_value(env: &Env, user: &Address) -> i128 {
    let _guard = ReentrancyGuard::new_read_only(env);
    get_debt_value(env, user)
}

pub fn query_health_factor(env: &Env, user: &Address) -> i128 {
    let _guard = ReentrancyGuard::new_read_only(env);
    get_health_factor(env, user)
}

pub fn query_user_position(env: &Env, user: &Address) -> UserPositionSummary {
    let _guard = ReentrancyGuard::new_read_only(env);
    get_user_position(env, user)
}

// ── Admin / config queries ──────────────────────────────────────────────────

pub fn query_admin(env: &Env) -> Option<Address> {
    get_borrow_admin(env)
}

pub fn query_interest_index(env: &Env) -> i128 {
    let _guard = ReentrancyGuard::new_read_only(env);
    interest::current_index(env)
}

pub fn query_packed_config(env: &Env) -> Option<PoolConfig> {
    crate::storage::load(env)
}

pub fn query_lazy_field(env: &Env, field: LazyField) -> i128 {
    let _guard = ReentrancyGuard::new_read_only(env);
    lazy_get(env, field)
}

// ── Insurance queries ───────────────────────────────────────────────────────

pub fn query_insurance_claim(env: &Env, claim_id: u64) -> Option<insurance::InsuranceClaim> {
    insurance::get_claim_by_id(env, claim_id)
}

pub fn query_insurance_premium_rate(env: &Env, asset: &Address) -> i128 {
    insurance::get_premium_rate(env, asset)
}

pub fn query_insurance_coverage_limit(env: &Env, asset: &Address) -> i128 {
    insurance::get_coverage_limit(env, asset)
}

pub fn query_insurance_analytics(env: &Env) -> insurance::InsuranceAnalytics {
    insurance::get_analytics(env)
}

pub fn query_insurance_all_claim_ids(env: &Env) -> Vec<u64> {
    insurance::get_all_claim_ids(env)
}

pub fn query_insurance_all_claims(env: &Env) -> Vec<insurance::InsuranceClaim> {
    insurance::get_all_claims(env)
}
