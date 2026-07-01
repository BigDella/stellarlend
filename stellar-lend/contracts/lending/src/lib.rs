#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, Val, Vec};

mod borrow;
mod deposit;
mod events;
mod flash_loan;
mod pause;
mod reentrancy;
mod rounding;
mod token_adapter;
mod token_adapter_erc20;
mod token_adapter_native;
mod token_adapter_verify;
mod token_adapter_wrapped;
mod token_receiver;
mod withdraw;

use borrow::{
    borrow as borrow_cmd, deposit as borrow_deposit, get_admin as get_borrow_admin,
    get_user_collateral as get_borrow_collateral, get_user_debt as get_borrow_debt,
    initialize_borrow_settings as initialize_borrow_logic, repay as borrow_repay,
    set_admin as set_borrow_admin,
    set_liquidation_threshold_bps as set_liquidation_threshold_logic,
    set_oracle as set_oracle_logic, BorrowCollateral, BorrowError, DebtPosition,
};
use deposit::{
    deposit as deposit_logic, get_user_collateral as get_deposit_collateral,
    initialize_deposit_settings as initialize_deposit_logic, DepositCollateral, DepositError,
};
use flash_loan::{
    flash_loan as flash_loan_logic, record_price_sample as flash_record_price_sample,
    set_flash_loan_fee_bps as set_flash_loan_fee_logic,
    set_manipulation_config as set_flash_manipulation_config, FlashLoanError,
    ManipulationConfig as FlashManipulationConfig,
};
use pause::{is_paused, set_pause as set_pause_logic, PauseType};
use reentrancy::{ReentrancyGuard, ReentrancyKey};
use token_receiver::receive as receive_logic;

mod views;
use views::{
    get_collateral_balance as view_collateral_balance,
    get_collateral_value as view_collateral_value, get_debt_balance as view_debt_balance,
    get_debt_value as view_debt_value, get_health_factor as view_health_factor,
    get_user_position as view_user_position, UserPositionSummary,
};

use withdraw::{
    initialize_withdraw_settings as initialize_withdraw_logic,
    set_withdraw_paused as set_withdraw_paused_logic, withdraw as withdraw_logic, WithdrawError,
};
mod data_store;
mod insurance;
mod upgrade;

pub mod interest_rate;
pub mod risk_monitor;
pub mod query;
pub mod mutation;

// Performance optimization suite (issues #631–#634)
pub mod interest;
pub mod lazy;
pub mod liquidation;
pub mod storage;

// Security & performance suite (issues #635–#638)
pub mod rate_guard;
pub mod sandwich_protection;
pub mod simulation_cache;
pub mod batch_view;

use interest::InterestCacheError;
use lazy::{LazyError, LazyField};
use liquidation::{LiquidationError, LiquidationPlan, PositionSnapshot};
use storage::{PackError, PoolConfig};

use rate_guard::{RateGuardConfig, RateGuardError, RateManipulationAttempt, RateTwap};
use sandwich_protection::{ProtectionLevel, SandwichConfig, SandwichDetection, SandwichError};
use simulation_cache::{SimCacheConfig, SimCacheError, SimCacheStats, SimulationResult};
use batch_view::{BatchHealthResult, BatchHealthSummary, BatchPositionQuery, BatchViewError};

use insurance::{
    cancel_claim as insurance_cancel_claim, collect_premium as insurance_collect_premium,
    evaluate_claim as insurance_evaluate_claim, fund_pool as insurance_fund_pool,
    get_all_claim_ids as insurance_get_all_claim_ids, get_all_claims as insurance_get_all_claims,
    get_analytics as insurance_get_analytics, get_claim_by_id as insurance_get_claim,
    get_coverage_limit as insurance_get_coverage_limit,
    get_premium_rate as insurance_get_premium_rate, initialize as insurance_initialize,
    set_coverage_limit as insurance_set_coverage_limit, submit_claim as insurance_submit_claim,
    InsuranceAnalytics, InsuranceClaim, InsuranceError,
};

#[cfg(test)]
mod borrow_test;
#[cfg(test)]
mod data_store_test;
#[cfg(test)]
mod deposit_test;
#[cfg(test)]
mod flash_loan_test;
#[cfg(test)]
mod insurance_test;
#[cfg(test)]
mod math_safety_test;
#[cfg(test)]
mod pause_test;
#[cfg(test)]
mod reentrancy_fuzz_test;
#[cfg(test)]
mod token_receiver_test;
#[cfg(test)]
mod upgrade_test;
#[cfg(test)]
mod views_test;
#[cfg(test)]
mod withdraw_test;

// Property-based tests (issue #359)
#[cfg(test)]
mod borrow_prop_test;
#[cfg(test)]
mod deposit_prop_test;
#[cfg(test)]
mod interest_rate_prop_test;
#[cfg(test)]
mod invariant_prop_test;
#[cfg(test)]
mod proptest_helpers;
#[cfg(test)]
mod withdraw_prop_test;

#[contract]
pub struct LendingContract;

#[contractimpl]
impl LendingContract {
    /// Initialize the protocol with admin and settings
    pub fn initialize(
        env: Env,
        admin: Address,
        debt_ceiling: i128,
        min_borrow_amount: i128,
    ) -> Result<(), BorrowError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard (constructor protection), validation
        let _guard =
            ReentrancyGuard::new_constructor(&env).map_err(|_| BorrowError::ReentrancyDetected)?;

        if get_borrow_admin(&env).is_some() {
            return Err(BorrowError::Unauthorized);
        }

        // 2. EFFECTS: Update state before any external interactions
        set_borrow_admin(&env, &admin);
        initialize_borrow_logic(&env, debt_ceiling, min_borrow_amount)?;

        // 3. INTERACTIONS: No external calls
        Ok(())
    }

    /// Borrow assets against deposited collateral
    pub fn borrow(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
        collateral_asset: Address,
        collateral_amount: i128,
    ) -> Result<(), BorrowError> {
        borrow_cmd(
            &env,
            user,
            asset,
            amount,
            collateral_asset,
            collateral_amount,
        )
    }

    /// Set protocol pause state for a specific operation (admin only)
    pub fn set_pause(
        env: Env,
        admin: Address,
        pause_type: PauseType,
        paused: bool,
    ) -> Result<(), BorrowError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard, authorization
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| BorrowError::ReentrancyDetected)?;

        let current_admin = get_borrow_admin(&env).ok_or(BorrowError::Unauthorized)?;
        if admin != current_admin {
            return Err(BorrowError::Unauthorized);
        }
        admin.require_auth();

        // 2. EFFECTS: Update state before any external interactions
        set_pause_logic(&env, admin, pause_type, paused);

        // 3. INTERACTIONS: No external calls
        Ok(())
    }

    /// Repay borrowed assets
    pub fn repay(env: Env, user: Address, asset: Address, amount: i128) -> Result<(), BorrowError> {
        user.require_auth();
        if is_paused(&env, PauseType::Repay) {
            return Err(BorrowError::ProtocolPaused);
        }
        borrow_repay(&env, user, asset, amount)
    }

    /// Deposit collateral into the protocol
    pub fn deposit(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<i128, DepositError> {
        if is_paused(&env, PauseType::Deposit) {
            return Err(DepositError::DepositPaused);
        }
        deposit_logic(&env, user, asset, amount)
    }

    /// Deposit collateral for a borrow position
    pub fn deposit_collateral(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), BorrowError> {
        user.require_auth();
        if is_paused(&env, PauseType::Deposit) {
            return Err(BorrowError::ProtocolPaused);
        }
        borrow_deposit(&env, user, asset, amount)
    }

    /// Liquidate a position
    pub fn liquidate(
        env: Env,
        liquidator: Address,
        _borrower: Address,
        _debt_asset: Address,
        _collateral_asset: Address,
        _amount: i128,
    ) -> Result<(), BorrowError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard, authorization, pause state
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::LiquidateLock, false)
            .map_err(|_| BorrowError::ReentrancyDetected)?;

        liquidator.require_auth();
        if is_paused(&env, PauseType::Liquidation) {
            return Err(BorrowError::ProtocolPaused);
        }
        Ok(())
    }

    /// Get user's debt position
    pub fn get_user_debt(env: Env, user: Address) -> DebtPosition {
        get_borrow_debt(&env, &user)
    }

    /// Get user's collateral position (borrow module)
    pub fn get_user_collateral(env: Env, user: Address) -> BorrowCollateral {
        get_borrow_collateral(&env, &user)
    }

    /// Returns the user's collateral balance (raw amount).
    pub fn get_collateral_balance(env: Env, user: Address) -> i128 {
        // READ-ONLY REENTRANCY DETECTION
        let _guard = ReentrancyGuard::new_read_only(&env);
        view_collateral_balance(&env, &user)
    }

    /// Returns the user's debt balance (principal + accrued interest).
    pub fn get_debt_balance(env: Env, user: Address) -> i128 {
        // READ-ONLY REENTRANCY DETECTION
        let _guard = ReentrancyGuard::new_read_only(&env);
        view_debt_balance(&env, &user)
    }

    /// Returns the user's collateral value in common unit. 0 if oracle not set.
    pub fn get_collateral_value(env: Env, user: Address) -> i128 {
        // READ-ONLY REENTRANCY DETECTION
        let _guard = ReentrancyGuard::new_read_only(&env);
        view_collateral_value(&env, &user)
    }

    /// Returns the user's debt value in common unit. 0 if oracle not set.
    pub fn get_debt_value(env: Env, user: Address) -> i128 {
        // READ-ONLY REENTRANCY DETECTION
        let _guard = ReentrancyGuard::new_read_only(&env);
        view_debt_value(&env, &user)
    }

    /// Returns health factor (scaled 10000 = 1.0).
    pub fn get_health_factor(env: Env, user: Address) -> i128 {
        // READ-ONLY REENTRANCY DETECTION
        let _guard = ReentrancyGuard::new_read_only(&env);
        view_health_factor(&env, &user)
    }

    /// Returns full position summary.
    pub fn get_user_position(env: Env, user: Address) -> UserPositionSummary {
        // READ-ONLY REENTRANCY DETECTION
        let _guard = ReentrancyGuard::new_read_only(&env);
        view_user_position(&env, &user)
    }

    /// Set oracle address for price feeds (admin only).
    pub fn set_oracle(env: Env, admin: Address, oracle: Address) -> Result<(), BorrowError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| BorrowError::ReentrancyDetected)?;

        set_oracle_logic(&env, &admin, oracle)
    }

    /// Set liquidation threshold in basis points (admin only).
    pub fn set_liquidation_threshold_bps(
        env: Env,
        admin: Address,
        bps: i128,
    ) -> Result<(), BorrowError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| BorrowError::ReentrancyDetected)?;

        set_liquidation_threshold_logic(&env, &admin, bps)
    }

    /// Initialize deposit settings (admin only)
    pub fn initialize_deposit_settings(
        env: Env,
        deposit_cap: i128,
        min_deposit_amount: i128,
    ) -> Result<(), DepositError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| DepositError::ReentrancyDetected)?;

        initialize_deposit_logic(&env, deposit_cap, min_deposit_amount)
    }

    /// Set deposit pause state (admin only)
    pub fn set_deposit_paused(env: Env, paused: bool) -> Result<(), DepositError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| DepositError::ReentrancyDetected)?;

        env.storage()
            .persistent()
            .set(&pause::PauseDataKey::State(PauseType::Deposit), &paused);
        Ok(())
    }

    /// Get user's deposit collateral position
    pub fn get_user_collateral_deposit(
        env: Env,
        user: Address,
        asset: Address,
    ) -> DepositCollateral {
        get_deposit_collateral(&env, &user, &asset)
    }

    /// Get protocol admin
    pub fn get_admin(env: Env) -> Option<Address> {
        get_borrow_admin(&env)
    }

    /// Execute a flash loan with attack-prevention guards.
    ///
    /// `spot_price` is the current oracle price of `asset` and is used for
    /// TWAP-deviation detection. Pass 0 to skip the TWAP check (not recommended
    /// in production).
    pub fn flash_loan(
        env: Env,
        receiver: Address,
        asset: Address,
        amount: i128,
        spot_price: i128,
        params: Bytes,
    ) -> Result<(), FlashLoanError> {
        flash_loan_logic(&env, receiver, asset, amount, spot_price, params)
    }

    /// Record an oracle price sample for a given asset (used to maintain the TWAP).
    /// Should be called by the oracle feed on each price update.
    pub fn flash_record_price(env: Env, asset: Address, price: i128) {
        flash_record_price_sample(&env, &asset, price);
    }

    /// Update the flash loan attack-prevention config (admin only).
    pub fn set_flash_manipulation_config(
        env: Env,
        admin: Address,
        config: FlashManipulationConfig,
    ) -> Result<(), FlashLoanError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard, authorization
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| FlashLoanError::Reentrancy)?;

        let current_admin = get_borrow_admin(&env).ok_or(FlashLoanError::Unauthorized)?;
        if admin != current_admin {
            return Err(FlashLoanError::Unauthorized);
        }
        admin.require_auth();

        // 2. EFFECTS: Update state before any external interactions
        set_flash_manipulation_config(&env, config)

        // 3. INTERACTIONS: No external calls
    }

    /// Set the flash loan fee in basis points (admin only)
    pub fn set_flash_loan_fee_bps(env: Env, fee_bps: i128) -> Result<(), FlashLoanError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard, authorization
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| FlashLoanError::Reentrancy)?;

        let current_admin = get_borrow_admin(&env).ok_or(FlashLoanError::Unauthorized)?;
        current_admin.require_auth();

        // 2. EFFECTS: Update state before any external interactions
        set_flash_loan_fee_logic(&env, fee_bps)

        // 3. INTERACTIONS: No external calls
    }

    /// Withdraw collateral from the protocol
    pub fn withdraw(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<i128, WithdrawError> {
        if is_paused(&env, PauseType::Withdraw) {
            return Err(WithdrawError::WithdrawPaused);
        }
        withdraw_logic(&env, user, asset, amount)
    }

    /// Initialize withdraw settings (admin only)
    pub fn initialize_withdraw_settings(
        env: Env,
        min_withdraw_amount: i128,
    ) -> Result<(), WithdrawError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| WithdrawError::ReentrancyDetected)?;

        initialize_withdraw_logic(&env, min_withdraw_amount)
    }

    /// Set withdraw pause state (admin only)
    pub fn set_withdraw_paused(env: Env, paused: bool) -> Result<(), WithdrawError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| WithdrawError::ReentrancyDetected)?;

        set_withdraw_paused_logic(&env, paused)
    }

    /// Token receiver hook
    pub fn receive(
        env: Env,
        token_asset: Address,
        from: Address,
        amount: i128,
        payload: Vec<Val>,
    ) -> Result<(), BorrowError> {
        receive_logic(env, token_asset, from, amount, payload)
    }

    /// Initialize borrow settings (admin only)
    pub fn initialize_borrow_settings(
        env: Env,
        debt_ceiling: i128,
        min_borrow_amount: i128,
    ) -> Result<(), BorrowError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| BorrowError::ReentrancyDetected)?;

        initialize_borrow_logic(&env, debt_ceiling, min_borrow_amount)
    }

    // ═══════════════════════════════════════════════ Insurance pool ═══

    pub fn insurance_initialize(env: Env, admin: Address) -> Result<(), InsuranceError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard (constructor protection)
        let _guard =
            ReentrancyGuard::new_constructor(&env).map_err(|_| InsuranceError::Unauthorized)?;

        insurance_initialize(&env, &admin)
    }

    pub fn insurance_fund_pool(env: Env, amount: i128) -> Result<(), InsuranceError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| InsuranceError::Unauthorized)?;

        insurance_fund_pool(&env, amount)
    }

    pub fn insurance_collect_premium(
        env: Env,
        payer: Address,
        asset: Address,
        coverage_amount: i128,
    ) -> Result<i128, InsuranceError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| InsuranceError::Unauthorized)?;

        insurance_collect_premium(&env, payer, asset, coverage_amount)
    }

    pub fn insurance_submit_claim(
        env: Env,
        claimant: Address,
        asset: Address,
        amount: i128,
    ) -> Result<u64, InsuranceError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| InsuranceError::Unauthorized)?;

        insurance_submit_claim(&env, claimant, asset, amount)
    }

    pub fn insurance_evaluate_claim(
        env: Env,
        admin: Address,
        claim_id: u64,
        approve: bool,
    ) -> Result<(), InsuranceError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| InsuranceError::Unauthorized)?;

        insurance_evaluate_claim(&env, admin, claim_id, approve)
    }

    pub fn insurance_set_coverage_limit(
        env: Env,
        admin: Address,
        asset: Address,
        limit_bps: i128,
    ) -> Result<(), InsuranceError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| InsuranceError::Unauthorized)?;

        insurance_set_coverage_limit(&env, admin, asset, limit_bps)
    }

    pub fn insurance_get_claim(env: Env, claim_id: u64) -> Option<InsuranceClaim> {
        insurance_get_claim(&env, claim_id)
    }

    pub fn insurance_get_premium_rate(env: Env, asset: Address) -> i128 {
        insurance_get_premium_rate(&env, &asset)
    }

    pub fn insurance_get_coverage_limit(env: Env, asset: Address) -> i128 {
        insurance_get_coverage_limit(&env, &asset)
    }

    pub fn insurance_get_analytics(env: Env) -> InsuranceAnalytics {
        insurance_get_analytics(&env)
    }

    pub fn insurance_cancel_claim(
        env: Env,
        claimant: Address,
        claim_id: u64,
    ) -> Result<(), InsuranceError> {
        // CHECKS-EFFECTS-INTERACTIONS PATTERN
        // 1. CHECKS: Reentrancy guard
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| InsuranceError::Unauthorized)?;

        insurance_cancel_claim(&env, claimant, claim_id)
    }

    pub fn insurance_get_all_claim_ids(env: Env) -> Vec<u64> {
        insurance_get_all_claim_ids(&env)
    }

    pub fn insurance_get_all_claims(env: Env) -> Vec<InsuranceClaim> {
        insurance_get_all_claims(&env)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Dust sweep functions
    // ═══════════════════════════════════════════════════════════════════

    /// Sweep dust amounts from user's deposit position
    pub fn sweep_deposit_dust(
        env: Env,
        user: Address,
        asset: Address,
    ) -> Result<i128, DepositError> {
        deposit::sweep_dust(&env, user, asset)
    }

    /// Sweep dust amounts from user's debt position
    pub fn sweep_borrow_dust(env: Env, user: Address, asset: Address) -> Result<i128, BorrowError> {
        borrow::sweep_dust(&env, user, asset)
    }

    /// Sweep dust amounts from user's withdraw position
    pub fn sweep_withdraw_dust(
        env: Env,
        user: Address,
        asset: Address,
    ) -> Result<i128, WithdrawError> {
        withdraw::sweep_dust(&env, user, asset)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Performance optimization suite (issues #631–#634)
    // ═══════════════════════════════════════════════════════════════════

    /// Read-optimised cumulative interest index (#631).
    ///
    /// Computes the index at the current ledger **without** a storage write, so
    /// `view` callers pay no write gas.
    pub fn interest_index(env: Env) -> i128 {
        let _guard = ReentrancyGuard::new_read_only(&env);
        interest::current_index(&env)
    }

    /// Advance and persist the cached interest index incrementally (#631).
    ///
    /// No-ops (no storage write) when already accrued in the current ledger, so
    /// multiple operations in the same block are charged interest once (batch).
    pub fn accrue_interest(env: Env) -> Result<i128, InterestCacheError> {
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| InterestCacheError::Overflow)?;
        Ok(interest::accrue(&env)?.cumulative_index)
    }

    /// Invalidate the interest cache after a rate/parameter/oracle change (#631).
    pub fn invalidate_interest_cache(env: Env, admin: Address) -> Result<(), InterestCacheError> {
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| InterestCacheError::Overflow)?;
        if get_borrow_admin(&env).as_ref() == Some(&admin) {
            admin.require_auth();
        }
        interest::invalidate(&env).map(|_| ())
    }

    /// Build a validated liquidation plan with cheapest-first early exits (#632).
    ///
    /// Pure validation entry point: runs every check (health factor,
    /// close-factor clamp, oracle freshness, gas-vs-profit) before any state
    /// mutation, so a doomed liquidation reverts having only read state.
    pub fn plan_liquidation(
        env: Env,
        snapshot: PositionSnapshot,
        requested_repay_value: i128,
        max_oracle_age_secs: u64,
        est_gas_cost: i128,
    ) -> Result<LiquidationPlan, LiquidationError> {
        let _guard = ReentrancyGuard::new_read_only(&env);
        liquidation::plan_liquidation(
            &snapshot,
            requested_repay_value,
            env.ledger().timestamp(),
            max_oracle_age_secs,
            est_gas_cost,
        )
    }

    /// Read the packed pool configuration, if migrated (#633).
    pub fn get_packed_config(env: Env) -> Option<PoolConfig> {
        storage::load(&env)
    }

    /// Migrate loose configuration values into the packed two-word layout (#633).
    pub fn migrate_packed_config(env: Env, admin: Address) -> Result<PoolConfig, PackError> {
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| PackError::BpsFieldOverflow)?;
        if get_borrow_admin(&env).as_ref() == Some(&admin) {
            admin.require_auth();
        }
        storage::migrate_from_legacy(&env)
    }

    /// Read a lazily-initialised pool-state field, returning its default if the
    /// slot has never been written (no storage allocation) (#634).
    pub fn get_lazy_field(env: Env, field: LazyField) -> i128 {
        let _guard = ReentrancyGuard::new_read_only(&env);
        lazy::get(&env, field)
    }

    /// Eagerly initialise all deferrable fields for a pre-existing pool (#634).
    pub fn migrate_lazy_fields(env: Env, admin: Address) -> Result<u32, LazyError> {
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| LazyError::InvalidValue)?;
        if get_borrow_admin(&env).as_ref() == Some(&admin) {
            admin.require_auth();
        }
        Ok(lazy::migrate_initialize_all(&env))
    }

    // ═══════════════════════════════════════════════════════════════════
    // Query module interface — gas-efficient read-only delegates (#623)
    // ═══════════════════════════════════════════════════════════════════

    pub fn query_user_debt(env: Env, user: Address) -> DebtPosition {
        query::query_user_debt(&env, &user)
    }

    pub fn query_user_collateral(env: Env, user: Address) -> BorrowCollateral {
        query::query_user_collateral(&env, &user)
    }

    pub fn query_collateral_balance(env: Env, user: Address) -> i128 {
        query::query_collateral_balance(&env, &user)
    }

    pub fn query_debt_balance(env: Env, user: Address) -> i128 {
        query::query_debt_balance(&env, &user)
    }

    pub fn query_collateral_value(env: Env, user: Address) -> i128 {
        query::query_collateral_value(&env, &user)
    }

    pub fn query_debt_value(env: Env, user: Address) -> i128 {
        query::query_debt_value(&env, &user)
    }

    pub fn query_health_factor(env: Env, user: Address) -> i128 {
        query::query_health_factor(&env, &user)
    }

    pub fn query_user_position(env: Env, user: Address) -> UserPositionSummary {
        query::query_user_position(&env, &user)
    }

    pub fn query_admin(env: Env) -> Option<Address> {
        query::query_admin(&env)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Rate manipulation guard (issue #638)
    // ═══════════════════════════════════════════════════════════════════

    /// Configure rate manipulation thresholds (admin only).
    pub fn set_rate_guard_config(
        env: Env,
        admin: Address,
        config: RateGuardConfig,
    ) -> Result<RateGuardConfig, RateGuardError> {
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| RateGuardError::Overflow)?;
        rate_guard::set_config(&env, admin, config)
    }

    /// Get current rate guard configuration.
    pub fn get_rate_guard_config(env: Env) -> RateGuardConfig {
        rate_guard::get_config(&env)
    }

    /// Get the rate TWAP accumulator.
    pub fn get_rate_twap(env: Env) -> RateTwap {
        rate_guard::get_twap(&env)
    }

    /// Get rate manipulation attempt log.
    pub fn get_rate_manipulation_log(env: Env) -> Vec<RateManipulationAttempt> {
        rate_guard::get_attempt_log(&env)
    }

    /// Check whether a rate would be accepted (dry run, no state change).
    pub fn check_rate(
        env: Env,
        new_rate_bps: i128,
    ) -> Result<(i128, bool, bool), RateGuardError> {
        let _guard = ReentrancyGuard::new_read_only(&env);
        rate_guard::check_rate(&env, new_rate_bps)
    }

    /// Whitelist or remove a known aggregator (admin only).
    pub fn set_whitelisted_aggregator(
        env: Env,
        admin: Address,
        address: Address,
        whitelisted: bool,
    ) -> Result<(), RateGuardError> {
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| RateGuardError::Overflow)?;
        rate_guard::set_whitelisted_aggregator(&env, admin, address, whitelisted)
    }

    /// Check if an address is a whitelisted aggregator.
    pub fn is_whitelisted_aggregator(env: Env, address: Address) -> bool {
        rate_guard::is_whitelisted_aggregator(&env, &address)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Sandwich attack protection (issue #637)
    // ═══════════════════════════════════════════════════════════════════

    /// Configure sandwich protection parameters (admin only).
    pub fn set_sandwich_config(
        env: Env,
        admin: Address,
        config: SandwichConfig,
    ) -> Result<SandwichConfig, SandwichError> {
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| SandwichError::Overflow)?;
        sandwich_protection::set_config(&env, admin, config)
    }

    /// Get current sandwich protection configuration.
    pub fn get_sandwich_config(env: Env) -> SandwichConfig {
        sandwich_protection::get_config(&env)
    }

    /// Set user's protection level (none, basic, max).
    pub fn set_user_sandwich_protection(
        env: Env,
        user: Address,
        level: ProtectionLevel,
    ) -> Result<(), SandwichError> {
        sandwich_protection::set_user_protection(&env, user, level)
    }

    /// Get user's protection level.
    pub fn get_user_sandwich_protection(env: Env, user: Address) -> ProtectionLevel {
        sandwich_protection::get_user_protection(&env, &user)
    }

    /// Commit a transaction for commit-reveal protection (Max level).
    pub fn commit_sandwich_transaction(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
        operation_type: u32,
    ) -> Result<soroban_sdk::Hash, SandwichError> {
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| SandwichError::Overflow)?;
        sandwich_protection::commit_transaction(&env, user, asset, amount, operation_type)
    }

    /// Reveal and execute a committed transaction (Max level).
    pub fn reveal_sandwich_transaction(
        env: Env,
        user: Address,
        commit_hash: soroban_sdk::Hash,
        nonce: u64,
    ) -> Result<(), SandwichError> {
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| SandwichError::Overflow)?;
        sandwich_protection::reveal_transaction(&env, user, commit_hash, nonce)
    }

    /// Submit a transaction to the pending batch for randomized ordering.
    pub fn submit_sandwich_pending_tx(
        env: Env,
        user: Address,
        operation_type: u32,
        asset: Address,
        amount: i128,
        protection_level: ProtectionLevel,
    ) -> Result<u32, SandwichError> {
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| SandwichError::Overflow)?;
        sandwich_protection::submit_pending_transaction(
            &env, user, operation_type, asset, amount, protection_level,
        )
    }

    /// Get randomized execution order for current block's pending transactions.
    pub fn get_sandwich_execution_order(env: Env) -> Vec<u32> {
        sandwich_protection::get_execution_order(&env)
    }

    /// Clear pending transactions after batch execution.
    pub fn clear_sandwich_pending_txs(env: Env) {
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .unwrap();
        sandwich_protection::clear_pending_transactions(&env)
    }

    /// Get sandwich detection log.
    pub fn get_sandwich_detection_log(env: Env) -> Vec<SandwichDetection> {
        sandwich_protection::get_detection_log(&env)
    }

    /// Calculate premium fee for sandwich reversal protection.
    pub fn calculate_sandwich_premium_fee(env: Env, amount: i128) -> i128 {
        sandwich_protection::calculate_premium_fee(&env, amount)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Transaction simulation cache (issue #636)
    // ═══════════════════════════════════════════════════════════════════

    /// Configure simulation cache (admin only).
    pub fn set_sim_cache_config(env: Env, config: SimCacheConfig) {
        simulation_cache::set_config(&env, config)
    }

    /// Get simulation cache configuration.
    pub fn get_sim_cache_config(env: Env) -> SimCacheConfig {
        simulation_cache::get_config(&env)
    }

    /// Look up a cached simulation result.
    pub fn sim_cache_lookup(
        env: Env,
        operation_type: u32,
        pool: Address,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Option<SimulationResult> {
        let _guard = ReentrancyGuard::new_read_only(&env);
        simulation_cache::cache_lookup(&env, operation_type, &pool, &user, &asset, amount)
    }

    /// Insert a simulation result into the cache.
    pub fn sim_cache_insert(
        env: Env,
        operation_type: u32,
        pool: Address,
        user: Address,
        asset: Address,
        amount: i128,
        health_after: i128,
        collateral_value_after: i128,
        debt_value_after: i128,
        would_succeed: bool,
    ) -> Result<(), SimCacheError> {
        let _guard = ReentrancyGuard::new_with_key(&env, ReentrancyKey::GlobalLock, false)
            .map_err(|_| SimCacheError::Overflow)?;
        simulation_cache::cache_insert(
            &env, operation_type, &pool, &user, &asset, amount,
            health_after, collateral_value_after, debt_value_after, would_succeed,
        )
    }

    /// Clear the simulation cache.
    pub fn sim_cache_clear(env: Env) {
        simulation_cache::cache_clear(&env)
    }

    /// Get simulation cache statistics.
    pub fn get_sim_cache_stats(env: Env) -> SimCacheStats {
        simulation_cache::get_stats(&env)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Batched multi-pool health check (issue #635)
    // ═══════════════════════════════════════════════════════════════════

    /// Batch health check across multiple pool positions in a single call.
    pub fn batch_health_check(
        env: Env,
        queries: Vec<BatchPositionQuery>,
    ) -> Result<BatchHealthSummary, BatchViewError> {
        let _guard = ReentrancyGuard::new_read_only(&env);
        batch_view::batch_health_check(&env, &queries)
    }

    /// Paginated batch health check for large query sets (100+ positions).
    pub fn batch_health_check_paged(
        env: Env,
        queries: Vec<BatchPositionQuery>,
        offset: u32,
        limit: u32,
    ) -> Result<BatchHealthSummary, BatchViewError> {
        let _guard = ReentrancyGuard::new_read_only(&env);
        batch_view::batch_health_check_paged(&env, &queries, offset, limit)
    }

    /// Get total collateral and debt values across queried positions.
    pub fn batch_total_value(
        env: Env,
        queries: Vec<BatchPositionQuery>,
    ) -> Result<(i128, i128), BatchViewError> {
        let _guard = ReentrancyGuard::new_read_only(&env);
        batch_view::batch_total_value(&env, &queries)
    }

    /// Get only liquidatable positions from a batch query.
    pub fn batch_liquidatable_positions(
        env: Env,
        queries: Vec<BatchPositionQuery>,
    ) -> Result<Vec<BatchHealthResult>, BatchViewError> {
        let _guard = ReentrancyGuard::new_read_only(&env);
        batch_view::batch_liquidatable_positions(&env, &queries)
    }
}
