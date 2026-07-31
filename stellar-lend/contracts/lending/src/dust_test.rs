use super::*;
use crate::borrow::{calculate_interest, BorrowDataKey};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

fn setup(env: &Env) -> (Address, LendingContractClient<'_>) {
    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(env, &contract_id);
    (contract_id, client)
}

fn initialize(client: &LendingContractClient, admin: &Address) {
    client.initialize(admin, &1_000_000_000, &1_000);
    client.initialize_deposit_settings(&1_000_000_000, &1);
    client.initialize_withdraw_settings(&100);
}

#[test]
fn test_withdraw_rejects_leftover_dust() {
    let env = Env::default();
    env.mock_all_auths();
    let (_contract_id, client) = setup(&env);
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    initialize(&client, &admin);
    client.deposit(&user, &asset, &150);

    let result = client.try_withdraw(&user, &asset, &100);
    assert_eq!(result, Err(Ok(WithdrawError::DustAmount)));

    let position = client.get_user_collateral_deposit(&user, &asset);
    assert_eq!(position.amount, 150);
}

#[test]
fn test_sweep_deposit_dust_clears_existing_dust() {
    let env = Env::default();
    env.mock_all_auths();
    let (_contract_id, client) = setup(&env);
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    initialize(&client, &admin);
    client.deposit(&user, &asset, &50);

    let swept = client.sweep_deposit_dust(&user, &asset);
    assert_eq!(swept, 50);

    let position = client.get_user_collateral_deposit(&user, &asset);
    assert_eq!(position.amount, 0);
}

#[test]
fn test_sweep_deposit_dust_rejects_non_dust_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let (_contract_id, client) = setup(&env);
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    initialize(&client, &admin);
    client.deposit(&user, &asset, &150);

    let result = client.try_sweep_deposit_dust(&user, &asset);
    assert_eq!(result, Err(Ok(WithdrawError::DustAmount)));
}

#[test]
fn test_repay_rejects_residual_debt_dust() {
    let env = Env::default();
    env.mock_all_auths();
    let (_contract_id, client) = setup(&env);
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);
    let collateral_asset = Address::generate(&env);

    initialize(&client, &admin);
    client.borrow(&user, &asset, &1_000, &collateral_asset, &1_500);

    let result = client.try_repay(&user, &asset, &500);
    assert_eq!(result, Err(Ok(BorrowError::DustAmount)));

    let debt = client.get_user_debt(&user);
    assert_eq!(debt.borrowed_amount, 1_000);
}

#[test]
fn test_sweep_debt_dust_clears_existing_dust() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client) = setup(&env);
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    initialize(&client, &admin);

    env.as_contract(&contract_id, || {
        let position = DebtPosition {
            borrowed_amount: 500,
            interest_accrued: 0,
            last_update: env.ledger().timestamp(),
            asset: asset.clone(),
            rate_type: RateType::Variable,
            stable_rate_bps: 0,
        };
        env.storage().persistent().set(
            &BorrowDataKey::BorrowUserVariableDebt(user.clone()),
            &position,
        );
        env.storage()
            .persistent()
            .set(&BorrowDataKey::BorrowUserDebt(user.clone()), &position);
        env.storage()
            .persistent()
            .set(&BorrowDataKey::BorrowTotalDebt, &500_i128);
    });

    let swept = client.sweep_debt_dust(&user, &asset);
    assert_eq!(swept, 500);

    let debt = client.get_user_debt(&user);
    assert_eq!(debt.borrowed_amount, 0);
    assert_eq!(debt.interest_accrued, 0);
}

#[test]
fn test_sweep_debt_dust_rejects_non_dust_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let (_contract_id, client) = setup(&env);
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);
    let collateral_asset = Address::generate(&env);

    initialize(&client, &admin);
    client.borrow(&user, &asset, &1_000, &collateral_asset, &1_500);

    let result = client.try_sweep_debt_dust(&user, &asset);
    assert_eq!(result, Err(Ok(BorrowError::DustAmount)));
}

#[test]
fn test_interest_rounding_is_depositor_friendly() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, _client) = setup(&env);
    let asset = Address::generate(&env);

    env.ledger().with_mut(|li| {
        li.timestamp = 1;
    });

    let position = DebtPosition {
        borrowed_amount: 1_000,
        interest_accrued: 0,
        last_update: 0,
        asset,
        rate_type: RateType::Variable,
        stable_rate_bps: 0,
    };

    let interest = env
        .as_contract(&contract_id, || calculate_interest(&env, &position))
        .unwrap();
    assert_eq!(interest, 1);
}
