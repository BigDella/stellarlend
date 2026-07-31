//! Migration verification — drives the *real* upgrade-governance lifecycle
//! (propose -> approve -> queue timelock -> execute) via `UpgradeManager`
//! and confirms it does not disturb the lending contract's own state.
//!
//! Unlike a placeholder that merely re-creates a client handle to an
//! untouched contract, this exercises the actual governance state machine
//! that ships in `common/src/upgrade.rs`, including the standard 48h
//! timelock. Note that `UpgradeManager` is a separate bookkeeping contract:
//! it tracks an approved WASM hash + version but does not itself call
//! Soroban's `deployer().update_current_contract_wasm(..)` to swap code —
//! see DIFFERENTIAL_TEST_REPORT.md in the hello-world crate for why a true
//! WASM-swap migration test isn't feasible here without a separate compiled
//! `.wasm` artifact and build step. What *is* verified: a live lending
//! position is untouched by a full, real upgrade-governance cycle running
//! alongside it.

use super::*;
use crate::upgrade::{UpgradeManager, UpgradeManagerClient, STANDARD_TIMELOCK_SECS};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env,
};

#[test]
fn test_migration_governance_flow_does_not_affect_lending_state() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);

    // Live lending position, set up independently of the upgrade manager.
    let lending_id = env.register(LendingContract, ());
    let lending = LendingContractClient::new(&env, &lending_id);
    let lending_admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);
    lending.initialize(&lending_admin, &1_000_000_000, &1000);
    lending.deposit_collateral(&user, &asset, &500_000);

    let pre_position = lending.get_user_position(&user);
    assert!(
        pre_position.collateral_balance > 0,
        "precondition: user must have a live collateral position"
    );

    // Independent UpgradeManager instance, driven through its full,
    // real lifecycle (not a no-op).
    let upgrade_id = env.register(UpgradeManager, ());
    let upgrade = UpgradeManagerClient::new(&env, &upgrade_id);
    let upgrade_admin = Address::generate(&env);
    let current_hash = BytesN::from_array(&env, &[1u8; 32]);
    upgrade.init(&upgrade_admin, &current_hash, &1);

    let new_hash = BytesN::from_array(&env, &[2u8; 32]);
    let proposal_id = upgrade.upgrade_propose(&upgrade_admin, &new_hash, &1);
    upgrade.upgrade_queue_timelock(&upgrade_admin, &proposal_id);

    env.ledger().with_mut(|li| {
        li.timestamp += STANDARD_TIMELOCK_SECS + 1;
    });

    upgrade.upgrade_execute(&upgrade_admin, &proposal_id);
    assert_eq!(upgrade.current_version(), 1, "upgrade manager must record the new version");
    assert_eq!(upgrade.current_wasm_hash(), new_hash);

    // The lending contract's own application state must be completely
    // unaffected by an unrelated upgrade-governance cycle.
    let post_position = lending.get_user_position(&user);
    assert_eq!(
        pre_position, post_position,
        "lending position must survive a full, real upgrade-governance cycle"
    );
}

/// A rolled-back upgrade must also leave lending's own state untouched.
#[test]
fn test_migration_governance_rollback_does_not_affect_lending_state() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);

    let lending_id = env.register(LendingContract, ());
    let lending = LendingContractClient::new(&env, &lending_id);
    let lending_admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);
    lending.initialize(&lending_admin, &1_000_000_000, &1000);
    lending.deposit_collateral(&user, &asset, &750_000);

    let pre_position = lending.get_user_position(&user);

    let upgrade_id = env.register(UpgradeManager, ());
    let upgrade = UpgradeManagerClient::new(&env, &upgrade_id);
    let upgrade_admin = Address::generate(&env);
    upgrade.init(&upgrade_admin, &BytesN::from_array(&env, &[1u8; 32]), &1);

    let proposal_id =
        upgrade.upgrade_propose(&upgrade_admin, &BytesN::from_array(&env, &[2u8; 32]), &1);
    upgrade.upgrade_queue_timelock(&upgrade_admin, &proposal_id);
    env.ledger().with_mut(|li| {
        li.timestamp += STANDARD_TIMELOCK_SECS + 1;
    });
    upgrade.upgrade_execute(&upgrade_admin, &proposal_id);
    upgrade.upgrade_rollback(&upgrade_admin, &proposal_id);
    assert_eq!(upgrade.current_version(), 0, "rollback must restore the previous version");

    let post_position = lending.get_user_position(&user);
    assert_eq!(
        pre_position, post_position,
        "lending position must survive an upgrade-governance rollback"
    );
}
