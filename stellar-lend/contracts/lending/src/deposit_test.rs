use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

#[test]
fn test_deposit_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);
    client.initialize_deposit_settings(&1_000_000_000, &100);

    let balance = client.deposit(&user, &asset, &10_000);
    assert_eq!(balance, 10_000);

    let position = client.get_user_collateral_deposit(&user, &asset);
    assert_eq!(position.amount, 10_000);
}

#[test]
fn test_deposit_invalid_amount_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);
    client.initialize_deposit_settings(&1_000_000_000, &100);

    let result = client.try_deposit(&user, &asset, &0);
    assert_eq!(result, Err(Ok(DepositError::InvalidAmount)));
}

#[test]
fn test_deposit_invalid_amount_negative() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);
    client.initialize_deposit_settings(&1_000_000_000, &100);

    let result = client.try_deposit(&user, &asset, &-500);
    assert_eq!(result, Err(Ok(DepositError::InvalidAmount)));
}

#[test]
fn test_deposit_below_minimum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);
    client.initialize_deposit_settings(&1_000_000_000, &5000);

    let result = client.try_deposit(&user, &asset, &1000);
    assert_eq!(result, Err(Ok(DepositError::InvalidAmount)));
}

#[test]
fn test_deposit_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);
    client.initialize_deposit_settings(&1_000_000_000, &100);
    client.set_deposit_paused(&true);

    let result = client.try_deposit(&user, &asset, &10_000);
    assert_eq!(result, Err(Ok(DepositError::DepositPaused)));
}

#[test]
fn test_deposit_exceeds_cap() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);
    client.initialize_deposit_settings(&50_000, &100);

    let result = client.try_deposit(&user, &asset, &100_000);
    assert_eq!(result, Err(Ok(DepositError::ExceedsDepositCap)));
}

#[test]
fn test_deposit_multiple_times() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);
    client.initialize_deposit_settings(&1_000_000_000, &100);

    let balance1 = client.deposit(&user, &asset, &10_000);
    assert_eq!(balance1, 10_000);

    let balance2 = client.deposit(&user, &asset, &5_000);
    assert_eq!(balance2, 15_000);

    let position = client.get_user_collateral_deposit(&user, &asset);
    assert_eq!(position.amount, 15_000);
}

#[test]
fn test_deposit_pause_unpause() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);
    client.initialize_deposit_settings(&1_000_000_000, &100);

    client.set_deposit_paused(&true);
    let result = client.try_deposit(&user, &asset, &10_000);
    assert_eq!(result, Err(Ok(DepositError::DepositPaused)));

    client.set_deposit_paused(&false);
    let balance = client.deposit(&user, &asset, &10_000);
    assert_eq!(balance, 10_000);
}

#[test]
fn test_deposit_overflow_protection() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let admin = Address::generate(&env);
    client.initialize(&admin, &i128::MAX, &1000);
    client.initialize_deposit_settings(&i128::MAX, &100);

    client.deposit(&user, &asset, &1_000_000);

    let huge_amount = i128::MAX - 500_000;
    let result = client.try_deposit(&user, &asset, &huge_amount);
    assert_eq!(result, Err(Ok(DepositError::Overflow)));
}

#[test]
fn test_deposit_updates_timestamp() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);
    client.initialize_deposit_settings(&1_000_000_000, &100);
    client.deposit(&user, &asset, &10_000);

    let position = client.get_user_collateral_deposit(&user, &asset);
    assert_eq!(position.last_deposit_time, 1000);

    env.ledger().with_mut(|li| {
        li.timestamp = 2000;
    });

    client.deposit(&user, &asset, &5_000);
    let position = client.get_user_collateral_deposit(&user, &asset);
    assert_eq!(position.last_deposit_time, 2000);
    assert_eq!(position.amount, 15_000);
}

#[test]
fn test_deposit_separate_users() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let asset = Address::generate(&env);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);
    client.initialize_deposit_settings(&1_000_000_000, &100);

    client.deposit(&user1, &asset, &10_000);
    client.deposit(&user2, &asset, &20_000);

    let pos1 = client.get_user_collateral_deposit(&user1, &asset);
    let pos2 = client.get_user_collateral_deposit(&user2, &asset);
    assert_eq!(pos1.amount, 10_000);
    assert_eq!(pos2.amount, 20_000);
}

#[test]
fn test_donation_detection_quarantines_unaccounted_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &asset);

    client.initialize(&admin, &1_000_000_000, &1000);
    client.initialize_deposit_settings(&1_000_000_000, &100);

    client.deposit(&user, &asset, &10_000);
    token_admin.mint(&contract_id, &10_000);

    let clean_report = client.sync_donation_balance(&asset);
    assert!(!clean_report.donation_detected);
    assert_eq!(clean_report.new_unaccounted_balance, 0);
    assert_eq!(client.get_virtual_share_price_bps(&asset), 10_000);

    token_admin.mint(&contract_id, &5_000);

    let report = client.sync_donation_balance(&asset);
    assert!(report.donation_detected);
    assert_eq!(report.accounted_balance, 10_000);
    assert_eq!(report.observed_balance, 15_000);
    assert_eq!(report.new_unaccounted_balance, 5_000);
    assert_eq!(report.quarantined_balance, 5_000);

    // Donation balance is quarantined and does not inflate the virtual share price.
    assert_eq!(report.virtual_share_price_bps, 10_000);
    assert_eq!(client.get_virtual_share_price_bps(&asset), 10_000);

    let stored = client.get_donation_report(&asset).unwrap();
    assert_eq!(stored, report);
}

#[test]
fn test_donation_defense_minimum_deposit_blocks_dust() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    client.initialize(&admin, &1_000_000_000, &1000);
    client.initialize_deposit_settings(&1_000_000_000, &100);
    client.set_donation_defense_config(
        &admin,
        &DonationDefenseConfig {
            virtual_assets: 1_000,
            virtual_shares: 1_000,
            max_unaccounted_bps: 100,
            min_deposit_amount: 1_000,
        },
    );

    let result = client.try_deposit(&user, &asset, &999);
    assert_eq!(result, Err(Ok(DepositError::InvalidAmount)));

    assert_eq!(client.deposit(&user, &asset, &1_000), 1_000);
}

#[test]
fn test_donation_alert_blocks_liquidation_until_acknowledged() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);
    let debt_asset = Address::generate(&env);
    let collateral_asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &collateral_asset);

    client.initialize(&admin, &1_000_000_000, &1000);
    token_admin.mint(&contract_id, &5_000);

    let report = client.sync_donation_balance(&collateral_asset);
    assert!(report.donation_detected);

    let blocked = client.try_liquidate(
        &liquidator,
        &borrower,
        &debt_asset,
        &collateral_asset,
        &1_000,
    );
    assert_eq!(blocked, Err(Ok(BorrowError::ProtocolPaused)));

    client.acknowledge_donation(&admin, &collateral_asset);
    let allowed = client.try_liquidate(
        &liquidator,
        &borrower,
        &debt_asset,
        &collateral_asset,
        &1_000,
    );
    assert!(allowed.is_ok());
}

#[test]
fn test_deposit_cap_boundary() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);
    client.initialize_deposit_settings(&50_000, &100);

    // Exact cap — should succeed
    let balance = client.deposit(&user, &asset, &50_000);
    assert_eq!(balance, 50_000);

    // Above cap — should fail
    let result = client.try_deposit(&user, &asset, &100);
    assert_eq!(result, Err(Ok(DepositError::ExceedsDepositCap)));
}
