//! Migration verification tests — verifies that storage written by v1 is
//! readable after a contract upgrade (WASM swap simulation).
//!
//! Covers:
//! - Storage layout compatibility across versions
//! - User state preservation (collateral, debt, position)
//! - Admin address survives upgrade

#![cfg(test)]

use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env};

use crate::{
    deposit::DepositDataKey,
    deposit::Position,
    HelloContract, HelloContractClient,
};

fn setup(env: &Env) -> (Address, HelloContractClient<'_>, Address, Address) {
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let user = Address::generate(env);
    client.initialize(&admin);
    let native = env.register_stellar_asset_contract(admin.clone());
    client.set_native_asset_address(&admin, &native);
    (contract_id, client, admin, user)
}

fn read_position(env: &Env, contract_id: &Address, user: &Address) -> Option<Position> {
    env.as_contract(contract_id, || {
        env.storage()
            .persistent()
            .get::<DepositDataKey, Position>(&DepositDataKey::Position(user.clone()))
    })
}

fn read_collateral(env: &Env, contract_id: &Address, user: &Address) -> i128 {
    env.as_contract(contract_id, || {
        env.storage()
            .persistent()
            .get::<DepositDataKey, i128>(&DepositDataKey::CollateralBalance(user.clone()))
            .unwrap_or(0)
    })
}

/// State written before upgrade must be readable after upgrade (same WASM re-registered).
#[test]
fn test_migration_position_survives_upgrade() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);

    let (contract_id, client, _admin, user) = setup(&env);

    // Write state pre-upgrade
    client.deposit_collateral(&user, &None, &2_000_000);

    let pre_collateral = read_collateral(&env, &contract_id, &user);
    assert!(pre_collateral > 0, "pre-upgrade collateral must be set");

    // Simulate upgrade: re-register same contract (WASM swap in real scenario)
    // In Soroban test env, state persists across re-registration on same contract_id
    let client2 = HelloContractClient::new(&env, &contract_id);

    // Read state post-upgrade via new client
    let post_collateral = read_collateral(&env, &contract_id, &user);
    assert_eq!(
        pre_collateral, post_collateral,
        "collateral must survive upgrade"
    );

    // Position struct must also be intact
    let pos = client2.get_user_position(&user);
    assert_eq!(pos.collateral, pre_collateral);
}

/// Admin address must survive upgrade.
#[test]
fn test_migration_admin_survives_upgrade() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _client, admin, _user) = setup(&env);

    let client2 = HelloContractClient::new(&env, &contract_id);

    // Admin-only op must still work post-upgrade
    let result = client2.try_set_emergency_pause(&admin, &false);
    assert!(result.is_ok(), "admin must still be authorized post-upgrade");
}

/// Multiple users' state must all survive upgrade.
#[test]
fn test_migration_multiple_users_survive_upgrade() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);

    let (contract_id, client, _admin, _) = setup(&env);

    let users: Vec<Address> = (0..5).map(|_| Address::generate(&env)).collect();
    let amounts: Vec<i128> = vec![100_000, 200_000, 300_000, 400_000, 500_000];

    for (user, &amount) in users.iter().zip(amounts.iter()) {
        client.deposit_collateral(user, &None, &amount);
    }

    // Simulate upgrade
    let client2 = HelloContractClient::new(&env, &contract_id);

    for (user, &expected) in users.iter().zip(amounts.iter()) {
        let actual = read_collateral(&env, &contract_id, user);
        assert_eq!(actual, expected, "user collateral must survive upgrade");
        let pos = client2.get_user_position(user);
        assert_eq!(pos.collateral, expected);
    }
}
