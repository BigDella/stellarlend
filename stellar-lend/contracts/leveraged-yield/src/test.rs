#![cfg(test)]
use crate::{
    LeverageConfig, LeveragedPosition, LeveragedYield, LeveragedYieldClient,
    LeveragedYieldError,
};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, Address, LeveragedYieldClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(LeveragedYield, ());
    let client = LeveragedYieldClient::new(&env, &contract_id);

    let config = LeverageConfig {
        min_leverage_bps: 10_000,
        max_leverage_bps: 50_000,
        target_ltv_bps: 8_000,
        min_health_factor: 15_000,
        auto_deleverage_threshold: 12_000,
        deleverage_target_bps: 20_000,
        rebalance_tolerance_bps: 500,
    };

    client.initialize(&admin, &config);
    (env, admin, client)
}

#[test]
fn test_initialize() {
    let (env, admin, client) = setup();
    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, Some(admin));
}

#[test]
fn test_open_position() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let position_id = client.open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &100_000,
        &20_000,
        &15_000,
    );

    let position = client.get_position(&position_id).unwrap();
    assert_eq!(position.owner, owner);
    assert_eq!(position.collateral_amount, 100_000);
    assert!(position.borrowed_amount > 0);
    assert_eq!(position.leverage_bps, 20_000);
    assert!(position.active);
}

#[test]
fn test_open_position_invalid_leverage() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let result = client.try_open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &100_000,
        &5_000,
        &15_000,
    );
    assert_eq!(result, Err(Ok(LeveragedYieldError::LeverageOutOfRange)));
}

#[test]
fn test_open_position_leverage_above_max() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let result = client.try_open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &100_000,
        &60_000,
        &15_000,
    );
    assert_eq!(result, Err(Ok(LeveragedYieldError::LeverageOutOfRange)));
}

#[test]
fn test_close_position() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let position_id = client.open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &100_000,
        &20_000,
        &15_000,
    );

    let closed = client.close_position(&owner, &position_id, &0);
    assert!(!closed.active);

    let position = client.get_position(&position_id).unwrap();
    assert!(!position.active);
}

#[test]
fn test_close_position_unauthorized() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let other = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let position_id = client.open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &100_000,
        &20_000,
        &15_000,
    );

    _env.mock_all_auths();
    let result = client.try_close_position(&other, &position_id, &0);
    assert_eq!(result, Err(Ok(LeveragedYieldError::Unauthorized)));
}

#[test]
fn test_adjust_leverage() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let position_id = client.open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &200_000,
        &10_000,
        &15_000,
    );

    let adjusted = client.adjust_leverage(
        &owner,
        &position_id,
        &15_000,
        &100_000,
        &15_000,
    );

    assert_eq!(adjusted.leverage_bps, 15_000);
    assert_eq!(adjusted.collateral_amount, 300_000);
}

#[test]
fn test_deleverage() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let position_id = client.open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &100_000,
        &30_000,
        &15_000,
    );

    let position = client.get_position(&position_id).unwrap();
    let half_debt = position.borrowed_amount / 2;

    let deleveraged = client.deleverage(
        &owner,
        &position_id,
        &20_000,
        &half_debt,
        &0,
    );

    assert_eq!(deleveraged.leverage_bps, 20_000);
    assert!(deleveraged.borrowed_amount < position.borrowed_amount);
}

#[test]
fn test_get_health() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let position_id = client.open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &100_000,
        &10_000,
        &15_000,
    );

    let health = client.get_health(&position_id);
    assert!(health.health_factor > 0);
    assert_eq!(health.collateral_value, 100_000);
    assert!(!health.is_liquidatable);
}

#[test]
fn test_auto_deleverage() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let position_id = client.open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &100_000,
        &20_000,
        &15_000,
    );

    let result = client.try_auto_deleverage(&position_id);
    assert!(result.is_err());
}

#[test]
fn test_check_and_deleverage_healthy() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let position_id = client.open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &100_000,
        &10_000,
        &15_000,
    );

    let result = client.check_and_deleverage(&position_id);
    assert!(result.is_none());
}

#[test]
fn test_get_owner_positions() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let id1 = client.open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &100_000,
        &20_000,
        &15_000,
    );
    let id2 = client.open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &50_000,
        &15_000,
        &15_000,
    );

    let positions = client.get_owner_positions(&owner);
    assert_eq!(positions.len(), 2);
    assert_eq!(positions.get(0).unwrap(), id1);
    assert_eq!(positions.get(1).unwrap(), id2);
}

#[test]
fn test_set_config() {
    let (_env, admin, client) = setup();

    let new_config = LeverageConfig {
        min_leverage_bps: 10_000,
        max_leverage_bps: 30_000,
        target_ltv_bps: 7_000,
        min_health_factor: 12_000,
        auto_deleverage_threshold: 10_000,
        deleverage_target_bps: 15_000,
        rebalance_tolerance_bps: 300,
    };

    client.set_config(&admin, &new_config);
    let stored = client.get_config();
    assert_eq!(stored.max_leverage_bps, 30_000);
}

#[test]
fn test_adjust_leverage_invalid_range() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let position_id = client.open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &500_000,
        &10_000,
        &15_000,
    );

    let result = client.try_adjust_leverage(
        &owner,
        &position_id,
        &60_000,
        &100_000,
        &15_000,
    );
    assert_eq!(result, Err(Ok(LeveragedYieldError::LeverageOutOfRange)));
}

#[test]
fn test_adjust_leverage_unauthorized() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let other = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let position_id = client.open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &500_000,
        &10_000,
        &15_000,
    );

    let result = client.try_adjust_leverage(
        &other,
        &position_id,
        &15_000,
        &100_000,
        &15_000,
    );
    assert_eq!(result, Err(Ok(LeveragedYieldError::Unauthorized)));
}

#[test]
fn test_adjust_leverage_closed_position() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let position_id = client.open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &500_000,
        &10_000,
        &15_000,
    );

    client.close_position(&owner, &position_id, &0);

    let result = client.try_adjust_leverage(
        &owner,
        &position_id,
        &15_000,
        &100_000,
        &15_000,
    );
    assert_eq!(result, Err(Ok(LeveragedYieldError::PositionNotActive)));
}

#[test]
fn test_open_position_zero_collateral() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let pool = Address::generate(&_env);
    let deposit_asset = Address::generate(&_env);
    let borrow_asset = Address::generate(&_env);

    let result = client.try_open_position(
        &owner,
        &pool,
        &deposit_asset,
        &borrow_asset,
        &0,
        &20_000,
        &15_000,
    );
    assert_eq!(result, Err(Ok(LeveragedYieldError::InvalidAmount)));
}
