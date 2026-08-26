#![allow(clippy::too_many_arguments)]
#![allow(deprecated)]

use soroban_sdk::{contract, contractimpl, Address, Env, IntoVal, String, Vec};

pub mod admin;
pub mod amm;
pub mod analytics;
pub mod borrow;
pub mod bridge;
pub mod circuit_breaker;
pub mod config;
pub mod credit_score;
pub mod cross_asset;
pub mod debt_token;
pub mod deposit;
pub mod emergency_withdrawal;
pub mod errors;
pub mod events;
pub mod flash_loan;
pub mod governance;
pub mod health;
pub mod intents;
pub mod interest;
pub mod interest_rate;
pub mod liquidate;
pub mod liquidation;
pub mod liquidation_queue;
pub mod mev_protection;
pub mod multi_collateral;
pub mod multisig;
pub mod oracle;
pub mod rate_limiter;
pub mod rebalancing;
pub mod recovery;
pub mod reentrancy;
pub mod repay;
pub mod reserve;
pub mod reserve_factor;
pub mod risk_management;
pub mod risk_params;
pub mod safe_math;
pub mod storage;
pub mod timelock;
pub mod treasury;
pub mod traits;
pub mod test_utils;
pub mod tests;
pub mod types;
pub mod withdraw;

use crate::deposit::Position;
use crate::errors::LendingError;
use crate::interest_rate::InterestRateError;
use crate::risk_management::RiskManagementError;

/// The StellarLend core contract.
#[contract]
pub struct HelloContract;

#[contractimpl]
impl HelloContract {
    pub fn hello(env: Env) -> String {
        String::from_str(&env, "Hello")
    }

    pub fn update_price_feed(
        env: Env,
        caller: Address,
        asset: Address,
        price: i128,
        decimals: u32,
        oracle_address: Address,
    ) -> Result<i128, oracle::OracleError> {
        oracle::update_price_feed(&env, caller, asset, price, decimals, oracle_address)
    }

    pub fn get_price(env: Env, asset: Address) -> Result<i128, oracle::OracleError> {
        oracle::get_price(&env, &asset)
    }

    pub fn set_primary_oracle(
        env: Env,
        caller: Address,
        asset: Address,
        primary_oracle: Address,
    ) -> Result<(), oracle::OracleError> {
        oracle::set_primary_oracle(&env, caller, asset, primary_oracle)
    }

    pub fn set_fallback_oracle(
        env: Env,
        caller: Address,
        asset: Address,
        fallback_oracle: Address,
    ) -> Result<(), oracle::OracleError> {
        oracle::set_fallback_oracle(&env, caller, asset, fallback_oracle)
    }

    pub fn configure_oracle(
        env: Env,
        caller: Address,
        config: oracle::OracleConfig,
    ) -> Result<(), oracle::OracleError> {
        oracle::configure_oracle(&env, caller, config)
    }

    pub fn set_oracle_sources(
        env: Env,
        caller: Address,
        asset: Address,
        sources: Vec<Address>,
    ) -> Result<(), oracle::OracleError> {
        oracle::set_oracle_sources(&env, caller, asset, sources)
    }

    pub fn emergency_pause_asset_oracle(
        env: Env,
        caller: Address,
        asset: Address,
        pause_seconds: u64,
    ) -> Result<(), oracle::OracleError> {
        oracle::emergency_pause_asset_oracle(&env, caller, asset, pause_seconds)
    }

    pub fn get_oracle_circuit_breaker_state(
        env: Env,
        asset: Address,
    ) -> oracle::CircuitBreakerState {
        oracle::get_oracle_circuit_breaker_state(&env, &asset)
    }

    pub fn get_oracle_incident_report(
        env: Env,
        asset: Address,
    ) -> Option<oracle::OracleIncidentReport> {
        oracle::get_oracle_incident_report(&env, &asset)
    }

    pub fn gov_initialize(
        env: Env,
        admin: Address,
        vote_token: Address,
        voting_period: Option<u64>,
        execution_delay: Option<u64>,
        quorum_bps: Option<u32>,
        proposal_threshold: Option<i128>,
        timelock_duration: Option<u64>,
        default_voting_threshold: Option<i128>,
    ) -> Result<(), LendingError> {
        governance::initialize(
            &env,
            admin,
            vote_token,
            voting_period,
            execution_delay,
            quorum_bps,
            proposal_threshold,
            timelock_duration,
            default_voting_threshold,
        )
        .map_err(Into::into)
    }

    pub fn gov_create_proposal(
        env: Env,
        proposer: Address,
        proposal_type: types::ProposalType,
        description: String,
        voting_threshold: Option<i128>,
    ) -> Result<u64, LendingError> {
        governance::create_proposal(&env, proposer, proposal_type, description, voting_threshold)
            .map_err(Into::into)
    }

    pub fn gov_vote(
        env: Env,
        voter: Address,
        proposal_id: u64,
        vote_type: types::VoteType,
    ) -> Result<(), LendingError> {
        governance::vote(&env, voter, proposal_id, vote_type).map_err(Into::into)
    }

    pub fn gov_queue_proposal(
        env: Env,
        caller: Address,
        proposal_id: u64,
    ) -> Result<types::ProposalOutcome, LendingError> {
        governance::queue_proposal(&env, caller, proposal_id).map_err(Into::into)
    }

    pub fn gov_execute_proposal(
        env: Env,
        executor: Address,
        proposal_id: u64,
    ) -> Result<(), LendingError> {
        governance::execute_proposal(&env, executor, proposal_id).map_err(Into::into)
    }

    pub fn gov_cancel_proposal(
        env: Env,
        caller: Address,
        proposal_id: u64,
    ) -> Result<(), LendingError> {
        governance::cancel_proposal(&env, caller, proposal_id).map_err(Into::into)
    }

    pub fn gov_approve_proposal(
        env: Env,
        approver: Address,
        proposal_id: u64,
    ) -> Result<(), LendingError> {
        governance::approve_proposal(&env, approver, proposal_id).map_err(Into::into)
    }

    pub fn gov_add_guardian(
        env: Env,
        caller: Address,
        guardian: Address,
    ) -> Result<(), LendingError> {
        governance::add_guardian(&env, caller, guardian).map_err(Into::into)
    }

    pub fn gov_get_guardian_config(env: Env) -> Option<storage::GuardianConfig> {
        env.storage()
            .instance()
            .get(&storage::GovernanceDataKey::GuardianConfig)
    }

    pub fn gov_get_proposal(env: Env, proposal_id: u64) -> Option<types::Proposal> {
        governance::get_proposal(&env, proposal_id)
    }

    pub fn gov_get_vote_lock(env: Env, voter: Address) -> Option<types::VoteLock> {
        governance::get_vote_lock(&env, &voter)
    }

    pub fn gov_is_vote_locked(env: Env, voter: Address) -> bool {
        governance::is_vote_locked(&env, &voter)
    }

    pub fn gov_get_vote_power_snapshot(
        env: Env,
        proposal_id: u64,
        voter: Address,
    ) -> Option<types::VotePowerSnapshot> {
        governance::get_vote_power_snapshot(&env, proposal_id, &voter)
    }

    pub fn gov_delegate_vote(
        env: Env,
        delegator: Address,
        delegatee: Address,
    ) -> Result<(), LendingError> {
        governance::delegate_vote(&env, delegator, delegatee).map_err(Into::into)
    }

    pub fn gov_get_analytics(env: Env) -> types::GovernanceAnalytics {
        governance::get_governance_analytics(&env)
    }

    pub fn gov_simulate_proposal(
        env: Env,
        proposal_id: u64,
    ) -> Result<types::ProposalSimulationResult, LendingError> {
        governance::simulate_proposal(&env, proposal_id).map_err(Into::into)
    }

    pub fn gov_get_simulation_cache(
        env: Env,
        proposal_id: u64,
    ) -> Option<types::ProposalSimulationResult> {
        governance::get_simulation_cache(&env, proposal_id)
    }

    pub fn gov_get_parameter_optimization(
        env: Env,
    ) -> Result<types::ParameterOptimizationRecommendation, LendingError> {
        governance::get_parameter_optimization_recommendation(&env).map_err(Into::into)
    }

    pub fn gov_create_emergency_proposal(
        env: Env,
        caller: Address,
        proposal_type: types::ProposalType,
        description: String,
    ) -> Result<u64, LendingError> {
        governance::create_emergency_proposal(&env, caller, proposal_type, description)
            .map_err(Into::into)
    }

    pub fn initialize(env: Env, admin: Address) -> Result<(), LendingError> {
        if crate::admin::has_admin(&env) {
            return Err(LendingError::Unauthorized);
        }
        crate::admin::set_admin(&env, admin.clone(), None)
            .map_err(|_| RiskManagementError::Unauthorized)?;
        risk_management::initialize_risk_management(&env, admin.clone())?;
        risk_params::initialize_risk_params(&env)
            .map_err(|_| RiskManagementError::InvalidParameter)?;
        interest_rate::initialize_interest_rate_config(&env, admin.clone()).map_err(|e| {
            if e == InterestRateError::AlreadyInitialized {
                RiskManagementError::AlreadyInitialized
            } else {
                RiskManagementError::Unauthorized
            }
        })?;
        emergency_withdrawal::initialize_emergency_withdrawal(&env);
        Ok(())
    }

    pub fn trigger_emergency(
        env: Env,
        caller: Address,
        trigger: emergency_withdrawal::EmergencyTrigger,
        withdrawal_cap_bps: Option<i128>,
        bad_debt: Option<i128>,
    ) -> Result<(), LendingError> {
        emergency_withdrawal::trigger_emergency(&env, caller, trigger, withdrawal_cap_bps, bad_debt)
            .map_err(Into::into)
    }

    pub fn cancel_emergency(env: Env, caller: Address) -> Result<(), LendingError> {
        emergency_withdrawal::cancel_emergency(&env, caller).map_err(Into::into)
    }

    pub fn get_emergency_state(env: Env) -> emergency_withdrawal::EmergencyState {
        emergency_withdrawal::get_emergency_state(&env)
    }

    pub fn emergency_withdraw(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
    ) -> Result<i128, LendingError> {
        emergency_withdrawal::emergency_withdraw(&env, user, asset, amount)
    }

    pub fn transfer_admin(
        env: Env,
        caller: Address,
        new_admin: Address,
    ) -> Result<(), LendingError> {
        admin::set_admin(&env, new_admin, Some(caller)).map_err(Into::into)
    }

    pub fn deposit_collateral(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
    ) -> Result<i128, LendingError> {
        deposit::deposit_collateral(&env, user, asset, amount).map_err(Into::into)
    }

    /// Deposit collateral using cross-asset lending
    pub fn deposit_cross_asset(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
    ) -> Result<(), LendingError> {
        cross_asset::cross_asset_deposit(&env, user, asset, amount).map_err(Into::into)?;
        Ok(())
    }

    pub fn set_risk_params(
        env: Env,
        caller: Address,
        min_collateral_ratio: Option<i128>,
        liquidation_threshold: Option<i128>,
        close_factor: Option<i128>,
        liquidation_incentive: Option<i128>,
    ) -> Result<(), LendingError> {
        // Authorization is handled by risk_management::require_admin.
        risk_management::require_admin(&env, &caller)?;
        risk_params::set_risk_params(
            &env,
            min_collateral_ratio,
            liquidation_threshold,
            close_factor,
            liquidation_incentive,
        )
        .map_err(|_| RiskManagementError::InvalidParameter)?;

        Ok(())
    }

    pub fn borrow_asset(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
    ) -> Result<i128, LendingError> {
        // Rate limiting: per-user and global-per-pool (pool = asset or native sentinel)
        let pool = asset
            .clone()
            .unwrap_or_else(|| env.current_contract_address());
        rate_limiter::consume(
            &env,
            &user, // caller is the authenticated user in this entrypoint
            &user,
            &soroban_sdk::Symbol::new(&env, "borrow"),
            &pool,
        )
        .map_err(|_| LendingError::LimitExceeded)?;
        borrow::borrow_asset(&env, user, asset, amount).map_err(Into::into)
    }

    /// Borrow against collateral basket using cross-asset lending
    pub fn borrow_cross_asset(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
    ) -> Result<(), LendingError> {
        cross_asset::cross_asset_borrow(&env, user, asset, amount).map_err(Into::into)
    }

    /// Withdraw collateral using cross-asset lending
    pub fn withdraw_cross_asset(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
    ) -> Result<(), LendingError> {
        cross_asset::cross_asset_withdraw(&env, user, asset, amount).map_err(Into::into)?;
        Ok(())
    }

    /// Get user's cross-asset position summary
    pub fn get_cross_asset_position_summary(
        env: Env,
        user: Address,
    ) -> Result<cross_asset::UserPositionSummary, LendingError> {
        cross_asset::get_user_position_summary(&env, &user).map_err(Into::into)
    }

    /// Liquidate an unhealthy cross-asset position
    pub fn liquidate_cross_asset(
        env: Env,
        liquidator: Address,
        user: Address,
        debt_asset: Option<Address>,
        collateral_asset: Option<Address>,
        debt_to_repay: i128,
        collateral_to_receive: i128,
    ) -> Result<i128, LendingError> {
        cross_asset::cross_asset_liquidate(
            &env,
            liquidator,
            user,
            debt_asset,
            collateral_asset,
            debt_to_repay,
            collateral_to_receive,
        ).map_err(Into::into)
    }

    /// Set reserve factor for an asset (admin only)
    pub fn set_reserve_factor(
        env: Env,
        caller: Address,
        asset: Option<Address>,
        reserve_factor_bps: i128,
    ) -> Result<(), LendingError> {
        reserve::set_reserve_factor(&env, caller, asset, reserve_factor_bps).map_err(Into::into)
    }

    pub fn set_reserve_factor_curve(
        env: Env,
        caller: Address,
        asset: Option<Address>,
        curve: reserve_factor::ReserveFactorCurve,
    ) -> Result<(), LendingError> {
        reserve_factor::set_reserve_factor_curve(&env, caller, asset, curve).map_err(Into::into)
    }

    pub fn set_reserve_factor_bounds(
        env: Env,
        caller: Address,
        asset: Option<Address>,
        min_bps: i128,
        max_bps: i128,
    ) -> Result<(), LendingError> {
        reserve_factor::set_reserve_factor_bounds(&env, caller, asset, min_bps, max_bps)
            .map_err(Into::into)
    }

    pub fn get_reserve_factor_curve(
        env: Env,
        asset: Option<Address>,
    ) -> reserve_factor::ReserveFactorCurve {
        reserve_factor::get_reserve_factor_curve(&env, asset)
    }

    pub fn preview_reserve_factor(
        env: Env,
        asset: Option<Address>,
        utilization_bps: Option<i128>,
    ) -> Result<reserve_factor::ReserveFactorPreview, LendingError> {
        reserve_factor::preview_reserve_factor(&env, asset, utilization_bps).map_err(Into::into)
    }

    pub fn get_health_factor_batch(
        env: Env,
        user: Address,
    ) -> health::HealthBatch {
        health::batch_read_health_data(&env, &user)
    }

    pub fn get_batched_health_summary(
        env: Env,
        user: Address,
    ) -> Result<cross_asset::UserPositionSummary, LendingError> {
        health::get_batched_user_position_summary(&env, &user).map_err(Into::into)
    }

    pub fn registered_pool_modules(env: Env) -> (String, String, String, String) {
        let modules = traits::registered_modules();
        (
            String::from_str(&env, modules.0),
            String::from_str(&env, modules.1),
            String::from_str(&env, modules.2),
            String::from_str(&env, modules.3),
        )
    }

    /// Set treasury address (admin only)
    pub fn set_treasury_address(
        env: Env,
        caller: Address,
        treasury: Address,
    ) -> Result<(), LendingError> {
        reserve::set_treasury_address(&env, caller, treasury).map_err(Into::into)
    }

    /// Withdraw reserves to treasury (admin only)
    pub fn withdraw_reserve_funds(
        env: Env,
        caller: Address,
        asset: Option<Address>,
        amount: i128,
    ) -> Result<i128, LendingError> {
        reserve::withdraw_reserve_funds(&env, caller, asset, amount).map_err(Into::into)
    }

    /// Get reserve balance for an asset
    pub fn get_reserve_balance(
        env: Env,
        asset: Option<Address>,
    ) -> i128 {
        reserve::get_reserve_balance(&env, asset)
    }

    /// Get reserve factor for an asset
    pub fn get_reserve_factor(
        env: Env,
        asset: Option<Address>,
    ) -> i128 {
        reserve::get_reserve_factor(&env, asset)
    }

    /// Get comprehensive reserve statistics
    pub fn get_reserve_stats(
        env: Env,
        asset: Option<Address>,
    ) -> (i128, i128, Option<Address>) {
        reserve::get_reserve_stats(&env, asset)
    }

    /// Configure rebalancing settings for a user
    pub fn configure_rebalancing(
        env: Env,
        user: Address,
        target_health_factor_min: i128,
        target_health_factor_max: i128,
        max_gas_cost: i128,
        auto_rebalance_enabled: bool,
        min_swap_size: i128,
        max_slippage_bps: i128,
        rebalance_cooldown: u64,
    ) -> Result<(), LendingError> {
        rebalancing::configure_rebalancing(
            &env,
            user,
            target_health_factor_min,
            target_health_factor_max,
            max_gas_cost,
            auto_rebalance_enabled,
            min_swap_size,
            max_slippage_bps,
            rebalance_cooldown,
        ).map_err(Into::into)
    }

    /// Execute automated rebalancing for a user
    pub fn execute_rebalancing(
        env: Env,
        user: Address,
    ) -> Result<(), LendingError> {
        rebalancing::execute_rebalancing(&env, user).map_err(Into::into)
    }

    /// Get user's rebalancing configuration
    pub fn get_rebalancing_config(
        env: Env,
        user: Address,
    ) -> rebalancing::RebalancingConfig {
        rebalancing::get_rebalancing_config(&env, &user)
    }

    /// Set emergency stop for rebalancing (admin only)
    pub fn set_rebalancing_emergency_stop(
        env: Env,
        admin: Address,
        stopped: bool,
    ) -> Result<(), LendingError> {
        rebalancing::set_emergency_stop(&env, admin, stopped).map_err(Into::into)
    }

    /// Set rebalancing pause (admin only)
    pub fn set_rebalancing_pause(
        env: Env,
        admin: Address,
        paused: bool,
    ) -> Result<(), LendingError> {
        rebalancing::set_rebalancing_pause(&env, admin, paused).map_err(Into::into)
    }

    /// Mint a new debt token for a position
    pub fn mint_debt_token(
        env: Env,
        user: Address,
        collateral_asset: Option<Address>,
        principal: i128,
        interest_rate_bps: i128,
    ) -> Result<u64, LendingError> {
        debt_token::mint_debt_token(&env, user, collateral_asset, principal, interest_rate_bps).map_err(Into::into)
    }

    /// Transfer a debt token to another address
    pub fn transfer_debt_token(
        env: Env,
        from: Address,
        to: Address,
        token_id: u64,
    ) -> Result<(), LendingError> {
        debt_token::transfer_debt_token(&env, from, to, token_id).map_err(Into::into)
    }

    /// Burn a debt token (debt repayment)
    pub fn burn_debt_token(
        env: Env,
        user: Address,
        token_id: u64,
        reason: Symbol,
    ) -> Result<(), LendingError> {
        debt_token::burn_debt_token(&env, user, token_id, reason).map_err(Into::into)
    }

    /// Get debt position information for a token
    pub fn get_debt_position(
        env: Env,
        token_id: u64,
    ) -> Option<debt_token::DebtPosition> {
        debt_token::get_debt_position(&env, token_id)
    }

    /// Get all debt tokens owned by a user
    pub fn get_user_debt_tokens(
        env: Env,
        user: Address,
    ) -> Vec<u64> {
        debt_token::get_user_debt_tokens(&env, &user)
    }

    /// Get total supply of debt tokens
    pub fn get_debt_token_total_supply(
        env: Env,
    ) -> u64 {
        debt_token::get_total_supply(&env)
    }

    /// Set transfer pause for debt tokens (admin only)
    pub fn set_debt_token_transfer_pause(
        env: Env,
        admin: Address,
        paused: bool,
    ) -> Result<(), LendingError> {
        debt_token::set_transfer_pause(&env, admin, paused).map_err(Into::into)
    }

    /// Block/unblock an address from debt token transfers (admin only)
    pub fn set_debt_token_address_blocked(
        env: Env,
        admin: Address,
        address: Address,
        blocked: bool,
    ) -> Result<(), LendingError> {
        debt_token::set_address_blocked(&env, admin, address, blocked).map_err(Into::into)
    }

    /// Meta-tx style borrow: user authorizes intent off-chain, relayer submits.
    pub fn borrow_asset_intent(
        env: Env,
        relayer: Address,
        user: Address,
        asset: Option<Address>,
        amount: i128,
        nonce: u64,
        expires_at: u64,
    ) -> Result<i128, LendingError> {
        // Relayer must authorize themselves (pays fees).
        relayer.require_auth();

        // Require user authorization for the typed payload.
        let mut args = Vec::new(&env);
        args.push_back(user.clone().into_val(&env));
        args.push_back(asset.clone().into_val(&env));
        args.push_back(amount.into_val(&env));
        intents::require_intent_auth(
            &env,
            &user,
            &soroban_sdk::Symbol::new(&env, "borrow"),
            nonce,
            expires_at,
            args,
        )
        .map_err(|_| LendingError::Unauthorized)?;

        // Apply rate limit keyed to user (actor).
        let pool = asset
            .clone()
            .unwrap_or_else(|| env.current_contract_address());
        rate_limiter::consume(
            &env,
            &relayer,
            &user,
            &soroban_sdk::Symbol::new(&env, "borrow"),
            &pool,
        )
        .map_err(|_| LendingError::LimitExceeded)?;

        borrow::borrow_asset(&env, user, asset, amount).map_err(Into::into)
    }

    pub fn repay_debt(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
    ) -> Result<(i128, i128, i128), LendingError> {
        repay::repay_debt(&env, user, asset, amount).map_err(Into::into)
    }

    pub fn withdraw_collateral(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
    ) -> Result<i128, LendingError> {
        withdraw::withdraw_collateral(&env, user, asset, amount).map_err(Into::into)
    }

    pub fn liquidate(
        env: Env,
        liquidator: Address,
        borrower: Address,
        debt_asset: Option<Address>,
        collateral_asset: Option<Address>,
        debt_amount: i128,
    ) -> Result<(i128, i128, i128), LendingError> {
        liquidator.require_auth();
        // Rate limiting: liquidator is the actor. Pool key uses the debt asset (or native sentinel).
        let pool = debt_asset
            .clone()
            .unwrap_or_else(|| env.current_contract_address());
        rate_limiter::consume(
            &env,
            &liquidator,
            &liquidator,
            &soroban_sdk::Symbol::new(&env, "liquidate"),
            &pool,
        )
        .map_err(|_| LendingError::LimitExceeded)?;
        liquidate::liquidate(
            &env,
            liquidator,
            borrower,
            debt_asset,
            collateral_asset,
            debt_amount,
        )
        .map_err(Into::into)
    }

    /// Liquidate up to `MAX_BATCH_SIZE` undercollateralized positions in one transaction.
    ///
    /// Amortizes authentication and submission overhead across all positions.
    /// Per-position failures are captured in the returned results and do not abort
    /// the entire batch.
    pub fn batch_liquidate(
        env: Env,
        liquidator: Address,
        requests: soroban_sdk::Vec<liquidate::BatchLiquidationRequest>,
    ) -> Result<soroban_sdk::Vec<liquidate::BatchLiquidationResult>, LendingError> {
        liquidator.require_auth();
        let pool = env.current_contract_address();
        rate_limiter::consume(
            &env,
            &liquidator,
            &liquidator,
            &soroban_sdk::Symbol::new(&env, "batch_liquidate"),
            &pool,
        )
        .map_err(|_| LendingError::LimitExceeded)?;
        liquidate::batch_liquidate(&env, liquidator, requests).map_err(Into::into)
    }

    pub fn configure_mev_protection(
        env: Env,
        caller: Address,
        config: mev_protection::MevProtectionConfig,
    ) -> Result<(), LendingError> {
        mev_protection::configure(&env, caller, config).map_err(Into::into)
    }

    pub fn get_mev_protection_config(env: Env) -> mev_protection::MevProtectionConfig {
        mev_protection::get_config(&env)
    }

    pub fn commit_borrow_protected(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
        max_fee_bps: i128,
        hint: mev_protection::TxOrderingHint,
    ) -> Result<u64, LendingError> {
        mev_protection::create_commit(
            &env,
            user,
            mev_protection::SensitiveOperation::Borrow,
            asset,
            None,
            None,
            amount,
            max_fee_bps,
            hint,
        )
        .map_err(Into::into)
    }

    /// Commit a borrow with explicit slippage tolerance and deadline.
    pub fn commit_borrow_with_slippage(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
        max_fee_bps: i128,
        hint: mev_protection::TxOrderingHint,
        max_slippage_bps: i128,
        deadline: u64,
    ) -> Result<u64, LendingError> {
        mev_protection::create_commit_with_slippage(
            &env,
            user,
            mev_protection::SensitiveOperation::Borrow,
            asset,
            None,
            None,
            amount,
            max_fee_bps,
            hint,
            max_slippage_bps,
            deadline,
        )
        .map_err(Into::into)
    }

    pub fn reveal_borrow_protected(
        env: Env,
        user: Address,
        commit_id: u64,
    ) -> Result<i128, LendingError> {
        let (asset, amount, _) = mev_protection::reveal_borrow(&env, user.clone(), commit_id)
            .map_err(LendingError::from)?;
        borrow::borrow_asset(&env, user, asset, amount).map_err(Into::into)
    }

    pub fn commit_withdraw_protected(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
        max_fee_bps: i128,
        hint: mev_protection::TxOrderingHint,
    ) -> Result<u64, LendingError> {
        mev_protection::create_commit(
            &env,
            user,
            mev_protection::SensitiveOperation::Withdraw,
            asset,
            None,
            None,
            amount,
            max_fee_bps,
            hint,
        )
        .map_err(Into::into)
    }

    /// Commit a withdrawal with explicit slippage tolerance and deadline.
    pub fn commit_withdraw_with_slippage(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
        max_fee_bps: i128,
        hint: mev_protection::TxOrderingHint,
        max_slippage_bps: i128,
        deadline: u64,
    ) -> Result<u64, LendingError> {
        mev_protection::create_commit_with_slippage(
            &env,
            user,
            mev_protection::SensitiveOperation::Withdraw,
            asset,
            None,
            None,
            amount,
            max_fee_bps,
            hint,
            max_slippage_bps,
            deadline,
        )
        .map_err(Into::into)
    }

    pub fn reveal_withdraw_protected(
        env: Env,
        user: Address,
        commit_id: u64,
    ) -> Result<i128, LendingError> {
        let (asset, amount) = mev_protection::reveal_withdraw(&env, user.clone(), commit_id)
            .map_err(LendingError::from)?;
        withdraw::withdraw_collateral(&env, user, asset, amount).map_err(Into::into)
    }

    pub fn commit_liquidation_protected(
        env: Env,
        liquidator: Address,
        borrower: Address,
        debt_asset: Option<Address>,
        collateral_asset: Option<Address>,
        debt_amount: i128,
        max_fee_bps: i128,
        hint: mev_protection::TxOrderingHint,
    ) -> Result<u64, LendingError> {
        mev_protection::create_commit(
            &env,
            liquidator,
            mev_protection::SensitiveOperation::Liquidate,
            debt_asset,
            collateral_asset,
            Some(borrower),
            debt_amount,
            max_fee_bps,
            hint,
        )
        .map_err(Into::into)
    }

    /// Commit a liquidation with explicit slippage tolerance and deadline.
    pub fn commit_liquidation_with_slippage(
        env: Env,
        liquidator: Address,
        borrower: Address,
        debt_asset: Option<Address>,
        collateral_asset: Option<Address>,
        debt_amount: i128,
        max_fee_bps: i128,
        hint: mev_protection::TxOrderingHint,
        max_slippage_bps: i128,
        deadline: u64,
    ) -> Result<u64, LendingError> {
        mev_protection::create_commit_with_slippage(
            &env,
            liquidator,
            mev_protection::SensitiveOperation::Liquidate,
            debt_asset,
            collateral_asset,
            Some(borrower),
            debt_amount,
            max_fee_bps,
            hint,
            max_slippage_bps,
            deadline,
        )
        .map_err(Into::into)
    }

    /// Place a bid in the current batch liquidation auction.
    ///
    /// Bids are collected during the open window and settled atomically via
    /// `settle_batch_auction` after the window closes.
    pub fn place_auction_bid(
        env: Env,
        bidder: Address,
        borrower: Address,
        debt_amount: i128,
        min_collateral_out: i128,
        max_fee_bps: i128,
        deadline: u64,
    ) -> Result<u64, LendingError> {
        mev_protection::place_auction_bid(
            &env,
            bidder,
            borrower,
            debt_amount,
            min_collateral_out,
            max_fee_bps,
            deadline,
        )
        .map_err(Into::into)
    }

    /// Settle a closed batch auction slot and return the clearing result.
    pub fn settle_batch_auction(
        env: Env,
        caller: Address,
        slot_id: u64,
    ) -> Result<mev_protection::AuctionResult, LendingError> {
        mev_protection::settle_batch_auction(&env, caller, slot_id).map_err(Into::into)
    }

    /// Return bids for a given auction slot.
    pub fn get_auction_bids(
        env: Env,
        slot_id: u64,
    ) -> soroban_sdk::Vec<mev_protection::AuctionBid> {
        mev_protection::get_auction_bids(&env, slot_id)
    }

    /// Return the settled result for a given auction slot.
    pub fn get_auction_result(
        env: Env,
        slot_id: u64,
    ) -> Option<mev_protection::AuctionResult> {
        mev_protection::get_auction_result(&env, slot_id)
    }

    /// Return the current open auction slot ID.
    pub fn get_current_auction_slot(env: Env) -> u64 {
        mev_protection::get_current_auction_slot(&env)
    }

    /// Return a gas bidding analysis snapshot for the given operation.
    pub fn get_gas_bid_analysis(
        env: Env,
        operation: mev_protection::SensitiveOperation,
        asset: Option<Address>,
        amount: i128,
    ) -> mev_protection::GasBidAnalysis {
        mev_protection::get_gas_bid_analysis(&env, operation, asset, amount)
    }

    pub fn reveal_liquidation_protected(
        env: Env,
        liquidator: Address,
        commit_id: u64,
    ) -> Result<(i128, i128, i128), LendingError> {
        let (borrower, debt_asset, collateral_asset, debt_amount) =
            mev_protection::reveal_liquidation(&env, liquidator.clone(), commit_id)
                .map_err(LendingError::from)?;
        liquidate::liquidate(
            &env,
            liquidator,
            borrower,
            debt_asset,
            collateral_asset,
            debt_amount,
        )
        .map_err(Into::into)
    }

    pub fn cancel_mev_commit(env: Env, user: Address, commit_id: u64) -> Result<(), LendingError> {
        mev_protection::cancel_commit(&env, user, commit_id).map_err(Into::into)
    }

    pub fn get_mev_commit(env: Env, commit_id: u64) -> Option<mev_protection::PendingCommit> {
        mev_protection::get_commit(&env, commit_id)
    }

    pub fn preview_mev_fee_bps(
        env: Env,
        operation: mev_protection::SensitiveOperation,
        asset: Option<Address>,
        amount: i128,
    ) -> i128 {
        mev_protection::preview_fee_bps(&env, operation, asset, amount)
    }

    pub fn get_mev_ordering_hint(
        env: Env,
        requested: mev_protection::TxOrderingHint,
    ) -> mev_protection::TxOrderingHint {
        mev_protection::execution_hint(&env, requested)
    }

    pub fn get_mev_user_guidance(
        env: Env,
        operation: mev_protection::SensitiveOperation,
    ) -> String {
        mev_protection::user_guidance(&env, operation)
    }

    pub fn get_mev_ordering_stats(env: Env) -> mev_protection::OrderingStats {
        mev_protection::get_ordering_stats(&env)
    }

    /// Meta-tx style liquidation: liquidator authorizes intent off-chain.
    pub fn liquidate_intent(
        env: Env,
        relayer: Address,
        liquidator: Address,
        borrower: Address,
        debt_asset: Option<Address>,
        collateral_asset: Option<Address>,
        debt_amount: i128,
        nonce: u64,
        expires_at: u64,
    ) -> Result<(i128, i128, i128), LendingError> {
        relayer.require_auth();

        let mut args = Vec::new(&env);
        args.push_back(liquidator.clone().into_val(&env));
        args.push_back(borrower.clone().into_val(&env));
        args.push_back(debt_asset.clone().into_val(&env));
        args.push_back(collateral_asset.clone().into_val(&env));
        args.push_back(debt_amount.into_val(&env));

        intents::require_intent_auth(
            &env,
            &liquidator,
            &soroban_sdk::Symbol::new(&env, "liquidate"),
            nonce,
            expires_at,
            args,
        )
        .map_err(|_| LendingError::Unauthorized)?;

        let pool = debt_asset
            .clone()
            .unwrap_or_else(|| env.current_contract_address());
        rate_limiter::consume(
            &env,
            &relayer,
            &liquidator,
            &soroban_sdk::Symbol::new(&env, "liquidate"),
            &pool,
        )
        .map_err(|_| LendingError::LimitExceeded)?;

        liquidate::liquidate(
            &env,
            liquidator,
            borrower,
            debt_asset,
            collateral_asset,
            debt_amount,
        )
        .map_err(Into::into)
    }

    pub fn set_emergency_pause(
        env: Env,
        caller: Address,
        paused: bool,
    ) -> Result<(), LendingError> {
        // Authorization is handled by risk_management::require_admin.
        risk_management::require_admin(&env, &caller)?;
        risk_management::set_emergency_pause(&env, caller, paused).map_err(Into::into)
    }

    pub fn execute_flash_loan(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
        callback: Address,
    ) -> Result<i128, LendingError> {
        flash_loan::execute_flash_loan(&env, user, asset, amount, callback).map_err(Into::into)
    }

    pub fn repay_flash_loan(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), LendingError> {
        flash_loan::repay_flash_loan(&env, user, asset, amount).map_err(Into::into)
    }

    pub fn can_be_liquidated(
        env: Env,
        collateral_value: i128,
        debt_value: i128,
    ) -> Result<bool, LendingError> {
        risk_params::can_be_liquidated(&env, collateral_value, debt_value).map_err(Into::into)
    }

    pub fn get_max_liquidatable_amount(env: Env, debt_value: i128) -> Result<i128, LendingError> {
        risk_params::get_max_liquidatable_amount(&env, debt_value).map_err(Into::into)
    }

    pub fn get_liquidation_incentive_amount(
        env: Env,
        liquidated_amount: i128,
    ) -> Result<i128, LendingError> {
        risk_params::get_liquidation_incentive_amount(&env, liquidated_amount).map_err(Into::into)
    }

    pub fn require_min_collateral_ratio(
        env: Env,
        collateral_value: i128,
        debt_value: i128,
    ) -> Result<(), LendingError> {
        risk_params::require_min_collateral_ratio(&env, collateral_value, debt_value)
            .map_err(Into::into)
    }

    // -------------------------------------------------------------------------
    // Treasury & Fee Management
    // -------------------------------------------------------------------------

    /// Set the protocol treasury address (admin-only)
    pub fn set_treasury(env: Env, caller: Address, treasury: Address) -> Result<(), LendingError> {
        treasury::set_treasury(&env, caller, treasury).map_err(Into::into)
    }

    /// Return the configured treasury address
    pub fn get_treasury(env: Env) -> Option<Address> {
        treasury::get_treasury(&env)
    }

    /// Return accumulated protocol reserves for the given asset
    pub fn get_reserve_balance(env: Env, asset: Option<Address>) -> i128 {
        treasury::get_reserve_balance(&env, asset)
    }

    pub fn set_reserve_amm_target(
        env: Env,
        caller: Address,
        asset: Option<Address>,
        amm_contract: Address,
    ) -> Result<(), LendingError> {
        reserve::set_reserve_amm_target(&env, caller, asset, amm_contract).map_err(Into::into)
    }

    pub fn get_reserve_amm_target(env: Env, asset: Option<Address>) -> Option<Address> {
        reserve::get_reserve_amm_target(&env, asset)
    }

    pub fn record_reserve_deploy_to_amm(
        env: Env,
        caller: Address,
        asset: Option<Address>,
        reserve_amount: i128,
        lp_tokens_received: i128,
    ) -> Result<(), LendingError> {
        reserve::record_reserve_deploy_to_amm(
            &env,
            caller,
            asset,
            reserve_amount,
            lp_tokens_received,
        )
        .map_err(Into::into)
    }

    pub fn get_reserve_amm_lp_balance(env: Env, asset: Option<Address>) -> i128 {
        reserve::get_reserve_amm_lp_balance(&env, asset)
    }

    // -------------------------------------------------------------------------
    // AMM Lending Integration (Issue #336)
    // -------------------------------------------------------------------------

    /// Initialize AMM lending integration (admin only)
    pub fn initialize_amm_lending(env: Env, admin: Address) -> Result<(), LendingError> {
        amm::initialize_amm_lending(&env, admin).map_err(|_| LendingError::Unauthorized)
    }

    /// Wrap lending pool deposits into AMM LP tokens
    pub fn amm_wrap_deposit_to_lp(
        env: Env,
        admin: Address,
        asset: Address,
        amount: i128,
        amm_protocol: Address,
    ) -> Result<amm::LpTokenPosition, LendingError> {
        amm::wrap_deposit_to_lp(&env, admin, asset, amount, amm_protocol)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Unwrap LP tokens back to lending pool assets
    pub fn amm_unwrap_lp_to_deposit(
        env: Env,
        admin: Address,
        asset: Address,
        lp_tokens: i128,
    ) -> Result<i128, LendingError> {
        amm::unwrap_lp_to_deposit(&env, admin, asset, lp_tokens)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Get LP token balance for an asset
    pub fn amm_get_lp_token_balance(env: Env, asset: Address) -> i128 {
        amm::get_lp_token_balance(&env, &asset)
    }

    /// Set withdrawal buffer for an asset (admin only)
    pub fn amm_set_withdrawal_buffer(
        env: Env,
        admin: Address,
        asset: Address,
        buffer_bps: i128,
    ) -> Result<(), LendingError> {
        amm::set_withdrawal_buffer(&env, admin, asset, buffer_bps)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Get withdrawal buffer for an asset
    pub fn amm_get_withdrawal_buffer(env: Env, asset: Address) -> i128 {
        amm::get_withdrawal_buffer(&env, &asset)
    }

    /// Calculate optimal AMM allocation based on pool utilization
    pub fn amm_calculate_optimal_allocation(
        env: Env,
        asset: Address,
        total_liquidity: i128,
        borrowed_amount: i128,
    ) -> Result<amm::AllocationSuggestion, LendingError> {
        amm::calculate_optimal_allocation(&env, &asset, total_liquidity, borrowed_amount)
            .map_err(|_| LendingError::InvalidParameter)
    }

    /// Execute automated AMM rebalancing
    pub fn amm_auto_rebalance_allocation(
        env: Env,
        admin: Address,
        asset: Address,
        total_liquidity: i128,
        borrowed_amount: i128,
        current_amm_balance: i128,
    ) -> Result<i128, LendingError> {
        amm::auto_rebalance_allocation(&env, admin, asset, total_liquidity, borrowed_amount, current_amm_balance)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Record LP fees accrued for distribution
    pub fn amm_record_lp_fees(
        env: Env,
        admin: Address,
        asset: Address,
        fee_amount: i128,
    ) -> Result<(), LendingError> {
        amm::record_lp_fees(&env, admin, asset, fee_amount)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Get accrued LP fees for an asset
    pub fn amm_get_accrued_lp_fees(env: Env, asset: Address) -> i128 {
        amm::get_accrued_lp_fees(&env, &asset)
    }

    /// Update impermanent loss tracking
    pub fn amm_update_il_tracking(env: Env, asset: Address, current_price: i128) -> Result<bool, LendingError> {
        amm::update_il_tracking(&env, &asset, current_price)
            .map_err(|_| LendingError::InvalidParameter)
    }

    /// Get impermanent loss snapshot
    pub fn amm_get_il_snapshot(env: Env, asset: Address) -> Option<amm::IlSnapshot> {
        amm::get_il_snapshot(&env, &asset)
    }

    /// Withdraw protocol reserves to a recipient (admin-only)
    pub fn claim_reserves(
        env: Env,
        caller: Address,
        asset: Option<Address>,
        recipient: Address,
        amount: i128,
    ) -> Result<(), LendingError> {
        treasury::claim_reserves(&env, caller, asset, recipient, amount).map_err(Into::into)
    }

    /// Update protocol fee percentages (admin-only)
    pub fn set_fee_config(
        env: Env,
        caller: Address,
        interest_fee_bps: i128,
        liquidation_fee_bps: i128,
    ) -> Result<(), LendingError> {
        treasury::set_fee_config(
            &env,
            caller,
            treasury::TreasuryFeeConfig {
                interest_fee_bps,
                liquidation_fee_bps,
            },
        )
        .map_err(Into::into)
    }

    /// Return the current fee configuration
    pub fn get_fee_config(env: Env) -> treasury::TreasuryFeeConfig {
        treasury::get_fee_config(&env)
    }

    // -------------------------------------------------------------------------
    // Multi-Asset Collateral
    // -------------------------------------------------------------------------

    /// Return the collateral balance for a specific (user, asset) pair
    pub fn get_user_asset_collateral(env: Env, user: Address, asset: Address) -> i128 {
        multi_collateral::get_user_asset_collateral(&env, &user, &asset)
    }

    /// Return the list of assets in which the user currently holds collateral
    pub fn get_user_asset_list(env: Env, user: Address) -> Vec<Address> {
        multi_collateral::get_user_asset_list(&env, &user)
    }

    /// Return the oracle-weighted total collateral value across all of the
    /// user's deposited assets (collateral factors applied per asset).
    /// Returns 0 for legacy single-asset users.
    pub fn get_user_total_collateral_value(env: Env, user: Address) -> i128 {
        multi_collateral::calculate_total_collateral_value(&env, &user).unwrap_or(0)
    }

    // -------------------------------------------------------------------------
    // Analytics
    // -------------------------------------------------------------------------

    /// Read-only user health factor query (collateral/debt in basis points).
    pub fn get_health_factor(env: Env, user: Address) -> Result<i128, LendingError> {
        analytics::calculate_health_factor(&env, &user).map_err(Into::into)
    }

    /// Read-only protocol metrics snapshot.
    pub fn get_protocol_stats(env: Env) -> Result<analytics::ProtocolMetrics, LendingError> {
        analytics::get_protocol_stats(&env).map_err(Into::into)
    }

    /// Read-only protocol analytics report.
    pub fn get_protocol_report(env: Env) -> Result<analytics::ProtocolReport, LendingError> {
        analytics::generate_protocol_report(&env).map_err(Into::into)
    }

    /// Read-only composite protocol health score (0-100), combining
    /// capital-efficiency and rate-stability sub-scores.
    pub fn get_protocol_health_score(
        env: Env,
    ) -> Result<analytics::ProtocolHealthScore, LendingError> {
        analytics::get_protocol_health_score(&env).map_err(Into::into)
    }

    /// Read-only user position query.
    pub fn get_user_position(env: Env, user: Address) -> Result<Position, LendingError> {
        analytics::get_user_position_summary(&env, &user).map_err(Into::into)
    }

    /// Read-only user analytics report.
    pub fn get_user_report(env: Env, user: Address) -> Result<analytics::UserReport, LendingError> {
        analytics::generate_user_report(&env, &user).map_err(Into::into)
    }

    /// Read-only recent protocol activity feed query.
    pub fn get_recent_activity(
        env: Env,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<analytics::ActivityEntry>, LendingError> {
        analytics::get_recent_activity(&env, limit, offset).map_err(Into::into)
    }

    /// Read-only: get next expected nonce for off-chain intents.
    pub fn get_intent_nonce(env: Env, user: Address, operation: soroban_sdk::Symbol) -> u64 {
        intents::get_next_nonce(&env, user, operation)
    }

    // -------------------------------------------------------------------------
    // Asset Configuration
    // -------------------------------------------------------------------------

    /// Set per-asset deposit/collateral parameters (admin-only).
    pub fn update_asset_config(
        env: Env,
        asset: Address,
        params: deposit::AssetParams,
    ) -> Result<(), LendingError> {
        let admin = crate::admin::get_admin(&env).ok_or(LendingError::Unauthorized)?;
        admin.require_auth();
        deposit::set_asset_params(&env, admin, asset, params).map_err(Into::into)
    }

    // -------------------------------------------------------------------------
    // Flash Loan Configuration
    // -------------------------------------------------------------------------

    /// Configure flash loan parameters (admin-only).
    pub fn configure_flash_loan(
        env: Env,
        caller: Address,
        config: flash_loan::FlashLoanConfig,
    ) -> Result<(), LendingError> {
        flash_loan::set_flash_loan_config(&env, caller, config).map_err(Into::into)
    }

    // -------------------------------------------------------------------------
    // Rate limiting configuration & monitoring
    // -------------------------------------------------------------------------

    /// Admin-only: configure default rate limits for an operation.
    pub fn configure_rate_limit_operation(
        env: Env,
        caller: Address,
        operation: soroban_sdk::Symbol,
        cfg: rate_limiter::RateLimitConfig,
    ) -> Result<(), LendingError> {
        rate_limiter::configure_operation_limit(&env, caller, operation, cfg).map_err(|e| match e {
            rate_limiter::RateLimitError::Unauthorized => LendingError::Unauthorized,
            rate_limiter::RateLimitError::InvalidConfig => LendingError::InvalidParameter,
            _ => LendingError::InvalidParameter,
        })
    }

    /// Admin-only: configure global-per-pool rate limits for an operation.
    pub fn configure_rate_limit_pool(
        env: Env,
        caller: Address,
        operation: soroban_sdk::Symbol,
        pool: Address,
        cfg: rate_limiter::RateLimitConfig,
    ) -> Result<(), LendingError> {
        rate_limiter::configure_pool_limit(&env, caller, operation, pool, cfg).map_err(
            |e| match e {
                rate_limiter::RateLimitError::Unauthorized => LendingError::Unauthorized,
                rate_limiter::RateLimitError::InvalidConfig => LendingError::InvalidParameter,
                _ => LendingError::InvalidParameter,
            },
        )
    }

    /// Admin-only: grant/revoke extra burst capacity for a (user, operation) pair.
    pub fn set_user_rate_limit_grace(
        env: Env,
        caller: Address,
        user: Address,
        operation: soroban_sdk::Symbol,
        enabled: bool,
    ) -> Result<(), LendingError> {
        rate_limiter::set_user_grace(&env, caller, user, operation, enabled).map_err(|e| match e {
            rate_limiter::RateLimitError::Unauthorized => LendingError::Unauthorized,
            _ => LendingError::InvalidParameter,
        })
    }

    /// Read-only: returns per-user bucket status.
    pub fn get_user_rate_limit_status(
        env: Env,
        user: Address,
        operation: soroban_sdk::Symbol,
        pool: Address,
    ) -> rate_limiter::RateLimitStatus {
        rate_limiter::get_user_status(&env, user, operation, pool)
    }

    /// Read-only: returns global-per-pool bucket status.
    pub fn get_global_rate_limit_status(
        env: Env,
        operation: soroban_sdk::Symbol,
        pool: Address,
    ) -> rate_limiter::RateLimitStatus {
        rate_limiter::get_global_status(&env, operation, pool)
    }

    // -------------------------------------------------------------------------
    // Interest Rate Views (Issue #180)
    // -------------------------------------------------------------------------

    /// Current borrow APY in basis points (e.g., 500 = 5%).
    pub fn get_borrow_rate(env: Env) -> i128 {
        interest_rate::get_current_borrow_rate(&env).unwrap_or(0)
    }

    /// Current supply APY in basis points.
    pub fn get_supply_rate(env: Env) -> i128 {
        interest_rate::get_current_supply_rate(&env).unwrap_or(0)
    }

    /// Current protocol utilization in basis points (0-10000).
    pub fn get_utilization_rate(env: Env) -> i128 {
        interest_rate::get_current_utilization(&env).unwrap_or(0)
    }

    /// Admin-only: update interest rate model parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn update_interest_rate_config(
        env: Env,
        caller: Address,
        base_rate_bps: Option<i128>,
        kink_utilization_bps: Option<i128>,
        multiplier_bps: Option<i128>,
        jump_multiplier_bps: Option<i128>,
        rate_floor_bps: Option<i128>,
        rate_ceiling_bps: Option<i128>,
        spread_bps: Option<i128>,
    ) -> Result<(), LendingError> {
        interest_rate::update_interest_rate_config(
            &env,
            caller,
            base_rate_bps,
            kink_utilization_bps,
            multiplier_bps,
            jump_multiplier_bps,
            rate_floor_bps,
            rate_ceiling_bps,
            spread_bps,
        )
        .map_err(Into::into)
    }

    /// Current global borrow index (scaled by 1e12; starts at 1e12 = "1.0").
    pub fn get_borrow_index(env: Env) -> i128 {
        interest_rate::get_borrow_index(&env)
    }

    /// Current global supply index (scaled by 1e12).
    pub fn get_supply_index(env: Env) -> i128 {
        interest_rate::get_supply_index(&env)
    }

    /// Get complete interest rate configuration.
    pub fn get_interest_rate_config(env: Env) -> Option<interest_rate::InterestRateConfig> {
        interest_rate::get_interest_rate_config(&env)
    }

    // -------------------------------------------------------------------------
    // Cross-Asset Lending Module (Issues #177, #178, #179)
    // -------------------------------------------------------------------------

    /// Initialize the cross-asset lending module (admin-only, once).
    pub fn initialize_ca(env: Env, admin: Address) -> Result<(), LendingError> {
        cross_asset::initialize(&env, admin).map_err(Into::into)
    }

    /// Register a new asset with per-asset parameters (admin-only).
    pub fn initialize_asset(
        env: Env,
        asset: Option<Address>,
        config: cross_asset::AssetConfig,
    ) -> Result<(), LendingError> {
        cross_asset::initialize_asset(&env, asset, config).map_err(Into::into)
    }

    /// Update an existing asset's configuration (admin-only).
    /// Emits SupplyCapChangedEvent / BorrowCapChangedEvent when caps change.
    #[allow(clippy::too_many_arguments)]
    pub fn update_ca_config(
        env: Env,
        asset: Option<Address>,
        collateral_factor: Option<i128>,
        liquidation_threshold: Option<i128>,
        max_supply: Option<i128>,
        max_borrow: Option<i128>,
        can_collateralize: Option<bool>,
        can_borrow: Option<bool>,
    ) -> Result<(), LendingError> {
        cross_asset::update_asset_config(
            &env,
            asset,
            collateral_factor,
            liquidation_threshold,
            max_supply,
            max_borrow,
            can_collateralize,
            can_borrow,
        )
        .map_err(Into::into)
    }

    /// Update oracle price for an asset (admin-only).
    pub fn update_asset_price(
        env: Env,
        asset: Option<Address>,
        price: i128,
    ) -> Result<(), LendingError> {
        cross_asset::update_asset_price(&env, asset, price).map_err(Into::into)
    }

    /// Deposit collateral into a specific asset pool.
    pub fn cross_asset_deposit(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
    ) -> Result<cross_asset::AssetPosition, LendingError> {
        cross_asset::cross_asset_deposit(&env, user, asset, amount).map_err(Into::into)
    }

    /// Withdraw collateral from a specific asset pool.
    pub fn cross_asset_withdraw(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
    ) -> Result<cross_asset::AssetPosition, LendingError> {
        cross_asset::cross_asset_withdraw(&env, user, asset, amount).map_err(Into::into)
    }

    /// Borrow from a specific asset pool against cross-pool (or isolated) collateral.
    pub fn cross_asset_borrow(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
    ) -> Result<cross_asset::AssetPosition, LendingError> {
        cross_asset::cross_asset_borrow(&env, user, asset, amount).map_err(Into::into)
    }

    /// Repay debt in a specific asset pool.
    pub fn ca_repay_debt(
        env: Env,
        user: Address,
        asset: Option<Address>,
        amount: i128,
    ) -> Result<cross_asset::AssetPosition, LendingError> {
        cross_asset::cross_asset_repay(&env, user, asset, amount).map_err(Into::into)
    }

    /// Get a user's cross-asset position summary (health factor, capacity, etc.).
    pub fn get_ca_position(
        env: Env,
        user: Address,
    ) -> Result<cross_asset::UserPositionSummary, LendingError> {
        cross_asset::get_user_position_summary(&env, &user).map_err(Into::into)
    }

    /// Read-only: look up asset configuration.
    pub fn get_ca_asset_config(
        env: Env,
        asset: Option<Address>,
    ) -> Result<cross_asset::AssetConfig, LendingError> {
        cross_asset::get_asset_config_by_address(&env, asset).map_err(Into::into)
    }

    /// Read-only: return the list of registered asset keys.
    pub fn get_ca_asset_list(env: Env) -> Vec<cross_asset::AssetKey> {
        cross_asset::get_asset_list(&env)
    }

    /// Supply headroom analytics: (available, cap, current_supply).
    /// Returns (i128::MAX, 0, current_supply) when cap is unlimited.
    pub fn get_supply_headroom(
        env: Env,
        asset: Option<Address>,
    ) -> Result<(i128, i128, i128), LendingError> {
        cross_asset::get_supply_headroom(&env, asset).map_err(Into::into)
    }

    /// Borrow utilization analytics: (current_borrows, cap).
    /// Returns (borrows, 0) when cap is unlimited.
    pub fn get_borrow_utilization(
        env: Env,
        asset: Option<Address>,
    ) -> Result<(i128, i128), LendingError> {
        cross_asset::get_borrow_utilization(&env, asset).map_err(Into::into)
    }

    /// Emergency freeze or unfreeze a pool (admin-only).
    pub fn freeze_pool(
        env: Env,
        caller: Address,
        asset: Option<Address>,
        freeze: bool,
    ) -> Result<(), LendingError> {
        cross_asset::freeze_pool(&env, caller, asset, freeze).map_err(Into::into)
    }

    // -------------------------------------------------------------------------
    // Liquidation Priority Queue (Issue #188)
    // -------------------------------------------------------------------------

    /// Initialize liquidation queue
    pub fn initialize_liquidation_queue(
        env: Env,
        config: liquidation_queue::QueueConfig,
    ) -> Result<(), LendingError> {
        liquidation_queue::initialize_queue(&env, config)
    }

    /// Register liquidator interest in unhealthy position
    pub fn register_liquidation_interest(
        env: Env,
        liquidator: Address,
        borrower: Address,
    ) -> Result<u64, LendingError> {
        liquidation_queue::register_liquidation_interest(&env, liquidator, borrower)
    }

    /// Get next liquidation from queue
    pub fn get_next_liquidation(
        env: Env,
    ) -> Option<liquidation_queue::LiquidationQueueEntry> {
        liquidation_queue::get_next_liquidation(&env)
    }

    /// Process liquidation from queue
    pub fn process_queue_liquidation(
        env: Env,
        entry_id: u64,
        executor: Address,
    ) -> Result<(), LendingError> {
        liquidation_queue::process_queue_liquidation(&env, entry_id, executor)
    }

    /// Cancel queue entry
    pub fn cancel_queue_entry(
        env: Env,
        entry_id: u64,
        caller: Address,
    ) -> Result<(), LendingError> {
        liquidation_queue::cancel_queue_entry(&env, entry_id, caller)
    }

    /// Get all pending queue entries
    pub fn get_pending_queue_entries(
        env: Env,
    ) -> Vec<liquidation_queue::LiquidationQueueEntry> {
        liquidation_queue::get_pending_queue_entries(&env)
    }

    /// Get queue entry by ID
    pub fn get_queue_entry(
        env: Env,
        entry_id: u64,
    ) -> Option<liquidation_queue::LiquidationQueueEntry> {
        liquidation_queue::get_queue_entry(&env, entry_id)
    }

    /// Cleanup expired entries
    pub fn cleanup_expired_queue_entries(env: Env) -> u32 {
        liquidation_queue::cleanup_expired_entries(&env)
    }

    /// Get queue statistics
    pub fn get_queue_stats(env: Env) -> liquidation_queue::QueueStats {
        liquidation_queue::get_queue_stats(&env)
    }
}

#[cfg(test)]
#[path = "tests/borrow_cap_test.rs"]
mod borrow_cap_test;
#[cfg(test)]
#[path = "tests/cross_contract_test.rs"]
mod cross_contract_test;
#[cfg(test)]
mod flash_loan_test;
#[cfg(test)]
mod governance_test;
#[cfg(test)]
#[path = "tests/isolated_pool_test.rs"]
mod isolated_pool_test;
#[cfg(test)]
#[path = "tests/mev_protection_test.rs"]
mod mev_protection_test;
#[cfg(test)]
mod multi_collateral_test;
#[cfg(test)]
#[path = "tests/supply_cap_test.rs"]
mod supply_cap_test;
#[cfg(test)]
mod test_reentrancy;
#[cfg(test)]
mod test_zero_amount;
#[cfg(test)]
mod treasury_test;
// Temporarily disabled due to pre-existing issues
// #[cfg(test)]
// #[path = "tests/timelock_test.rs"]
// mod timelock_test;
// Disabled until the full governance attack-prevention surface is implemented.
// mod governance_attack_prevention_test;

    // -------------------------------------------------------------------------
    // Credit Scoring System (Issue #189)
    // -------------------------------------------------------------------------

    /// Initialize credit score for a user
    pub fn initialize_credit_score(env: Env, user: Address) -> Result<(), LendingError> {
        credit_score::initialize_credit_score(&env, &user)
    }

    /// Get credit score for a user
    pub fn get_credit_score(env: Env, user: Address) -> Result<credit_score::CreditScore, LendingError> {
        credit_score::get_credit_score(&env, &user)
    }

    /// Calculate adjusted LTV based on credit score
    pub fn get_adjusted_ltv(env: Env, user: Address) -> Result<i128, LendingError> {
        credit_score::calculate_adjusted_ltv(&env, &user)
    }

    /// Calculate adjusted interest rate based on credit score
    pub fn get_adjusted_interest_rate(
        env: Env,
        user: Address,
        base_rate_bps: i128,
    ) -> Result<i128, LendingError> {
        credit_score::calculate_adjusted_interest_rate(&env, &user, base_rate_bps)
    }

    // -------------------------------------------------------------------------
    // Timelock Controller (Issue #187)
    // -------------------------------------------------------------------------

    /// Initialize timelock configuration
    pub fn initialize_timelock(
        env: Env,
        config: timelock::TimelockConfig,
    ) -> Result<(), LendingError> {
        timelock::initialize_timelock(&env, config).map_err(|e| match e {
            crate::errors::GovernanceError::InvalidTimelockConfig => LendingError::InvalidParameter,
            _ => LendingError::Unauthorized,
        })
    }

    /// Queue a timelock operation
    pub fn queue_timelock_operation(
        env: Env,
        proposer: Address,
        proposal_type: types::ProposalType,
        description: String,
        custom_delay: Option<u64>,
    ) -> Result<u64, LendingError> {
        timelock::queue_timelock_operation(&env, proposer, proposal_type, description, custom_delay)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Execute a timelock operation
    pub fn execute_timelock_operation(
        env: Env,
        executor: Address,
        operation_id: u64,
    ) -> Result<(), LendingError> {
        timelock::execute_timelock_operation(&env, executor, operation_id)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Cancel a timelock operation
    pub fn cancel_timelock_operation(
        env: Env,
        caller: Address,
        operation_id: u64,
    ) -> Result<(), LendingError> {
        timelock::cancel_timelock_operation(&env, caller, operation_id)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Get timelock operation
    pub fn get_timelock_operation(
        env: Env,
        operation_id: u64,
    ) -> Option<timelock::TimelockOperation> {
        timelock::get_timelock_operation(&env, operation_id)
    }

    /// Get all pending timelock operations
    pub fn get_pending_timelock_operations(env: Env) -> Vec<timelock::TimelockOperation> {
        timelock::get_pending_timelock_operations(&env)
    }

    /// Queue a batch timelock operation (multiple proposal types)
    pub fn queue_batch_timelock_operation(
        env: Env,
        proposer: Address,
        actions: Vec<types::ProposalType>,
        description: String,
        custom_delay: Option<u64>,
    ) -> Result<u64, LendingError> {
        timelock::queue_batch_timelock_operation(&env, proposer, actions, description, custom_delay)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Execute a batch timelock operation
    pub fn execute_batch_timelock_operation(
        env: Env,
        executor: Address,
        operation_id: u64,
    ) -> Result<(), LendingError> {
        timelock::execute_batch_timelock_operation(&env, executor, operation_id)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Get the priority-ordered timelock queue
    pub fn get_timelock_queue(env: Env) -> Vec<timelock::PriorityQueueEntry> {
        timelock::get_timelock_queue(&env)
    }

    /// Clean expired timelock queue entries
    pub fn clean_timelock_queue(env: Env) -> u32 {
        timelock::clean_timelock_queue(&env)
    }

    /// Get a batch timelock operation
    pub fn get_batch_timelock_operation(
        env: Env,
        operation_id: u64,
    ) -> Option<timelock::BatchTimelockOperation> {
        timelock::get_batch_timelock_operation(&env, operation_id)
    }

    // -------------------------------------------------------------------------
    // Circuit Breaker (Issue #186)
    // -------------------------------------------------------------------------

    /// Initialize circuit breaker
    pub fn initialize_circuit_breaker(
        env: Env,
        config: circuit_breaker::CircuitBreakerConfig,
    ) -> Result<(), LendingError> {
        circuit_breaker::initialize_circuit_breaker(&env, config)
    }

    /// Activate circuit breaker (governance or admin only)
    pub fn activate_circuit_breaker(
        env: Env,
        caller: Address,
        reason: circuit_breaker::CircuitBreakerReason,
        emergency_mode: bool,
    ) -> Result<(), LendingError> {
        circuit_breaker::activate_circuit_breaker(&env, caller, reason, emergency_mode)
    }

    /// Deactivate circuit breaker (governance or admin only)
    pub fn deactivate_circuit_breaker(env: Env, caller: Address) -> Result<(), LendingError> {
        circuit_breaker::deactivate_circuit_breaker(&env, caller)
    }

    /// Get circuit breaker state
    pub fn get_circuit_breaker_state(
        env: Env,
    ) -> Result<circuit_breaker::CircuitBreakerState, LendingError> {
        circuit_breaker::get_circuit_breaker_state(&env)
    }

    /// Check if liquidations are allowed
    pub fn is_liquidation_allowed(env: Env, liquidator: Address) -> Result<bool, LendingError> {
        circuit_breaker::is_liquidation_allowed(&env, &liquidator)
    }

    /// Add address to emergency liquidator whitelist
    pub fn add_to_whitelist(
        env: Env,
        admin: Address,
        liquidator: Address,
    ) -> Result<(), LendingError> {
        circuit_breaker::add_to_whitelist(&env, admin, liquidator)
    }

    /// Remove address from emergency liquidator whitelist
    pub fn remove_from_whitelist(
        env: Env,
        admin: Address,
        liquidator: Address,
    ) -> Result<(), LendingError> {
        circuit_breaker::remove_from_whitelist(&env, admin, liquidator)
    }

    /// Get whitelist
    pub fn get_circuit_breaker_whitelist(env: Env) -> Vec<Address> {
        circuit_breaker::get_whitelist(&env)
    }
