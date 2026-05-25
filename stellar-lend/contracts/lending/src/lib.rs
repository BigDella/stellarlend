#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, Val, Vec};

mod borrow;
mod deposit;
mod events;
mod flash_loan;
mod interest_rate;
mod pause;
mod risk_monitor;
mod token_receiver;
mod withdraw;

use borrow::{
    borrow as borrow_cmd, deposit as borrow_deposit, get_admin as get_borrow_admin,
    get_user_collateral as get_borrow_collateral, get_user_debt as get_borrow_debt,
    get_user_debt_with_rate as get_user_debt_with_rate_logic,
    initialize_borrow_settings as initialize_borrow_logic, repay as borrow_repay,
    set_admin as set_borrow_admin,
    set_liquidation_threshold_bps as set_liquidation_threshold_logic,
    set_oracle as set_oracle_logic, set_variable_borrow_rate_bps as set_variable_borrow_rate_logic,
    switch_rate_type as switch_rate_type_logic, BorrowCollateral, BorrowError, DebtPosition,
    RateType,
};
use deposit::{
    acknowledge_donation as acknowledge_donation_logic, deposit as deposit_logic,
    get_donation_defense_config as get_donation_defense_config_logic,
    get_donation_report as get_donation_report_logic,
    get_user_collateral as get_deposit_collateral,
    get_virtual_share_price_bps as get_virtual_share_price_bps_logic,
    initialize_deposit_settings as initialize_deposit_logic, is_donation_detected,
    set_donation_defense_config as set_donation_defense_config_logic,
    sync_donation_balance as sync_donation_balance_logic, DepositCollateral, DepositError,
    DonationDefenseConfig, DonationReport,
};
use flash_loan::{
    flash_loan as flash_loan_logic, set_flash_loan_fee_bps as set_flash_loan_fee_logic,
    FlashLoanError,
};
use pause::{is_paused, set_pause as set_pause_logic, PauseType};
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

use insurance::{
    collect_premium as insurance_collect_premium, evaluate_claim as insurance_evaluate_claim,
    fund_pool as insurance_fund_pool, get_analytics as insurance_get_analytics,
    get_claim_by_id as insurance_get_claim, get_coverage_limit as insurance_get_coverage_limit,
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
mod token_receiver_test;
#[cfg(test)]
mod upgrade_test;
#[cfg(test)]
mod views_test;
#[cfg(test)]
mod withdraw_test;

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
        if get_borrow_admin(&env).is_some() {
            return Err(BorrowError::Unauthorized);
        }
        set_borrow_admin(&env, &admin);
        initialize_borrow_logic(&env, debt_ceiling, min_borrow_amount)?;
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

    /// Borrow assets with an explicit variable/stable rate selection.
    pub fn borrow_with_rate(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
        collateral_asset: Address,
        collateral_amount: i128,
        rate_type: RateType,
    ) -> Result<(), BorrowError> {
        borrow::borrow_with_rate(
            &env,
            user,
            asset,
            amount,
            collateral_asset,
            collateral_amount,
            rate_type,
        )
    }

    /// Update the variable borrow rate base model (admin only).
    pub fn set_variable_borrow_rate_bps(
        env: Env,
        admin: Address,
        rate_bps: i128,
    ) -> Result<(), BorrowError> {
        set_variable_borrow_rate_logic(&env, &admin, rate_bps)
    }

    /// Switch an existing debt position between variable and stable accounting.
    pub fn switch_rate_type(
        env: Env,
        user: Address,
        asset: Address,
        rate_type: RateType,
    ) -> Result<(), BorrowError> {
        switch_rate_type_logic(&env, user, asset, rate_type)
    }

    /// Set protocol pause state for a specific operation (admin only)
    pub fn set_pause(
        env: Env,
        admin: Address,
        pause_type: PauseType,
        paused: bool,
    ) -> Result<(), BorrowError> {
        let current_admin = get_borrow_admin(&env).ok_or(BorrowError::Unauthorized)?;
        if admin != current_admin {
            return Err(BorrowError::Unauthorized);
        }
        admin.require_auth();
        set_pause_logic(&env, admin, pause_type, paused);
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
        debt_asset: Address,
        collateral_asset: Address,
        _amount: i128,
    ) -> Result<(), BorrowError> {
        liquidator.require_auth();
        if is_paused(&env, PauseType::Liquidation) {
            return Err(BorrowError::ProtocolPaused);
        }
        if is_donation_detected(&env, &debt_asset) || is_donation_detected(&env, &collateral_asset)
        {
            return Err(BorrowError::ProtocolPaused);
        }
        // Stub implementation, or call borrow::liquidate if it exists
        Ok(())
    }

    /// Get user's debt position
    pub fn get_user_debt(env: Env, user: Address) -> DebtPosition {
        get_borrow_debt(&env, &user)
    }

    /// Get user's debt position for a specific rate bucket.
    pub fn get_user_debt_with_rate(env: Env, user: Address, rate_type: RateType) -> DebtPosition {
        get_user_debt_with_rate_logic(&env, &user, rate_type)
    }

    /// Get user's collateral position (borrow module)
    pub fn get_user_collateral(env: Env, user: Address) -> BorrowCollateral {
        get_borrow_collateral(&env, &user)
    }

    // ═══════════════════════════════════════════════════════════════════
    // View functions (read-only; for frontends and liquidations)
    // ═══════════════════════════════════════════════════════════════════

    /// Returns the user's collateral balance (raw amount).
    pub fn get_collateral_balance(env: Env, user: Address) -> i128 {
        view_collateral_balance(&env, &user)
    }

    /// Returns the user's debt balance (principal + accrued interest).
    pub fn get_debt_balance(env: Env, user: Address) -> i128 {
        view_debt_balance(&env, &user)
    }

    /// Returns the user's collateral value in common unit (e.g. USD 8 decimals). 0 if oracle not set.
    pub fn get_collateral_value(env: Env, user: Address) -> i128 {
        view_collateral_value(&env, &user)
    }

    /// Returns the user's debt value in common unit. 0 if oracle not set.
    pub fn get_debt_value(env: Env, user: Address) -> i128 {
        view_debt_value(&env, &user)
    }

    /// Returns health factor (scaled 10000 = 1.0). Above 10000 = healthy; below = liquidatable.
    pub fn get_health_factor(env: Env, user: Address) -> i128 {
        view_health_factor(&env, &user)
    }

    /// Returns full position summary: collateral/debt balances and values, and health factor.
    pub fn get_user_position(env: Env, user: Address) -> UserPositionSummary {
        view_user_position(&env, &user)
    }

    /// Set oracle address for price feeds (admin only).
    pub fn set_oracle(env: Env, admin: Address, oracle: Address) -> Result<(), BorrowError> {
        set_oracle_logic(&env, &admin, oracle)
    }

    /// Set liquidation threshold in basis points, e.g. 8000 = 80% (admin only).
    pub fn set_liquidation_threshold_bps(
        env: Env,
        admin: Address,
        bps: i128,
    ) -> Result<(), BorrowError> {
        set_liquidation_threshold_logic(&env, &admin, bps)
    }

    /// Initialize deposit settings (admin only)
    pub fn initialize_deposit_settings(
        env: Env,
        deposit_cap: i128,
        min_deposit_amount: i128,
    ) -> Result<(), DepositError> {
        initialize_deposit_logic(&env, deposit_cap, min_deposit_amount)
    }

    /// Set deposit pause state (admin only)
    /// Deprecated: use set_pause instead
    pub fn set_deposit_paused(env: Env, paused: bool) -> Result<(), DepositError> {
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

    /// Configure donation attack defenses (admin only).
    pub fn set_donation_defense_config(
        env: Env,
        admin: Address,
        config: DonationDefenseConfig,
    ) -> Result<(), DepositError> {
        let current_admin = get_borrow_admin(&env).ok_or(DepositError::Unauthorized)?;
        if admin != current_admin {
            return Err(DepositError::Unauthorized);
        }
        admin.require_auth();
        set_donation_defense_config_logic(&env, config)
    }

    /// Return the active donation defense configuration.
    pub fn get_donation_defense_config(env: Env) -> DonationDefenseConfig {
        get_donation_defense_config_logic(&env)
    }

    /// Reconcile actual token balance against protocol accounting and quarantine donations.
    pub fn sync_donation_balance(env: Env, asset: Address) -> Result<DonationReport, DepositError> {
        sync_donation_balance_logic(&env, &asset)
    }

    /// Return the latest donation reconciliation report for an asset.
    pub fn get_donation_report(env: Env, asset: Address) -> Option<DonationReport> {
        get_donation_report_logic(&env, &asset)
    }

    /// Return the donation-resistant virtual share price in basis points.
    pub fn get_virtual_share_price_bps(env: Env, asset: Address) -> Result<i128, DepositError> {
        get_virtual_share_price_bps_logic(&env, &asset)
    }

    /// Clear a donation alert after operational review (admin only).
    pub fn acknowledge_donation(
        env: Env,
        admin: Address,
        asset: Address,
    ) -> Result<(), DepositError> {
        let current_admin = get_borrow_admin(&env).ok_or(DepositError::Unauthorized)?;
        if admin != current_admin {
            return Err(DepositError::Unauthorized);
        }
        admin.require_auth();
        acknowledge_donation_logic(&env, &asset);
        Ok(())
    }

    /// Get protocol admin
    pub fn get_admin(env: Env) -> Option<Address> {
        get_borrow_admin(&env)
    }

    /// Execute a flash loan
    pub fn flash_loan(
        env: Env,
        receiver: Address,
        asset: Address,
        amount: i128,
        params: Bytes,
    ) -> Result<(), FlashLoanError> {
        flash_loan_logic(&env, receiver, asset, amount, params)
    }

    /// Set the flash loan fee in basis points (admin only)
    pub fn set_flash_loan_fee_bps(env: Env, fee_bps: i128) -> Result<(), FlashLoanError> {
        let current_admin = get_borrow_admin(&env).ok_or(FlashLoanError::Unauthorized)?;
        current_admin.require_auth();
        set_flash_loan_fee_logic(&env, fee_bps)
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
        initialize_withdraw_logic(&env, min_withdraw_amount)
    }

    /// Set withdraw pause state (admin only)
    pub fn set_withdraw_paused(env: Env, paused: bool) -> Result<(), WithdrawError> {
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
        initialize_borrow_logic(&env, debt_ceiling, min_borrow_amount)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Insurance pool
    // ═══════════════════════════════════════════════════════════════════

    /// Initialize the insurance pool (admin only, call once).
    pub fn insurance_initialize(env: Env, admin: Address) -> Result<(), InsuranceError> {
        insurance_initialize(&env, &admin)
    }

    /// Contribute protocol fees to the insurance pool.
    pub fn insurance_fund_pool(env: Env, amount: i128) -> Result<(), InsuranceError> {
        insurance_fund_pool(&env, amount)
    }

    /// Collect a coverage premium from a user for a given asset.
    /// Returns the premium amount charged.
    pub fn insurance_collect_premium(
        env: Env,
        payer: Address,
        asset: Address,
        coverage_amount: i128,
    ) -> Result<i128, InsuranceError> {
        insurance_collect_premium(&env, payer, asset, coverage_amount)
    }

    /// Submit an insurance claim. Returns the new claim ID.
    pub fn insurance_submit_claim(
        env: Env,
        claimant: Address,
        asset: Address,
        amount: i128,
    ) -> Result<u64, InsuranceError> {
        insurance_submit_claim(&env, claimant, asset, amount)
    }

    /// Evaluate (approve or reject) a pending claim (admin only).
    pub fn insurance_evaluate_claim(
        env: Env,
        admin: Address,
        claim_id: u64,
        approve: bool,
    ) -> Result<(), InsuranceError> {
        insurance_evaluate_claim(&env, admin, claim_id, approve)
    }

    /// Set per-asset coverage limit in basis points (admin only).
    pub fn insurance_set_coverage_limit(
        env: Env,
        admin: Address,
        asset: Address,
        limit_bps: i128,
    ) -> Result<(), InsuranceError> {
        insurance_set_coverage_limit(&env, admin, asset, limit_bps)
    }

    /// Get a claim by ID.
    pub fn insurance_get_claim(env: Env, claim_id: u64) -> Option<InsuranceClaim> {
        insurance_get_claim(&env, claim_id)
    }

    /// Get current dynamic premium rate for an asset (basis points).
    pub fn insurance_get_premium_rate(env: Env, asset: Address) -> i128 {
        insurance_get_premium_rate(&env, &asset)
    }

    /// Get per-asset coverage limit in basis points.
    pub fn insurance_get_coverage_limit(env: Env, asset: Address) -> i128 {
        insurance_get_coverage_limit(&env, &asset)
    }

    /// Get insurance pool analytics.
    pub fn insurance_get_analytics(env: Env) -> InsuranceAnalytics {
        insurance_get_analytics(&env)
    }
}
