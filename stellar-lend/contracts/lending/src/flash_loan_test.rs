use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Bytes, Env};

// Mock receiver contract that implements the flash loan callback
#[contract]
pub struct FlashLoanReceiver;

#[contractimpl]
impl FlashLoanReceiver {
    pub fn on_flash_loan(
        env: Env,
        initiator: Address,
        asset: Address,
        amount: i128,
        fee: i128,
        params: Bytes,
    ) -> bool {
        let mut total_repayment = amount + fee;

        // If params is not empty (16 bytes), it contains the requested repayment amount
        if params.len() == 16 {
            let mut arr = [0u8; 16];
            params.copy_into_slice(&mut arr);
            total_repayment = i128::from_be_bytes(arr);
        }

        let token_client = token::Client::new(&env, &asset);

        // Transfer back to the lender
        token_client.transfer(
            &env.current_contract_address(),
            &initiator,
            &total_repayment,
        );
        true
    }
}

#[contract]
pub struct FalseFlashLoanReceiver;

#[contractimpl]
impl FalseFlashLoanReceiver {
    pub fn on_flash_loan(
        _env: Env,
        _initiator: Address,
        _asset: Address,
        _amount: i128,
        _fee: i128,
        _params: Bytes,
    ) -> bool {
        false
    }
}

#[contract]
pub struct RevertingFlashLoanReceiver;

#[contractimpl]
impl RevertingFlashLoanReceiver {
    pub fn on_flash_loan(
        _env: Env,
        _initiator: Address,
        _asset: Address,
        _amount: i128,
        _fee: i128,
        _params: Bytes,
    ) -> bool {
        panic!("Callback reverted")
    }
}

#[test]
fn test_flash_loan_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &asset);

    // Register receiver
    let receiver_id = env.register(FlashLoanReceiver, ());
    let receiver_address = receiver_id.clone();

    // 1. Initial setup
    client.initialize(&admin, &1_000_000_000, &1000);
    client.set_flash_loan_fee_bps(&100); // 1% fee

    // Mint some assets to the lending contract so it can lend
    token_admin.mint(&contract_id, &100_000);

    // Mint some assets to the receiver to cover the fee
    token_admin.mint(&receiver_address, &1000);

    // 2. Execute flash loan
    let amount = 10_000;
    let fee = 100; // 1% of 10,000

    client.flash_loan(
        &receiver_address,
        &asset,
        &amount,
        &1_000_000,
        &Bytes::new(&env),
    );

    // 3. Verify balances
    let token_client = token::Client::new(&env, &asset);
    assert_eq!(token_client.balance(&contract_id), 100_000 + fee);
    assert_eq!(token_client.balance(&receiver_address), 1000 - fee);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn test_flash_loan_insufficient_repayment() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &asset);

    let receiver_id = env.register(FlashLoanReceiver, ());
    let receiver_address = receiver_id.clone();

    client.initialize(&admin, &1_000_000_000, &1000);

    token_admin.mint(&contract_id, &100_000);

    // Receiver only tries to repay the principal
    let amount = 10_000;
    let repay_amount: i128 = 10_000;
    let params = Bytes::from_slice(&env, &repay_amount.to_be_bytes());

    client.flash_loan(&receiver_address, &asset, &amount, &1_000_000, &params);
}

#[test]
fn test_set_flash_loan_fee_bps() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);

    client.set_flash_loan_fee_bps(&50);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #4)")]
fn test_set_flash_loan_fee_too_high() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);

    client.set_flash_loan_fee_bps(&2000); // Exceeds MAX_FEE_BPS (1000)
}

// Mock receiver contract that attempts reentrancy
#[contract]
pub struct ReentrantFlashLoanReceiver;

#[contractimpl]
impl ReentrantFlashLoanReceiver {
    pub fn on_flash_loan(
        env: Env,
        initiator: Address,
        asset: Address,
        _amount: i128,
        _fee: i128,
        _params: Bytes,
    ) -> bool {
        let client = LendingContractClient::new(&env, &initiator);
        client.flash_loan(
            &env.current_contract_address(),
            &asset,
            &100,
            &1_000_000,
            &Bytes::new(&env),
        );
        true
    }
}

#[contract]
pub struct SequenceJumpFlashLoanReceiver;

#[contractimpl]
impl SequenceJumpFlashLoanReceiver {
    pub fn on_flash_loan(
        env: Env,
        initiator: Address,
        asset: Address,
        amount: i128,
        fee: i128,
        _params: Bytes,
    ) -> bool {
        let total = amount + fee;
        let token_client = token::Client::new(&env, &asset);
        token_client.approve(&env.current_contract_address(), &initiator, &total, &9999);
        env.ledger().with_mut(|li| li.sequence_number += 1);
        true
    }
}

#[test]
#[should_panic(expected = "HostError: Error(Context, InvalidAction)")]
fn test_flash_loan_reentrancy() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &asset);

    let receiver_id = env.register(ReentrantFlashLoanReceiver, ());
    let receiver_address = receiver_id.clone();

    client.initialize(&admin, &1_000_000_000, &1000);
    token_admin.mint(&contract_id, &100_000);

    let amount = 10_000;
    client.flash_loan(
        &receiver_address,
        &asset,
        &amount,
        &1_000_000,
        &Bytes::new(&env),
    );
}

#[test]
fn test_flash_loan_expired_when_sequence_changes() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &asset);

    let receiver_id = env.register(SequenceJumpFlashLoanReceiver, ());
    let receiver_address = receiver_id.clone();

    client.initialize(&admin, &1_000_000_000, &1000);
    token_admin.mint(&contract_id, &100_000);

    let result = client.try_flash_loan(
        &receiver_address,
        &asset,
        &10_000,
        &1_000_000,
        &Bytes::new(&env),
    );
    assert!(result.is_err());
}

#[test]
fn test_flash_loan_callback_false() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &asset);

    let receiver_id = env.register(FalseFlashLoanReceiver, ());
    let receiver_address = receiver_id.clone();

    client.initialize(&admin, &1_000_000_000, &1000);
    token_admin.mint(&contract_id, &100_000);

    let amount = 10_000;

    // Should fail with CallbackFailed (5)
    let result = client.try_flash_loan(
        &receiver_address,
        &asset,
        &amount,
        &1_000_000,
        &Bytes::new(&env),
    );
    assert_eq!(result, Err(Ok(FlashLoanError::CallbackFailed)));
}

#[test]
#[should_panic(expected = "Callback reverted")]
fn test_flash_loan_callback_revert() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &asset);

    let receiver_id = env.register(RevertingFlashLoanReceiver, ());
    let receiver_address = receiver_id.clone();

    client.initialize(&admin, &1_000_000_000, &1000);
    token_admin.mint(&contract_id, &100_000);

    let amount = 10_000;
    client.flash_loan(
        &receiver_address,
        &asset,
        &amount,
        &1_000_000,
        &Bytes::new(&env),
    );
}

#[test]
#[should_panic] // Should panic due to insufficient balance in lending contract
fn test_flash_loan_exceed_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &asset);

    let receiver_id = env.register(FlashLoanReceiver, ());
    let receiver_address = receiver_id.clone();

    client.initialize(&admin, &1_000_000_000, &1000);
    token_admin.mint(&contract_id, &10_000); // Only 10k available

    let amount = 20_000; // Requesting 20k
    client.flash_loan(
        &receiver_address,
        &asset,
        &amount,
        &1_000_000,
        &Bytes::new(&env),
    );
}

#[test]
fn test_flash_loan_minimal_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &asset);

    let receiver_id = env.register(FlashLoanReceiver, ());
    let receiver_address = receiver_id.clone();

    client.initialize(&admin, &1_000_000_000, &1000);
    client.set_flash_loan_fee_bps(&5); // 0.05% fee

    token_admin.mint(&contract_id, &1_000_000);
    token_admin.mint(&receiver_address, &100);

    // amount = 1000, fee = 1000 * 5 / 10000 = 0.5 -> 0 (integer division)
    // Wait, let's test a case where it's exactly 1
    // amount = 2000, fee = 2000 * 5 / 10000 = 1
    let amount = 2000;
    client.flash_loan(
        &receiver_address,
        &asset,
        &amount,
        &1_000_000,
        &Bytes::new(&env),
    );

    let token_client = token::Client::new(&env, &asset);
    assert_eq!(token_client.balance(&contract_id), 1_000_000 + 1);
}

#[test]
fn test_flash_loan_max_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &asset);

    let receiver_id = env.register(FlashLoanReceiver, ());
    let receiver_address = receiver_id.clone();

    client.initialize(&admin, &1_000_000_000, &1000);
    client.set_flash_loan_fee_bps(&1000); // 10% fee

    token_admin.mint(&contract_id, &100_000);
    token_admin.mint(&receiver_address, &2000);

    let amount = 10_000;
    let expected_fee = 1000;
    client.flash_loan(
        &receiver_address,
        &asset,
        &amount,
        &1_000_000,
        &Bytes::new(&env),
    );

    let token_client = token::Client::new(&env, &asset);
    assert_eq!(token_client.balance(&contract_id), 100_000 + expected_fee);
}

// ─── Attack-prevention tests ──────────────────────────────────────────────────

use crate::flash_loan::{FlashLoanError, ManipulationConfig};

fn setup_with_liquidity(pool_balance: i128) -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(LendingContract, ());
    let admin = Address::generate(&env);
    let asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &asset);
    let client = LendingContractClient::new(&env, &contract_id);
    client.initialize(&admin, &1_000_000_000, &1000);
    token_admin.mint(&contract_id, &pool_balance);
    (env, contract_id, admin, asset)
}

#[test]
fn test_flash_loan_blocked_by_liquidity_cap() {
    let pool_balance: i128 = 100_000;
    let (env, contract_id, admin, asset) = setup_with_liquidity(pool_balance);
    let client = LendingContractClient::new(&env, &contract_id);
    let receiver_id = env.register(FlashLoanReceiver, ());
    let token_admin = token::StellarAssetClient::new(&env, &asset);
    token_admin.mint(&receiver_id, &10_000);

    // Configure a tight 10% liquidity cap.
    client.set_flash_manipulation_config(
        &admin,
        &ManipulationConfig {
            max_borrow_liquidity_bps: 1_000,
            max_price_impact_bps: 10_000,
            max_twap_deviation_bps: 10_000,
            min_twap_samples: 100,
            twap_window_secs: 300,
        },
    );

    // 60 % of pool — exceeds the 10 % cap.
    let result =
        client.try_flash_loan(&receiver_id, &asset, &60_000, &1_000_000, &Bytes::new(&env));
    assert_eq!(result, Err(Ok(FlashLoanError::ExceedsLiquidityCap)));
}

#[test]
fn test_flash_loan_blocked_by_price_impact() {
    let pool_balance: i128 = 100_000;
    let (env, contract_id, admin, asset) = setup_with_liquidity(pool_balance);
    let client = LendingContractClient::new(&env, &contract_id);
    let receiver_id = env.register(FlashLoanReceiver, ());

    // Allow large borrows but only tiny price impact (1 bps).
    client.set_flash_manipulation_config(
        &admin,
        &ManipulationConfig {
            max_borrow_liquidity_bps: 10_000,
            max_price_impact_bps: 1,
            max_twap_deviation_bps: 10_000,
            min_twap_samples: 100,
            twap_window_secs: 300,
        },
    );

    // 99 % of pool — huge price impact.
    let result =
        client.try_flash_loan(&receiver_id, &asset, &99_000, &1_000_000, &Bytes::new(&env));
    assert_eq!(result, Err(Ok(FlashLoanError::ExcessivePriceImpact)));
}

#[test]
fn test_flash_loan_blocked_by_twap_deviation() {
    let pool_balance: i128 = 1_000_000;
    let (env, contract_id, admin, asset) = setup_with_liquidity(pool_balance);
    let client = LendingContractClient::new(&env, &contract_id);
    let receiver_id = env.register(FlashLoanReceiver, ());
    let token_admin = token::StellarAssetClient::new(&env, &asset);
    token_admin.mint(&receiver_id, &100_000);

    // Set a very tight TWAP deviation tolerance (10 bps = 0.1 %).
    client.set_flash_manipulation_config(
        &admin,
        &ManipulationConfig {
            max_borrow_liquidity_bps: 5_000,
            max_price_impact_bps: 10_000,
            max_twap_deviation_bps: 10,
            min_twap_samples: 3,
            twap_window_secs: 300,
        },
    );

    // Seed 3 samples at price 1_000_000.
    client.flash_record_price(&asset, &1_000_000);
    client.flash_record_price(&asset, &1_000_000);
    client.flash_record_price(&asset, &1_000_000);

    // Spot price now 200 % higher — TWAP check should block it.
    let manipulated_price: i128 = 3_000_000;
    let result = client.try_flash_loan(
        &receiver_id,
        &asset,
        &10_000,
        &manipulated_price,
        &Bytes::new(&env),
    );
    assert_eq!(result, Err(Ok(FlashLoanError::PriceManipulationDetected)));
}

#[test]
fn test_flash_loan_twap_check_passes_within_tolerance() {
    let pool_balance: i128 = 1_000_000;
    let (env, contract_id, admin, asset) = setup_with_liquidity(pool_balance);
    let client = LendingContractClient::new(&env, &contract_id);
    let receiver_id = env.register(FlashLoanReceiver, ());
    let token_admin = token::StellarAssetClient::new(&env, &asset);
    token_admin.mint(&receiver_id, &10_000);

    // 200 bps TWAP tolerance.
    client.set_flash_manipulation_config(
        &admin,
        &ManipulationConfig {
            max_borrow_liquidity_bps: 5_000,
            max_price_impact_bps: 10_000,
            max_twap_deviation_bps: 200,
            min_twap_samples: 3,
            twap_window_secs: 300,
        },
    );

    // Seed 3 samples at 1_000_000.
    client.flash_record_price(&asset, &1_000_000);
    client.flash_record_price(&asset, &1_000_000);
    client.flash_record_price(&asset, &1_000_000);

    // Spot 1 % above TWAP — within 2 % tolerance.
    let result =
        client.try_flash_loan(&receiver_id, &asset, &10_000, &1_010_000, &Bytes::new(&env));
    // Should not fail with PriceManipulationDetected (may fail for other reasons in test env).
    assert_ne!(result, Err(Ok(FlashLoanError::PriceManipulationDetected)));
}
