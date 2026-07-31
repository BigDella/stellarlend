use crate::{LendingContract, LendingContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

pub const MIN_AMOUNT: i128 = 100;
pub const MAX_AMOUNT: i128 = 1_000_000_000_i128;
pub const LARGE_CEILING: i128 = 100_000_000_000_i128;

pub fn make_harness() -> (
    Env,
    LendingContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1_000);
    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);
    let col = Address::generate(&env);
    // These all return () or i128 — no .expect() or .unwrap()
    client.initialize(&admin, &LARGE_CEILING, &MIN_AMOUNT);
    client.initialize_deposit_settings(&LARGE_CEILING, &MIN_AMOUNT);
    client.initialize_withdraw_settings(&MIN_AMOUNT);
    (env, client, admin, user, asset, col)
}

pub fn assert_non_negative_balances(client: &LendingContractClient, user: &Address) {
    assert!(
        client.get_collateral_balance(user) >= 0,
        "collateral balance < 0"
    );
    assert!(client.get_debt_balance(user) >= 0, "debt balance < 0");
}
