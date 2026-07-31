#![cfg(test)]

use crate::cross_asset::AssetConfig;
use crate::{HelloContract, HelloContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn create_test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn make_config(
    env: &Env,
    asset: Option<Address>,
    is_isolated: bool,
    is_frozen: bool,
) -> AssetConfig {
    AssetConfig {
        asset: asset.clone(),
        collateral_factor: 8000,
        liquidation_threshold: 8500,
        reserve_factor: 500,
        max_supply: 0,
        max_borrow: 0,
        can_collateralize: true,
        can_borrow: true,
        price: 1_0000000,
        price_updated_at: env.ledger().timestamp(),
        is_isolated,
        is_frozen,
    }
}

fn setup_protocol<'a>(env: &'a Env, admin: &'a Address) -> HelloContractClient<'a> {
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(env, &contract_id);
    client.initialize(admin);
    client.initialize_ca(admin);
    client
}

#[test]
fn test_isolated_pool_borrow_within_own_collateral() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let token_a = Address::generate(&env);

    let client = setup_protocol(&env, &admin);
    client.initialize_asset(
        &Some(token_a.clone()),
        &make_config(&env, Some(token_a.clone()), true, false),
    );

    // Deposit 1000 into isolated pool
    client.cross_asset_deposit(&user, &Some(token_a.clone()), &1000);

    // Borrow 700 (≤ 1000 * 8000/10000 = 800 max)
    let result = client.try_cross_asset_borrow(&user, &Some(token_a.clone()), &700);
    assert!(
        result.is_ok(),
        "borrow within isolated pool limit should succeed"
    );
}

#[test]
fn test_isolated_pool_borrow_exceeds_own_collateral() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let token_a = Address::generate(&env);

    let client = setup_protocol(&env, &admin);
    client.initialize_asset(
        &Some(token_a.clone()),
        &make_config(&env, Some(token_a.clone()), true, false),
    );

    // Deposit only 100 into isolated pool
    client.cross_asset_deposit(&user, &Some(token_a.clone()), &100);

    // Try borrow 200 — exceeds 100 * 8000/10000 = 80 max
    let result = client.try_cross_asset_borrow(&user, &Some(token_a.clone()), &200);
    assert!(
        result.is_err(),
        "borrow exceeding isolated pool collateral should fail"
    );
}

#[test]
fn test_freeze_pool_blocks_new_deposits() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let token = Address::generate(&env);

    let client = setup_protocol(&env, &admin);
    client.initialize_asset(
        &Some(token.clone()),
        &make_config(&env, Some(token.clone()), false, false),
    );

    // Deposit works before freeze
    assert!(client
        .try_cross_asset_deposit(&user, &Some(token.clone()), &500)
        .is_ok());

    // Admin freezes pool
    client.freeze_pool(&admin, &Some(token.clone()), &true);

    // Deposit fails after freeze
    assert!(
        client
            .try_cross_asset_deposit(&user, &Some(token.clone()), &100)
            .is_err(),
        "deposit into frozen pool should fail"
    );
}

#[test]
fn test_freeze_pool_blocks_new_borrows() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let token = Address::generate(&env);

    let client = setup_protocol(&env, &admin);
    client.initialize_asset(
        &Some(token.clone()),
        &make_config(&env, Some(token.clone()), false, false),
    );

    // Deposit collateral first
    client.cross_asset_deposit(&user, &Some(token.clone()), &1000);

    // Freeze pool
    client.freeze_pool(&admin, &Some(token.clone()), &true);

    // Borrow fails while frozen
    assert!(
        client
            .try_cross_asset_borrow(&user, &Some(token.clone()), &100)
            .is_err(),
        "borrow from frozen pool should fail"
    );
}

#[test]
fn test_unfreeze_pool_restores_operations() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let token = Address::generate(&env);

    let client = setup_protocol(&env, &admin);
    client.initialize_asset(
        &Some(token.clone()),
        &make_config(&env, Some(token.clone()), false, false),
    );

    // Freeze then unfreeze
    client.freeze_pool(&admin, &Some(token.clone()), &true);
    client.freeze_pool(&admin, &Some(token.clone()), &false);

    // Deposit works again after unfreeze
    assert!(
        client
            .try_cross_asset_deposit(&user, &Some(token.clone()), &300)
            .is_ok(),
        "deposit should work after unfreeze"
    );
}
