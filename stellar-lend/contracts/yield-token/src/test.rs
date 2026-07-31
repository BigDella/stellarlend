#![cfg(test)]
use crate::{YieldToken, YieldTokenClient, YieldTokenError};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup() -> (Env, Address, YieldTokenClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(YieldToken, ());
    let client = YieldTokenClient::new(&env, &contract_id);

    let underlying = Address::generate(&env);
    let splitter = Address::generate(&env);
    let name = String::from_str(&env, "Yield Token");
    let symbol = String::from_str(&env, "YT");
    let maturity = env.ledger().timestamp() + 86400 * 365;

    client.initialize(&admin, &name, &symbol, &underlying, &splitter, &maturity);
    (env, splitter, client)
}

#[test]
fn test_initialize() {
    let (_env, _splitter, client) = setup();
    assert_eq!(client.name(), String::from_str(&_env, "Yield Token"));
    assert_eq!(client.symbol(), String::from_str(&_env, "YT"));
    assert_eq!(client.decimals(), 7);
    assert_eq!(client.total_supply(), 0);
    assert!(client.is_active());
}

#[test]
fn test_mint() {
    let (_env, splitter, client) = setup();
    let user = Address::generate(&_env);
    client.mint(&user, &1000);
    assert_eq!(client.balance(&user), 1000);
    assert_eq!(client.total_supply(), 1000);
}

#[test]
fn test_burn() {
    let (_env, splitter, client) = setup();
    let user = Address::generate(&_env);
    client.mint(&user, &1000);
    client.burn(&user, &400);
    assert_eq!(client.balance(&user), 600);
    assert_eq!(client.total_supply(), 600);
}

#[test]
fn test_transfer() {
    let (_env, splitter, client) = setup();
    let user1 = Address::generate(&_env);
    let user2 = Address::generate(&_env);
    client.mint(&user1, &1000);
    client.transfer(&user1, &user2, &300);
    assert_eq!(client.balance(&user1), 700);
    assert_eq!(client.balance(&user2), 300);
}

#[test]
fn test_approve_and_transfer_from() {
    let (_env, splitter, client) = setup();
    let owner = Address::generate(&_env);
    let spender = Address::generate(&_env);
    let recipient = Address::generate(&_env);
    client.mint(&owner, &1000);
    client.approve(&owner, &spender, &500);
    assert_eq!(client.allowance(&owner, &spender), 500);
    client.transfer_from(&spender, &owner, &recipient, &200);
    assert_eq!(client.balance(&owner), 800);
    assert_eq!(client.balance(&recipient), 200);
}

#[test]
fn test_insufficient_balance_transfer() {
    let (_env, splitter, client) = setup();
    let user1 = Address::generate(&_env);
    let user2 = Address::generate(&_env);
    client.mint(&user1, &50);
    _env.mock_all_auths();
    let result = client.try_transfer(&user1, &user2, &100);
    assert_eq!(result, Err(Ok(YieldTokenError::InsufficientBalance)));
}

#[test]
fn test_insufficient_allowance() {
    let (_env, splitter, client) = setup();
    let owner = Address::generate(&_env);
    let spender = Address::generate(&_env);
    let recipient = Address::generate(&_env);
    client.mint(&owner, &1000);
    client.approve(&owner, &spender, &100);
    let result = client.try_transfer_from(&spender, &owner, &recipient, &200);
    assert_eq!(result, Err(Ok(YieldTokenError::InsufficientAllowance)));
}
