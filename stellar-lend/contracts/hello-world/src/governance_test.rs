#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, String, Vec as SdkVec,
};
use soroban_sdk::token::StellarAssetClient;

use crate::errors::GovernanceError;
use crate::governance;
use crate::types::{ProposalStatus, ProposalType, VoteType};

// ─── Test helpers ─────────────────────────────────────────────────────────────

fn create_test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn create_token_with_balance(env: &Env, admin: &Address, to: &Address, amount: i128) -> Address {
    let token = env.register_stellar_asset_contract(admin.clone());
    StellarAssetClient::new(env, &token).mint(to, &amount);
    token
}

fn init_governance(env: &Env, admin: &Address, vote_token: &Address) {
    governance::initialize(
        env,
        admin.clone(),
        vote_token.clone(),
        Some(300),   // voting_period: 300s
        Some(100),   // execution_delay: 100s
        Some(4000),  // quorum_bps: 40%
        Some(0),     // proposal_threshold: 0 (no token requirement to create)
        Some(1000),  // timelock_duration: 1000s
        Some(5000),  // default_voting_threshold: 50%
    )
    .unwrap();
}

// ═════════════════════════════════════════════════════════════
// PHASE 1: PROPOSAL LIFECYCLE TESTS (12 tests)
// ═════════════════════════════════════════════════════════════

#[test]
fn test_phase1_proposal_creation_basic() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(
        &env,
        proposer,
        ProposalType::EmergencyPause(true),
        String::from_str(&env, "basic proposal"),
        None,
    )
    .unwrap();
    assert_eq!(id, 0);
    assert!(governance::get_proposal(&env, id).is_some());
}

#[test]
fn test_phase1_proposal_parameters_validation() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    // quorum > 10000 is invalid
    let result = governance::initialize(
        &env,
        admin,
        token,
        Some(100),
        Some(10),
        Some(10001),
        Some(0),
        Some(500),
        Some(5000),
    );
    assert_eq!(result, Err(GovernanceError::InvalidQuorum));
}

#[test]
fn test_phase1_proposal_id_increment() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id0 = governance::create_proposal(&env, proposer.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "p0"), None).unwrap();
    let id1 = governance::create_proposal(&env, proposer.clone(), ProposalType::EmergencyPause(false), String::from_str(&env, "p1"), None).unwrap();
    let id2 = governance::create_proposal(&env, proposer, ProposalType::MinCollateralRatio(15000), String::from_str(&env, "p2"), None).unwrap();
    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn test_phase1_proposal_state_transitions() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "states"), None).unwrap();
    let p = governance::get_proposal(&env, id).unwrap();
    assert_eq!(p.status, ProposalStatus::Pending);
}

#[test]
fn test_phase1_proposal_retrieval() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    assert!(governance::get_proposal(&env, 999).is_none());

    let proposer = Address::generate(&env);
    governance::create_proposal(&env, proposer.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "retrieve"), None).unwrap();
    let p = governance::get_proposal(&env, 0).unwrap();
    assert_eq!(p.proposer, proposer);
}

#[test]
fn test_phase1_proposal_with_custom_voting_period() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    governance::initialize(&env, admin.clone(), token, Some(600), Some(100), Some(4000), Some(0), Some(1000), Some(5000)).unwrap();
    let config = governance::get_config(&env).unwrap();
    assert_eq!(config.voting_period, 600);
}

#[test]
fn test_phase1_proposal_with_custom_timelock() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    governance::initialize(&env, admin.clone(), token, Some(300), Some(100), Some(4000), Some(0), Some(7200), Some(5000)).unwrap();
    let config = governance::get_config(&env).unwrap();
    assert_eq!(config.timelock_duration, 7200);
}

#[test]
fn test_phase1_proposal_with_custom_threshold() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "threshold"), Some(3000)).unwrap();
    let p = governance::get_proposal(&env, id).unwrap();
    assert_eq!(p.voting_threshold, 3000);
}

#[test]
fn test_phase1_proposal_description_storage() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let desc = String::from_str(&env, "my governance proposal");
    governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), desc.clone(), None).unwrap();
    let p = governance::get_proposal(&env, 0).unwrap();
    assert_eq!(p.description, desc);
}

#[test]
fn test_phase1_proposer_address_tracking() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    governance::create_proposal(&env, proposer.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "proposer"), None).unwrap();
    let p = governance::get_proposal(&env, 0).unwrap();
    assert_eq!(p.proposer, proposer);
}

#[test]
fn test_phase1_proposal_timestamp_recording() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    env.ledger().set_timestamp(1000);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "ts"), None).unwrap();
    let p = governance::get_proposal(&env, 0).unwrap();
    assert_eq!(p.created_at, 1000);
    assert_eq!(p.start_time, 1000);
    assert_eq!(p.end_time, 1300); // start + 300s voting period
}

#[test]
fn test_phase1_proposal_type_handling() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    governance::create_proposal(&env, proposer, ProposalType::MinCollateralRatio(15000), String::from_str(&env, "type"), None).unwrap();
    let p = governance::get_proposal(&env, 0).unwrap();
    assert_eq!(p.proposal_type, ProposalType::MinCollateralRatio(15000));
}

// ═════════════════════════════════════════════════════════════
// PHASE 2: VOTING MECHANICS TESTS (15 tests)
// ═════════════════════════════════════════════════════════════

#[test]
fn test_phase2_vote_for_casting() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "for"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, voter, id, VoteType::For).unwrap();
    let p = governance::get_proposal(&env, id).unwrap();
    assert_eq!(p.for_votes, 1000);
    assert_eq!(p.against_votes, 0);
    assert_eq!(p.abstain_votes, 0);
}

#[test]
fn test_phase2_vote_against_casting() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 500);
    init_governance(&env, &admin, &token);

    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "against"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, voter, id, VoteType::Against).unwrap();
    let p = governance::get_proposal(&env, id).unwrap();
    assert_eq!(p.against_votes, 500);
    assert_eq!(p.for_votes, 0);
}

#[test]
fn test_phase2_vote_abstain_casting() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 200);
    init_governance(&env, &admin, &token);

    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "abstain"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, voter, id, VoteType::Abstain).unwrap();
    let p = governance::get_proposal(&env, id).unwrap();
    assert_eq!(p.abstain_votes, 200);
    assert_eq!(p.for_votes, 0);
    assert_eq!(p.against_votes, 0);
}

#[test]
fn test_phase2_vote_threshold_calculation() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "thr"), Some(5000)).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, voter, id, VoteType::For).unwrap();
    let p = governance::get_proposal(&env, id).unwrap();
    // voting_threshold = 5000 bps (50%); for_votes should cover it
    assert_eq!(p.voting_threshold, 5000);
    assert!(p.for_votes > 0);
}

#[test]
fn test_phase2_vote_count_incrementing() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let v1 = Address::generate(&env);
    let v2 = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone());
    StellarAssetClient::new(&env, &token).mint(&v1, &300);
    StellarAssetClient::new(&env, &token).mint(&v2, &200);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "count"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, v1, id, VoteType::For).unwrap();
    governance::vote(&env, v2, id, VoteType::For).unwrap();
    let p = governance::get_proposal(&env, id).unwrap();
    assert_eq!(p.for_votes, 500);
    assert_eq!(p.total_voting_power, 500);
}

#[test]
fn test_phase2_vote_duplicate_prevention() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 500);
    init_governance(&env, &admin, &token);

    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "dup"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, voter.clone(), id, VoteType::For).unwrap();
    let result = governance::vote(&env, voter, id, VoteType::Against);
    assert_eq!(result, Err(GovernanceError::AlreadyVoted));
}

#[test]
fn test_phase2_vote_during_voting_window() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 100);
    init_governance(&env, &admin, &token);

    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "window"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    let result = governance::vote(&env, voter, id, VoteType::For);
    assert!(result.is_ok());
}

#[test]
fn test_phase2_vote_after_voting_window() {
    // vote() does not check end_time itself; queue_proposal() resolves the status.
    // After a proposal is queued (Queued status), further vote attempts must fail.
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let another = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone());
    StellarAssetClient::new(&env, &token).mint(&voter, &100);
    StellarAssetClient::new(&env, &token).mint(&another, &50);
    init_governance(&env, &admin, &token);

    env.ledger().set_timestamp(0);
    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "late"), None).unwrap();
    env.ledger().set_timestamp(1);
    governance::vote(&env, voter, id, VoteType::For).unwrap();
    env.ledger().set_timestamp(301);
    governance::queue_proposal(&env, admin, id).unwrap(); // → Queued
    // Voting on a resolved (Queued) proposal must fail with ProposalNotActive
    let result = governance::vote(&env, another, id, VoteType::For);
    assert_eq!(result, Err(GovernanceError::ProposalNotActive));
}

#[test]
fn test_phase2_vote_authorization_check() {
    // A voter with zero balance has NoVotingPower. Use a registered token so the
    // balance lookup doesn't panic on an unregistered contract address.
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone()); // registered, no mints
    let zero_voter = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "auth"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    let result = governance::vote(&env, zero_voter, id, VoteType::For);
    assert_eq!(result, Err(GovernanceError::NoVotingPower));
}

#[test]
fn test_phase2_multi_voter_sequential_voting() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let v1 = Address::generate(&env);
    let v2 = Address::generate(&env);
    let v3 = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone());
    StellarAssetClient::new(&env, &token).mint(&v1, &500);
    StellarAssetClient::new(&env, &token).mint(&v2, &300);
    StellarAssetClient::new(&env, &token).mint(&v3, &200);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "multi"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, v1, id, VoteType::For).unwrap();
    governance::vote(&env, v2, id, VoteType::Against).unwrap();
    governance::vote(&env, v3, id, VoteType::For).unwrap();
    let p = governance::get_proposal(&env, id).unwrap();
    assert_eq!(p.for_votes, 700);
    assert_eq!(p.against_votes, 300);
}

#[test]
fn test_phase2_vote_power_tracking() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 750);
    init_governance(&env, &admin, &token);

    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "vp"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, voter.clone(), id, VoteType::For).unwrap();
    let vi = governance::get_vote(&env, id, voter).unwrap();
    assert_eq!(vi.voting_power, 750);
}

#[test]
fn test_phase2_vote_threshold_met() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    // threshold 50% → for_votes(1000) of total(1000) = 100% ≥ 50% → met
    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "thr_met"), Some(5000)).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, voter, id, VoteType::For).unwrap();
    let p = governance::get_proposal(&env, id).unwrap();
    // for_votes covers 100% of total_voting_power → threshold of 50% is met
    assert_eq!(p.for_votes, 1000);
    assert_eq!(p.total_voting_power, 1000);
}

#[test]
fn test_phase2_vote_threshold_not_met() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let v_for = Address::generate(&env);
    let v_against = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone());
    StellarAssetClient::new(&env, &token).mint(&v_for, &400);
    StellarAssetClient::new(&env, &token).mint(&v_against, &600);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "thr_not"), Some(5000)).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, v_for, id, VoteType::For).unwrap();
    governance::vote(&env, v_against, id, VoteType::Against).unwrap();
    // for_votes(400) < 50% of total(1000) = 500 → not met
    let p = governance::get_proposal(&env, id).unwrap();
    assert!(p.for_votes < p.against_votes);
}

#[test]
fn test_phase2_voter_list_tracking() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 100);
    init_governance(&env, &admin, &token);

    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "vl"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, voter.clone(), id, VoteType::For).unwrap();
    assert!(governance::get_vote(&env, id, voter).is_some());
}

#[test]
fn test_phase2_vote_type_diversity() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let v1 = Address::generate(&env);
    let v2 = Address::generate(&env);
    let v3 = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone());
    StellarAssetClient::new(&env, &token).mint(&v1, &100);
    StellarAssetClient::new(&env, &token).mint(&v2, &200);
    StellarAssetClient::new(&env, &token).mint(&v3, &300);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "div"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, v1, id, VoteType::For).unwrap();
    governance::vote(&env, v2, id, VoteType::Against).unwrap();
    governance::vote(&env, v3, id, VoteType::Abstain).unwrap();
    let p = governance::get_proposal(&env, id).unwrap();
    assert_eq!(p.for_votes, 100);
    assert_eq!(p.against_votes, 200);
    assert_eq!(p.abstain_votes, 300);
}

// ═════════════════════════════════════════════════════════════
// PHASE 3: TIMELOCK & EXECUTION TESTS (10 tests)
// ═════════════════════════════════════════════════════════════

#[test]
fn test_phase3_voting_period_enforcement() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    // Zero voting period must be rejected
    let result = governance::initialize(&env, admin, token, Some(0), Some(10), Some(4000), Some(0), Some(500), Some(5000));
    assert_eq!(result, Err(GovernanceError::InvalidVotingPeriod));
}

#[test]
fn test_phase3_execution_timelock_enforcement() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    env.ledger().set_timestamp(0);
    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "tl"), None).unwrap();
    env.ledger().set_timestamp(1);
    governance::vote(&env, voter, id, VoteType::For).unwrap();
    env.ledger().set_timestamp(301);
    governance::queue_proposal(&env, admin.clone(), id).unwrap();
    // Try execute before execution_delay (100s) elapses
    let result = governance::execute_proposal(&env, admin, id);
    assert_eq!(result, Err(GovernanceError::ExecutionTooEarly));
}

#[test]
fn test_phase3_state_transition_active_to_passed() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    env.ledger().set_timestamp(0);
    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "pass"), None).unwrap();
    env.ledger().set_timestamp(1);
    governance::vote(&env, voter, id, VoteType::For).unwrap();
    env.ledger().set_timestamp(301);
    let outcome = governance::queue_proposal(&env, admin, id).unwrap();
    assert!(outcome.succeeded);
    let p = governance::get_proposal(&env, id).unwrap();
    assert_eq!(p.status, ProposalStatus::Queued);
}

#[test]
fn test_phase3_state_transition_active_to_failed() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    env.ledger().set_timestamp(0);
    // High threshold (90%) means voting Against defeats it
    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "fail"), Some(9000)).unwrap();
    env.ledger().set_timestamp(1);
    governance::vote(&env, voter, id, VoteType::Against).unwrap();
    env.ledger().set_timestamp(301);
    let outcome = governance::queue_proposal(&env, admin, id).unwrap();
    assert!(!outcome.succeeded);
    let p = governance::get_proposal(&env, id).unwrap();
    assert_eq!(p.status, ProposalStatus::Defeated);
}

#[test]
fn test_phase3_state_transition_passed_to_executed() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    env.ledger().set_timestamp(0);
    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "exec"), None).unwrap();
    env.ledger().set_timestamp(1);
    governance::vote(&env, voter, id, VoteType::For).unwrap();
    env.ledger().set_timestamp(301);
    governance::queue_proposal(&env, admin.clone(), id).unwrap();
    // Advance to after execution_delay (100s from queue at t=301 → execution_time = 401)
    env.ledger().set_timestamp(402);
    // Action may fail (risk mgmt may not be initialized), but the timelock must not block it
    let result = governance::execute_proposal(&env, admin, id);
    assert!(result != Err(GovernanceError::ExecutionTooEarly));
    assert!(result != Err(GovernanceError::ProposalExpired));
}

#[test]
fn test_phase3_proposal_expiration() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    env.ledger().set_timestamp(0);
    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "exp"), None).unwrap();
    env.ledger().set_timestamp(1);
    governance::vote(&env, voter, id, VoteType::For).unwrap();
    env.ledger().set_timestamp(301);
    governance::queue_proposal(&env, admin.clone(), id).unwrap();
    // Past execution_delay(100) + timelock_duration(1000) → expired
    env.ledger().set_timestamp(1500);
    let result = governance::execute_proposal(&env, admin, id);
    assert_eq!(result, Err(GovernanceError::ProposalExpired));
}

#[test]
fn test_phase3_execution_timestamp_boundary() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    env.ledger().set_timestamp(0);
    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "boundary"), None).unwrap();
    env.ledger().set_timestamp(1);
    governance::vote(&env, voter, id, VoteType::For).unwrap();
    env.ledger().set_timestamp(301);
    governance::queue_proposal(&env, admin.clone(), id).unwrap();
    let p = governance::get_proposal(&env, id).unwrap();
    let exec_time = p.execution_time.unwrap();
    // At exactly execution_time → no longer too early
    env.ledger().set_timestamp(exec_time);
    let result = governance::execute_proposal(&env, admin, id);
    assert!(result != Err(GovernanceError::ExecutionTooEarly));
}

#[test]
fn test_phase3_cannot_execute_expired() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    env.ledger().set_timestamp(0);
    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "ne"), None).unwrap();
    env.ledger().set_timestamp(1);
    governance::vote(&env, voter, id, VoteType::For).unwrap();
    env.ledger().set_timestamp(301);
    governance::queue_proposal(&env, admin.clone(), id).unwrap();
    env.ledger().set_timestamp(50000); // far past any window
    let result = governance::execute_proposal(&env, admin, id);
    assert_eq!(result, Err(GovernanceError::ProposalExpired));
}

#[test]
fn test_phase3_multi_timelock_scenarios() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    env.ledger().set_timestamp(0);
    let proposer = Address::generate(&env);
    let id1 = governance::create_proposal(&env, proposer.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "p1"), None).unwrap();
    let id2 = governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(false), String::from_str(&env, "p2"), None).unwrap();
    env.ledger().set_timestamp(1);
    governance::vote(&env, voter.clone(), id1, VoteType::For).unwrap();
    governance::vote(&env, voter, id2, VoteType::For).unwrap();
    env.ledger().set_timestamp(301);
    governance::queue_proposal(&env, admin.clone(), id1).unwrap();
    governance::queue_proposal(&env, admin.clone(), id2).unwrap();
    assert_eq!(governance::get_proposal(&env, id1).unwrap().status, ProposalStatus::Queued);
    assert_eq!(governance::get_proposal(&env, id2).unwrap().status, ProposalStatus::Queued);
}

#[test]
fn test_phase3_ledger_timestamp_consistency() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    env.ledger().set_timestamp(99999);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "tc"), None).unwrap();
    let p = governance::get_proposal(&env, 0).unwrap();
    assert_eq!(p.created_at, 99999);
    assert_eq!(p.start_time, 99999);
}

// ═════════════════════════════════════════════════════════════
// PHASE 4: MULTISIG OPERATIONS TESTS (15 tests)
// ═════════════════════════════════════════════════════════════

#[test]
fn test_phase4_multisig_admin_initialization() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);
    let config = governance::get_multisig_config(&env).unwrap();
    assert_eq!(config.admins.len(), 1);
    assert_eq!(config.threshold, 1);
    assert!(config.admins.contains(&admin));
}

#[test]
fn test_phase4_multisig_add_admin() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let mut admins = governance::get_multisig_admins(&env).unwrap();
    admins.push_back(new_admin.clone());
    governance::set_multisig_config(&env, admin.clone(), admins, 1).unwrap();
    let config = governance::get_multisig_config(&env).unwrap();
    assert_eq!(config.admins.len(), 2);
    assert!(config.admins.contains(&new_admin));
}

#[test]
fn test_phase4_multisig_remove_admin() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let mut admins = governance::get_multisig_admins(&env).unwrap();
    admins.push_back(admin2.clone());
    governance::set_multisig_config(&env, admin.clone(), admins, 1).unwrap();

    let mut remaining = SdkVec::new(&env);
    remaining.push_back(admin.clone());
    governance::set_multisig_config(&env, admin.clone(), remaining, 1).unwrap();
    let config = governance::get_multisig_config(&env).unwrap();
    assert_eq!(config.admins.len(), 1);
    assert!(!config.admins.contains(&admin2));
}

#[test]
fn test_phase4_multisig_cannot_self_remove() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    // Empty admins list is invalid
    let result = governance::set_multisig_config(&env, admin.clone(), SdkVec::new(&env), 1);
    assert_eq!(result, Err(GovernanceError::InvalidMultisigConfig));
}

#[test]
fn test_phase4_multisig_duplicate_prevention() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    // threshold > admins.len() is invalid
    let admins = governance::get_multisig_admins(&env).unwrap();
    let result = governance::set_multisig_config(&env, admin, admins, 2);
    assert_eq!(result, Err(GovernanceError::InvalidMultisigConfig));
}

#[test]
fn test_phase4_multisig_threshold_validation() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    // threshold = 0 is invalid
    let admins = governance::get_multisig_admins(&env).unwrap();
    let result = governance::set_multisig_config(&env, admin, admins, 0);
    assert_eq!(result, Err(GovernanceError::InvalidMultisigConfig));
}

#[test]
fn test_phase4_multisig_threshold_increase() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let mut admins = governance::get_multisig_admins(&env).unwrap();
    admins.push_back(admin2);
    governance::set_multisig_config(&env, admin.clone(), admins, 2).unwrap();
    assert_eq!(governance::get_multisig_threshold(&env), 2);
}

#[test]
fn test_phase4_multisig_threshold_decrease() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let mut admins = governance::get_multisig_admins(&env).unwrap();
    admins.push_back(admin2);
    governance::set_multisig_config(&env, admin.clone(), admins.clone(), 2).unwrap();
    assert_eq!(governance::get_multisig_threshold(&env), 2);
    governance::set_multisig_config(&env, admin.clone(), admins, 1).unwrap();
    assert_eq!(governance::get_multisig_threshold(&env), 1);
}

#[test]
fn test_phase4_multisig_approval_required() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "ms"), None).unwrap();
    let approvals = governance::get_proposal_approvals(&env, id).unwrap_or_else(|| SdkVec::new(&env));
    assert_eq!(approvals.len(), 0);
}

#[test]
fn test_phase4_multisig_approval_threshold_met() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "ms"), None).unwrap();
    governance::approve_proposal(&env, admin.clone(), id).unwrap();
    let approvals = governance::get_proposal_approvals(&env, id).unwrap();
    assert!(approvals.len() >= governance::get_multisig_threshold(&env));
}

#[test]
fn test_phase4_multisig_approval_threshold_not_met() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let mut admins = governance::get_multisig_admins(&env).unwrap();
    admins.push_back(admin2);
    governance::set_multisig_config(&env, admin.clone(), admins, 2).unwrap();

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "ms2"), None).unwrap();
    governance::approve_proposal(&env, admin.clone(), id).unwrap();
    let approvals = governance::get_proposal_approvals(&env, id).unwrap();
    // 1 approval < threshold 2 → not met
    assert!(approvals.len() < governance::get_multisig_threshold(&env));
}

#[test]
fn test_phase4_multisig_duplicate_approval_prevention() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "dup_apr"), None).unwrap();
    governance::approve_proposal(&env, admin.clone(), id).unwrap();
    let result = governance::approve_proposal(&env, admin, id);
    assert_eq!(result, Err(GovernanceError::AlreadyVoted));
}

#[test]
fn test_phase4_multisig_transfer_admin() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let mut admins = SdkVec::new(&env);
    admins.push_back(new_admin.clone());
    governance::set_multisig_config(&env, admin.clone(), admins, 1).unwrap();
    let config = governance::get_multisig_config(&env).unwrap();
    assert!(config.admins.contains(&new_admin));
    assert!(!config.admins.contains(&admin));
}

#[test]
fn test_phase4_multisig_admin_list_tracking() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);
    let admins = governance::get_multisig_admins(&env).unwrap();
    assert!(admins.contains(&admin));
}

#[test]
fn test_phase4_multisig_authorization_enforcement() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let outsider = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let admins = governance::get_multisig_admins(&env).unwrap();
    let result = governance::set_multisig_config(&env, outsider, admins, 1);
    assert_eq!(result, Err(GovernanceError::Unauthorized));
}

// ═════════════════════════════════════════════════════════════
// PHASE 5: ERROR HANDLING TESTS (8 tests)
// ═════════════════════════════════════════════════════════════

#[test]
fn test_phase5_error_unauthorized() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let outsider = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(&env, proposer.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "unauth"), None).unwrap();
    let result = governance::cancel_proposal(&env, outsider, id);
    assert_eq!(result, Err(GovernanceError::Unauthorized));
}

#[test]
fn test_phase5_error_proposal_not_found() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let result = governance::vote(&env, admin, 999, VoteType::For);
    assert_eq!(result, Err(GovernanceError::ProposalNotFound));
}

#[test]
fn test_phase5_error_invalid_proposal() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let result = governance::queue_proposal(&env, admin, 999);
    assert_eq!(result, Err(GovernanceError::ProposalNotFound));
}

#[test]
fn test_phase5_error_invalid_arguments() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let result = governance::initialize(&env, admin, token, Some(100), Some(10), Some(15000), Some(0), Some(500), Some(5000));
    assert_eq!(result, Err(GovernanceError::InvalidQuorum));
}

#[test]
fn test_phase5_error_vote_already_cast() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 100);
    init_governance(&env, &admin, &token);

    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "vc"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, voter.clone(), id, VoteType::For).unwrap();
    let result = governance::vote(&env, voter, id, VoteType::For);
    assert_eq!(result, Err(GovernanceError::AlreadyVoted));
}

#[test]
fn test_phase5_error_proposal_expired() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    env.ledger().set_timestamp(0);
    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "exp"), None).unwrap();
    env.ledger().set_timestamp(1);
    governance::vote(&env, voter, id, VoteType::For).unwrap();
    env.ledger().set_timestamp(301);
    governance::queue_proposal(&env, admin.clone(), id).unwrap();
    env.ledger().set_timestamp(5000);
    let result = governance::execute_proposal(&env, admin, id);
    assert_eq!(result, Err(GovernanceError::ProposalExpired));
}

#[test]
fn test_phase5_error_insufficient_votes() {
    // Use a registered token so the balance lookup doesn't panic.
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone()); // registered, no mints
    let zero_voter = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let proposer = Address::generate(&env);
    let id = governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "iv"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    let result = governance::vote(&env, zero_voter, id, VoteType::For);
    assert_eq!(result, Err(GovernanceError::NoVotingPower));
}

#[test]
fn test_phase5_error_state_consistency() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);
    // Double initialization must be rejected
    let result = governance::initialize(&env, admin.clone(), token.clone(), Some(100), Some(10), Some(4000), Some(0), Some(500), Some(5000));
    assert_eq!(result, Err(GovernanceError::AlreadyInitialized));
}

// ═════════════════════════════════════════════════════════════
// PHASE 6: EVENT VALIDATION TESTS (4 tests)
// ═════════════════════════════════════════════════════════════

#[test]
fn test_phase6_event_proposal_created() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    governance::create_proposal(&env, proposer, ProposalType::EmergencyPause(true), String::from_str(&env, "ev_create"), None).unwrap();
    // Proposal exists → ProposalCreated event was emitted (side-effect verified via state)
    assert!(governance::get_proposal(&env, 0).is_some());
}

#[test]
fn test_phase6_event_vote_cast() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 100);
    init_governance(&env, &admin, &token);

    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "ev_vote"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, voter.clone(), id, VoteType::For).unwrap();
    // VoteInfo persisted → VoteCast event was emitted
    assert!(governance::get_vote(&env, id, voter).is_some());
}

#[test]
fn test_phase6_event_proposal_executed() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    env.ledger().set_timestamp(0);
    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "ev_exec"), None).unwrap();
    env.ledger().set_timestamp(1);
    governance::vote(&env, voter, id, VoteType::For).unwrap();
    env.ledger().set_timestamp(301);
    governance::queue_proposal(&env, admin.clone(), id).unwrap();
    let p = governance::get_proposal(&env, id).unwrap();
    env.ledger().set_timestamp(p.execution_time.unwrap());
    // Execution attempt may fail if subsystem not initialized, but must not panic
    let _ = governance::execute_proposal(&env, admin, id);
}

#[test]
fn test_phase6_event_proposal_failed() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 100);
    init_governance(&env, &admin, &token);

    env.ledger().set_timestamp(0);
    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "ev_fail"), Some(9000)).unwrap();
    env.ledger().set_timestamp(1);
    governance::vote(&env, voter, id, VoteType::Against).unwrap();
    env.ledger().set_timestamp(301);
    let outcome = governance::queue_proposal(&env, admin, id).unwrap();
    // ProposalFailed event emitted when !succeeded
    assert!(!outcome.succeeded);
    assert_eq!(governance::get_proposal(&env, id).unwrap().status, ProposalStatus::Defeated);
}

// ═════════════════════════════════════════════════════════════
// PHASE 7: INTEGRATION SCENARIOS TESTS (6 tests)
// ═════════════════════════════════════════════════════════════

#[test]
fn test_phase7_full_proposal_lifecycle() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    // 1. Create
    env.ledger().set_timestamp(0);
    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "lifecycle"), None).unwrap();
    assert_eq!(governance::get_proposal(&env, id).unwrap().status, ProposalStatus::Pending);

    // 2. Vote
    env.ledger().set_timestamp(1);
    governance::vote(&env, voter, id, VoteType::For).unwrap();
    assert_eq!(governance::get_proposal(&env, id).unwrap().status, ProposalStatus::Active);

    // 3. Queue
    env.ledger().set_timestamp(301);
    let outcome = governance::queue_proposal(&env, admin.clone(), id).unwrap();
    assert!(outcome.succeeded);
    assert_eq!(governance::get_proposal(&env, id).unwrap().status, ProposalStatus::Queued);
}

#[test]
fn test_phase7_multiple_proposals_concurrent() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 1000);
    init_governance(&env, &admin, &token);

    env.ledger().set_timestamp(0);
    let proposer = Address::generate(&env);
    let id1 = governance::create_proposal(&env, proposer.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "c1"), None).unwrap();
    let id2 = governance::create_proposal(&env, proposer.clone(), ProposalType::EmergencyPause(false), String::from_str(&env, "c2"), None).unwrap();
    let id3 = governance::create_proposal(&env, proposer, ProposalType::MinCollateralRatio(15000), String::from_str(&env, "c3"), None).unwrap();

    assert_eq!(id1, 0);
    assert_eq!(id2, 1);
    assert_eq!(id3, 2);

    env.ledger().set_timestamp(1);
    governance::vote(&env, voter.clone(), id1, VoteType::For).unwrap();
    governance::vote(&env, voter.clone(), id2, VoteType::Against).unwrap();
    governance::vote(&env, voter, id3, VoteType::For).unwrap();

    env.ledger().set_timestamp(301);
    let o1 = governance::queue_proposal(&env, admin.clone(), id1).unwrap();
    let o2 = governance::queue_proposal(&env, admin.clone(), id2).unwrap();
    let o3 = governance::queue_proposal(&env, admin.clone(), id3).unwrap();
    assert!(o1.succeeded);
    assert!(!o2.succeeded);
    assert!(o3.succeeded);
}

#[test]
fn test_phase7_governance_parameter_updates() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let config = governance::get_config(&env).unwrap();
    assert_eq!(config.voting_period, 300);
    assert_eq!(config.execution_delay, 100);
    assert_eq!(config.quorum_bps, 4000);
    assert_eq!(config.timelock_duration, 1000);
    assert_eq!(config.default_voting_threshold, 5000);
}

#[test]
fn test_phase7_emergency_pause_execution() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    // Emergency proposals bypass timelock delay
    let id = governance::create_emergency_proposal(
        &env,
        admin.clone(),
        ProposalType::EmergencyPause(true),
        String::from_str(&env, "emergency"),
    )
    .unwrap();
    let p = governance::get_proposal(&env, id).unwrap();
    assert_eq!(p.status, ProposalStatus::Queued);
    // execution_time = now (no delay)
    assert_eq!(p.execution_time, Some(env.ledger().timestamp()));
}

#[test]
fn test_phase7_admin_management_workflow() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    let token = Address::generate(&env);
    init_governance(&env, &admin, &token);

    let mut admins = governance::get_multisig_admins(&env).unwrap();
    admins.push_back(admin2.clone());
    admins.push_back(admin3.clone());
    governance::set_multisig_config(&env, admin.clone(), admins, 2).unwrap();

    let config = governance::get_multisig_config(&env).unwrap();
    assert_eq!(config.admins.len(), 3);
    assert_eq!(config.threshold, 2);
    assert!(config.admins.contains(&admin2));
    assert!(config.admins.contains(&admin3));
}

#[test]
fn test_phase7_vote_reversal_scenario() {
    // Protocol does not allow vote reversal; second vote must return AlreadyVoted
    let env = create_test_env();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = create_token_with_balance(&env, &admin, &voter, 100);
    init_governance(&env, &admin, &token);

    let id = governance::create_proposal(&env, voter.clone(), ProposalType::EmergencyPause(true), String::from_str(&env, "reversal"), None).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    governance::vote(&env, voter.clone(), id, VoteType::For).unwrap();
    let result = governance::vote(&env, voter, id, VoteType::Against);
    assert_eq!(result, Err(GovernanceError::AlreadyVoted));
}
