#![allow(clippy::too_many_arguments)]
#![allow(deprecated)]

use soroban_sdk::{contract, contractimpl, Address, Env, IntoVal, String, Vec};

pub mod admin;
pub mod analytics;
pub mod borrow;
pub mod bridge;
pub mod config;
pub mod cross_asset;
pub mod deposit;
pub mod errors;
pub mod events;
pub mod flash_loan;
pub mod governance;
pub mod intents;
pub mod interest_rate;
pub mod liquidate;
pub mod mev_protection;
pub mod multi_collateral;
pub mod multisig;
pub mod oracle;
pub mod rate_limiter;
pub mod recovery;
pub mod reentrancy;
pub mod repay;
pub mod reserve;
pub mod risk_management;
pub mod risk_params;
pub mod safe_math;
pub mod storage;
pub mod treasury;
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
        interest_rate::initialize_interest_rate_config(&env, admin).map_err(|e| {
            if e == InterestRateError::AlreadyInitialized {
                RiskManagementError::AlreadyInitialized
            } else {
                RiskManagementError::Unauthorized
            }
        })?;
        Ok(())
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

    /// Set the native asset address used when `asset = None` (admin-only).
    pub fn set_native_asset_address(
        env: Env,
        caller: Address,
        native_asset: Address,
    ) -> Result<(), LendingError> {
        deposit::set_native_asset_address(&env, caller, native_asset).map_err(Into::into)
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
}

#[cfg(test)]
#[path = "tests/borrow_cap_test.rs"]
mod borrow_cap_test;
#[cfg(test)]
#[path = "tests/supply_cap_test.rs"]
mod supply_cap_test;
#[cfg(test)]
#[path = "tests/isolated_pool_test.rs"]
mod isolated_pool_test;
#[cfg(test)]
#[path = "tests/cross_contract_test.rs"]
mod cross_contract_test;
#[cfg(test)]
mod flash_loan_test;
#[cfg(test)]
#[path = "tests/governance_test.rs"]
mod governance_test;
#[cfg(test)]
#[path = "tests/mev_protection_test.rs"]
mod mev_protection_test;
#[cfg(test)]
mod multi_collateral_test;
#[cfg(test)]
mod test_reentrancy;
#[cfg(test)]
mod test_zero_amount;
#[cfg(test)]
mod treasury_test;
#[cfg(test)]
#[path = "tests/diff_harness.rs"]
mod diff_harness;
#[cfg(test)]
#[path = "tests/differential_test.rs"]
mod differential_test;
#[cfg(test)]
#[path = "tests/migration_verification_test.rs"]
mod migration_verification_test;
