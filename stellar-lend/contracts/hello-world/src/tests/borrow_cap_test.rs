#![cfg(test)]

use crate::cross_asset::AssetConfig;
use crate::{HelloContract, HelloContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn create_test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn collateral_config(env: &Env, asset: Option<Address>) -> AssetConfig {
    AssetConfig {
        asset: asset.clone(),
        collateral_factor: 8000,
        liquidation_threshold: 8000,
        reserve_factor: 1000,
        max_supply: 0,
        max_borrow: 0,
        can_collateralize: true,
        can_borrow: false,
        price: 1_0000000,
        price_updated_at: env.ledger().timestamp(),
        is_isolated: false,
        is_frozen: false,
    }
}

fn borrow_config(env: &Env, asset: Option<Address>, price: i128, max_borrow: i128) -> AssetConfig {
    AssetConfig {
        asset: asset.clone(),
        collateral_factor: 8000,
        liquidation_threshold: 8000,
        reserve_factor: 1000,
        max_supply: 0,
        max_borrow,
        can_collateralize: false,
        can_borrow: true,
        price,
        price_updated_at: env.ledger().timestamp(),
        is_isolated: false,
        is_frozen: false,
    }
}

fn setup_protocol<'a>(
    env: &'a Env,
    admin: &'a Address,
    collateral_asset: Option<Address>,
    borrow_asset: Option<Address>,
    borrow_cap: i128,
) -> HelloContractClient<'a> {
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(env, &contract_id);
    client.initialize(admin);
    client.initialize_ca(admin);
    // Register collateral asset (native XLM)
    client.initialize_asset(
        &collateral_asset,
        &collateral_config(env, collateral_asset.clone()),
    );
    // Register borrow asset (e.g. USDC)
    client.initialize_asset(
        &borrow_asset,
        &borrow_config(env, borrow_asset.clone(), 1_0000000, borrow_cap),
    );
    client
}

#[test]
fn test_borrow_cap_enforcement() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let usdc = Address::generate(&env);

    // Cap = 1000 USDC
    let client = setup_protocol(&env, &admin, None, Some(usdc.clone()), 1000);

    // User 1 deposits 5000 XLM as collateral
    client.cross_asset_deposit(&user1, &None, &5000);

    // User 1 borrows 600 USDC (within cap)
    client.cross_asset_borrow(&user1, &Some(usdc.clone()), &600);

    // User 2 deposits collateral
    client.cross_asset_deposit(&user2, &None, &5000);

    // User 2 tries to borrow 500 USDC (600 + 500 = 1100 > cap 1000)
    let result = client.try_cross_asset_borrow(&user2, &Some(usdc.clone()), &500);
    assert!(result.is_err(), "borrow exceeding cap should fail");
}

#[test]
fn test_borrow_cap_update_via_admin() {
    let env = create_test_env();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let usdc = Address::generate(&env);

    // Start with tight cap of 500
    let client = setup_protocol(&env, &admin, None, Some(usdc.clone()), 500);

    client.cross_asset_deposit(&user, &None, &5000);

    // Borrow 600 fails (exceeds cap 500)
    assert!(
        client
            .try_cross_asset_borrow(&user, &Some(usdc.clone()), &600)
            .is_err(),
        "borrow over cap should fail"
    );

    // Admin raises cap to 1000
    client.update_ca_config(
        &Some(usdc.clone()),
        &None,       // collateral_factor
        &None,       // liquidation_threshold
        &None,       // max_supply
        &Some(1000), // max_borrow
        &None,       // can_collateralize
        &None,       // can_borrow
    );

    // Now 600 succeeds
    assert!(
        client
            .try_cross_asset_borrow(&user, &Some(usdc.clone()), &600)
            .is_ok(),
        "borrow within raised cap should succeed"
    );
}
