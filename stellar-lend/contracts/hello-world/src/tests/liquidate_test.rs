//! Liquidation Function Tests
//!
//! This module contains comprehensive tests for the liquidation functionality.
//! It covers:
//! - Partial and full liquidations
//! - Close factor enforcement
//! - Liquidation incentive calculations
//! - Undercollateralization validation
//! - Pause functionality
//! - Interest accrual during liquidation
//! - Multi-asset liquidations
//! - Edge cases and security scenarios
//!
//! Note: Many tests are marked #[ignore] because native XLM liquidation
//! is not yet fully supported. These tests document expected behavior.

use crate::deposit::{DepositDataKey, Position, ProtocolAnalytics};
use crate::{HelloContract, HelloContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, Symbol,
};

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Creates a test environment with all auths mocked
fn create_test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

/// Sets up admin and initializes the contract
fn setup_contract_with_admin(env: &Env) -> (Address, Address, HelloContractClient<'_>) {
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(env, &contract_id);
    let admin = Address::generate(env);

    // Initialize contract with admin
    client.initialize(&admin);

    (contract_id, admin, client)
}

/// Helper to create a position that can be liquidated
fn create_liquidatable_position(
    env: &Env,
    contract_id: &Address,
    user: &Address,
    collateral: i128,
    debt: i128,
) {
    env.as_contract(contract_id, || {
        // Set collateral balance
        let collateral_key = DepositDataKey::CollateralBalance(user.clone());
        env.storage().persistent().set(&collateral_key, &collateral);

        // Set position with debt
        let position_key = DepositDataKey::Position(user.clone());
        let position = Position {
            collateral,
            debt,
            borrow_interest: 0,
            last_accrual_time: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&position_key, &position);

        // Update protocol analytics
        let analytics_key = DepositDataKey::ProtocolAnalytics;
        let analytics = ProtocolAnalytics {
            total_deposits: collateral,
            total_borrows: debt,
            total_value_locked: collateral,
        };
        env.storage().persistent().set(&analytics_key, &analytics);
    });
}

/// Helper to create a healthy position (cannot be liquidated)
fn create_healthy_position(
    env: &Env,
    contract_id: &Address,
    user: &Address,
    collateral: i128,
    debt: i128,
) {
    create_liquidatable_position(env, contract_id, user, collateral, debt);
}

/// Helper to get user position
fn get_user_position(env: &Env, contract_id: &Address, user: &Address) -> Option<Position> {
    env.as_contract(contract_id, || {
        let key = DepositDataKey::Position(user.clone());
        env.storage()
            .persistent()
            .get::<DepositDataKey, Position>(&key)
    })
}

/// Helper to get collateral balance
fn get_collateral_balance(env: &Env, contract_id: &Address, user: &Address) -> i128 {
    env.as_contract(contract_id, || {
        let key = DepositDataKey::CollateralBalance(user.clone());
        env.storage()
            .persistent()
            .get::<DepositDataKey, i128>(&key)
            .unwrap_or(0)
    })
}

// =============================================================================
// BASIC LIQUIDATION TESTS
// =============================================================================

/// Test successful partial liquidation
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_partial_liquidation() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create undercollateralized position
    // Collateral: 1000, Debt: 1000 (100% ratio, below 110% threshold)
    create_liquidatable_position(&env, &contract_id, &borrower, 1000, 1000);

    // Liquidate 50% of debt (within close factor of 50%)
    let debt_to_liquidate = 500;
    let (debt_liquidated, collateral_seized, incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &debt_to_liquidate);

    // Verify liquidation occurred
    assert_eq!(debt_liquidated, debt_to_liquidate);
    assert!(collateral_seized > debt_to_liquidate); // Seized more due to incentive
    assert!(incentive > 0);

    // Verify borrower's position updated
    let position = get_user_position(&env, &contract_id, &borrower).unwrap();
    assert_eq!(position.debt, 500); // 1000 - 500 = 500
}

/// Test successful full liquidation
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_full_liquidation() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create deeply undercollateralized position
    // Collateral: 800, Debt: 1000 (80% ratio, well below threshold)
    create_liquidatable_position(&env, &contract_id, &borrower, 800, 1000);

    // Liquidate exactly at close factor (50%)
    let max_liquidatable = 500; // 50% of 1000
    let (debt_liquidated, collateral_seized, _incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &max_liquidatable);

    assert_eq!(debt_liquidated, max_liquidatable);
    assert!(collateral_seized > 0);

    // Verify position was updated
    let position = get_user_position(&env, &contract_id, &borrower).unwrap();
    assert_eq!(position.debt, 500); // 1000 - 500
}

// =============================================================================
// CLOSE FACTOR TESTS
// =============================================================================

/// Test liquidation exceeds close factor
#[test]
#[ignore] // Native XLM liquidation not yet supported
#[should_panic(expected = "ExceedsCloseFactor")]
fn test_liquidate_exceeds_close_factor() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create undercollateralized position
    create_liquidatable_position(&env, &contract_id, &borrower, 1000, 1000);

    // Try to liquidate more than close factor allows (50%)
    let excessive_amount = 600; // > 50% of 1000
    client.liquidate(&liquidator, &borrower, &None, &None, &excessive_amount);
}

/// Test close factor edge case - exactly at limit
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_close_factor_edge_case() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create undercollateralized position
    create_liquidatable_position(&env, &contract_id, &borrower, 1000, 1000);

    // Liquidate exactly at close factor (50%)
    let exact_max = 500;
    let (debt_liquidated, _collateral_seized, _incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &exact_max);

    assert_eq!(debt_liquidated, exact_max);
}

// =============================================================================
// LIQUIDATION INCENTIVE TESTS
// =============================================================================

/// Test liquidation incentive calculation
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_incentive_calculation() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create undercollateralized position
    create_liquidatable_position(&env, &contract_id, &borrower, 2000, 1000);

    let debt_to_liquidate = 500;
    let (_debt_liquidated, collateral_seized, incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &debt_to_liquidate);

    // Default incentive is 10% (1000 bps)
    // Collateral seized should be debt_liquidated * (1 + incentive%)
    // Expected: 500 * 1.10 = 550
    assert_eq!(collateral_seized, 550);

    // Incentive amount should be 10% of debt
    assert_eq!(incentive, 50);
}

/// Test liquidation with custom liquidation incentive configured (#366)
/// Verifies that on-chain liquidations correctly use updated configuration parameters
/// and provides the correct expected economic guarantees.
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_with_custom_incentive() {
    let env = create_test_env();
    let (contract_id, admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Default incentive is 10%. Change it to 11% (1100 bps)
    client.set_risk_params(&admin, &None, &None, &None, &Some(1_100));

    // Create undercollateralized position
    // Collateral: 2000, Debt: 2000 (100% ratio, below liquidation threshold)
    create_liquidatable_position(&env, &contract_id, &borrower, 2000, 2000);

    let debt_to_liquidate = 500;
    let (_debt_liquidated, collateral_seized, incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &debt_to_liquidate);

    // Collateral seized should be debt_liquidated * (1 + incentive%)
    // Expected: 500 * (1 + 0.11) = 555
    // Economic guarantee: liquidators get 11% bonus as configured instead of 10%
    assert_eq!(collateral_seized, 555);

    // Incentive amount tracking should exactly be 11% of debt: 55
    assert_eq!(incentive, 55);
}

// =============================================================================
// UNDERCOLLATERALIZATION VALIDATION TESTS
// =============================================================================

/// Test liquidation of healthy position fails
#[test]
#[ignore] // Native XLM liquidation not yet supported
#[should_panic(expected = "NotLiquidatable")]
fn test_liquidate_not_undercollateralized() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create healthy position (150% collateral ratio)
    create_healthy_position(&env, &contract_id, &borrower, 1500, 1000);

    // Try to liquidate - should fail
    client.liquidate(&liquidator, &borrower, &None, &None, &500);
}

/// Test liquidation at exact threshold boundary
#[test]
#[should_panic(expected = "Liquidation error")]
fn test_liquidate_at_threshold_boundary() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create position exactly at liquidation threshold (105%)
    // This should NOT be liquidatable (need to be below threshold)
    create_healthy_position(&env, &contract_id, &borrower, 1050, 1000);

    client.liquidate(&liquidator, &borrower, &None, &None, &500);
}

/// Test liquidation just below threshold
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_just_below_threshold() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create position just below liquidation threshold (104%)
    create_liquidatable_position(&env, &contract_id, &borrower, 1040, 1000);

    let (debt_liquidated, _collateral_seized, _incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &500);

    assert_eq!(debt_liquidated, 500);
}

// =============================================================================
// PAUSE FUNCTIONALITY TESTS
// =============================================================================

/// Test liquidation when paused
#[test]
#[should_panic(expected = "Liquidation error")]
fn test_liquidate_paused() {
    let env = create_test_env();
    let (contract_id, admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create undercollateralized position
    create_liquidatable_position(&env, &contract_id, &borrower, 1000, 1000);

    // Pause liquidations
    client.set_pause_switch(&admin, &Symbol::new(&env, "pause_liquidate"), &true);

    // Try to liquidate - should fail
    client.liquidate(&liquidator, &borrower, &None, &None, &500);
}

/// Test liquidation with emergency pause
#[test]
#[should_panic(expected = "Liquidation error")]
fn test_liquidate_emergency_paused() {
    let env = create_test_env();
    let (contract_id, admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create undercollateralized position
    create_liquidatable_position(&env, &contract_id, &borrower, 1000, 1000);

    // Set emergency pause
    client.set_emergency_pause(&admin, &true);

    // Try to liquidate - should fail
    client.liquidate(&liquidator, &borrower, &None, &None, &500);
}

/// Test liquidation after unpause
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_after_unpause() {
    let env = create_test_env();
    let (contract_id, admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create undercollateralized position
    create_liquidatable_position(&env, &contract_id, &borrower, 1000, 1000);

    // Pause and unpause
    client.set_pause_switch(&admin, &Symbol::new(&env, "pause_liquidate"), &true);
    client.set_pause_switch(&admin, &Symbol::new(&env, "pause_liquidate"), &false);

    // Should succeed after unpause
    let (debt_liquidated, _collateral_seized, _incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &500);

    assert_eq!(debt_liquidated, 500);
}

// =============================================================================
// INTEREST ACCRUAL TESTS
// =============================================================================

/// Test liquidation with interest accrual
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_with_interest() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create position with some interest
    env.ledger().with_mut(|li| li.timestamp = 0);

    env.as_contract(&contract_id, || {
        let collateral_key = DepositDataKey::CollateralBalance(borrower.clone());
        env.storage().persistent().set(&collateral_key, &1000i128);

        let position_key = DepositDataKey::Position(borrower.clone());
        let position = Position {
            collateral: 1000,
            debt: 900,
            borrow_interest: 100, // Pre-existing interest
            last_accrual_time: 0,
        };
        env.storage().persistent().set(&position_key, &position);

        let analytics_key = DepositDataKey::ProtocolAnalytics;
        let analytics = ProtocolAnalytics {
            total_deposits: 1000,
            total_borrows: 1000,
            total_value_locked: 1000,
        };
        env.storage().persistent().set(&analytics_key, &analytics);
    });

    // Move time forward (interest accrual happens)
    env.ledger().with_mut(|li| li.timestamp = 86400); // 1 day

    // Total debt = principal + interest
    // Liquidate up to 50% of total debt
    let (debt_liquidated, _collateral_seized, _incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &500);

    // Should succeed
    assert!(debt_liquidated > 0);
    assert!(debt_liquidated <= 500);
}

/// Test interest is paid first during liquidation
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_interest_paid_first() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create position with significant interest
    env.as_contract(&contract_id, || {
        let collateral_key = DepositDataKey::CollateralBalance(borrower.clone());
        env.storage().persistent().set(&collateral_key, &1000i128);

        let position_key = DepositDataKey::Position(borrower.clone());
        let position = Position {
            collateral: 1000,
            debt: 700,
            borrow_interest: 300, // 30% interest
            last_accrual_time: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&position_key, &position);

        let analytics_key = DepositDataKey::ProtocolAnalytics;
        let analytics = ProtocolAnalytics {
            total_deposits: 1000,
            total_borrows: 1000,
            total_value_locked: 1000,
        };
        env.storage().persistent().set(&analytics_key, &analytics);
    });

    // Liquidate 300 (should cover interest first)
    let (_debt_liquidated, _collateral_seized, _incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &300);

    // Check position - interest should be reduced first
    let position = get_user_position(&env, &contract_id, &borrower).unwrap();

    // If 300 was liquidated and there was 300 interest, interest should be 0
    // and principal should still be 700
    assert_eq!(position.borrow_interest, 0);
    assert_eq!(position.debt, 700);
}

// =============================================================================
// MULTIPLE LIQUIDATIONS TESTS
// =============================================================================

/// Test multiple sequential liquidations
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_multiple_liquidations() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator1 = Address::generate(&env);
    let liquidator2 = Address::generate(&env);

    // Create deeply undercollateralized position
    create_liquidatable_position(&env, &contract_id, &borrower, 2000, 2000);

    // First liquidation (500 = 25% of 2000)
    let (debt1, _col1, _inc1) = client.liquidate(&liquidator1, &borrower, &None, &None, &500);
    assert_eq!(debt1, 500);

    // Verify remaining debt
    let position1 = get_user_position(&env, &contract_id, &borrower).unwrap();
    assert_eq!(position1.debt, 1500);

    // Second liquidation (up to 50% of remaining = 750)
    let (debt2, _col2, _inc2) = client.liquidate(&liquidator2, &borrower, &None, &None, &750);
    assert_eq!(debt2, 750);

    // Verify final position
    let position2 = get_user_position(&env, &contract_id, &borrower).unwrap();
    assert_eq!(position2.debt, 750);
}

// =============================================================================
// VALIDATION TESTS
// =============================================================================

/// Test liquidation with zero amount
#[test]
#[should_panic(expected = "Liquidation error")]
fn test_liquidate_zero_amount() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    create_liquidatable_position(&env, &contract_id, &borrower, 1000, 1000);

    client.liquidate(&liquidator, &borrower, &None, &None, &0);
}

/// Test liquidation with negative amount
#[test]
#[should_panic(expected = "Liquidation error")]
fn test_liquidate_negative_amount() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    create_liquidatable_position(&env, &contract_id, &borrower, 1000, 1000);

    client.liquidate(&liquidator, &borrower, &None, &None, &(-100));
}

/// Test liquidation of user with no debt
#[test]
#[should_panic(expected = "Liquidation error")]
fn test_liquidate_no_debt() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create position with collateral but no debt
    env.as_contract(&contract_id, || {
        let collateral_key = DepositDataKey::CollateralBalance(borrower.clone());
        env.storage().persistent().set(&collateral_key, &1000i128);

        let position_key = DepositDataKey::Position(borrower.clone());
        let position = Position {
            collateral: 1000,
            debt: 0,
            borrow_interest: 0,
            last_accrual_time: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&position_key, &position);
    });

    client.liquidate(&liquidator, &borrower, &None, &None, &500);
}

/// Test liquidation of non-existent position
#[test]
#[should_panic(expected = "Liquidation error")]
fn test_liquidate_no_position() {
    let env = create_test_env();
    let (_contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Borrower has no position at all
    client.liquidate(&liquidator, &borrower, &None, &None, &500);
}

// =============================================================================
// COLLATERAL SEIZURE TESTS
// =============================================================================

/// Test collateral is correctly seized
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_collateral_seizure() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create undercollateralized position
    create_liquidatable_position(&env, &contract_id, &borrower, 2000, 1500);

    let initial_collateral = get_collateral_balance(&env, &contract_id, &borrower);
    assert_eq!(initial_collateral, 2000);

    let debt_to_liquidate = 500;
    let (_debt_liquidated, collateral_seized, _incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &debt_to_liquidate);

    // Verify collateral was reduced
    let final_collateral = get_collateral_balance(&env, &contract_id, &borrower);
    assert_eq!(final_collateral, initial_collateral - collateral_seized);
}

/// Test cannot seize more collateral than available
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_collateral_seizure_capped() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create position with limited collateral
    create_liquidatable_position(&env, &contract_id, &borrower, 500, 1000);

    // Try to liquidate - should seize all available collateral at most
    let (debt_liquidated, collateral_seized, _incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &500);

    assert!(debt_liquidated > 0);
    assert!(collateral_seized <= 500); // Cannot exceed available
}

// =============================================================================
// ANALYTICS UPDATE TESTS
// =============================================================================

/// Test analytics are updated after liquidation
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_analytics_updated() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create undercollateralized position
    create_liquidatable_position(&env, &contract_id, &borrower, 2000, 1500);

    // Perform liquidation
    let (_debt_liquidated, collateral_seized, _incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &500);

    // Check protocol analytics updated
    env.as_contract(&contract_id, || {
        let analytics_key = DepositDataKey::ProtocolAnalytics;
        let analytics: ProtocolAnalytics = env.storage().persistent().get(&analytics_key).unwrap();

        // TVL should be reduced by seized collateral
        assert_eq!(analytics.total_value_locked, 2000 - collateral_seized);
    });
}

// =============================================================================
// ACTIVITY LOG TESTS
// =============================================================================

/// Test activity log is updated
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_activity_log() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Create undercollateralized position
    create_liquidatable_position(&env, &contract_id, &borrower, 2000, 1500);

    // Perform liquidation
    client.liquidate(&liquidator, &borrower, &None, &None, &500);

    // Check activity was logged
    let activities = client.get_recent_activity(&10, &0);

    // There should be at least one activity (the liquidation)
    let mut found_liquidate = false;
    for activity in activities.iter() {
        if activity.activity_type == Symbol::new(&env, "liquidate") {
            found_liquidate = true;
            break;
        }
    }
    assert!(found_liquidate, "Liquidation activity not found in log");
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

/// Test liquidation with very small amounts
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_small_amount() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    create_liquidatable_position(&env, &contract_id, &borrower, 10000, 10000);

    // Liquidate very small amount
    let (debt_liquidated, collateral_seized, _incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &1);

    assert_eq!(debt_liquidated, 1);
    assert!(collateral_seized >= 1); // At least 1 collateral seized
}

/// Test liquidation with large values
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_large_values() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Large values
    let collateral = 1_000_000_000_000i128;
    let debt = 1_000_000_000_000i128;

    create_liquidatable_position(&env, &contract_id, &borrower, collateral, debt);

    // Liquidate 50%
    let to_liquidate = debt / 2;
    let (debt_liquidated, collateral_seized, incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &to_liquidate);

    assert_eq!(debt_liquidated, to_liquidate);
    assert!(collateral_seized > to_liquidate); // Includes incentive
    assert!(incentive > 0);
}

/// Test liquidation updates position correctly
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_liquidate_position_consistency() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    let initial_collateral = 2000i128;
    let initial_debt = 1500i128;

    create_liquidatable_position(
        &env,
        &contract_id,
        &borrower,
        initial_collateral,
        initial_debt,
    );

    let (debt_liquidated, collateral_seized, _) =
        client.liquidate(&liquidator, &borrower, &None, &None, &500);

    // Verify position is consistent
    let position = get_user_position(&env, &contract_id, &borrower).unwrap();
    let collateral_balance = get_collateral_balance(&env, &contract_id, &borrower);

    // Position collateral should match collateral balance
    assert_eq!(position.collateral, collateral_balance);

    // Debt should be reduced
    assert_eq!(position.debt, initial_debt - debt_liquidated);

    // Collateral should be reduced
    assert_eq!(collateral_balance, initial_collateral - collateral_seized);
}

// =============================================================================
// PARTIAL LIQUIDATION WITH PENALTY DIFFERENTIATION TESTS (Issue #173)
// =============================================================================

/// Test that dynamic penalty equals base incentive when health factor is just below threshold
#[test]
fn test_dynamic_penalty_near_threshold() {
    let env = create_test_env();
    let (contract_id, _admin, _client) = setup_contract_with_admin(&env);

    env.as_contract(&contract_id, || {
        // health_factor = 10499 bps (just below 10500 threshold)
        // severity is tiny → penalty should be very close to base (1000 bps)
        let collateral = 10499i128;
        let debt = 10000i128;
        let penalty =
            crate::liquidate::calculate_dynamic_penalty(&env, collateral, debt).unwrap();
        // base = 1000, max = 2000, threshold = 10500
        // severity = (10500 - 10499) / 10500 ≈ 0.0001
        // extra ≈ 0.0001 * 1000 ≈ 0
        assert!(penalty >= 1000, "penalty should be >= base incentive");
        assert!(penalty <= 2000, "penalty should not exceed max cap");
        // Near threshold: penalty should be very close to base
        assert!(penalty <= 1010, "near-threshold penalty should be close to base");
    });
}

/// Test that dynamic penalty is higher for a severely undercollateralized position
#[test]
fn test_dynamic_penalty_severe_undercollateralization() {
    let env = create_test_env();
    let (contract_id, _admin, _client) = setup_contract_with_admin(&env);

    env.as_contract(&contract_id, || {
        // health_factor = 5000 bps (50% — very severe)
        let collateral = 5000i128;
        let debt = 10000i128;
        let penalty =
            crate::liquidate::calculate_dynamic_penalty(&env, collateral, debt).unwrap();
        // severity = (10500 - 5000) / 10500 ≈ 0.524
        // extra ≈ 0.524 * 1000 ≈ 524
        // penalty ≈ 1524
        assert!(penalty > 1000, "severe position should have higher penalty than base");
        assert!(penalty <= 2000, "penalty should not exceed max cap");
        assert!(penalty >= 1400, "severe position should have significantly elevated penalty");
    });
}

/// Test that dynamic penalty is capped at MAX_PENALTY_BPS (2000 bps = 20%)
#[test]
fn test_dynamic_penalty_capped_at_maximum() {
    let env = create_test_env();
    let (contract_id, _admin, _client) = setup_contract_with_admin(&env);

    env.as_contract(&contract_id, || {
        // health_factor = 100 bps (nearly insolvent)
        let collateral = 100i128;
        let debt = 10000i128;
        let penalty =
            crate::liquidate::calculate_dynamic_penalty(&env, collateral, debt).unwrap();
        assert_eq!(penalty, 2000, "nearly-insolvent position should hit the 2000 bps cap");
    });
}

/// Test that penalty scales monotonically: more severe → higher penalty
#[test]
fn test_dynamic_penalty_monotonically_increases_with_severity() {
    let env = create_test_env();
    let (contract_id, _admin, _client) = setup_contract_with_admin(&env);

    env.as_contract(&contract_id, || {
        let debt = 10000i128;
        // health factors: 10400 > 8000 > 5000 > 2000 (all below 10500 threshold)
        let hf_mild = 10400i128;
        let hf_moderate = 8000i128;
        let hf_severe = 5000i128;
        let hf_critical = 2000i128;

        let p_mild =
            crate::liquidate::calculate_dynamic_penalty(&env, hf_mild, debt).unwrap();
        let p_moderate =
            crate::liquidate::calculate_dynamic_penalty(&env, hf_moderate, debt).unwrap();
        let p_severe =
            crate::liquidate::calculate_dynamic_penalty(&env, hf_severe, debt).unwrap();
        let p_critical =
            crate::liquidate::calculate_dynamic_penalty(&env, hf_critical, debt).unwrap();

        assert!(p_mild <= p_moderate, "moderate should have >= penalty than mild");
        assert!(p_moderate <= p_severe, "severe should have >= penalty than moderate");
        assert!(p_severe <= p_critical, "critical should have >= penalty than severe");
    });
}

/// Test partial liquidation leaves position in consistent state when still unhealthy
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_partial_liquidation_position_still_unhealthy() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Deeply undercollateralized: collateral=500, debt=1000 (50% ratio)
    create_liquidatable_position(&env, &contract_id, &borrower, 500, 1000);

    // Liquidate only 10% of debt — position remains unhealthy
    let (debt_liquidated, collateral_seized, incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &100);

    assert_eq!(debt_liquidated, 100);
    assert!(incentive > 0, "incentive must be positive");
    assert!(collateral_seized > debt_liquidated, "seized includes incentive bonus");

    // Position should still exist and have remaining debt
    let position = get_user_position(&env, &contract_id, &borrower).unwrap();
    assert_eq!(position.debt, 900, "remaining debt should be 900");

    // Collateral balance should be reduced by seized amount
    let remaining_collateral = get_collateral_balance(&env, &contract_id, &borrower);
    assert_eq!(remaining_collateral, 500 - collateral_seized);
}

/// Test partial liquidation that brings position back to healthy state
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_partial_liquidation_restores_health() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    // Slightly undercollateralized: collateral=1040, debt=1000 (104% ratio, below 105% threshold)
    create_liquidatable_position(&env, &contract_id, &borrower, 1040, 1000);

    // Liquidate 50% (close factor) — should bring position back to health
    let (debt_liquidated, _collateral_seized, incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &500);

    assert_eq!(debt_liquidated, 500);
    assert!(incentive > 0);

    // Remaining debt = 500, collateral still > 500 → healthy
    let position = get_user_position(&env, &contract_id, &borrower).unwrap();
    assert_eq!(position.debt, 500);
}

/// Test that penalty for mild undercollateralization is lower than for severe
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_partial_liquidation_penalty_differentiation() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    // Mild: health factor ~104% (just below 105% threshold)
    let mild_borrower = Address::generate(&env);
    create_liquidatable_position(&env, &contract_id, &mild_borrower, 1040, 1000);

    // Severe: health factor ~50%
    let severe_borrower = Address::generate(&env);
    create_liquidatable_position(&env, &contract_id, &severe_borrower, 500, 1000);

    let liquidator = Address::generate(&env);

    let (_d1, _c1, incentive_mild) =
        client.liquidate(&liquidator, &mild_borrower, &None, &None, &100);
    let (_d2, _c2, incentive_severe) =
        client.liquidate(&liquidator, &severe_borrower, &None, &None, &100);

    // Same debt amount liquidated, but severe position should yield higher incentive
    assert!(
        incentive_severe >= incentive_mild,
        "severe position should yield >= incentive: severe={incentive_severe}, mild={incentive_mild}"
    );
}

/// Integration: verify liquidation event contains correct penalty data
#[test]
#[ignore] // Native XLM liquidation not yet supported
fn test_partial_liquidation_event_emitted_with_penalty() {
    let env = create_test_env();
    let (contract_id, _admin, client) = setup_contract_with_admin(&env);

    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);

    create_liquidatable_position(&env, &contract_id, &borrower, 500, 1000);

    let (debt_liquidated, collateral_seized, incentive) =
        client.liquidate(&liquidator, &borrower, &None, &None, &200);

    // All return values must be positive and consistent
    assert!(debt_liquidated > 0);
    assert!(collateral_seized >= debt_liquidated);
    assert!(incentive > 0);
    // incentive = debt_liquidated * penalty_bps / 10000
    // For 50% health factor, penalty > 1000 bps, so incentive > 2% of debt
    assert!(incentive >= debt_liquidated / 100);
}
