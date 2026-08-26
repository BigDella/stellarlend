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
pub mod health;
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
pub mod amm;

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

    /// Dry-run proposal execution with state diff, impact metrics, and gas estimate.
    pub fn gov_simulate_proposal_dry_run(
        env: Env,
        proposal_id: u64,
    ) -> Result<types::ProposalDryRunResult, LendingError> {
        governance::simulate_proposal_dry_run(&env, proposal_id).map_err(Into::into)
    }

    pub fn gov_get_dry_run_cache(
        env: Env,
        proposal_id: u64,
    ) -> Option<types::ProposalDryRunResult> {
        governance::get_dry_run_cache(&env, proposal_id)
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

    /// Pre-execution profit simulation for a flash-loan-funded liquidation.
    pub fn simulate_flash_loan_liquidation(
        env: Env,
        debt_asset: Option<Address>,
        collateral_asset: Option<Address>,
        debt_amount: i128,
    ) -> Result<flash_loan::FlashLoanLiquidationSim, LendingError> {
        flash_loan::simulate_flash_loan_liquidation(&env, debt_asset, collateral_asset, debt_amount)
            .map_err(Into::into)
    }

    /// Atomic flash loan + liquidation combo. Reverts when unprofitable.
    pub fn execute_flash_loan_liquidation(
        env: Env,
        liquidator: Address,
        borrower: Address,
        debt_asset: Option<Address>,
        collateral_asset: Option<Address>,
        debt_amount: i128,
    ) -> Result<flash_loan::FlashLoanLiquidationResult, LendingError> {
        flash_loan::execute_flash_loan_liquidation(
            &env,
            liquidator,
            borrower,
            debt_asset,
            collateral_asset,
            debt_amount,
        )
        .map_err(Into::into)
    }

    /// Multi-asset flash loan with a single callback and atomic repayment.
    pub fn execute_multi_asset_flash_loan(
        env: Env,
        user: Address,
        legs: Vec<flash_loan::FlashLoanLeg>,
        callback: Address,
    ) -> Result<i128, LendingError> {
        flash_loan::execute_multi_asset_flash_loan(&env, user, legs, callback).map_err(Into::into)
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

    // -------------------------------------------------------------------------
    // Debt Token Marketplace — Secondary Trading  (Issue #787)
    // -------------------------------------------------------------------------

    /// Mint a new debt-position NFT.
    pub fn dt_mint(
        env: Env,
        user: Address,
        collateral_asset: Option<Address>,
        principal: i128,
        interest_rate_bps: i128,
    ) -> Result<u64, LendingError> {
        use crate::debt_token::{mint_debt_token, DebtTokenError};
        mint_debt_token(&env, user, collateral_asset, principal, interest_rate_bps)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Direct transfer of a debt token.
    pub fn dt_transfer(
        env: Env,
        from: Address,
        to: Address,
        token_id: u64,
    ) -> Result<(), LendingError> {
        use crate::debt_token::{transfer_debt_token, DebtTokenError};
        transfer_debt_token(&env, from, to, token_id)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Burn a debt token (debt repaid/liquidated).
    pub fn dt_burn(
        env: Env,
        user: Address,
        token_id: u64,
        reason: soroban_sdk::Symbol,
    ) -> Result<(), LendingError> {
        use crate::debt_token::{burn_debt_token, DebtTokenError};
        burn_debt_token(&env, user, token_id, reason)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// List a debt token at a fixed price (with marketplace stats bookkeeping).
    pub fn dt_list(
        env: Env,
        seller: Address,
        token_id: u64,
        price: i128,
        payment_token: Address,
    ) -> Result<(), LendingError> {
        use crate::debt_token::list_debt_token_tracked;
        list_debt_token_tracked(&env, seller, token_id, price, payment_token)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Cancel an active fixed-price listing.
    pub fn dt_cancel_listing(
        env: Env,
        seller: Address,
        token_id: u64,
    ) -> Result<(), LendingError> {
        use crate::debt_token::{cancel_listing, DebtTokenError};
        cancel_listing(&env, seller, token_id)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Buy a listed debt token at its fixed asking price (with price discovery recording).
    pub fn dt_buy(
        env: Env,
        buyer: Address,
        token_id: u64,
    ) -> Result<(), LendingError> {
        use crate::debt_token::buy_listed_debt_token_tracked;
        buy_listed_debt_token_tracked(&env, buyer, token_id)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Place a bid (purchase offer) on a debt token.
    pub fn dt_place_bid(
        env: Env,
        bidder: Address,
        token_id: u64,
        price: i128,
        payment_token: Address,
        expires_at: u64,
    ) -> Result<(), LendingError> {
        use crate::debt_token::place_bid;
        place_bid(&env, bidder, token_id, price, payment_token, expires_at)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Cancel an active bid.
    pub fn dt_cancel_bid(
        env: Env,
        bidder: Address,
        token_id: u64,
    ) -> Result<(), LendingError> {
        use crate::debt_token::cancel_bid;
        cancel_bid(&env, bidder, token_id)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Accept a bidder's offer and transfer the token.
    pub fn dt_accept_bid(
        env: Env,
        seller: Address,
        token_id: u64,
        bidder: Address,
    ) -> Result<(), LendingError> {
        use crate::debt_token::accept_bid;
        accept_bid(&env, seller, token_id, bidder)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Read-only: get a specific bid.
    pub fn dt_get_bid(
        env: Env,
        token_id: u64,
        bidder: Address,
    ) -> Option<crate::debt_token::DebtTokenBid> {
        crate::debt_token::get_bid(&env, token_id, bidder)
    }

    /// Read-only: get all bidder addresses for a token.
    pub fn dt_get_bidders(env: Env, token_id: u64) -> Vec<Address> {
        crate::debt_token::get_bidders(&env, token_id)
    }

    /// Read-only: last traded price for a token.
    pub fn dt_get_last_trade_price(
        env: Env,
        token_id: u64,
    ) -> Option<crate::debt_token::TradePrice> {
        crate::debt_token::get_last_trade_price(&env, token_id)
    }

    /// Read-only: TWAP over the last 20 trades for a token.
    pub fn dt_get_twap(env: Env, token_id: u64) -> Option<i128> {
        crate::debt_token::get_twap_price(&env, token_id)
    }

    /// Read-only: global marketplace analytics snapshot.
    pub fn dt_get_marketplace_analytics(env: Env) -> crate::debt_token::MarketplaceStats {
        crate::debt_token::get_marketplace_analytics(&env)
    }

    /// Read-only: bounded log of recent trades across all tokens.
    pub fn dt_get_recent_trades(env: Env) -> Vec<crate::debt_token::TradeRecord> {
        crate::debt_token::get_recent_trades(&env)
    }

    /// Read-only: get the active listing for a token (if any).
    pub fn dt_get_listing(
        env: Env,
        token_id: u64,
    ) -> Option<crate::debt_token::DebtTokenListing> {
        crate::debt_token::get_listing(&env, token_id)
    }

    /// Read-only: get a user's debt token IDs.
    pub fn dt_get_user_tokens(env: Env, user: Address) -> Vec<u64> {
        crate::debt_token::get_user_debt_tokens(&env, &user)
    }

    /// Read-only: get a debt position.
    pub fn dt_get_position(
        env: Env,
        token_id: u64,
    ) -> Option<crate::debt_token::DebtPosition> {
        crate::debt_token::get_debt_position(&env, token_id)
    }

    /// Read-only: total supply of debt tokens.
    pub fn dt_total_supply(env: Env) -> u64 {
        crate::debt_token::get_total_supply(&env)
    }

    // -------------------------------------------------------------------------
    // Rate Limiter Administration  (Issue #790)
    // -------------------------------------------------------------------------

    /// Configure default rate-limit parameters for an operation (admin-only).
    pub fn rl_configure_operation(
        env: Env,
        caller: Address,
        op: soroban_sdk::Symbol,
        cfg: rate_limiter::RateLimitConfig,
    ) -> Result<(), LendingError> {
        rate_limiter::configure_operation_limit(&env, caller, op, cfg)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Configure per-pool rate-limit override for an operation (admin-only).
    pub fn rl_configure_pool(
        env: Env,
        caller: Address,
        op: soroban_sdk::Symbol,
        pool: Address,
        cfg: rate_limiter::RateLimitConfig,
    ) -> Result<(), LendingError> {
        rate_limiter::configure_pool_limit(&env, caller, op, pool, cfg)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Enable/disable grace burst for a (user, operation) pair (admin-only).
    pub fn rl_set_user_grace(
        env: Env,
        caller: Address,
        user: Address,
        op: soroban_sdk::Symbol,
        enabled: bool,
    ) -> Result<(), LendingError> {
        rate_limiter::set_user_grace(&env, caller, user, op, enabled)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Configure congestion-based adaptive throttling (admin-only).
    pub fn rl_configure_congestion(
        env: Env,
        caller: Address,
        cfg: rate_limiter::CongestionConfig,
    ) -> Result<(), LendingError> {
        rate_limiter::configure_congestion(&env, caller, cfg)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Report network congestion index in bps — callable by congestion_reporter role.
    pub fn rl_report_congestion(
        env: Env,
        caller: Address,
        congestion_bps: i128,
    ) -> Result<(), LendingError> {
        rate_limiter::report_congestion(&env, caller, congestion_bps)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Read-only: current congestion adaptation state (for dashboards).
    pub fn rl_get_congestion_state(env: Env) -> rate_limiter::CongestionState {
        rate_limiter::get_congestion_state(&env)
    }

    /// Read-only: current effective rate-limit status for a user.
    pub fn rl_get_user_status(
        env: Env,
        user: Address,
        op: soroban_sdk::Symbol,
        pool: Address,
    ) -> rate_limiter::RateLimitStatus {
        rate_limiter::get_user_status(&env, user, op, pool)
    }

    /// Read-only: current effective rate-limit status for the global pool bucket.
    pub fn rl_get_global_status(
        env: Env,
        op: soroban_sdk::Symbol,
        pool: Address,
    ) -> rate_limiter::RateLimitStatus {
        rate_limiter::get_global_status(&env, op, pool)
    }

    /// Read-only: aggregated analytics snapshot for an (op, pool) pair (Issue #790).
    pub fn rl_get_analytics(
        env: Env,
        op: soroban_sdk::Symbol,
        pool: Address,
    ) -> rate_limiter::RateLimitAnalytics {
        rate_limiter::get_rate_limit_analytics(&env, op, pool)
    }

    /// Admin-only: reset a user's rate-limit bucket to full capacity.
    pub fn rl_reset_user_bucket(
        env: Env,
        caller: Address,
        user: Address,
        op: soroban_sdk::Symbol,
        pool: Address,
    ) -> Result<(), LendingError> {
        rate_limiter::reset_user_bucket(&env, caller, user, op, pool)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Admin-only: reset the global-per-pool rate-limit bucket to full capacity.
    pub fn rl_reset_global_bucket(
        env: Env,
        caller: Address,
        op: soroban_sdk::Symbol,
        pool: Address,
    ) -> Result<(), LendingError> {
        rate_limiter::reset_global_bucket(&env, caller, op, pool)
            .map_err(|_| LendingError::Unauthorized)
    }

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

    /// Read-only: full real-time dashboard snapshot (Issue #795).
    pub fn get_dashboard_snapshot(
        env: Env,
    ) -> Result<analytics::DashboardSnapshot, LendingError> {
        analytics::get_dashboard_snapshot(&env).map_err(Into::into)
    }

    /// Read-only: risk-level distribution across sampled users (Issue #795).
    pub fn get_risk_distribution(env: Env) -> analytics::RiskDistributionSummary {
        analytics::get_risk_distribution(&env)
    }

    /// Read-only: protocol volume summary from activity log (Issue #795).
    pub fn get_volume_summary(env: Env) -> analytics::VolumeSummary {
        analytics::get_volume_summary(&env)
    }

    /// Record a historical metrics snapshot (callable by off-chain keeper).
    pub fn record_metrics_snapshot(
        env: Env,
    ) -> Result<analytics::MetricsSnapshot, LendingError> {
        analytics::record_metrics_snapshot(&env).map_err(Into::into)
    }

    /// Read-only: metrics snapshot history (oldest-first).
    pub fn get_metrics_history(env: Env) -> Vec<analytics::MetricsSnapshot> {
        analytics::get_metrics_history(&env)
    }

    /// Read-only: linear TVL forecast.
    pub fn forecast_tvl(env: Env, periods_ahead: u32) -> Result<i128, LendingError> {
        analytics::forecast_tvl(&env, periods_ahead).map_err(Into::into)
    }

    /// Admin-only: configure a metric alert threshold.
    pub fn set_metric_alert_threshold(
        env: Env,
        admin: Address,
        metric: soroban_sdk::Symbol,
        threshold: i128,
    ) -> Result<(), LendingError> {
        analytics::set_metric_alert_threshold(&env, admin, metric, threshold)
            .map_err(Into::into)
    }

    /// Read-only: all configured alert thresholds.
    pub fn get_metric_alert_thresholds(
        env: Env,
    ) -> Vec<analytics::MetricAlertThreshold> {
        analytics::get_metric_alert_thresholds(&env)
    }

    /// Read-only: triggered alert log.
    pub fn get_triggered_alerts(env: Env) -> Vec<analytics::TriggeredAlert> {
        analytics::get_triggered_alerts(&env)
    }

    /// Check current metrics against alert thresholds; returns breached metric names.
    pub fn check_metric_alerts(env: Env) -> Result<Vec<soroban_sdk::Symbol>, LendingError> {
        analytics::check_metric_alerts(&env).map_err(Into::into)
    }

    /// Record a collateral ratio snapshot for an asset.
    pub fn record_collateral_ratio_snapshot(
        env: Env,
        asset: soroban_sdk::Symbol,
        current_ratio: i128,
        required_ratio: i128,
        collateral_value: i128,
        debt_value: i128,
    ) -> Result<analytics::CollateralRatioSnapshot, LendingError> {
        analytics::record_collateral_ratio_snapshot(
            &env, asset, current_ratio, required_ratio, collateral_value, debt_value,
        )
        .map_err(Into::into)
    }

    /// Read-only: all current collateral ratio snapshots.
    pub fn get_collateral_ratio_snapshots(
        env: Env,
    ) -> Vec<analytics::CollateralRatioSnapshot> {
        analytics::get_collateral_ratio_snapshots(&env)
    }

    /// Read-only: historical collateral ratio trend for an asset.
    pub fn get_collateral_ratio_history(
        env: Env,
        asset: soroban_sdk::Symbol,
    ) -> Vec<analytics::CollateralRatioTrend> {
        analytics::get_collateral_ratio_history(&env, asset)
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

    pub fn set_asset_correlation(
        env: Env,
        asset_a: Option<Address>,
        asset_b: Option<Address>,
        correlation_bps: i128,
    ) -> Result<(), LendingError> {
        cross_asset::set_asset_correlation(&env, asset_a, asset_b, correlation_bps).map_err(Into::into)
    }

    pub fn get_asset_correlation(
        env: Env,
        asset_a: Option<Address>,
        asset_b: Option<Address>,
    ) -> i128 {
        cross_asset::get_asset_correlation(&env, asset_a, asset_b)
    }

    pub fn set_asset_volatility(
        env: Env,
        asset: Option<Address>,
        volatility_bps: i128,
    ) -> Result<(), LendingError> {
        cross_asset::set_asset_volatility(&env, asset, volatility_bps).map_err(Into::into)
    }

    pub fn get_dynamic_collateral_factor(
        env: Env,
        asset: Option<Address>,
    ) -> Result<i128, LendingError> {
        cross_asset::get_dynamic_collateral_factor(&env, asset).map_err(Into::into)
    }

    pub fn get_pair_liquidation_threshold(
        env: Env,
        debt_asset: Option<Address>,
        collateral_asset: Option<Address>,
    ) -> Result<i128, LendingError> {
        cross_asset::get_pair_liquidation_threshold(&env, debt_asset, collateral_asset)
            .map_err(Into::into)
    }

    pub fn get_unified_health_factor(
        env: Env,
        user: Address,
    ) -> Result<cross_asset::UserPositionSummary, LendingError> {
        cross_asset::get_unified_health_factor(&env, &user).map_err(Into::into)
    }

    pub fn detect_cross_asset_arbitrage(env: Env) -> Vec<cross_asset::ArbitrageOpportunity> {
        cross_asset::detect_cross_asset_arbitrage(&env)
    }

    pub fn get_portfolio_risk_score(env: Env, user: Address) -> Result<i128, LendingError> {
        analytics::portfolio_risk_score(&env, &user).map_err(Into::into)
    }

    // -------------------------------------------------------------------------
    // AMM-Lending Integration: LP Wrapping & Auto-Allocation
    // -------------------------------------------------------------------------

    /// Initialise the AMM-lending module (admin-only, once).
    pub fn amm_initialize(env: Env, admin: Address) -> Result<(), LendingError> {
        amm::initialize_amm_lending(&env, admin)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Wrap lending pool deposits into AMM LP positions.
    pub fn amm_wrap_deposit(
        env: Env,
        admin: Address,
        asset: Address,
        amount: i128,
        amm_protocol: Address,
    ) -> Result<amm::LpTokenPosition, LendingError> {
        amm::wrap_deposit_to_lp(&env, admin, asset, amount, amm_protocol)
            .map_err(|_| LendingError::InvalidAmount)
    }

    /// Unwrap LP tokens back into lending pool assets.
    pub fn amm_unwrap_deposit(
        env: Env,
        admin: Address,
        asset: Address,
        lp_tokens: i128,
    ) -> Result<i128, LendingError> {
        amm::unwrap_lp_to_deposit(&env, admin, asset, lp_tokens)
            .map_err(|_| LendingError::InvalidAmount)
    }

    /// Return the LP token balance for an asset.
    pub fn amm_get_lp_balance(env: Env, asset: Address) -> i128 {
        amm::get_lp_token_balance(&env, &asset)
    }

    /// Set the withdrawal buffer BPS for an asset (admin-only).
    pub fn amm_set_withdrawal_buffer(
        env: Env,
        admin: Address,
        asset: Address,
        buffer_bps: i128,
    ) -> Result<(), LendingError> {
        amm::set_withdrawal_buffer(&env, admin, asset, buffer_bps)
            .map_err(|_| LendingError::InvalidAmount)
    }

    /// Record accrued LP fees for an asset (admin-only).
    pub fn amm_record_lp_fees(
        env: Env,
        admin: Address,
        asset: Address,
        fee_amount: i128,
    ) -> Result<(), LendingError> {
        amm::record_lp_fees(&env, admin, asset, fee_amount)
            .map_err(|_| LendingError::InvalidAmount)
    }

    /// Auto-compound accrued LP fees back into the LP position for a single asset.
    ///
    /// Returns the total amount compounded (0 when nothing was accrued).
    pub fn amm_compound_lp_fees(
        env: Env,
        admin: Address,
        asset: Address,
    ) -> Result<i128, LendingError> {
        amm::compound_lp_fees(&env, admin, asset)
            .map_err(|_| LendingError::InvalidAmount)
    }

    /// Calculate optimal AMM allocation given current pool utilization.
    pub fn amm_calculate_optimal_allocation(
        env: Env,
        asset: Address,
        total_liquidity: i128,
        borrowed_amount: i128,
    ) -> Result<amm::AllocationSuggestion, LendingError> {
        amm::calculate_optimal_allocation(&env, &asset, total_liquidity, borrowed_amount)
            .map_err(|_| LendingError::InvalidAmount)
    }

    /// Execute automated AMM rebalancing based on pool utilization.
    pub fn amm_auto_rebalance(
        env: Env,
        admin: Address,
        asset: Address,
        total_liquidity: i128,
        borrowed_amount: i128,
        current_amm_balance: i128,
    ) -> Result<i128, LendingError> {
        amm::auto_rebalance_allocation(
            &env,
            admin,
            asset,
            total_liquidity,
            borrowed_amount,
            current_amm_balance,
        )
        .map_err(|_| LendingError::InvalidAmount)
    }

    /// Run the pool allocation optimizer across a set of pool addresses and
    /// return per-pool rebalancing recommendations.
    pub fn amm_optimize_allocation(
        env: Env,
        pools: Vec<Address>,
    ) -> Result<amm::OptimizationResult, LendingError> {
        amm::optimize_allocation(&env, &pools)
            .map_err(|_| LendingError::InvalidAmount)
    }

    /// Update the impermanent-loss tracking snapshot for an asset.
    /// Returns `true` when the IL alert threshold has been crossed.
    pub fn amm_update_il_tracking(
        env: Env,
        asset: Address,
        current_price: i128,
    ) -> Result<bool, LendingError> {
        amm::update_il_tracking(&env, &asset, current_price)
            .map_err(|_| LendingError::InvalidAmount)
    }

    /// Return the current IL snapshot for an asset, or `None` if not tracked.
    pub fn amm_get_il_snapshot(env: Env, asset: Address) -> Option<amm::IlSnapshot> {
        amm::get_il_snapshot(&env, &asset)
    }

    /// Update the utilization snapshot for a pool.
    pub fn amm_update_pool_utilization(env: Env, asset: Address, utilization_bps: i128) {
        amm::update_pool_utilization(&env, &asset, utilization_bps);
    }

    /// Return the current utilization snapshot for a pool.
    pub fn amm_get_pool_utilization(env: Env, asset: Address) -> i128 {
        amm::get_pool_utilization(&env, &asset)
    }

    // -------------------------------------------------------------------------
    // Yield Farming Strategy Optimizer (#789)
    // -------------------------------------------------------------------------

    /// Create a new named yield farming strategy for the admin.
    ///
    /// Returns the newly assigned `strategy_id`.
    pub fn yield_create_strategy(
        env: Env,
        admin: Address,
        name: String,
        objective: amm::YieldStrategyObjective,
        risk: amm::YieldStrategyRisk,
        compounding_interval: amm::CompoundingInterval,
        pools: Vec<Address>,
    ) -> Result<u64, LendingError> {
        amm::create_yield_strategy(&env, admin, name, objective, risk, compounding_interval, pools)
            .map_err(|_| LendingError::InvalidAmount)
    }

    /// Retrieve a previously created strategy by ID.
    pub fn yield_get_strategy(
        env: Env,
        admin: Address,
        strategy_id: u64,
    ) -> Option<amm::YieldStrategy> {
        amm::get_yield_strategy(&env, &admin, strategy_id)
    }

    /// Activate or deactivate a yield farming strategy (admin-only).
    pub fn yield_set_strategy_active(
        env: Env,
        admin: Address,
        strategy_id: u64,
        active: bool,
    ) -> Result<(), LendingError> {
        amm::set_yield_strategy_active(&env, admin, strategy_id, active)
            .map_err(|_| LendingError::Unauthorized)
    }

    /// Harvest and auto-compound accrued LP fees for every pool in a strategy.
    ///
    /// This is the primary on-chain entry point for the yield farming
    /// auto-compounding feature.  Returns the total amount compounded.
    pub fn yield_harvest_and_compound(
        env: Env,
        admin: Address,
        strategy_id: u64,
    ) -> Result<i128, LendingError> {
        amm::harvest_and_compound(&env, admin, strategy_id)
            .map_err(|_| LendingError::InvalidAmount)
    }

    /// Score a strategy based on current pool utilization and IL snapshots.
    ///
    /// Returns a `StrategyScore` with estimated APY, IL risk, and a composite
    /// ranking score so callers can compare strategies and pick the best one
    /// for the current market regime.
    pub fn yield_score_strategy(
        env: Env,
        admin: Address,
        strategy_id: u64,
    ) -> Result<amm::StrategyScore, LendingError> {
        amm::score_yield_strategy(&env, &admin, strategy_id)
            .map_err(|_| LendingError::InvalidAmount)
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
#[cfg(test)]
mod cross_asset_risk_test;
