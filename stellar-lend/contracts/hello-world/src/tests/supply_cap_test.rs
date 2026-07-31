#![cfg(test)]

use crate::cross_asset::AssetConfig;
use crate::{HelloContract, HelloContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn create_test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn make_config(env: &Env, asset: Option<Address>, max_supply: i128) -> AssetConfig {
    AssetConfig {
        asset: asset.clone(),
        collateral_factor: 8000,
        liquidation_threshold: 8500,
        reserve_factor: 500,
        max_supply,
        max_borrow: 0,
        can_collateralize: true,
        can_borrow: false,
        price: 1_0000000,
        price_updated_at: env.ledger().timestamp(),
        is_isolated: false,
        is_frozen: false,
    }
}

fn setup_protocol<'a>(
    env: &'a Env,
    admin: &'a Address,
    asset: Option<Address>,
    max_supply: i128,
) -> HelloContractClient<'a> {
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(env, &contract_id);
    client.initialize(admin);
    client.initialize_ca(admin);
    client.initialize_asset(&asset, &make_config(env, asset.clone(), max_supply));
    client
}

#[test]
fn test_supply_cap_blocks_deposit_over_limit() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let dai = Address::generate(&env);

    let client = setup_protocol(&env, &admin, Some(dai.clone()), 1000);

    // Deposit 700 succeeds (under cap)
    client.cross_asset_deposit(&user1, &Some(dai.clone()), &700);

    // Deposit 400 would push total to 1100 > 1000 — must fail
    let result = client.try_cross_asset_deposit(&user2, &Some(dai.clone()), &400);
    assert!(result.is_err(), "deposit exceeding supply cap should fail");
}

#[test]
fn test_supply_cap_at_exact_boundary() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let dai = Address::generate(&env);

    let client = setup_protocol(&env, &admin, Some(dai.clone()), 500);

    // Deposit exactly at cap must succeed
    let result = client.try_cross_asset_deposit(&user, &Some(dai.clone()), &500);
    assert!(result.is_ok(), "deposit at cap should succeed");
}

#[test]
fn test_supply_cap_update_allows_more_deposits() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let dai = Address::generate(&env);

    let client = setup_protocol(&env, &admin, Some(dai.clone()), 300);

    // 400 fails (over cap)
    assert!(
        client
            .try_cross_asset_deposit(&user, &Some(dai.clone()), &400)
            .is_err(),
        "deposit over cap should fail"
    );

    // Admin raises cap to 1000
    client.update_ca_config(
        &Some(dai.clone()),
        &None,
        &None,
        &Some(1000), // new max_supply
        &None,
        &None,
        &None,
    );

    // Now 400 succeeds
    assert!(
        client
            .try_cross_asset_deposit(&user, &Some(dai.clone()), &400)
            .is_ok(),
        "deposit within raised cap should succeed"
    );
}

#[test]
fn test_supply_headroom_analytics() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let dai = Address::generate(&env);

    let client = setup_protocol(&env, &admin, Some(dai.clone()), 1000);

    // Before any deposit: headroom = full cap
    let (avail, cap, current) = client.get_supply_headroom(&Some(dai.clone()));
    assert_eq!(cap, 1000);
    assert_eq!(current, 0);
    assert_eq!(avail, 1000);

    // Deposit 300
    client.cross_asset_deposit(&user, &Some(dai.clone()), &300);

    let (avail2, cap2, current2) = client.get_supply_headroom(&Some(dai.clone()));
    assert_eq!(cap2, 1000);
    assert_eq!(current2, 300);
    assert_eq!(avail2, 700);
}
