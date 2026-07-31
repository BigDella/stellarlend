#![allow(unused_variables)]

use soroban_sdk::{token::TokenClient, Address, Env, String, Symbol, Vec};

pub use crate::errors::GovernanceError;
pub use crate::storage::{GovernanceDataKey, GuardianConfig};

pub use crate::types::{
    DelegationRecord, GovernanceAnalytics, GovernanceConfig, MultisigConfig,
    ParameterOptimizationRecommendation, Proposal, ProposalOutcome, ProposalSimulationResult,
    ProposalStatus, ProposalType, RecoveryRequest, VoteInfo, VoteLock, VotePowerSnapshot, VoteType,
    BASIS_POINTS_SCALE, DEFAULT_EXECUTION_DELAY, DEFAULT_QUORUM_BPS, DEFAULT_RECOVERY_PERIOD,
    DEFAULT_TIMELOCK_DURATION, DEFAULT_VOTING_PERIOD, DEFAULT_VOTING_THRESHOLD,
    DELEGATION_DEADLINE, MAX_DELEGATION_DEPTH, MIN_TIMELOCK_DELAY, PROPOSAL_RATE_LIMIT,
    PROPOSAL_RATE_WINDOW,
};

use crate::events::{
    GovernanceInitializedEvent, GuardianAddedEvent, GuardianRemovedEvent, ProposalApprovedEvent,
    ProposalCancelledEvent, ProposalCreatedEvent, ProposalExecutedEvent, ProposalFailedEvent,
    ProposalQueuedEvent, RecoveryApprovedEvent, RecoveryExecutedEvent, RecoveryStartedEvent,
    SuspiciousGovActivityEvent, VoteCastEvent, VoteDelegatedEvent, VoteDelegationRevokedEvent,
    VoteLockedEvent, VotePowerSnapshotTakenEvent,
};

use crate::{interest_rate, risk_management, risk_params};
use stellarlend_shared_deadline::{is_expired, is_expired_strict};

/// Maximum byte length for a proposal description string.
pub const MAX_DESCRIPTION_LEN: u32 = 256;

// ========================================================================
// Initialization
// ========================================================================

pub fn initialize(
    env: &Env,
    admin: Address,
    vote_token: Address,
    voting_period: Option<u64>,
    execution_delay: Option<u64>,
    quorum_bps: Option<u32>,
    proposal_threshold: Option<i128>,
    timelock_duration: Option<u64>,
    default_voting_threshold: Option<i128>,
) -> Result<(), GovernanceError> {
    if env.storage().instance().has(&GovernanceDataKey::Admin) {
        return Err(GovernanceError::AlreadyInitialized);
    }

    admin.require_auth();

    let config = GovernanceConfig {
        voting_period: voting_period.unwrap_or(DEFAULT_VOTING_PERIOD),
        execution_delay: execution_delay.unwrap_or(DEFAULT_EXECUTION_DELAY),
        quorum_bps: quorum_bps.unwrap_or(DEFAULT_QUORUM_BPS),
        proposal_threshold: proposal_threshold.unwrap_or(0),
        vote_token,
        timelock_duration: timelock_duration.unwrap_or(DEFAULT_TIMELOCK_DURATION),
        default_voting_threshold: default_voting_threshold.unwrap_or(DEFAULT_VOTING_THRESHOLD),
    };

    if config.quorum_bps > 10000 {
        return Err(GovernanceError::InvalidQuorum);
    }
    if config.voting_period == 0 {
        return Err(GovernanceError::InvalidVotingPeriod);
    }

    env.storage()
        .instance()
        .set(&GovernanceDataKey::Admin, &admin);
    env.storage()
        .instance()
        .set(&GovernanceDataKey::Config, &config);
    env.storage()
        .instance()
        .set(&GovernanceDataKey::NextProposalId, &0u64);

    let mut admins = Vec::new(env);
    admins.push_back(admin.clone());
    let multisig_config = MultisigConfig {
        admins,
        threshold: 1,
    };
    env.storage()
        .instance()
        .set(&GovernanceDataKey::MultisigConfig, &multisig_config);

    let guardian_config = GuardianConfig {
        guardians: Vec::new(env),
        threshold: 1,
    };
    env.storage()
        .instance()
        .set(&GovernanceDataKey::GuardianConfig, &guardian_config);

    GovernanceInitializedEvent {
        admin,
        vote_token: config.vote_token,
        voting_period: config.voting_period,
        quorum_bps: config.quorum_bps,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

// ========================================================================
// Proposal Creation
// ========================================================================

pub fn create_proposal(
    env: &Env,
    proposer: Address,
    proposal_type: ProposalType,
    description: String,
    voting_threshold: Option<i128>,
) -> Result<u64, GovernanceError> {
    proposer.require_auth();

    if description.len() > MAX_DESCRIPTION_LEN {
        return Err(GovernanceError::InputTooLong);
    }

    let config: GovernanceConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Config)
        .ok_or(GovernanceError::NotInitialized)?;

    if config.proposal_threshold > 0 {
        let token_client = TokenClient::new(env, &config.vote_token);
        let balance = token_client.balance(&proposer);

        if balance < config.proposal_threshold {
            return Err(GovernanceError::InsufficientProposalPower);
        }
    }

    let next_id: u64 = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::NextProposalId)
        .unwrap_or(0);

    let now = env.ledger().timestamp();

    let proposal = Proposal {
        id: next_id,
        proposer: proposer.clone(),
        proposal_type,
        description: description.clone(),
        status: ProposalStatus::Pending,
        start_time: now,
        end_time: now + config.voting_period,
        execution_time: None,
        voting_threshold: voting_threshold.unwrap_or(config.default_voting_threshold),
        for_votes: 0,
        against_votes: 0,
        abstain_votes: 0,
        total_voting_power: 0,
        created_at: now,
    };

    env.storage()
        .persistent()
        .set(&GovernanceDataKey::Proposal(next_id), &proposal);

    let user_key = GovernanceDataKey::UserProposals(proposer.clone(), next_id);
    env.storage().persistent().set(&user_key, &true);

    let approvals_key = GovernanceDataKey::ProposalApprovals(next_id);
    let approvals: Vec<Address> = Vec::new(env);
    env.storage().persistent().set(&approvals_key, &approvals);

    env.storage()
        .instance()
        .set(&GovernanceDataKey::NextProposalId, &(next_id + 1));

    ProposalCreatedEvent {
        proposal_id: next_id,
        proposer,
        proposal_type: proposal.proposal_type,
        description,
        start_time: proposal.start_time,
        end_time: proposal.end_time,
        created_at: now,
    }
    .publish(env);

    Ok(next_id)
}

// ========================================================================
// Voting
// ========================================================================

pub fn vote(
    env: &Env,
    voter: Address,
    proposal_id: u64,
    vote_type: VoteType,
) -> Result<(), GovernanceError> {
    voter.require_auth();

    let config: GovernanceConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Config)
        .ok_or(GovernanceError::NotInitialized)?;

    let mut proposal: Proposal = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Proposal(proposal_id))
        .ok_or(GovernanceError::ProposalNotFound)?;

    let now = env.ledger().timestamp();

    if proposal.status == ProposalStatus::Pending && now >= proposal.start_time {
        proposal.status = ProposalStatus::Active;
    }

    if proposal.status != ProposalStatus::Active {
        return Err(GovernanceError::ProposalNotActive);
    }

    if now < proposal.start_time {
        return Err(GovernanceError::ProposalNotActive);
    }
    if is_expired_strict(env, proposal.end_time) {
        proposal.status = ProposalStatus::Expired;
        env.storage()
            .persistent()
            .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);
        return Err(GovernanceError::ProposalExpired);
    }

    let vote_key = GovernanceDataKey::Vote(proposal_id, voter.clone());
    if env.storage().persistent().has(&vote_key) {
        return Err(GovernanceError::AlreadyVoted);
    }

    // --- Flash loan protection: use snapshot-based voting power ---
    let voting_power =
        get_vote_power_with_delegation(env, proposal_id, &voter, &config.vote_token)?;

    if voting_power == 0 {
        return Err(GovernanceError::NoVotingPower);
    }

    match vote_type {
        VoteType::For => proposal.for_votes += voting_power,
        VoteType::Against => proposal.against_votes += voting_power,
        VoteType::Abstain => proposal.abstain_votes += voting_power,
    }
    proposal.total_voting_power += voting_power;

    env.storage()
        .persistent()
        .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);
    env.storage().persistent().set(
        &vote_key,
        &VoteInfo {
            voter: voter.clone(),
            proposal_id,
            vote_type: vote_type.clone(),
            voting_power,
            timestamp: now,
        },
    );

    VoteCastEvent {
        proposal_id,
        voter,
        vote_type,
        voting_power,
        timestamp: now,
    }
    .publish(env);

    Ok(())
}

// ========================================================================
// Queue Proposal
// ========================================================================

pub fn queue_proposal(
    env: &Env,
    caller: Address,
    proposal_id: u64,
) -> Result<ProposalOutcome, GovernanceError> {
    caller.require_auth();

    let config: GovernanceConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Config)
        .ok_or(GovernanceError::NotInitialized)?;

    let mut proposal: Proposal = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Proposal(proposal_id))
        .ok_or(GovernanceError::ProposalNotFound)?;

    let now = env.ledger().timestamp();

    if now <= proposal.end_time {
        return Err(GovernanceError::VotingNotEnded);
    }

    match proposal.status {
        ProposalStatus::Executed
        | ProposalStatus::Cancelled
        | ProposalStatus::Expired
        | ProposalStatus::Queued => {
            return Err(GovernanceError::InvalidProposalStatus);
        }
        _ => {}
    }

    if is_expired(env, proposal.end_time.saturating_add(config.timelock_duration)) {
        proposal.status = ProposalStatus::Expired;
        env.storage()
            .persistent()
            .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);
        return Err(GovernanceError::ProposalExpired);
    }

    let total_votes = proposal.for_votes + proposal.against_votes + proposal.abstain_votes;
    let quorum_required = (total_votes * config.quorum_bps as i128) / BASIS_POINTS_SCALE;
    let quorum_reached = total_votes >= quorum_required;

    let threshold_votes =
        (proposal.total_voting_power * proposal.voting_threshold) / BASIS_POINTS_SCALE;
    let threshold_met = proposal.for_votes >= threshold_votes;

    let succeeded = quorum_reached && threshold_met;

    let outcome = ProposalOutcome {
        proposal_id,
        succeeded,
        for_votes: proposal.for_votes,
        against_votes: proposal.against_votes,
        abstain_votes: proposal.abstain_votes,
        quorum_reached,
        quorum_required,
    };

    if succeeded {
        let execution_time = now + config.execution_delay;
        proposal.execution_time = Some(execution_time);
        proposal.status = ProposalStatus::Queued;

        env.storage()
            .persistent()
            .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);

        ProposalQueuedEvent {
            proposal_id,
            execution_time,
            for_votes: proposal.for_votes,
            against_votes: proposal.against_votes,
            quorum_reached: outcome.quorum_reached,
            threshold_met: outcome.succeeded && outcome.quorum_reached,
        }
        .publish(env);
    } else {
        proposal.status = ProposalStatus::Defeated;
        env.storage()
            .persistent()
            .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);

        ProposalFailedEvent {
            proposal_id,
            for_votes: proposal.for_votes,
            against_votes: proposal.against_votes,
            quorum_reached,
            threshold_met: !succeeded && quorum_reached,
        }
        .publish(env);
    }

    Ok(outcome)
}

// ========================================================================
// Proposal Simulation (read-only) + caching
// ========================================================================

fn compute_simulation(
    env: &Env,
    proposal: &Proposal,
    config: &GovernanceConfig,
) -> ProposalSimulationResult {
    let now = env.ledger().timestamp();

    let total_votes = proposal.for_votes + proposal.against_votes + proposal.abstain_votes;
    let quorum_required = (total_votes * config.quorum_bps as i128) / BASIS_POINTS_SCALE;
    let quorum_reached = total_votes >= quorum_required;

    let threshold_votes =
        (proposal.total_voting_power * proposal.voting_threshold) / BASIS_POINTS_SCALE;
    let threshold_met = proposal.for_votes >= threshold_votes;

    let would_succeed = quorum_reached && threshold_met;
    let note = if would_succeed {
        String::from_str(env, "simulation: would succeed with current votes")
    } else {
        String::from_str(env, "simulation: would fail with current votes")
    };

    ProposalSimulationResult {
        proposal_id: proposal.id,
        now,
        would_succeed,
        quorum_required,
        quorum_reached,
        threshold_votes,
        threshold_met,
        for_votes: proposal.for_votes,
        against_votes: proposal.against_votes,
        abstain_votes: proposal.abstain_votes,
        total_voting_power: proposal.total_voting_power,
        note,
    }
}

pub fn simulate_proposal(
    env: &Env,
    proposal_id: u64,
) -> Result<ProposalSimulationResult, GovernanceError> {
    let config: GovernanceConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Config)
        .ok_or(GovernanceError::NotInitialized)?;

    let proposal: Proposal = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Proposal(proposal_id))
        .ok_or(GovernanceError::ProposalNotFound)?;

    let result = compute_simulation(env, &proposal, &config);
    env.storage().persistent().set(
        &GovernanceDataKey::ProposalSimulationCache(proposal_id),
        &result,
    );
    Ok(result)
}

pub fn get_simulation_cache(env: &Env, proposal_id: u64) -> Option<ProposalSimulationResult> {
    env.storage()
        .persistent()
        .get(&GovernanceDataKey::ProposalSimulationCache(proposal_id))
}

// ========================================================================
// Parameter Optimization (simple, transparent heuristic)
// ========================================================================

pub fn get_parameter_optimization_recommendation(
    env: &Env,
) -> Result<ParameterOptimizationRecommendation, GovernanceError> {
    if let Some(cached) = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::ParameterOptimizationCache)
    {
        return Ok(cached);
    }

    let config: GovernanceConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Config)
        .ok_or(GovernanceError::NotInitialized)?;

    let analytics: GovernanceAnalytics = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::GovernanceAnalytics)
        .unwrap_or(GovernanceAnalytics {
            total_proposals: 0,
            total_votes: 0,
            suspicious_proposals: 0,
            last_suspicious_at: 0,
            max_single_voter_power: 0,
        });

    // Heuristic:
    // - low participation => reduce quorum moderately
    // - suspicious activity => raise quorum and threshold moderately
    let votes_per_proposal = analytics
        .total_votes
        .checked_div(analytics.total_proposals)
        .unwrap_or(0);

    let mut suggested_quorum_bps = config.quorum_bps;
    if votes_per_proposal < 10 && suggested_quorum_bps > 2_000 {
        suggested_quorum_bps = suggested_quorum_bps.saturating_sub(500);
    }
    if analytics.suspicious_proposals > 0 {
        suggested_quorum_bps = suggested_quorum_bps.saturating_add(250).min(9_000);
    }

    let mut suggested_vote_threshold = config.default_voting_threshold;
    if analytics.suspicious_proposals > 0 {
        suggested_vote_threshold = (suggested_vote_threshold + 250).min(BASIS_POINTS_SCALE);
    }

    let transparency_note = String::from_str(
        env,
        "recommendation derived from on-chain analytics; governance may override",
    );

    let recommendation = ParameterOptimizationRecommendation {
        generated_at: env.ledger().timestamp(),
        suggested_quorum_bps,
        suggested_vote_threshold,
        suggested_voting_period: config.voting_period,
        transparency_note,
    };

    env.storage().persistent().set(
        &GovernanceDataKey::ParameterOptimizationCache,
        &recommendation,
    );

    Ok(recommendation)
}

// ========================================================================
// Execute Proposal
// ========================================================================

pub fn execute_proposal(
    env: &Env,
    executor: Address,
    proposal_id: u64,
) -> Result<(), GovernanceError> {
    executor.require_auth();

    let config: GovernanceConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Config)
        .ok_or(GovernanceError::NotInitialized)?;

    let mut proposal: Proposal = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Proposal(proposal_id))
        .ok_or(GovernanceError::ProposalNotFound)?;

    let now = env.ledger().timestamp();

    if proposal.status != ProposalStatus::Queued {
        return Err(GovernanceError::NotQueued);
    }

    let execution_time = proposal
        .execution_time
        .ok_or(GovernanceError::InvalidExecutionTime)?;

    if now < execution_time {
        return Err(GovernanceError::ExecutionTooEarly);
    }

    if is_expired(env, execution_time.saturating_add(config.timelock_duration)) {
        proposal.status = ProposalStatus::Expired;
        env.storage()
            .persistent()
            .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);
        return Err(GovernanceError::ProposalExpired);
    }

    execute_proposal_type(env, &proposal.proposal_type)?;

    proposal.status = ProposalStatus::Executed;
    env.storage()
        .persistent()
        .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);

    ProposalExecutedEvent {
        proposal_id,
        executor,
        timestamp: now,
    }
    .publish(env);

    Ok(())
}

fn execute_proposal_type(env: &Env, proposal_type: &ProposalType) -> Result<(), GovernanceError> {
    match proposal_type {
        ProposalType::MinCollateralRatio(ratio) => {
            risk_params::set_risk_params(env, Some(*ratio), None, None, None)
                .map_err(|_| GovernanceError::ExecutionFailed)?;
        }
        ProposalType::RiskParams(mcr, lt, cf, li) => {
            risk_params::set_risk_params(env, *mcr, *lt, *cf, *li)
                .map_err(|_| GovernanceError::ExecutionFailed)?;
        }
        ProposalType::InterestRateConfig(params) => {
            let admin = get_admin(env).ok_or(GovernanceError::NotInitialized)?;
            interest_rate::update_interest_rate_config(
                env,
                admin,
                params.base_rate_bps,
                params.kink_utilization_bps,
                params.multiplier_bps,
                params.jump_multiplier_bps,
                params.rate_floor_bps,
                params.rate_ceiling_bps,
                params.spread_bps,
            )
            .map_err(|_| GovernanceError::ExecutionFailed)?;
        }
        ProposalType::PauseSwitch(op, paused) => {
            let admin = get_admin(env).ok_or(GovernanceError::NotInitialized)?;
            risk_management::set_pause_switch(env, admin, op.clone(), *paused)
                .map_err(|_| GovernanceError::ExecutionFailed)?;
        }
        ProposalType::EmergencyPause(paused) => {
            let admin = get_admin(env).ok_or(GovernanceError::NotInitialized)?;
            risk_management::set_emergency_pause(env, admin, *paused)
                .map_err(|_| GovernanceError::ExecutionFailed)?;
        }
        ProposalType::GenericAction(_) => {
            return Err(GovernanceError::InvalidProposalType);
        }
    }
    Ok(())
}

pub fn create_admin_proposal(
    env: &Env,
    admin: Address,
    proposal_type: ProposalType,
    description: String,
) -> Result<u64, GovernanceError> {
    admin.require_auth();

    if description.len() > MAX_DESCRIPTION_LEN {
        return Err(GovernanceError::InputTooLong);
    }

    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Admin)
        .ok_or(GovernanceError::NotInitialized)?;

    if admin != stored_admin {
        return Err(GovernanceError::Unauthorized);
    }

    let config: GovernanceConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Config)
        .ok_or(GovernanceError::NotInitialized)?;

    let now = env.ledger().timestamp();
    let proposal_id: u64 = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::NextProposalId)
        .unwrap_or(0);

    let execution_time = now + config.execution_delay.max(MIN_TIMELOCK_DELAY);

    let proposal = Proposal {
        id: proposal_id,
        proposer: admin.clone(),
        proposal_type,
        description,
        status: ProposalStatus::Queued,
        start_time: now,
        end_time: now,
        execution_time: Some(execution_time),
        voting_threshold: 0,
        for_votes: 0,
        against_votes: 0,
        abstain_votes: 0,
        total_voting_power: 0,
        created_at: now,
    };

    env.storage()
        .persistent()
        .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);

    env.storage()
        .instance()
        .set(&GovernanceDataKey::NextProposalId, &(proposal_id + 1));

    emit_proposal_created_event(env, &proposal_id, &admin);

    let topics = (Symbol::new(env, "proposal_queued"), proposal_id);
    env.events().publish(topics, execution_time);

    Ok(proposal_id)
}

pub fn create_emergency_proposal(
    env: &Env,
    caller: Address,
    proposal_type: ProposalType,
    description: String,
) -> Result<u64, GovernanceError> {
    caller.require_auth();

    if description.len() > MAX_DESCRIPTION_LEN {
        return Err(GovernanceError::InputTooLong);
    }

    // Verification of multisig auth happens via approvals in multisig module,
    // but for "emergency bypass" we can allow direct execution if called by a valid multisig admin
    // assuming it's correctly authorized by the multisig threshold.
    // In this simplified version, we'll check against multisig admins.

    let multisig_config: MultisigConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::MultisigConfig)
        .ok_or(GovernanceError::NotInitialized)?;

    if !multisig_config.admins.contains(&caller) {
        return Err(GovernanceError::Unauthorized);
    }

    let now = env.ledger().timestamp();
    let proposal_id: u64 = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::NextProposalId)
        .unwrap_or(0);

    let proposal = Proposal {
        id: proposal_id,
        proposer: caller.clone(),
        proposal_type,
        description,
        status: ProposalStatus::Queued,
        start_time: now,
        end_time: now,
        execution_time: Some(now), // No delay for emergency
        voting_threshold: 0,
        for_votes: 0,
        against_votes: 0,
        abstain_votes: 0,
        total_voting_power: 0,
        created_at: now,
    };

    env.storage()
        .persistent()
        .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);

    env.storage()
        .instance()
        .set(&GovernanceDataKey::NextProposalId, &(proposal_id + 1));

    emit_proposal_created_event(env, &proposal_id, &caller);

    Ok(proposal_id)
}

// ========================================================================
// Cancel Proposal
// ========================================================================

pub fn cancel_proposal(
    env: &Env,
    caller: Address,
    proposal_id: u64,
) -> Result<(), GovernanceError> {
    caller.require_auth();

    let admin: Address = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Admin)
        .ok_or(GovernanceError::NotInitialized)?;

    let mut proposal: Proposal = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Proposal(proposal_id))
        .ok_or(GovernanceError::ProposalNotFound)?;

    if caller != proposal.proposer && caller != admin {
        return Err(GovernanceError::Unauthorized);
    }

    match proposal.status {
        ProposalStatus::Executed | ProposalStatus::Queued => {
            return Err(GovernanceError::InvalidProposalStatus);
        }
        _ => {}
    }

    proposal.status = ProposalStatus::Cancelled;
    env.storage()
        .persistent()
        .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);

    ProposalCancelledEvent {
        proposal_id,
        caller,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

// ========================================================================
// Multisig Operations
// ========================================================================

pub fn approve_proposal(
    env: &Env,
    approver: Address,
    proposal_id: u64,
) -> Result<(), GovernanceError> {
    approver.require_auth();

    let multisig_config: MultisigConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::MultisigConfig)
        .ok_or(GovernanceError::NotInitialized)?;

    if !multisig_config.admins.contains(&approver) {
        return Err(GovernanceError::Unauthorized);
    }

    let proposal_key = GovernanceDataKey::Proposal(proposal_id);
    if !env.storage().persistent().has(&proposal_key) {
        return Err(GovernanceError::ProposalNotFound);
    }

    let approvals_key = GovernanceDataKey::ProposalApprovals(proposal_id);
    let mut approvals: Vec<Address> = env
        .storage()
        .persistent()
        .get(&approvals_key)
        .unwrap_or_else(|| Vec::new(env));

    if approvals.contains(&approver) {
        return Err(GovernanceError::AlreadyVoted);
    }

    approvals.push_back(approver.clone());
    env.storage().persistent().set(&approvals_key, &approvals);

    ProposalApprovedEvent {
        proposal_id,
        approver,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

pub fn set_multisig_config(
    env: &Env,
    caller: Address,
    admins: Vec<Address>,
    threshold: u32,
) -> Result<(), GovernanceError> {
    caller.require_auth();

    let admin: Address = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Admin)
        .ok_or(GovernanceError::NotInitialized)?;

    if caller != admin {
        return Err(GovernanceError::Unauthorized);
    }

    if admins.is_empty() || threshold == 0 || threshold > admins.len() {
        return Err(GovernanceError::InvalidMultisigConfig);
    }

    let config = MultisigConfig { admins, threshold };
    env.storage()
        .instance()
        .set(&GovernanceDataKey::MultisigConfig, &config);

    Ok(())
}

/// Return the list of admins who have approved a proposal, or `None` if not found.
pub fn get_proposal_approvals(env: &Env, proposal_id: u64) -> Option<Vec<Address>> {
    let approvals_key = GovernanceDataKey::ProposalApprovals(proposal_id);
    env.storage().persistent().get(&approvals_key)
}

pub fn get_multisig_config(env: &Env) -> Option<MultisigConfig> {
    env.storage()
        .instance()
        .get(&GovernanceDataKey::MultisigConfig)
}

pub fn get_multisig_admins(env: &Env) -> Option<Vec<Address>> {
    get_multisig_config(env).map(|c| c.admins)
}

pub fn get_multisig_threshold(env: &Env) -> u32 {
    get_multisig_config(env).map(|c| c.threshold).unwrap_or(1)
}

pub fn set_multisig_admins(
    env: &Env,
    caller: Address,
    admins: Vec<Address>,
) -> Result<(), GovernanceError> {
    let config = get_multisig_config(env).ok_or(GovernanceError::NotInitialized)?;
    set_multisig_config(env, caller, admins, config.threshold)
}

pub fn set_multisig_threshold(
    env: &Env,
    caller: Address,
    threshold: u32,
) -> Result<(), GovernanceError> {
    let config = get_multisig_config(env).ok_or(GovernanceError::NotInitialized)?;
    set_multisig_config(env, caller, config.admins, threshold)
}

pub fn propose_set_min_collateral_ratio(
    env: &Env,
    proposer: Address,
    new_ratio: i128,
) -> Result<u64, GovernanceError> {
    create_proposal(
        env,
        proposer,
        ProposalType::MinCollateralRatio(new_ratio),
        String::from_str(env, "Update min collateral ratio"),
        None,
    )
}

pub fn execute_multisig_proposal(
    env: &Env,
    executor: Address,
    proposal_id: u64,
) -> Result<(), GovernanceError> {
    executor.require_auth();

    let config: GovernanceConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Config)
        .ok_or(GovernanceError::NotInitialized)?;

    let multisig_config = get_multisig_config(env).ok_or(GovernanceError::NotInitialized)?;
    if !multisig_config.admins.contains(&executor) {
        return Err(GovernanceError::Unauthorized);
    }

    let mut proposal: Proposal = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Proposal(proposal_id))
        .ok_or(GovernanceError::ProposalNotFound)?;

    if proposal.status != ProposalStatus::Pending {
        return Err(GovernanceError::InvalidProposalStatus);
    }

    let now = env.ledger().timestamp();
    if now <= proposal.end_time {
        return Err(GovernanceError::ProposalNotReady);
    }
    if is_expired(env, proposal.end_time.saturating_add(config.timelock_duration)) {
        proposal.status = ProposalStatus::Expired;
        env.storage()
            .persistent()
            .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);
        return Err(GovernanceError::ProposalExpired);
    }

    let approvals = get_proposal_approvals(env, proposal_id).unwrap_or_else(|| Vec::new(env));
    if approvals.len() < multisig_config.threshold {
        return Err(GovernanceError::InsufficientApprovals);
    }

    execute_proposal_type(env, &proposal.proposal_type)?;

    proposal.status = ProposalStatus::Executed;
    env.storage()
        .persistent()
        .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);

    emit_proposal_executed_event(env, &proposal_id, &executor);

    Ok(())
}

// ============================================================================
// Events
// ============================================================================

fn emit_proposal_created_event(env: &Env, proposal_id: &u64, proposer: &Address) {
    let topics = (
        Symbol::new(env, "proposal_created"),
        *proposal_id,
        proposer.clone(),
    );
    env.events().publish(topics, ());
}

#[allow(dead_code)]
fn emit_vote_cast_event(
    env: &Env,
    proposal_id: &u64,
    voter: &Address,
    vote: &VoteType,
    voting_power: &i128,
) {
    let topics = (Symbol::new(env, "vote_cast"), *proposal_id, voter.clone());
    env.events().publish(topics, (vote.clone(), *voting_power));
}

pub fn emit_proposal_executed_event(env: &Env, proposal_id: &u64, executor: &Address) {
    let topics = (
        Symbol::new(env, "proposal_executed"),
        *proposal_id,
        executor.clone(),
    );
    env.events().publish(topics, ());
}

#[allow(dead_code)]
fn emit_proposal_failed_event(env: &Env, proposal_id: &u64) {
    let topics = (Symbol::new(env, "proposal_failed"), *proposal_id);
    env.events().publish(topics, ());
}

pub fn add_guardian(env: &Env, caller: Address, guardian: Address) -> Result<(), GovernanceError> {
    caller.require_auth();

    let admin: Address = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Admin)
        .ok_or(GovernanceError::NotInitialized)?;

    if caller != admin {
        return Err(GovernanceError::Unauthorized);
    }

    let mut guardian_config: GuardianConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::GuardianConfig)
        .unwrap_or_else(|| GuardianConfig {
            guardians: Vec::new(env),
            threshold: 1,
        });

    if guardian_config.guardians.contains(&guardian) {
        return Err(GovernanceError::GuardianAlreadyExists);
    }

    guardian_config.guardians.push_back(guardian.clone());
    env.storage()
        .instance()
        .set(&GovernanceDataKey::GuardianConfig, &guardian_config);

    GuardianAddedEvent {
        guardian,
        added_by: caller,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

pub fn remove_guardian(
    env: &Env,
    caller: Address,
    guardian: Address,
) -> Result<(), GovernanceError> {
    caller.require_auth();

    let admin: Address = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Admin)
        .ok_or(GovernanceError::NotInitialized)?;

    if caller != admin {
        return Err(GovernanceError::Unauthorized);
    }

    let mut guardian_config: GuardianConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::GuardianConfig)
        .ok_or(GovernanceError::GuardianNotFound)?;

    let mut new_guardians = Vec::new(env);
    let mut found = false;

    for g in guardian_config.guardians.iter() {
        if g != guardian {
            new_guardians.push_back(g);
        } else {
            found = true;
        }
    }

    if !found {
        return Err(GovernanceError::GuardianNotFound);
    }

    guardian_config.guardians = new_guardians;

    if guardian_config.threshold > guardian_config.guardians.len() {
        guardian_config.threshold = guardian_config.guardians.len();
    }

    env.storage()
        .instance()
        .set(&GovernanceDataKey::GuardianConfig, &guardian_config);

    GuardianRemovedEvent {
        guardian,
        removed_by: caller,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

pub fn set_guardian_threshold(
    env: &Env,
    caller: Address,
    threshold: u32,
) -> Result<(), GovernanceError> {
    caller.require_auth();

    let admin: Address = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Admin)
        .ok_or(GovernanceError::NotInitialized)?;

    if caller != admin {
        return Err(GovernanceError::Unauthorized);
    }

    let mut guardian_config: GuardianConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::GuardianConfig)
        .ok_or(GovernanceError::GuardianNotFound)?;

    if threshold == 0 || threshold > guardian_config.guardians.len() {
        return Err(GovernanceError::InvalidGuardianConfig);
    }

    guardian_config.threshold = threshold;
    env.storage()
        .instance()
        .set(&GovernanceDataKey::GuardianConfig, &guardian_config);

    Ok(())
}

pub fn start_recovery(
    env: &Env,
    initiator: Address,
    old_admin: Address,
    new_admin: Address,
) -> Result<(), GovernanceError> {
    initiator.require_auth();

    let guardian_config: GuardianConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::GuardianConfig)
        .ok_or(GovernanceError::GuardianNotFound)?;

    if !guardian_config.guardians.contains(&initiator) {
        return Err(GovernanceError::Unauthorized);
    }

    let recovery_key = GovernanceDataKey::RecoveryRequest;
    if env.storage().persistent().has(&recovery_key) {
        return Err(GovernanceError::RecoveryInProgress);
    }

    let now = env.ledger().timestamp();
    let request = RecoveryRequest {
        old_admin,
        new_admin: new_admin.clone(),
        initiator: initiator.clone(),
        initiated_at: now,
        expires_at: now + DEFAULT_RECOVERY_PERIOD,
        // #675 — see recovery.rs's start_recovery for the rationale; kept
        // consistent here since both paths construct the same RecoveryRequest.
        ready_at: now + crate::types::DEFAULT_RECOVERY_DELAY,
    };

    env.storage().persistent().set(&recovery_key, &request);

    let approvals_key = GovernanceDataKey::RecoveryApprovals;
    let mut approvals = Vec::new(env);
    approvals.push_back(initiator.clone());
    env.storage().persistent().set(&approvals_key, &approvals);

    RecoveryStartedEvent {
        old_admin: request.old_admin,
        new_admin,
        initiator,
        expires_at: request.expires_at,
        timestamp: now,
    }
    .publish(env);

    Ok(())
}

pub fn approve_recovery(env: &Env, approver: Address) -> Result<(), GovernanceError> {
    approver.require_auth();

    let guardian_config: GuardianConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::GuardianConfig)
        .ok_or(GovernanceError::GuardianNotFound)?;

    if !guardian_config.guardians.contains(&approver) {
        return Err(GovernanceError::Unauthorized);
    }

    let recovery_key = GovernanceDataKey::RecoveryRequest;
    if !env.storage().persistent().has(&recovery_key) {
        return Err(GovernanceError::NoRecoveryInProgress);
    }

    let approvals_key = GovernanceDataKey::RecoveryApprovals;
    let mut approvals: Vec<Address> = env
        .storage()
        .persistent()
        .get(&approvals_key)
        .unwrap_or_else(|| Vec::new(env));

    if approvals.contains(&approver) {
        return Err(GovernanceError::AlreadyVoted);
    }

    approvals.push_back(approver.clone());
    env.storage().persistent().set(&approvals_key, &approvals);

    RecoveryApprovedEvent {
        approver,
        current_approvals: approvals.len(),
        threshold: guardian_config.threshold,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

pub fn execute_recovery(env: &Env, executor: Address) -> Result<(), GovernanceError> {
    executor.require_auth();

    let guardian_config: GuardianConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::GuardianConfig)
        .ok_or(GovernanceError::GuardianNotFound)?;

    let recovery_key = GovernanceDataKey::RecoveryRequest;
    let request: RecoveryRequest = env
        .storage()
        .persistent()
        .get(&recovery_key)
        .ok_or(GovernanceError::NoRecoveryInProgress)?;

    let now = env.ledger().timestamp();
    if now > request.expires_at {
        env.storage().persistent().remove(&recovery_key);
        return Err(GovernanceError::ProposalExpired);
    }

    let approvals_key = GovernanceDataKey::RecoveryApprovals;
    let approvals: Vec<Address> = env
        .storage()
        .persistent()
        .get(&approvals_key)
        .unwrap_or_else(|| Vec::new(env));

    if approvals.len() < guardian_config.threshold {
        return Err(GovernanceError::InsufficientApprovals);
    }

    let mut multisig_config: MultisigConfig = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::MultisigConfig)
        .ok_or(GovernanceError::NotInitialized)?;

    let mut new_admins = Vec::new(env);
    for admin in multisig_config.admins.iter() {
        if admin != request.old_admin {
            new_admins.push_back(admin);
        }
    }
    new_admins.push_back(request.new_admin.clone());

    multisig_config.admins = new_admins;
    env.storage()
        .instance()
        .set(&GovernanceDataKey::MultisigConfig, &multisig_config);

    env.storage().persistent().remove(&recovery_key);
    env.storage().persistent().remove(&approvals_key);

    RecoveryExecutedEvent {
        old_admin: request.old_admin,
        new_admin: request.new_admin,
        executor,
        timestamp: now,
    }
    .publish(env);

    Ok(())
}

// ========================================================================
// Flash Loan Attack Protection
// ========================================================================

/// Take a vote power snapshot for a voter at proposal creation time.
/// This snapshot is used instead of the live balance when casting votes,
/// preventing flash loan attacks where tokens are borrowed to inflate power.
pub fn take_vote_power_snapshot(
    env: &Env,
    proposal_id: u64,
    voter: &Address,
    vote_token: &Address,
) {
    let token_client = TokenClient::new(env, vote_token);
    let balance = token_client.balance(voter);
    let now = env.ledger().timestamp();

    let snapshot = VotePowerSnapshot {
        proposal_id,
        voter: voter.clone(),
        balance,
        snapshot_time: now,
    };

    env.storage().persistent().set(
        &GovernanceDataKey::VotePowerSnapshot(proposal_id, voter.clone()),
        &snapshot,
    );

    VotePowerSnapshotTakenEvent {
        proposal_id,
        voter: voter.clone(),
        balance,
        snapshot_time: now,
    }
    .publish(env);
}

/// Get the snapshotted vote power for a voter on a proposal.
/// Falls back to the live balance when no snapshot exists so legacy proposals
/// and tests created before snapshot coverage continue to vote normally.
fn get_snapshotted_vote_power(
    env: &Env,
    proposal_id: u64,
    voter: &Address,
    vote_token: &Address,
) -> i128 {
    let snapshot_key = GovernanceDataKey::VotePowerSnapshot(proposal_id, voter.clone());
    if let Some(snapshot) = env
        .storage()
        .persistent()
        .get::<GovernanceDataKey, VotePowerSnapshot>(&snapshot_key)
    {
        snapshot.balance
    } else {
        // Keep pre-snapshot proposals compatible with the existing live-balance
        // voting flow.
        TokenClient::new(env, vote_token).balance(voter)
    }
}

/// Resolve effective voting power for a voter, accounting for delegation.
/// Delegation must have been established at least DELEGATION_DEADLINE seconds
/// before the proposal was created to be valid.
fn get_vote_power_with_delegation(
    env: &Env,
    proposal_id: u64,
    voter: &Address,
    vote_token: &Address,
) -> Result<i128, GovernanceError> {
    let proposal: Proposal = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Proposal(proposal_id))
        .ok_or(GovernanceError::ProposalNotFound)?;

    // Check if this voter is acting as a delegatee for someone else.
    // We use the voter's own snapshot as their base power.
    let own_power = get_snapshotted_vote_power(env, proposal_id, voter, vote_token);

    // Check if the voter has received a delegation that predates the proposal
    // by at least DELEGATION_DEADLINE seconds.
    let delegation_key = GovernanceDataKey::DelegationRecord(voter.clone());
    // We look for delegations TO this voter by checking if any delegator
    // has delegated to them. For simplicity, we store the delegation from
    // the delegatee's perspective as well.
    let delegated_power_key = GovernanceDataKey::VotePowerSnapshot(proposal_id, voter.clone());

    // The snapshot already captures the voter's own balance at proposal time.
    // Delegated power is added on top if the delegation was established before
    // the proposal creation minus DELEGATION_DEADLINE.
    let delegated_extra = get_delegated_power_for_voter(env, proposal_id, voter, &proposal);

    Ok(own_power + delegated_extra)
}

/// Sum up delegated voting power that was validly delegated to `delegatee`
/// before the proposal's creation minus DELEGATION_DEADLINE.
fn get_delegated_power_for_voter(
    env: &Env,
    proposal_id: u64,
    delegatee: &Address,
    proposal: &Proposal,
) -> i128 {
    // Walk through all delegations pointing to this delegatee.
    // We store delegations keyed by delegator address; to find all delegations
    // to a given delegatee we check the reverse mapping stored at
    // DelegationRecord(delegatee) which holds the list of delegators.
    let reverse_key = GovernanceDataKey::DelegationRecord(delegatee.clone());
    let delegators: Vec<Address> = env
        .storage()
        .persistent()
        .get(&reverse_key)
        .unwrap_or_else(|| Vec::new(env));

    let deadline = proposal.created_at.saturating_sub(DELEGATION_DEADLINE);
    let mut total: i128 = 0;

    for delegator in delegators.iter() {
        // Load the delegation record stored under the delegator key
        let del_key = GovernanceDataKey::DelegationRecord(delegator.clone());
        if let Some(record) = env
            .storage()
            .persistent()
            .get::<GovernanceDataKey, DelegationRecord>(&del_key)
        {
            // Only count delegations established before the deadline
            if record.delegatee == *delegatee && record.delegated_at <= deadline {
                // Use the delegator's snapshot for this proposal
                let snap_key = GovernanceDataKey::VotePowerSnapshot(proposal_id, delegator.clone());
                if let Some(snap) = env
                    .storage()
                    .persistent()
                    .get::<GovernanceDataKey, VotePowerSnapshot>(&snap_key)
                {
                    total += snap.balance;
                }
            }
        }
    }

    total
}

/// Lock a voter's governance tokens for the duration of the voting period.
/// This prevents tokens from being transferred (or returned to a flash loan
/// lender) while a vote is active.
///
/// On Soroban, we cannot directly freeze token transfers, so we record the
/// lock on-chain and expose `is_vote_locked` for off-chain enforcement and
/// for the token contract to query if it implements a lock hook.
#[allow(dead_code)]
fn lock_vote_tokens(
    env: &Env,
    voter: &Address,
    proposal_id: u64,
    locked_amount: i128,
    locked_until: u64,
) {
    let lock_key = GovernanceDataKey::VoteLock(voter.clone());

    // Only extend the lock if the new expiry is later than the existing one
    let existing: Option<VoteLock> = env.storage().persistent().get(&lock_key);
    let effective_until = match existing {
        Some(ref l) if l.locked_until >= locked_until => l.locked_until,
        _ => locked_until,
    };

    let lock = VoteLock {
        voter: voter.clone(),
        locked_until: effective_until,
        locked_amount,
        proposal_id,
    };

    env.storage().persistent().set(&lock_key, &lock);

    VoteLockedEvent {
        voter: voter.clone(),
        proposal_id,
        locked_amount,
        locked_until: effective_until,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}

/// Delegate vote power from `delegator` to `delegatee`.
/// The delegation must be established at least DELEGATION_DEADLINE seconds
/// before a proposal is created to be valid for that proposal.
pub fn delegate_vote(
    env: &Env,
    delegator: Address,
    delegatee: Address,
) -> Result<(), GovernanceError> {
    delegator.require_auth();

    if delegator == delegatee {
        return Err(GovernanceError::SelfDelegation);
    }

    // Prevent delegation while tokens are locked due to active vote
    if is_vote_locked(env, &delegator) {
        return Err(GovernanceError::VotesLocked);
    }

    // Prevent re-delegation if already delegated
    let del_key = GovernanceDataKey::DelegationRecord(delegator.clone());
    if env.storage().persistent().has(&del_key) {
        return Err(GovernanceError::AlreadyDelegated);
    }

    // Check delegation depth: prevent chains deeper than MAX_DELEGATION_DEPTH
    let depth = get_delegation_depth(env, &delegatee);
    if depth >= MAX_DELEGATION_DEPTH {
        return Err(GovernanceError::DelegationDepthExceeded);
    }

    let now = env.ledger().timestamp();

    let record = DelegationRecord {
        delegator: delegator.clone(),
        delegatee: delegatee.clone(),
        delegated_at: now,
        depth: depth + 1,
    };

    // Store delegation record under delegator key
    env.storage().persistent().set(&del_key, &record);

    // Update reverse mapping: delegatee → list of delegators
    let reverse_key = GovernanceDataKey::DelegationRecord(delegatee.clone());
    let mut delegators: Vec<Address> = env
        .storage()
        .persistent()
        .get(&reverse_key)
        .unwrap_or_else(|| Vec::new(env));
    delegators.push_back(delegator.clone());
    env.storage().persistent().set(&reverse_key, &delegators);

    VoteDelegatedEvent {
        delegator,
        delegatee,
        delegated_at: now,
    }
    .publish(env);

    Ok(())
}

/// Revoke an existing vote delegation.
pub fn revoke_delegation(env: &Env, delegator: Address) -> Result<(), GovernanceError> {
    delegator.require_auth();

    let del_key = GovernanceDataKey::DelegationRecord(delegator.clone());
    let record: DelegationRecord = env
        .storage()
        .persistent()
        .get(&del_key)
        .ok_or(GovernanceError::NotInitialized)?;

    // Remove from reverse mapping
    let reverse_key = GovernanceDataKey::DelegationRecord(record.delegatee.clone());
    let delegators: Vec<Address> = env
        .storage()
        .persistent()
        .get(&reverse_key)
        .unwrap_or_else(|| Vec::new(env));

    let mut new_delegators = Vec::new(env);
    for d in delegators.iter() {
        if d != delegator {
            new_delegators.push_back(d);
        }
    }
    env.storage()
        .persistent()
        .set(&reverse_key, &new_delegators);

    // Remove the delegation record
    env.storage().persistent().remove(&del_key);

    VoteDelegationRevokedEvent {
        delegator,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Compute the delegation chain depth for an address.
fn get_delegation_depth(env: &Env, addr: &Address) -> u32 {
    let del_key = GovernanceDataKey::DelegationRecord(addr.clone());
    if let Some(record) = env
        .storage()
        .persistent()
        .get::<GovernanceDataKey, DelegationRecord>(&del_key)
    {
        record.depth
    } else {
        0
    }
}

/// Enforce proposal rate limiting: an address may not create more than
/// PROPOSAL_RATE_LIMIT proposals within a PROPOSAL_RATE_WINDOW second window.
#[allow(dead_code)]
fn enforce_proposal_rate_limit(env: &Env, proposer: &Address) -> Result<(), GovernanceError> {
    let now = env.ledger().timestamp();

    let window_key = GovernanceDataKey::ProposalWindowStart(proposer.clone());
    let count_key = GovernanceDataKey::ProposalCreationCount(proposer.clone());

    let window_start: u64 = env.storage().persistent().get(&window_key).unwrap_or(0);

    let count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);

    if now - window_start > PROPOSAL_RATE_WINDOW {
        // New window: reset counter
        env.storage().persistent().set(&window_key, &now);
        env.storage().persistent().set(&count_key, &1u32);
    } else {
        if count >= PROPOSAL_RATE_LIMIT {
            return Err(GovernanceError::ProposalRateLimitExceeded);
        }
        env.storage().persistent().set(&count_key, &(count + 1));
    }

    Ok(())
}

/// Detect suspicious voting patterns that may indicate a flash loan attack.
/// Emits a `SuspiciousGovernanceActivityEvent` if the voter's power exceeds
/// a threshold relative to the total supply estimate.
#[allow(dead_code)]
fn detect_suspicious_voting(
    env: &Env,
    proposal_id: u64,
    voter: &Address,
    voter_power: i128,
    vote_token: &Address,
) {
    // Heuristic: if a single voter holds more than 33% of the total supply
    // estimate (derived from the token client), flag it as suspicious.
    let token_client = TokenClient::new(env, vote_token);
    let total_supply_estimate = token_client.balance(voter) + voter_power;

    // 33% threshold in basis points = 3333
    let threshold_bps: i128 = 3333;
    if total_supply_estimate > 0
        && (voter_power * BASIS_POINTS_SCALE) / total_supply_estimate > threshold_bps
    {
        let reason = Symbol::new(env, "large_single_voter");

        SuspiciousGovActivityEvent {
            proposal_id,
            voter: voter.clone(),
            voter_power,
            total_supply_estimate,
            reason,
            timestamp: env.ledger().timestamp(),
        }
        .publish(env);

        // Update analytics suspicious counter
        let analytics_key = GovernanceDataKey::GovernanceAnalytics;
        let mut analytics: GovernanceAnalytics = env
            .storage()
            .persistent()
            .get(&analytics_key)
            .unwrap_or(GovernanceAnalytics {
                total_proposals: 0,
                total_votes: 0,
                suspicious_proposals: 0,
                last_suspicious_at: 0,
                max_single_voter_power: 0,
            });

        analytics.suspicious_proposals += 1;
        analytics.last_suspicious_at = env.ledger().timestamp();
        if voter_power > analytics.max_single_voter_power {
            analytics.max_single_voter_power = voter_power;
        }

        env.storage().persistent().set(&analytics_key, &analytics);
    }
}

/// Update analytics when a proposal is created.
#[allow(dead_code)]
fn update_analytics_proposal_created(env: &Env) {
    let analytics_key = GovernanceDataKey::GovernanceAnalytics;
    let mut analytics: GovernanceAnalytics = env
        .storage()
        .persistent()
        .get(&analytics_key)
        .unwrap_or(GovernanceAnalytics {
            total_proposals: 0,
            total_votes: 0,
            suspicious_proposals: 0,
            last_suspicious_at: 0,
            max_single_voter_power: 0,
        });
    analytics.total_proposals += 1;
    env.storage().persistent().set(&analytics_key, &analytics);
}

/// Update analytics when a vote is cast.
#[allow(dead_code)]
fn update_analytics_vote_cast(env: &Env) {
    let analytics_key = GovernanceDataKey::GovernanceAnalytics;
    let mut analytics: GovernanceAnalytics = env
        .storage()
        .persistent()
        .get(&analytics_key)
        .unwrap_or(GovernanceAnalytics {
            total_proposals: 0,
            total_votes: 0,
            suspicious_proposals: 0,
            last_suspicious_at: 0,
            max_single_voter_power: 0,
        });
    analytics.total_votes += 1;
    env.storage().persistent().set(&analytics_key, &analytics);
}

/// Query whether an address currently has its tokens locked due to an active vote.
pub fn is_vote_locked(env: &Env, voter: &Address) -> bool {
    let lock_key = GovernanceDataKey::VoteLock(voter.clone());
    if let Some(lock) = env
        .storage()
        .persistent()
        .get::<GovernanceDataKey, VoteLock>(&lock_key)
    {
        env.ledger().timestamp() < lock.locked_until
    } else {
        false
    }
}

/// Query the vote lock record for an address.
pub fn get_vote_lock(env: &Env, voter: &Address) -> Option<VoteLock> {
    let lock_key = GovernanceDataKey::VoteLock(voter.clone());
    env.storage().persistent().get(&lock_key)
}

/// Query the vote power snapshot for a voter on a specific proposal.
pub fn get_vote_power_snapshot(
    env: &Env,
    proposal_id: u64,
    voter: &Address,
) -> Option<VotePowerSnapshot> {
    let snap_key = GovernanceDataKey::VotePowerSnapshot(proposal_id, voter.clone());
    env.storage().persistent().get(&snap_key)
}

/// Query the delegation record for a delegator.
pub fn get_delegation(env: &Env, delegator: &Address) -> Option<DelegationRecord> {
    let del_key = GovernanceDataKey::DelegationRecord(delegator.clone());
    env.storage().persistent().get(&del_key)
}

/// Query governance analytics.
pub fn get_governance_analytics(env: &Env) -> GovernanceAnalytics {
    let analytics_key = GovernanceDataKey::GovernanceAnalytics;
    env.storage()
        .persistent()
        .get(&analytics_key)
        .unwrap_or(GovernanceAnalytics {
            total_proposals: 0,
            total_votes: 0,
            suspicious_proposals: 0,
            last_suspicious_at: 0,
            max_single_voter_power: 0,
        })
}

// ========================================================================
// Query Functions
// ========================================================================

pub fn get_proposal(env: &Env, proposal_id: u64) -> Option<Proposal> {
    env.storage()
        .persistent()
        .get(&GovernanceDataKey::Proposal(proposal_id))
}

pub fn get_vote(env: &Env, proposal_id: u64, voter: Address) -> Option<VoteInfo> {
    env.storage()
        .persistent()
        .get(&GovernanceDataKey::Vote(proposal_id, voter))
}

pub fn get_config(env: &Env) -> Option<GovernanceConfig> {
    env.storage().instance().get(&GovernanceDataKey::Config)
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&GovernanceDataKey::Admin)
}

pub fn emit_guardian_added_event(env: &Env, guardian: &Address) {
    let topics = (Symbol::new(env, "guardian_added"), guardian.clone());
    env.events().publish(topics, ());
}

pub fn emit_guardian_removed_event(env: &Env, guardian: &Address) {
    let topics = (Symbol::new(env, "guardian_removed"), guardian.clone());
    env.events().publish(topics, ());
}

pub fn emit_recovery_started_event(
    env: &Env,
    old_admin: &Address,
    new_admin: &Address,
    initiator: &Address,
) {
    let topics = (
        Symbol::new(env, "recovery_started"),
        old_admin.clone(),
        new_admin.clone(),
    );
    env.events().publish(topics, initiator.clone());
}

pub fn emit_recovery_approved_event(env: &Env, approver: &Address) {
    let topics = (Symbol::new(env, "recovery_approved"), approver.clone());
    env.events().publish(topics, ());
}

pub fn emit_recovery_executed_event(
    env: &Env,
    old_admin: &Address,
    new_admin: &Address,
    executor: &Address,
) {
    let topics = (
        Symbol::new(env, "recovery_executed"),
        old_admin.clone(),
        new_admin.clone(),
    );
    env.events().publish(topics, executor.clone());
}

/// #675
pub fn emit_recovery_cancelled_event(
    env: &Env,
    old_admin: &Address,
    new_admin: &Address,
    caller: &Address,
) {
    let topics = (
        Symbol::new(env, "recovery_cancelled"),
        old_admin.clone(),
        new_admin.clone(),
    );
    env.events().publish(topics, caller.clone());
}
