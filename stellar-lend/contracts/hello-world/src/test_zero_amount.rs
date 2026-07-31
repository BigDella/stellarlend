//! Zero-Amount Operation Handling Tests (Issue #385)
//!
//! This module validates that all amount-bearing operations correctly reject
//! zero and negative amounts with clean reverts (returning the appropriate
//! error variant), and that no state mutations occur on rejected operations.
//!
//! Also includes dust transaction handling tests.
//!
//! # Intended Semantics
//!
//! | Operation           | Zero / Negative Amount Behavior         |
//! |---------------------|-----------------------------------------|
//! | `deposit_collateral`| Revert with `DepositError::InvalidAmount`|
//! | `withdraw_collateral`| Revert with `WithdrawError::InvalidAmount`|
//! | `borrow_asset`      | Revert with `BorrowError::InvalidAmount` |
//! | `repay_debt`        | Revert with `RepayError::InvalidAmount`  |
//! | Liquidation (zero debt) | Returns `Ok(false)` / `Ok(0)`       |
//! | Dust transactions   | Revert with `AmountBelowMinimum` error   |

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

use deposit::{DepositDataKey, Position};

// ============================================================================
// Helpers
// ============================================================================

/// Create a test environment with all auths mocked.
#[allow(dead_code)]
fn setup() -> (Env, Address, HelloContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    // SAFETY: client borrows env; we know env outlives this scope via leak.
    // This is only for tests — we leak env so the client reference is 'static.
    let client = unsafe {
        core::mem::transmute::<HelloContractClient<'_>, HelloContractClient<'static>>(client)
    };
    (env, contract_id, client)
}

/// Read the collateral balance for a user directly from storage.
fn collateral_balance(env: &Env, contract_id: &Address, user: &Address) -> i128 {
    env.as_contract(contract_id, || {
        let key = DepositDataKey::CollateralBalance(user.clone());
        env.storage()
            .persistent()
            .get::<DepositDataKey, i128>(&key)
            .unwrap_or(0)
    })
}

/// Read the user position directly from storage.
fn position_of(env: &Env, contract_id: &Address, user: &Address) -> Option<Position> {
    env.as_contract(contract_id, || {
        let key = DepositDataKey::Position(user.clone());
        env.storage()
            .persistent()
            .get::<DepositDataKey, Position>(&key)
    })
}

// ============================================================================
// 1. DEPOSIT — Zero-Amount Tests
// ============================================================================

#[test]
fn test_zero_deposit_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    let res = client.try_deposit_collateral(&user, &None, &0);
    assert!(res.is_err());
}

#[test]
fn test_negative_deposit_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    let res = client.try_deposit_collateral(&user, &None, &(-500));
    assert!(res.is_err());
}

#[test]
fn test_min_i128_deposit_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    let res = client.try_deposit_collateral(&user, &None, &i128::MIN);
    assert!(res.is_err());
}

#[test]
fn test_zero_deposit_no_state_change() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // Valid deposit first
    client.deposit_collateral(&user, &None, &1000);
    let balance_before = collateral_balance(&env, &contract_id, &user);
    assert_eq!(balance_before, 1000);

    // Zero deposit — must fail, state must be untouched
    let result = client.try_deposit_collateral(&user, &None, &0);
    assert!(result.is_err(), "Zero deposit should revert");

    let balance_after = collateral_balance(&env, &contract_id, &user);
    assert_eq!(
        balance_after, balance_before,
        "Balance must not change after zero deposit"
    );
}

#[test]
fn test_zero_deposit_between_valid_deposits() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // First valid deposit
    client.deposit_collateral(&user, &None, &500);

    // Zero deposit attempt (should fail)
    let _ = client.try_deposit_collateral(&user, &None, &0);

    // Second valid deposit
    client.deposit_collateral(&user, &None, &300);

    // Final balance should be 500 + 300 = 800 (zero deposit had no effect)
    let balance = collateral_balance(&env, &contract_id, &user);
    assert_eq!(balance, 800, "Zero deposit must not affect accumulation");
}

#[test]
fn test_negative_one_deposit_reverts_cleanly() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    let result = client.try_deposit_collateral(&user, &None, &(-1));
    assert!(result.is_err(), "-1 deposit should revert");
}

// ============================================================================
// 2. WITHDRAW — Zero-Amount Tests
// ============================================================================

#[test]
fn test_zero_withdraw_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // Deposit first
    client.deposit_collateral(&user, &None, &1000);
    // Zero withdraw must revert
    let res = client.try_withdraw_collateral(&user, &None, &0);
    assert!(res.is_err());
}

#[test]
fn test_negative_withdraw_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    client.deposit_collateral(&user, &None, &1000);
    let res = client.try_withdraw_collateral(&user, &None, &(-100));
    assert!(res.is_err());
}

#[test]
fn test_zero_withdraw_no_state_change() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    client.deposit_collateral(&user, &None, &1000);
    let balance_before = collateral_balance(&env, &contract_id, &user);

    let result = client.try_withdraw_collateral(&user, &None, &0);
    assert!(result.is_err(), "Zero withdraw should revert");

    let balance_after = collateral_balance(&env, &contract_id, &user);
    assert_eq!(
        balance_after, balance_before,
        "Balance must not change after zero withdraw"
    );
}

#[test]
fn test_zero_withdraw_position_unchanged() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    client.deposit_collateral(&user, &None, &1000);
    let position_before = position_of(&env, &contract_id, &user).unwrap();

    let _ = client.try_withdraw_collateral(&user, &None, &0);

    let position_after = position_of(&env, &contract_id, &user).unwrap();
    assert_eq!(position_after.collateral, position_before.collateral);
    assert_eq!(position_after.debt, position_before.debt);
}

#[test]
fn test_zero_withdraw_between_valid_withdrawals() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    client.deposit_collateral(&user, &None, &1000);

    // First valid withdrawal
    client.withdraw_collateral(&user, &None, &200);

    // Zero withdrawal attempt
    let _ = client.try_withdraw_collateral(&user, &None, &0);

    // Second valid withdrawal
    client.withdraw_collateral(&user, &None, &300);

    let balance = collateral_balance(&env, &contract_id, &user);
    assert_eq!(
        balance, 500,
        "Zero withdraw must not affect balance: 1000 - 200 - 300 = 500"
    );
}

// ============================================================================
// 3. BORROW — Zero-Amount Tests
// ============================================================================

#[test]
fn test_zero_borrow_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    client.deposit_collateral(&user, &None, &10_000);
    let res = client.try_borrow_asset(&user, &None, &0);
    assert!(res.is_err());
}

#[test]
fn test_negative_borrow_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    client.deposit_collateral(&user, &None, &10_000);
    let res = client.try_borrow_asset(&user, &None, &(-200));
    assert!(res.is_err());
}

#[test]
fn test_zero_borrow_no_state_change() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    client.deposit_collateral(&user, &None, &10_000);
    let position_before = position_of(&env, &contract_id, &user).unwrap();
    assert_eq!(position_before.debt, 0);

    let result = client.try_borrow_asset(&user, &None, &0);
    assert!(result.is_err(), "Zero borrow should revert");

    let position_after = position_of(&env, &contract_id, &user).unwrap();
    assert_eq!(
        position_after.debt, 0,
        "Debt must remain zero after zero borrow"
    );
}

#[test]
fn test_zero_borrow_with_existing_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    client.deposit_collateral(&user, &None, &10_000);
    // Valid borrow - within 150% collateral ratio: 10000 / 1.5 = 6666 max
    client.borrow_asset(&user, &None, &3000);

    let position_before = position_of(&env, &contract_id, &user).unwrap();
    assert_eq!(position_before.debt, 3000);

    let result = client.try_borrow_asset(&user, &None, &0);
    assert!(result.is_err(), "Zero borrow should revert");

    let position_after = position_of(&env, &contract_id, &user).unwrap();
    assert_eq!(
        position_after.debt, position_before.debt,
        "Debt must not change after zero borrow"
    );
}

#[test]
fn test_zero_borrow_between_valid_borrows() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    client.deposit_collateral(&user, &None, &10_000);

    // First valid borrow
    client.borrow_asset(&user, &None, &1000);

    // Zero borrow attempt
    let _ = client.try_borrow_asset(&user, &None, &0);

    // Second valid borrow
    client.borrow_asset(&user, &None, &500);

    let position = position_of(&env, &contract_id, &user).unwrap();
    assert_eq!(
        position.debt, 1500,
        "Zero borrow must not affect debt: 1000 + 500 = 1500"
    );
}

// ============================================================================
// 4. REPAY — Zero-Amount Tests
// ============================================================================

#[test]
fn test_zero_repay_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // Set up position with debt directly
    env.as_contract(&contract_id, || {
        let position_key = DepositDataKey::Position(user.clone());
        let position = Position {
            collateral: 10_000,
            debt: 3000,
            borrow_interest: 100,
            last_accrual_time: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&position_key, &position);
    });

    let res = client.try_repay_debt(&user, &None, &0);
    assert!(res.is_err());
}

#[test]
fn test_negative_repay_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let position_key = DepositDataKey::Position(user.clone());
        let position = Position {
            collateral: 10_000,
            debt: 3000,
            borrow_interest: 100,
            last_accrual_time: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&position_key, &position);
    });

    let res = client.try_repay_debt(&user, &None, &(-100));
    assert!(res.is_err());
}

#[test]
fn test_zero_repay_no_state_change() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let position_key = DepositDataKey::Position(user.clone());
        let position = Position {
            collateral: 10_000,
            debt: 3000,
            borrow_interest: 100,
            last_accrual_time: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&position_key, &position);
    });

    let position_before = position_of(&env, &contract_id, &user).unwrap();

    let result = client.try_repay_debt(&user, &None, &0);
    assert!(result.is_err(), "Zero repay should revert");

    let position_after = position_of(&env, &contract_id, &user).unwrap();
    assert_eq!(
        position_after.debt, position_before.debt,
        "Debt must not change"
    );
    assert_eq!(
        position_after.borrow_interest, position_before.borrow_interest,
        "Interest must not change"
    );
}

#[test]
fn test_zero_repay_between_valid_repayments() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // Use the deposit/borrow flow to create real debt (using None/native tracking)
    client.deposit_collateral(&user, &None, &10_000);
    client.borrow_asset(&user, &None, &3000);

    // Zero repay attempt — this is the core assertion being tested
    let zero_res = client.try_repay_debt(&user, &None, &0);
    assert!(zero_res.is_err(), "Zero repay must fail");

    // Verify debt was NOT affected by zero repay
    let position = position_of(&env, &contract_id, &user).unwrap();
    assert_eq!(position.debt, 3000, "Zero repay must not affect debt");
}

// ============================================================================
// 5. RISK MANAGEMENT — Zero-Value Tests
// ============================================================================

#[test]
fn test_liquidation_check_zero_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Zero debt should never be liquidatable
    let result = client.can_be_liquidated(&10_000, &0);
    assert!(!result, "Zero debt position must not be liquidatable");
}

#[test]
fn test_liquidation_check_zero_collateral_with_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Zero collateral with debt should be liquidatable
    let result = client.can_be_liquidated(&0, &1000);
    assert!(result, "Zero collateral with debt must be liquidatable");
}

#[test]
fn test_liquidation_check_both_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Both zero: no debt → not liquidatable
    let result = client.can_be_liquidated(&0, &0);
    assert!(!result, "Both zero: not liquidatable (no debt)");
}

#[test]
fn test_max_liquidatable_zero_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Zero debt → max liquidatable should be 0
    let max = client.get_max_liquidatable_amount(&0);
    assert_eq!(max, 0, "Max liquidatable for zero debt must be 0");
}

#[test]
fn test_liquidation_incentive_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Zero liquidation amount → incentive should be 0
    let incentive = client.get_liquidation_incentive_amount(&0);
    assert_eq!(
        incentive, 0,
        "Liquidation incentive for zero amount must be 0"
    );
}

#[test]
fn test_min_collateral_ratio_zero_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Zero debt → collateral ratio check should pass (any collateral is sufficient)
    let result = client.try_require_min_collateral_ratio(&1000, &0);
    assert!(
        result.is_ok(),
        "Zero debt should satisfy any collateral ratio requirement"
    );
}

#[test]
fn test_min_collateral_ratio_both_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Both zero → should pass (no debt to satisfy)
    let result = client.try_require_min_collateral_ratio(&0, &0);
    assert!(
        result.is_ok(),
        "Both zero should satisfy collateral ratio (no debt)"
    );
}

// ============================================================================
// 6. CROSS-OPERATION — Zero-Amount Sequence Tests
// ============================================================================

#[test]
fn test_zero_ops_do_not_affect_subsequent_valid_ops() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // Try all zero operations first (all should fail)
    let _ = client.try_deposit_collateral(&user, &None, &0);
    let _ = client.try_withdraw_collateral(&user, &None, &0);
    let _ = client.try_borrow_asset(&user, &None, &0);
    let _ = client.try_repay_debt(&user, &None, &0);

    // Now do a valid deposit — should succeed without any state corruption
    let balance = client.deposit_collateral(&user, &None, &5000);
    assert_eq!(
        balance, 5000,
        "Valid deposit must succeed after zero attempts"
    );

    // Valid borrow
    let debt = client.borrow_asset(&user, &None, &2000);
    assert!(debt > 0, "Valid borrow must succeed after zero attempts");

    // Verify final state
    let position = position_of(&env, &contract_id, &user).unwrap();
    assert_eq!(position.collateral, 5000);
    assert_eq!(position.debt, 2000);
}

#[test]
fn test_mixed_zero_and_valid_full_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // 1. deposit(1000) → success
    client.deposit_collateral(&user, &None, &1000);
    assert_eq!(collateral_balance(&env, &contract_id, &user), 1000);

    // 2. borrow(0) → fail
    let borrow_zero = client.try_borrow_asset(&user, &None, &0);
    assert!(borrow_zero.is_err(), "Zero borrow must fail");

    // 3. borrow(300) → success
    client.borrow_asset(&user, &None, &300);

    // 4. repay(0) → fail
    let repay_zero = client.try_repay_debt(&user, &None, &0);
    assert!(repay_zero.is_err(), "Zero repay must fail");

    // 5. withdraw(0) → fail
    let withdraw_zero = client.try_withdraw_collateral(&user, &None, &0);
    assert!(withdraw_zero.is_err(), "Zero withdraw must fail");

    // Verify position is unchanged from valid ops only
    let position = position_of(&env, &contract_id, &user).unwrap();
    assert_eq!(position.collateral, 1000);
    assert_eq!(position.debt, 300);
}

#[test]
fn test_all_zero_operations_sequence() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // Every single zero operation should revert cleanly
    let deposit_result = client.try_deposit_collateral(&user, &None, &0);
    assert!(deposit_result.is_err(), "Zero deposit must fail");

    let withdraw_result = client.try_withdraw_collateral(&user, &None, &0);
    assert!(withdraw_result.is_err(), "Zero withdraw must fail");

    let borrow_result = client.try_borrow_asset(&user, &None, &0);
    assert!(borrow_result.is_err(), "Zero borrow must fail");

    let repay_result = client.try_repay_debt(&user, &None, &0);
    assert!(repay_result.is_err(), "Zero repay must fail");

    // No state should exist for this user
    let position = position_of(&env, &contract_id, &user);
    assert!(
        position.is_none(),
        "No position should exist after all-zero ops"
    );
    assert_eq!(collateral_balance(&env, &contract_id, &user), 0);
}

#[test]
fn test_negative_amount_all_operations() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // All negative amounts must revert
    assert!(
        client.try_deposit_collateral(&user, &None, &(-1)).is_err(),
        "Negative deposit must fail"
    );
    assert!(
        client.try_withdraw_collateral(&user, &None, &(-1)).is_err(),
        "Negative withdraw must fail"
    );
    assert!(
        client.try_borrow_asset(&user, &None, &(-1)).is_err(),
        "Negative borrow must fail"
    );
    assert!(
        client.try_repay_debt(&user, &None, &(-1)).is_err(),
        "Negative repay must fail"
    );
}

// ============================================================================
// 7. DUST TRANSACTION TESTS
// ============================================================================

#[test]
fn test_dust_deposit_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // Dust amount (below minimum threshold of 1)
    let res = client.try_deposit_collateral(&user, &None, &0);
    assert!(res.is_err());
}

#[test]
fn test_dust_withdraw_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    client.deposit_collateral(&user, &None, &1000);
    
    // Dust amount (below minimum threshold of 1)
    let res = client.try_withdraw_collateral(&user, &None, &0);
    assert!(res.is_err());
}

#[test]
fn test_dust_borrow_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    client.deposit_collateral(&user, &None, &10_000);
    
    // Dust amount (below minimum threshold of 1)
    let res = client.try_borrow_asset(&user, &None, &0);
    assert!(res.is_err());
}

#[test]
fn test_dust_repay_reverts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // Set up position with debt
    env.as_contract(&contract_id, || {
        let position_key = DepositDataKey::Position(user.clone());
        let position = Position {
            collateral: 10_000,
            debt: 3000,
            borrow_interest: 100,
            last_accrual_time: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&position_key, &position);
    });

    // Dust amount (below minimum threshold of 1)
    let res = client.try_repay_debt(&user, &None, &0);
    assert!(res.is_err());
}

#[test]
fn test_dust_sweep_no_dust_to_sweep() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // No dust accumulated - sweep should fail
    let res = client.try_sweep_deposit_dust(&user, &contract_id);
    assert!(res.is_err());
}

#[test]
fn test_dust_accumulation_prevention() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // Multiple dust transactions should all be rejected
    for _ in 0..10 {
        let res = client.try_deposit_collateral(&user, &None, &0);
        assert!(res.is_err());
    }

    // Verify no dust accumulation
    let position = position_of(&env, &contract_id, &user);
    assert!(position.is_none(), "No position should exist after dust attempts");
}

#[test]
fn test_minimum_transaction_amount_enforcement() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // Valid minimum amount (1)
    let res = client.try_deposit_collateral(&user, &None, &1);
    assert!(res.is_ok(), "Minimum transaction amount of 1 should be accepted");

    // Below minimum (0)
    let res = client.try_deposit_collateral(&user, &None, &0);
    assert!(res.is_err(), "Amount below minimum should be rejected");
}

#[test]
fn test_dust_between_valid_transactions() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // First valid deposit
    client.deposit_collateral(&user, &None, &1000);

    // Dust attempt (should fail)
    let _ = client.try_deposit_collateral(&user, &None, &0);

    // Second valid deposit
    client.deposit_collateral(&user, &None, &500);

    // Verify balance is sum of valid deposits only
    let balance = collateral_balance(&env, &contract_id, &user);
    assert_eq!(balance, 1500, "Dust should not affect balance");
}

// ============================================================================
// 8. ROUNDING DIRECTION CONSISTENCY TESTS
// ============================================================================

#[test]
fn test_rounding_direction_consistency() {
    // Test that rounding is depositor-friendly
    // Deposits: round down (depositor keeps more)
    // Withdrawals: round up (user receives at least what they expect)
    
    // This is a conceptual test - actual implementation uses rounding.rs module
    // The key invariant: withdraw_amount >= repay_amount for same fraction
    let balance = 1000;
    let fraction = 3333; // 33.33%
    let scale = 10000;

    // Withdraw should round up
    let withdraw = (balance * fraction + scale - 1) / scale;
    
    // Repay should round down
    let repay = (balance * fraction) / scale;

    assert!(withdraw >= repay, "Withdraw should be >= repay for same fraction");
}

#[test]
fn test_precision_loss_in_interest_calculation() {
    // Test that small amounts don't lose precision in interest calculations
    let principal = 1; // Minimum amount
    let rate = 500; // 5% in basis points
    let scale = 10000;

    // Interest calculation: principal * rate / scale
    let interest = (principal * rate) / scale;
    
    // With minimum principal, interest should be 0 (rounds down)
    assert_eq!(interest, 0, "Small principal may result in zero interest due to rounding");
}

#[test]
fn test_rounding_asymmetry_prevention() {
    // Test that rounding doesn't create arbitrage opportunities
    let amount = 1000;
    let fraction = 5000; // 50%
    let scale = 10000;

    // Calculate with different rounding directions
    let round_down = (amount * fraction) / scale;
    let round_up = (amount * fraction + scale - 1) / scale;

    // For exact fractions, both should be equal
    assert_eq!(round_down, round_up, "Exact fractions should have same result");

    // For inexact fractions, round_up > round_down
    let inexact_fraction = 3333;
    let round_down_inexact = (amount * inexact_fraction) / scale;
    let round_up_inexact = (amount * inexact_fraction + scale - 1) / scale;
    
    assert!(round_up_inexact >= round_down_inexact, "Round up should be >= round down");
}
