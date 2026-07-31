//! # Flash Loan Test Suite
//!
//! Comprehensive tests for flash loan functionality including:
//! - Successful flash loan execution and repayment
//! - Fee calculation and validation
//! - Unpaid loan revert scenarios
//! - Callback validation
//! - RAII guard behavior (no leak on failure)

use soroban_sdk::{contract, contractimpl, testutils::Address as _, token, Address, Env, Symbol};

use crate::flash_loan::{execute_flash_loan, set_flash_loan_config, FlashLoanConfig};
use crate::HelloContract;

#[contract]
pub struct MockReceiver;

#[contractimpl]
impl MockReceiver {
    pub fn on_flash_loan(env: Env, _user: Address, asset: Address, amount: i128, fee: i128) {
        let total = amount + fee;
        let token = token::TokenClient::new(&env, &asset);
        // Approve the core contract to pull the total amount back
        let target_key = Symbol::new(&env, "CORE_CONTRACT");
        let core_contract = env
            .storage()
            .temporary()
            .get::<Symbol, Address>(&target_key)
            .unwrap();
        token.approve(
            &env.current_contract_address(),
            &core_contract,
            &total,
            &9999,
        );
    }
}

/// Setup test environment
fn setup_env() -> (Env, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(HelloContract, ());
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract.address();

    env.as_contract(&contract_id, || {
        crate::admin::set_admin(&env, admin.clone(), None).unwrap();
    });

    (env, contract_id, admin, user, token_address)
}

/// Setup with token balance
fn setup_with_balance(balance: i128) -> (Env, Address, Address, Address, Address) {
    let (env, contract_id, admin, user, token_address) = setup_env();
    let token_asset_client = token::StellarAssetClient::new(&env, &token_address);
    token_asset_client.mint(&contract_id, &balance);
    (env, contract_id, admin, user, token_address)
}

#[test]
fn test_flash_loan_success() {
    let (env, contract_id, _admin, user, token_address) = setup_with_balance(10_000_000);

    // Setup receiver
    let receiver_id = env.register(MockReceiver, ());
    let target_key = Symbol::new(&env, "CORE_CONTRACT");
    env.as_contract(&receiver_id, || {
        env.storage().temporary().set(&target_key, &contract_id);
    });

    // Fund receiver for fee
    let token_asset_client = token::StellarAssetClient::new(&env, &token_address);
    token_asset_client.mint(&receiver_id, &900); // 9bps of 1M is 900

    let result = env.as_contract(&contract_id, || {
        execute_flash_loan(
            &env,
            user.clone(),
            token_address.clone(),
            1_000_000,
            receiver_id,
        )
    });

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1_000_900);
}

#[test]
#[should_panic(expected = "HostError")]
fn test_flash_loan_insufficient_repayment_fails() {
    let (env, contract_id, _admin, user, token_address) = setup_with_balance(10_000_000);

    let receiver_id = env.register(CheapReceiver, ());
    let target_key = Symbol::new(&env, "CORE_CONTRACT");
    env.as_contract(&receiver_id, || {
        env.storage().temporary().set(&target_key, &contract_id);
    });

    // We do NOT fund the receiver for the fee here on purpose,
    // and the receiver will only approve the principal amount.
    // This will cause `transfer_from` in `execute_flash_loan` to fail
    // and naturally panic the transaction.
    env.as_contract(&contract_id, || {
        let _ = execute_flash_loan(
            &env,
            user.clone(),
            token_address.clone(),
            1_000_000,
            receiver_id,
        );
    });
}

#[test]
fn test_set_flash_loan_config_admin_only() {
    let (env, contract_id, admin, user, _token_address) = setup_env();

    let config = FlashLoanConfig {
        fee_bps: 20,
        max_amount: 100_000_000,
        min_amount: 100,
    };

    // Admin should succeed
    let res = env.as_contract(&contract_id, || {
        set_flash_loan_config(&env, admin, config.clone())
    });
    assert!(res.is_ok());

    // User should fail
    let res = env.as_contract(&contract_id, || set_flash_loan_config(&env, user, config));
    assert!(res.is_err());
}

#[contract]
pub struct CheapReceiver;

#[contractimpl]
impl CheapReceiver {
    pub fn on_flash_loan(env: Env, _user: Address, asset: Address, amount: i128, _fee: i128) {
        let token = token::TokenClient::new(&env, &asset);
        let target_key = Symbol::new(&env, "CORE_CONTRACT");
        let core_contract = env
            .storage()
            .temporary()
            .get::<Symbol, Address>(&target_key)
            .unwrap();
        // Maliciously approve ONLY the principal, not the fee, to trigger insufficient repayment
        token.approve(
            &env.current_contract_address(),
            &core_contract,
            &amount,
            &9999,
        );
    }
}

#[contract]
pub struct SequenceJumpReceiver;

#[contractimpl]
impl SequenceJumpReceiver {
    pub fn on_flash_loan(env: Env, _user: Address, asset: Address, amount: i128, fee: i128) {
        let total = amount + fee;
        let token = token::TokenClient::new(&env, &asset);
        let target_key = Symbol::new(&env, "CORE_CONTRACT");
        let core_contract = env
            .storage()
            .temporary()
            .get::<Symbol, Address>(&target_key)
            .unwrap();
        token.approve(
            &env.current_contract_address(),
            &core_contract,
            &total,
            &9999,
        );

        // Simulate a late callback that crosses into a later ledger sequence.
        env.ledger().with_mut(|li| li.sequence_number += 1);
    }
}

#[test]
fn test_flash_loan_sequence_expiry_rejects_late_callback() {
    let (env, contract_id, _admin, user, token_address) = setup_with_balance(10_000_000);

    let receiver_id = env.register(SequenceJumpReceiver, ());
    let target_key = Symbol::new(&env, "CORE_CONTRACT");
    env.as_contract(&receiver_id, || {
        env.storage().temporary().set(&target_key, &contract_id);
    });

    let token_asset_client = token::StellarAssetClient::new(&env, &token_address);
    token_asset_client.mint(&receiver_id, &900);

    let result = env.as_contract(&contract_id, || {
        execute_flash_loan(
            &env,
            user.clone(),
            token_address.clone(),
            1_000_000,
            receiver_id,
        )
    });

    assert!(result.is_err());
}
