use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env, Error, InvokeError,
};

use crate::upgrade::{
    UpgradeManager, UpgradeManagerClient, UpgradeStage, EMERGENCY_TIMELOCK_SECS,
    STANDARD_TIMELOCK_SECS,
};

fn hash(env: &Env, b: u8) -> BytesN<32> {
    BytesN::from_array(env, &[b; 32])
}

fn setup(env: &Env, required_approvals: u32) -> (UpgradeManagerClient<'_>, Address) {
    let contract_id = env.register(UpgradeManager, ());
    let client = UpgradeManagerClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.init(&admin, &hash(env, 1), &required_approvals);
    (client, admin)
}

fn assert_failed<T, E>(result: Result<Result<T, E>, Result<Error, InvokeError>>) {
    assert!(
        !matches!(result, Ok(Ok(_))),
        "expected invocation to fail, but it succeeded"
    );
}

/// Verifies initialization and baseline status fields.
#[test]
fn test_init_sets_defaults() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 2);

    assert_eq!(client.current_version(), 0);
    assert_eq!(client.required_approvals(), 2);
    assert_eq!(client.current_wasm_hash(), hash(&env, 1));
    assert!(client.is_approver(&admin));
}

#[test]
fn test_init_rejects_zero_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(UpgradeManager, ());
    let client = UpgradeManagerClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let result = client.try_init(&admin, &hash(&env, 1), &0);
    assert_failed(result);
}

#[test]
fn test_init_rejects_second_call() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 1);

    let result = client.try_init(&admin, &hash(&env, 2), &1);
    assert_failed(result);
}

#[test]
fn test_add_approver_admin_only_and_idempotent() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 2);
    let approver = Address::generate(&env);
    let stranger = Address::generate(&env);

    let denied = client.try_add_approver(&stranger, &approver);
    assert_failed(denied);

    client.add_approver(&admin, &approver);
    client.add_approver(&admin, &approver);
    assert!(client.is_approver(&approver));
}

#[test]
fn test_upgrade_propose_sets_initial_status() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 2);

    let proposal_id = client.upgrade_propose(&admin, &hash(&env, 2), &1);
    let status = client.upgrade_status(&proposal_id);
    assert_eq!(proposal_id, 1);
    assert_eq!(status.id, 1);
    assert_eq!(status.stage, UpgradeStage::Proposed);
    assert_eq!(status.approval_count, 1);
    assert_eq!(status.required_approvals, 2);
    assert_eq!(status.target_version, 1);
}

#[test]
fn test_upgrade_propose_auto_approved_at_threshold_one() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 1);

    let proposal_id = client.upgrade_propose(&admin, &hash(&env, 3), &1);
    let status = client.upgrade_status(&proposal_id);
    assert_eq!(status.stage, UpgradeStage::Approved);
}

#[test]
fn test_upgrade_approve_flow_and_status_transition() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 2);
    let approver = Address::generate(&env);
    let stranger = Address::generate(&env);
    client.add_approver(&admin, &approver);

    let proposal_id = client.upgrade_propose(&admin, &hash(&env, 2), &1);
    let denied = client.try_upgrade_approve(&stranger, &proposal_id);
    assert_failed(denied);

    let count = client.upgrade_approve(&approver, &proposal_id);
    assert_eq!(count, 2);
    assert_eq!(
        client.upgrade_status(&proposal_id).stage,
        UpgradeStage::Approved
    );

    let duplicate = client.try_upgrade_approve(&approver, &proposal_id);
    assert_failed(duplicate);
}

#[test]
fn test_upgrade_approve_missing_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 1);

    let missing = client.try_upgrade_approve(&admin, &99);
    assert_failed(missing);
}

#[test]
fn test_upgrade_execute_requires_approvals() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 2);
    let stranger = Address::generate(&env);

    let proposal_id = client.upgrade_propose(&admin, &hash(&env, 2), &1);
    let denied = client.try_upgrade_execute(&stranger, &proposal_id);
    assert_failed(denied);

    // Not enough approvals yet — should fail.
    let not_ready = client.try_upgrade_execute(&admin, &proposal_id);
    assert_failed(not_ready);
}

#[test]
fn test_upgrade_execute_requires_timelock_elapsed() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 1);

    let proposal_id = client.upgrade_propose(&admin, &hash(&env, 9), &3);
    // Proposal auto-approved (threshold=1), queue the timelock.
    client.upgrade_queue_timelock(&admin, &proposal_id);
    let status = client.upgrade_status(&proposal_id);
    assert_eq!(status.stage, UpgradeStage::TimelockQueued);

    // Attempt execute before timelock elapses — must fail.
    let too_early = client.try_upgrade_execute(&admin, &proposal_id);
    assert_failed(too_early);

    // Advance ledger past the standard 48 h timelock.
    env.ledger().with_mut(|li| {
        li.timestamp += STANDARD_TIMELOCK_SECS + 1;
    });

    client.upgrade_execute(&admin, &proposal_id);
    assert_eq!(client.current_version(), 3);
}

#[test]
fn test_upgrade_execute_updates_current_version_and_hash() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 1);

    let next_hash = hash(&env, 9);
    let proposal_id = client.upgrade_propose(&admin, &next_hash, &3);
    // Queue the standard timelock then advance past it.
    client.upgrade_queue_timelock(&admin, &proposal_id);
    env.ledger().with_mut(|li| {
        li.timestamp += STANDARD_TIMELOCK_SECS + 1;
    });
    client.upgrade_execute(&admin, &proposal_id);

    assert_eq!(client.current_version(), 3);
    assert_eq!(client.current_wasm_hash(), next_hash);
    assert_eq!(
        client.upgrade_status(&proposal_id).stage,
        UpgradeStage::Executed
    );

    let repeated = client.try_upgrade_execute(&admin, &proposal_id);
    assert_failed(repeated);
}

#[test]
fn test_upgrade_rollback_requires_admin_and_executed_stage() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 1);
    let stranger = Address::generate(&env);

    let proposal_id = client.upgrade_propose(&admin, &hash(&env, 7), &1);
    let denied = client.try_upgrade_rollback(&stranger, &proposal_id);
    assert_failed(denied);

    let invalid_status = client.try_upgrade_rollback(&admin, &proposal_id);
    assert_failed(invalid_status);
}

fn execute_after_timelock(
    env: &Env,
    client: &UpgradeManagerClient,
    admin: &Address,
    proposal_id: u64,
) {
    client.upgrade_queue_timelock(admin, &proposal_id);
    env.ledger().with_mut(|li| {
        li.timestamp += STANDARD_TIMELOCK_SECS + 1;
    });
    client.upgrade_execute(admin, &proposal_id);
}

#[test]
fn test_upgrade_rollback_restores_previous_version_and_hash() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 1);
    let initial_hash = client.current_wasm_hash();

    let proposal_id = client.upgrade_propose(&admin, &hash(&env, 8), &5);
    execute_after_timelock(&env, &client, &admin, proposal_id);
    assert_eq!(client.current_version(), 5);

    client.upgrade_rollback(&admin, &proposal_id);
    assert_eq!(client.current_version(), 0);
    assert_eq!(client.current_wasm_hash(), initial_hash);
    assert_eq!(
        client.upgrade_status(&proposal_id).stage,
        UpgradeStage::RolledBack
    );

    let repeated = client.try_upgrade_rollback(&admin, &proposal_id);
    assert_failed(repeated);
}

#[test]
fn test_upgrade_propose_rejects_non_admin_and_invalid_version_updated() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 1);
    let stranger = Address::generate(&env);

    let denied = client.try_upgrade_propose(&stranger, &hash(&env, 2), &1);
    assert_failed(denied);

    let first = client.upgrade_propose(&admin, &hash(&env, 2), &1);
    execute_after_timelock(&env, &client, &admin, first);
    let invalid = client.try_upgrade_propose(&admin, &hash(&env, 3), &1);
    assert_failed(invalid);
}

#[test]
fn test_emergency_upgrade_uses_short_timelock() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 1);

    let proposal_id = client.upgrade_propose_emergency(&admin, &hash(&env, 5), &2);
    let status = client.upgrade_status(&proposal_id);
    // With threshold=1 the proposal goes straight to TimelockQueued.
    assert_eq!(status.stage, UpgradeStage::TimelockQueued);
    assert!(status.is_emergency);

    // Standard timelock not elapsed yet.
    env.ledger().with_mut(|li| {
        li.timestamp += STANDARD_TIMELOCK_SECS + 1;
    });
    // execute_after was set to EMERGENCY_TIMELOCK_SECS from proposal time (t=0),
    // so after STANDARD_TIMELOCK_SECS the 4 h window has long passed.
    client.upgrade_execute(&admin, &proposal_id);
    assert_eq!(client.current_version(), 2);
}

#[test]
fn test_emergency_upgrade_shorter_than_standard() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 1);

    let proposal_id = client.upgrade_propose_emergency(&admin, &hash(&env, 6), &3);

    // Advance only past the emergency window (4 h) but not 48 h.
    env.ledger().with_mut(|li| {
        li.timestamp += EMERGENCY_TIMELOCK_SECS + 1;
    });

    // Execute should succeed because emergency timelock elapsed.
    client.upgrade_execute(&admin, &proposal_id);
    assert_eq!(client.current_version(), 3);
}

#[test]
fn test_non_admin_cannot_propose_emergency() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env, 1);
    let stranger = Address::generate(&env);

    let denied = client.try_upgrade_propose_emergency(&stranger, &hash(&env, 7), &4);
    assert_failed(denied);
}

#[test]
fn test_upgrade_status_missing_proposal_errors() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup(&env, 1);

    let result = client.try_upgrade_status(&42);
    assert_failed(result);
}

#[test]
fn test_is_approver_false_before_init() {
    let env = Env::default();
    let contract_id = env.register(UpgradeManager, ());
    let client = UpgradeManagerClient::new(&env, &contract_id);
    let random = Address::generate(&env);

    assert!(!client.is_approver(&random));
}
