use super::*;

use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

use delegation_registry::DelegationRegistry;

#[test]
fn test_execute_delegated_batch_deposit_and_borrow() {
    let env = Env::default();
    env.mock_all_auths();

    let lending_id = env.register(LendingContract, ());
    let lending = LendingContractClient::new(&env, &lending_id);

    let registry_id = env.register(DelegationRegistry, ());
    let registry = delegation_registry::DelegationRegistryClient::new(&env, &registry_id);

    let admin = Address::generate(&env);
    lending.initialize(&admin, &1_000_000_000, &1000);
    lending.initialize_deposit_settings(&1_000_000_000, &100);
    lending.initialize_withdraw_settings(&100);

    lending.set_delegation_registry(&admin, &registry_id);

    let user = Address::generate(&env);
    let delegate = Address::generate(&env);

    let asset = Address::generate(&env);
    let collateral_asset = Address::generate(&env);

    // Grant delegate permission for deposit + borrow
    let permissions = 1u32 | 4u32;
    registry.grant(&user, &delegate, &permissions, &0u64);

    let calls = Vec::from_array(
        &env,
        [
            MetaCall {
                action: MetaAction::Deposit,
                asset: asset.clone(),
                amount: 10_000,
                collateral_asset: None,
                collateral_amount: None,
            },
            MetaCall {
                action: MetaAction::Borrow,
                asset: asset.clone(),
                amount: 1000,
                collateral_asset: Some(collateral_asset.clone()),
                collateral_amount: Some(2000),
            },
        ],
    );

    // nonce starts at 0
    assert_eq!(lending.try_execute_delegated(&user, &delegate, &1u64, &0u64, &calls), Err(Ok(MetaTxError::InvalidNonce)));

    lending.execute_delegated(&user, &delegate, &0u64, &0u64, &calls);

    // replay should fail due to nonce increment
    assert_eq!(lending.try_execute_delegated(&user, &delegate, &0u64, &0u64, &calls), Err(Ok(MetaTxError::InvalidNonce)));
}

#[test]
fn test_execute_delegated_expired_deadline() {
    let env = Env::default();
    env.mock_all_auths();

    let lending_id = env.register(LendingContract, ());
    let lending = LendingContractClient::new(&env, &lending_id);

    let registry_id = env.register(DelegationRegistry, ());
    let registry = delegation_registry::DelegationRegistryClient::new(&env, &registry_id);

    let admin = Address::generate(&env);
    lending.initialize(&admin, &1_000_000_000, &1000);
    lending.initialize_deposit_settings(&1_000_000_000, &100);

    lending.set_delegation_registry(&admin, &registry_id);

    let user = Address::generate(&env);
    let delegate = Address::generate(&env);

    let asset = Address::generate(&env);

    let permissions = 1u32;
    registry.grant(&user, &delegate, &permissions, &0u64);

    let calls = Vec::from_array(
        &env,
        [MetaCall {
            action: MetaAction::Deposit,
            asset: asset.clone(),
            amount: 10_000,
            collateral_asset: None,
            collateral_amount: None,
        }],
    );

    // advance time
    env.ledger().set_timestamp(100);
    assert_eq!(
        lending.try_execute_delegated(&user, &delegate, &0u64, &50u64, &calls),
        Err(Ok(MetaTxError::Expired))
    );
}

#[test]
fn test_delegated_supply_cap_blocks_whale_deposit() {
    let env = Env::default();
    env.mock_all_auths();

    let lending_id = env.register(LendingContract, ());
    let lending = LendingContractClient::new(&env, &lending_id);

    let registry_id = env.register(DelegationRegistry, ());
    let registry = delegation_registry::DelegationRegistryClient::new(&env, &registry_id);

    let admin = Address::generate(&env);
    lending.initialize(&admin, &1_000_000_000, &1000);
    lending.initialize_deposit_settings(&1_000_000_000, &100);
    lending.set_delegation_registry(&admin, &registry_id);

    let user = Address::generate(&env);
    let delegate = Address::generate(&env);
    let asset = Address::generate(&env);

    registry.grant(&user, &delegate, &1u32, &0u64);

    lending.configure_caps(
        &admin,
        &asset,
        &MetaCapConfig {
            user_supply_cap_bps: 5_000,
            user_borrow_cap_bps: 5_000,
            pool_supply_cap: 10_000,
            pool_borrow_cap: 10_000,
        },
    );

    let calls = Vec::from_array(
        &env,
        [MetaCall {
            action: MetaAction::Deposit,
            asset: asset.clone(),
            amount: 6_000,
            collateral_asset: None,
            collateral_amount: None,
        }],
    );

    assert_eq!(
        lending.try_execute_delegated(&user, &delegate, &0u64, &0u64, &calls),
        Err(Ok(MetaTxError::UserSupplyCapExceeded))
    );
}

#[test]
fn test_delegated_borrow_cap_uses_collateral_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let lending_id = env.register(LendingContract, ());
    let lending = LendingContractClient::new(&env, &lending_id);

    let registry_id = env.register(DelegationRegistry, ());
    let registry = delegation_registry::DelegationRegistryClient::new(&env, &registry_id);

    let admin = Address::generate(&env);
    lending.initialize(&admin, &1_000_000_000, &1000);
    lending.initialize_deposit_settings(&1_000_000_000, &100);
    lending.set_delegation_registry(&admin, &registry_id);

    let user = Address::generate(&env);
    let delegate = Address::generate(&env);
    let asset = Address::generate(&env);
    let collateral_asset = Address::generate(&env);

    registry.grant(&user, &delegate, &4u32, &0u64);

    lending.configure_caps(
        &admin,
        &asset,
        &MetaCapConfig {
            user_supply_cap_bps: 10_000,
            user_borrow_cap_bps: 2_500,
            pool_supply_cap: 1_000_000,
            pool_borrow_cap: 1_000_000,
        },
    );

    let calls = Vec::from_array(
        &env,
        [MetaCall {
            action: MetaAction::Borrow,
            asset: asset.clone(),
            amount: 600,
            collateral_asset: Some(collateral_asset.clone()),
            collateral_amount: Some(2_000),
        }],
    );

    assert_eq!(
        lending.try_execute_delegated(&user, &delegate, &0u64, &0u64, &calls),
        Err(Ok(MetaTxError::UserBorrowCapExceeded))
    );
}
