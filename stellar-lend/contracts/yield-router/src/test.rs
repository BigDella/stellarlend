#![cfg(test)]
use crate::{RouterError, YieldRouter, YieldRouterClient};
use pool_interfaces::{RiskProfile, RouterConfig};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, Address, YieldRouterClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(YieldRouter, ());
    let client = YieldRouterClient::new(&env, &contract_id);

    let config = RouterConfig {
        min_apy_differential_bps: 100,
        max_pools: 5,
        rebalance_cooldown_secs: 3600,
        slippage_tolerance_bps: 100,
        default_risk_profile: RiskProfile::Moderate,
    };

    client.initialize(&admin, &config);
    (env, admin, client)
}

#[test]
fn test_initialize() {
    let (_env, admin, client) = setup();
    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, Some(admin));
}

#[test]
fn test_register_pool() {
    let (_env, admin, client) = setup();
    let pool = Address::generate(&_env);
    let asset = Address::generate(&_env);
    client.register_pool(&admin, &pool, &asset);
    assert!(client.is_pool_registered(&pool));
}

#[test]
fn test_register_multiple_pools() {
    let (_env, admin, client) = setup();
    let asset = Address::generate(&_env);

    for i in 0..5 {
        let pool = Address::generate(&_env);
        client.register_pool(&admin, &pool, &asset);
    }

    let pools = client.get_registered_pools();
    assert_eq!(pools.len(), 5);
}

#[test]
fn test_register_pool_max_limit() {
    let (_env, admin, client) = setup();
    let asset = Address::generate(&_env);

    for i in 0..5 {
        let pool = Address::generate(&_env);
        client.register_pool(&admin, &pool, &asset);
    }

    let extra_pool = Address::generate(&_env);
    let result = client.try_register_pool(&admin, &extra_pool, &asset);
    assert_eq!(result, Err(Ok(RouterError::MaxPoolsExceeded)));
}

#[test]
fn test_deposit_no_pools() {
    let (_env, _admin, client) = setup();
    let user = Address::generate(&_env);
    let asset = Address::generate(&_env);
    let result = client.try_deposit(
        &user,
        &asset,
        &1000,
        &RiskProfile::Conservative,
        &0,
    );
    assert_eq!(result, Err(Ok(RouterError::NoPoolsConfigured)));
}

#[test]
fn test_deposit_with_pools() {
    let (_env, admin, client) = setup();
    let user = Address::generate(&_env);
    let asset = Address::generate(&_env);
    let pool = Address::generate(&_env);

    client.register_pool(&admin, &pool, &asset);
    let result = client.deposit(
        &user,
        &asset,
        &1000,
        &RiskProfile::Moderate,
        &0,
    );
    assert!(result > 0);
}

#[test]
fn test_withdraw_no_position() {
    let (_env, _admin, client) = setup();
    let user = Address::generate(&_env);
    let asset = Address::generate(&_env);
    let result = client.try_withdraw(&user, &asset, &100, &0);
    assert_eq!(result, Err(Ok(RouterError::UserPositionNotFound)));
}

#[test]
fn test_get_user_position_no_position() {
    let (_env, _admin, client) = setup();
    let user = Address::generate(&_env);
    let asset = Address::generate(&_env);
    let position = client.get_user_position(&user, &asset);
    assert!(position.is_none());
}

#[test]
fn test_rebalance_no_position() {
    let (_env, _admin, client) = setup();
    let user = Address::generate(&_env);
    let asset = Address::generate(&_env);
    let result = client.try_rebalance(
        &user,
        &asset,
        &Some(RiskProfile::Conservative),
        &0,
    );
    assert_eq!(result, Err(Ok(RouterError::UserPositionNotFound)));
}

#[test]
fn test_set_paused() {
    let (_env, admin, client) = setup();
    client.set_paused(&admin, &true);
    let user = Address::generate(&_env);
    let asset = Address::generate(&_env);

    let result = client.try_deposit(
        &user,
        &asset,
        &1000,
        &RiskProfile::Conservative,
        &0,
    );
    assert_eq!(result, Err(Ok(RouterError::DepositPaused)));
}

#[test]
fn test_unauthorized_set_config() {
    let (env, _admin, client) = setup();
    let bad_admin = Address::generate(&env);
    let config = client.get_config();
    let result = client.try_set_config(&bad_admin, &config);
    assert_eq!(result, Err(Ok(RouterError::Unauthorized)));
}

#[test]
fn test_get_allocations_empty() {
    let (_env, _admin, client) = setup();
    let user = Address::generate(&_env);
    let asset = Address::generate(&_env);
    let allocs = client.get_allocations(&user, &asset);
    assert_eq!(allocs.len(), 0);
}

#[test]
fn test_config_update() {
    let (_env, admin, client) = setup();
    let new_config = RouterConfig {
        min_apy_differential_bps: 200,
        max_pools: 3,
        rebalance_cooldown_secs: 7200,
        slippage_tolerance_bps: 50,
        default_risk_profile: RiskProfile::Aggressive,
    };
    client.set_config(&admin, &new_config);
    let stored = client.get_config();
    assert_eq!(stored.max_pools, 3);
    assert_eq!(stored.min_apy_differential_bps, 200);
}
