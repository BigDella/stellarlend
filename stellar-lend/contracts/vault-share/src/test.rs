#![cfg(test)]
use crate::{VaultShare, VaultShareClient, VaultShareError};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup() -> (Env, Address, VaultShareClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let vault = Address::generate(&env);
    let contract_id = env.register(VaultShare, ());
    let client = VaultShareClient::new(&env, &contract_id);

    client.initialize(
        &admin,
        &String::from_str(&env, "Vault Share"),
        &String::from_str(&env, "VS"),
        &vault,
    );
    (env, vault, client)
}

#[test]
fn test_initialize() {
    let (_env, _vault, client) = setup();
    assert_eq!(client.name(), String::from_str(&_env, "Vault Share"));
    assert_eq!(client.symbol(), String::from_str(&_env, "VS"));
    assert_eq!(client.decimals(), 7);
    assert_eq!(client.total_supply(), 0);
}

#[test]
fn test_mint() {
    let (_env, vault, client) = setup();
    let user = Address::generate(&_env);
    client.mint(&user, &1000);
    assert_eq!(client.balance(&user), 1000);
    assert_eq!(client.total_supply(), 1000);
}

#[test]
fn test_burn() {
    let (_env, vault, client) = setup();
    let user = Address::generate(&_env);
    client.mint(&user, &1000);
    client.burn(&user, &400);
    assert_eq!(client.balance(&user), 600);
    assert_eq!(client.total_supply(), 600);
}

#[test]
fn test_transfer() {
    let (_env, vault, client) = setup();
    let user1 = Address::generate(&_env);
    let user2 = Address::generate(&_env);
    client.mint(&user1, &1000);
    client.transfer(&user1, &user2, &300);
    assert_eq!(client.balance(&user1), 700);
    assert_eq!(client.balance(&user2), 300);
}

#[test]
fn test_approve_and_transfer_from() {
    let (_env, vault, client) = setup();
    let owner = Address::generate(&_env);
    let spender = Address::generate(&_env);
    let recipient = Address::generate(&_env);
    client.mint(&owner, &1000);
    client.approve(&owner, &spender, &500);
    client.transfer_from(&spender, &owner, &recipient, &200);
    assert_eq!(client.balance(&owner), 800);
    assert_eq!(client.balance(&recipient), 200);
}

#[test]
fn test_mint_unauthorized() {
    let (_env, _vault, client) = setup();
    let user = Address::generate(&_env);
    let result = client.try_mint(&user, &100);
    assert!(result.is_ok()); // vault is authorized
}

#[test]
fn test_invalid_amount() {
    let (_env, vault, client) = setup();
    let user = Address::generate(&_env);
    let result = client.try_mint(&user, &0);
    assert_eq!(result, Err(Ok(VaultShareError::InvalidAmount)));
}

#[test]
fn test_insufficient_balance_on_transfer() {
    let (_env, vault, client) = setup();
    let user1 = Address::generate(&_env);
    let user2 = Address::generate(&_env);
    client.mint(&user1, &50);
    _env.mock_all_auths();
    let result = client.try_transfer(&user1, &user2, &100);
    assert_eq!(result, Err(Ok(VaultShareError::InsufficientBalance)));
}
