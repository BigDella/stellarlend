use soroban_sdk::{testutils::Address as _, Address, Env, String, Symbol, Val, Vec};
use stellar_lend::hello_world::HelloContractClient;

mod common;
use common::*;

#[test]
fn test_governance_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vote_token = Address::generate(&env);
    let contract = setup_governance_contract(&env, &admin, &vote_token);

    let config = contract.governance_config();
    assert_eq!(config.quorum_bps, 5000);
    assert!(config.voting_period > 0);
}

#[test]
fn test_create_proposal() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vote_token = Address::generate(&env);
    let proposer = Address::generate(&env);
    let contract = setup_governance_contract(&env, &admin, &vote_token);

    // Give proposer voting power
    setup_vote_token(&env, &vote_token, &proposer, 1000);

    let proposal_id = contract.create_governance_proposal(
        &proposer,
        &String::from_slice(&env, "proposal_type"),
        &String::from_slice(&env, "Test proposal"),
        &None,
    );

    assert!(proposal_id >= 0);
}

#[test]
fn test_proposal_lifecycle_approve() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vote_token = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);

    let contract = setup_governance_contract(&env, &admin, &vote_token);

    // Setup voting power
    setup_vote_token(&env, &vote_token, &proposer, 1000);
    setup_vote_token(&env, &vote_token, &voter1, 5000);
    setup_vote_token(&env, &vote_token, &voter2, 5000);

    // Create proposal
    let proposal_id = contract.create_governance_proposal(
        &proposer,
        &String::from_slice(&env, "parameter_change"),
        &String::from_slice(&env, "Increase max LTV"),
        &None,
    );

    // Cast votes
    contract.cast_governance_vote(&voter1, &proposal_id, &true);
    contract.cast_governance_vote(&voter2, &proposal_id, &true);

    // Advance time past voting period
    env.ledger().set_sequence(env.ledger().sequence() + 100000);

    // Queue proposal
    contract.queue_governance_proposal(&proposal_id);

    // Execute after timelock
    env.ledger().set_sequence(env.ledger().sequence() + 100000);
    contract.execute_governance_proposal(&proposal_id);

    // Verify proposal executed
    let status = contract.proposal_status(&proposal_id);
    assert_eq!(status, String::from_slice(&env, "Executed"));
}

#[test]
fn test_proposal_rejection_below_quorum() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vote_token = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    let contract = setup_governance_contract(&env, &admin, &vote_token);

    setup_vote_token(&env, &vote_token, &proposer, 1000);
    setup_vote_token(&env, &vote_token, &voter, 500);

    let proposal_id = contract.create_governance_proposal(
        &proposer,
        &String::from_slice(&env, "parameter_change"),
        &String::from_slice(&env, "Test"),
        &None,
    );

    contract.cast_governance_vote(&voter, &proposal_id, &false);

    env.ledger().set_sequence(env.ledger().sequence() + 100000);

    let result = contract.finalize_governance_proposal(&proposal_id);
    assert!(!result);
}

#[test]
fn test_proposal_cancellation() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vote_token = Address::generate(&env);
    let proposer = Address::generate(&env);

    let contract = setup_governance_contract(&env, &admin, &vote_token);

    setup_vote_token(&env, &vote_token, &proposer, 1000);

    let proposal_id = contract.create_governance_proposal(
        &proposer,
        &String::from_slice(&env, "parameter_change"),
        &String::from_slice(&env, "Test"),
        &None,
    );

    contract.cancel_governance_proposal(&proposer, &proposal_id);

    let status = contract.proposal_status(&proposal_id);
    assert_eq!(status, String::from_slice(&env, "Cancelled"));
}

#[test]
fn test_timelock_enforcement() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vote_token = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    let contract = setup_governance_contract(&env, &admin, &vote_token);

    setup_vote_token(&env, &vote_token, &proposer, 1000);
    setup_vote_token(&env, &vote_token, &voter, 10000);

    let proposal_id = contract.create_governance_proposal(
        &proposer,
        &String::from_slice(&env, "parameter_change"),
        &String::from_slice(&env, "Test"),
        &None,
    );

    contract.cast_governance_vote(&voter, &proposal_id, &true);

    env.ledger().set_sequence(env.ledger().sequence() + 100000);
    contract.queue_governance_proposal(&proposal_id);

    // Try to execute before timelock expires - should fail
    let should_fail = contract.try_execute_governance_proposal(&proposal_id);
    assert!(!should_fail);

    // Advance past timelock
    env.ledger().set_sequence(env.ledger().sequence() + 100000);
    contract.execute_governance_proposal(&proposal_id);

    let status = contract.proposal_status(&proposal_id);
    assert_eq!(status, String::from_slice(&env, "Executed"));
}

#[test]
fn test_batch_parameter_changes() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vote_token = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    let contract = setup_governance_contract(&env, &admin, &vote_token);

    setup_vote_token(&env, &vote_token, &proposer, 1000);
    setup_vote_token(&env, &vote_token, &voter, 10000);

    let proposal_id = contract.create_batch_proposal(
        &proposer,
        &vec![
            &env,
            String::from_slice(&env, "max_ltv"),
            String::from_slice(&env, "reserve_factor"),
            String::from_slice(&env, "liquidation_bonus"),
        ],
    );

    contract.cast_governance_vote(&voter, &proposal_id, &true);

    env.ledger().set_sequence(env.ledger().sequence() + 100000);
    contract.queue_governance_proposal(&proposal_id);

    env.ledger().set_sequence(env.ledger().sequence() + 100000);
    contract.execute_governance_proposal(&proposal_id);

    let status = contract.proposal_status(&proposal_id);
    assert_eq!(status, String::from_slice(&env, "Executed"));
}

#[test]
fn test_delegation_voting_power() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vote_token = Address::generate(&env);
    let proposer = Address::generate(&env);
    let delegator = Address::generate(&env);
    let delegate = Address::generate(&env);

    let contract = setup_governance_contract(&env, &admin, &vote_token);

    setup_vote_token(&env, &vote_token, &proposer, 1000);
    setup_vote_token(&env, &vote_token, &delegator, 5000);
    setup_vote_token(&env, &vote_token, &delegate, 0);

    // Delegate voting power
    contract.delegate_voting_power(&delegator, &delegate);

    let proposal_id = contract.create_governance_proposal(
        &proposer,
        &String::from_slice(&env, "parameter_change"),
        &String::from_slice(&env, "Test"),
        &None,
    );

    // Vote with delegated power
    contract.cast_governance_vote(&delegate, &proposal_id, &true);

    env.ledger().set_sequence(env.ledger().sequence() + 100000);
    contract.queue_governance_proposal(&proposal_id);

    env.ledger().set_sequence(env.ledger().sequence() + 100000);
    contract.execute_governance_proposal(&proposal_id);

    let status = contract.proposal_status(&proposal_id);
    assert_eq!(status, String::from_slice(&env, "Executed"));
}

#[test]
fn test_competing_proposals() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vote_token = Address::generate(&env);
    let proposer1 = Address::generate(&env);
    let proposer2 = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);

    let contract = setup_governance_contract(&env, &admin, &vote_token);

    setup_vote_token(&env, &vote_token, &proposer1, 1000);
    setup_vote_token(&env, &vote_token, &proposer2, 1000);
    setup_vote_token(&env, &vote_token, &voter1, 5000);
    setup_vote_token(&env, &vote_token, &voter2, 5000);

    // Create two competing proposals
    let proposal_id_1 = contract.create_governance_proposal(
        &proposer1,
        &String::from_slice(&env, "parameter_change"),
        &String::from_slice(&env, "Increase LTV"),
        &None,
    );

    let proposal_id_2 = contract.create_governance_proposal(
        &proposer2,
        &String::from_slice(&env, "parameter_change"),
        &String::from_slice(&env, "Decrease LTV"),
        &None,
    );

    // Split votes
    contract.cast_governance_vote(&voter1, &proposal_id_1, &true);
    contract.cast_governance_vote(&voter2, &proposal_id_2, &true);

    env.ledger().set_sequence(env.ledger().sequence() + 100000);

    // Both should be quorum-eligible, outcomes determined by vote totals
    let status1 = contract.proposal_status(&proposal_id_1);
    let status2 = contract.proposal_status(&proposal_id_2);

    assert!(!status1.is_empty());
    assert!(!status2.is_empty());
}

#[test]
fn test_emergency_proposal() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vote_token = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    let contract = setup_governance_contract(&env, &admin, &vote_token);

    setup_vote_token(&env, &vote_token, &proposer, 1000);
    setup_vote_token(&env, &vote_token, &voter, 10000);

    let proposal_id = contract.create_emergency_proposal(
        &proposer,
        &String::from_slice(&env, "Pause all lending"),
    );

    contract.cast_governance_vote(&voter, &proposal_id, &true);

    env.ledger().set_sequence(env.ledger().sequence() + 10000);

    contract.queue_governance_proposal(&proposal_id);
    contract.execute_governance_proposal(&proposal_id);

    let status = contract.proposal_status(&proposal_id);
    assert_eq!(status, String::from_slice(&env, "Executed"));
}

#[test]
fn test_governance_upgrade_via_governance() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let vote_token = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    let contract = setup_governance_contract(&env, &admin, &vote_token);

    setup_vote_token(&env, &vote_token, &proposer, 1000);
    setup_vote_token(&env, &vote_token, &voter, 10000);

    let proposal_id = contract.create_governance_proposal(
        &proposer,
        &String::from_slice(&env, "upgrade_governance"),
        &String::from_slice(&env, "Upgrade governance contract"),
        &None,
    );

    contract.cast_governance_vote(&voter, &proposal_id, &true);

    env.ledger().set_sequence(env.ledger().sequence() + 100000);
    contract.queue_governance_proposal(&proposal_id);

    env.ledger().set_sequence(env.ledger().sequence() + 100000);
    contract.execute_governance_proposal(&proposal_id);

    let status = contract.proposal_status(&proposal_id);
    assert_eq!(status, String::from_slice(&env, "Executed"));
}
