//! Cross-Asset Lending Tests
//!
//! Comprehensive test suite for cross-collateral borrowing functionality.
//! Tests various scenarios including edge cases and error conditions.

use soroban_sdk::{Address, Env, Symbol};

use crate::cross_asset::{
    cross_asset_borrow, cross_asset_deposit, cross_asset_withdraw, cross_asset_liquidate,
    get_user_position_summary, AssetConfig, AssetPosition, CrossAssetError,
    initialize_asset, AssetKey,
};
use crate::deposit::DepositDataKey;
use crate::test_utils::*;

#[test]
fn test_cross_asset_deposit_success() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Some(Address::generate(&env));

    // Setup
    setup_admin(&env, &admin);
    initialize_asset(&env, asset.clone(), create_test_asset_config()).unwrap();

    // Test successful deposit
    let amount = 1000000;
    let result = cross_asset_deposit(&env, user.clone(), asset.clone(), amount);
    
    assert!(result.is_ok());
    let position = result.unwrap();
    assert_eq!(position.collateral, amount);
    
    // Verify position summary
    let summary = get_user_position_summary(&env, &user).unwrap();
    assert!(summary.total_collateral_value > 0);
}

#[test]
fn test_cross_asset_deposit_insufficient_balance() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Some(Address::generate(&env));

    setup_admin(&env, &admin);
    initialize_asset(&env, asset.clone(), create_test_asset_config()).unwrap();

    // Test deposit with insufficient balance (mock scenario)
    let amount = 1000000000; // Very large amount
    let result = cross_asset_deposit(&env, user.clone(), asset.clone(), amount);
    
    // Should fail due to insufficient balance (in real scenario)
    assert!(result.is_err());
}

#[test]
fn test_cross_asset_borrow_success() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Some(Address::generate(&env));

    // Setup
    setup_admin(&env, &admin);
    let config = create_test_asset_config();
    initialize_asset(&env, asset.clone(), config).unwrap();
    
    // Deposit collateral first
    cross_asset_deposit(&env, user.clone(), asset.clone(), 1000000).unwrap();
    
    // Test successful borrow
    let borrow_amount = 500000; // 50% of collateral value
    let result = cross_asset_borrow(&env, user.clone(), asset.clone(), borrow_amount);
    
    assert!(result.is_ok());
    let position = result.unwrap();
    assert_eq!(position.debt_principal, borrow_amount);
    
    // Verify position summary
    let summary = get_user_position_summary(&env, &user).unwrap();
    assert!(summary.total_debt_value > 0);
    assert!(summary.health_factor < 20000); // Should be less than 2.0x
}

#[test]
fn test_cross_asset_borrow_insufficient_collateral() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Some(Address::generate(&env));

    setup_admin(&env, &admin);
    initialize_asset(&env, asset.clone(), create_test_asset_config()).unwrap();
    
    // Try to borrow without sufficient collateral
    let borrow_amount = 2000000; // More than collateral value
    let result = cross_asset_borrow(&env, user.clone(), asset.clone(), borrow_amount);
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), CrossAssetError::InsufficientCollateral);
}

#[test]
fn test_cross_asset_borrow_exceeds_capacity() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Some(Address::generate(&env));

    setup_admin(&env, &admin);
    let config = create_test_asset_config();
    initialize_asset(&env, asset.clone(), config).unwrap();
    
    // Deposit collateral
    cross_asset_deposit(&env, user.clone(), asset.clone(), 1000000).unwrap();
    
    // Try to borrow more than capacity
    let borrow_amount = 1500000; // Exceeds 75% LTV
    let result = cross_asset_borrow(&env, user.clone(), asset.clone(), borrow_amount);
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), CrossAssetError::ExceedsBorrowCapacity);
}

#[test]
fn test_cross_asset_withdraw_success() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Some(Address::generate(&env));

    setup_admin(&env, &admin);
    initialize_asset(&env, asset.clone(), create_test_asset_config()).unwrap();
    
    // Deposit collateral first
    cross_asset_deposit(&env, user.clone(), asset.clone(), 1000000).unwrap();
    
    // Test successful withdrawal
    let withdraw_amount = 300000;
    let result = cross_asset_withdraw(&env, user.clone(), asset.clone(), withdraw_amount);
    
    assert!(result.is_ok());
    let position = result.unwrap();
    assert_eq!(position.collateral, 700000); // 1000000 - 300000
    
    // Verify position summary
    let summary = get_user_position_summary(&env, &user).unwrap();
    assert!(summary.total_collateral_value > 0);
}

#[test]
fn test_cross_asset_withdraw_insufficient_collateral() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Some(Address::generate(&env));

    setup_admin(&env, &admin);
    initialize_asset(&env, asset.clone(), create_test_asset_config()).unwrap();
    
    // Deposit collateral
    cross_asset_deposit(&env, user.clone(), asset.clone(), 1000000).unwrap();
    
    // Try to withdraw more than available
    let withdraw_amount = 1500000;
    let result = cross_asset_withdraw(&env, user.clone(), asset.clone(), withdraw_amount);
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), CrossAssetError::InsufficientCollateral);
}

#[test]
fn test_cross_asset_liquidate_success() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let liquidator = Address::generate(&env);
    let debt_asset = Some(Address::generate(&env));
    let collateral_asset = Some(Address::generate(&env));

    setup_admin(&env, &admin);
    initialize_asset(&env, debt_asset.clone(), create_test_asset_config()).unwrap();
    initialize_asset(&env, collateral_asset.clone(), create_test_asset_config()).unwrap();
    
    // Setup unhealthy position
    setup_unhealthy_position(&env, &user, &debt_asset, &collateral_asset);
    
    // Test successful liquidation
    let debt_to_repay = 500000;
    let collateral_to_receive = 400000;
    let result = cross_asset_liquidate(
        &env,
        liquidator.clone(),
        user.clone(),
        debt_asset.clone(),
        collateral_asset.clone(),
        debt_to_repay,
        collateral_to_receive,
    );
    
    assert!(result.is_ok());
    let actual_collateral = result.unwrap();
    assert!(actual_collateral > 0);
    
    // Verify liquidation incentive was applied
    assert!(actual_collateral < collateral_to_receive);
}

#[test]
fn test_cross_asset_liquidate_healthy_position() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let liquidator = Address::generate(&env);
    let debt_asset = Some(Address::generate(&env));
    let collateral_asset = Some(Address::generate(&env));

    setup_admin(&env, &admin);
    initialize_asset(&env, debt_asset.clone(), create_test_asset_config()).unwrap();
    initialize_asset(&env, collateral_asset.clone(), create_test_asset_config()).unwrap();
    
    // Setup healthy position
    setup_healthy_position(&env, &user, &debt_asset, &collateral_asset);
    
    // Try to liquidate healthy position
    let debt_to_repay = 500000;
    let collateral_to_receive = 400000;
    let result = cross_asset_liquidate(
        &env,
        liquidator.clone(),
        user.clone(),
        debt_asset.clone(),
        collateral_asset.clone(),
        debt_to_repay,
        collateral_to_receive,
    );
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), CrossAssetError::InsufficientCollateral);
}

#[test]
fn test_position_summary_calculation() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset1 = Some(Address::generate(&env));
    let asset2 = Some(Address::generate(&env));

    setup_admin(&env, &admin);
    initialize_asset(&env, asset1.clone(), create_test_asset_config()).unwrap();
    initialize_asset(&env, asset2.clone(), create_test_asset_config()).unwrap();
    
    // Setup multi-asset position
    cross_asset_deposit(&env, user.clone(), asset1.clone(), 1000000).unwrap();
    cross_asset_deposit(&env, user.clone(), asset2.clone(), 2000000).unwrap();
    
    // Test position summary calculation
    let summary = get_user_position_summary(&env, &user).unwrap();
    
    assert!(summary.total_collateral_value > 0);
    assert!(summary.weighted_collateral_value > 0);
    assert_eq!(summary.total_debt_value, 0); // No debt yet
    assert_eq!(summary.weighted_debt_value, 0);
    assert_eq!(summary.health_factor, i128::MAX); // Infinite health with no debt
    assert!(!summary.is_liquidatable);
    assert!(summary.borrow_capacity > 0);
}

#[test]
fn test_health_factor_calculation() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Some(Address::generate(&env));

    setup_admin(&env, &admin);
    let config = create_test_asset_config();
    initialize_asset(&env, asset.clone(), config).unwrap();
    
    // Setup position with specific collateral/debt ratio
    cross_asset_deposit(&env, user.clone(), asset.clone(), 1000000).unwrap();
    cross_asset_borrow(&env, user.clone(), asset.clone(), 500000).unwrap();
    
    // Test health factor calculation
    let summary = get_user_position_summary(&env, &user).unwrap();
    
    // With 75% LTV and 80% liquidation threshold:
    // Collateral value: 1000000, Weighted: 800000 (80% of 1000000)
    // Debt value: 500000, Weighted: 500000 (100% of debt)
    // Health factor: (800000 * 10000) / 500000 = 16000 (1.6x)
    assert_eq!(summary.health_factor, 16000);
    assert!(!summary.is_liquidatable); // 1.6x > 1.0x, so not liquidatable
}

// Helper functions

fn create_test_asset_config() -> AssetConfig {
    AssetConfig {
        asset: Some(Address::generate(&Env::default())),
        collateral_factor: 7500,  // 75% LTV
        liquidation_threshold: 8000,  // 80% liquidation threshold
        reserve_factor: 1000,  // 10% reserve
        max_supply: 10000000,
        max_borrow: 5000000,
        can_collateralize: true,
        can_borrow: true,
        price: 1000000,  // 1:1 with base asset
        price_updated_at: 0,
        is_isolated: false,
        is_frozen: false,
    }
}

fn setup_admin(env: &Env, admin: &Address) {
    env.storage()
        .persistent()
        .set(&DepositDataKey::Admin, admin);
}

fn setup_unhealthy_position(
    env: &Env,
    user: &Address,
    debt_asset: &Option<Address>,
    collateral_asset: &Option<Address>,
) {
    // Deposit collateral
    cross_asset_deposit(env, user.clone(), collateral_asset.clone(), 1000000).unwrap();
    
    // Borrow more than safe amount to make position unhealthy
    cross_asset_borrow(env, user.clone(), debt_asset.clone(), 900000).unwrap();
}

fn setup_healthy_position(
    env: &Env,
    user: &Address,
    debt_asset: &Option<Address>,
    collateral_asset: &Option<Address>,
) {
    // Deposit collateral
    cross_asset_deposit(env, user.clone(), collateral_asset.clone(), 1000000).unwrap();
    
    // Borrow safe amount
    cross_asset_borrow(env, user.clone(), debt_asset.clone(), 500000).unwrap();
}
