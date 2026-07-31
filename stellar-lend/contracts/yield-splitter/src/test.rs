#![cfg(test)]
use crate::{YieldSplitter, YieldSplitterClient, YieldSplitterError};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup() -> (Env, Address, YieldSplitterClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(YieldSplitter, ());
    let client = YieldSplitterClient::new(&env, &contract_id);

    client.initialize(&admin);
    (env, admin, client)
}

#[test]
fn test_initialize() {
    let (_env, admin, client) = setup();
    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, Some(admin));
}

#[test]
fn test_preview_split() {
    let (_env, _admin, client) = setup();
    let now = _env.ledger().timestamp();
    let maturity = now + 86400 * 365;

    let preview = client.preview_split(&100_000, &maturity);
    assert_eq!(preview.pt_amount, 100_000);
    assert_eq!(preview.yt_amount, 100_000);
    assert!(preview.estimated_yield > 0);
    assert!(preview.time_to_maturity_secs > 0);
}

#[test]
fn test_preview_split_invalid_maturity() {
    let (_env, _admin, client) = setup();
    let now = _env.ledger().timestamp();
    let result = client.try_preview_split(&100_000, &now);
    assert_eq!(result, Err(Ok(YieldSplitterError::InvalidMaturity)));
}

#[test]
fn test_get_yield_accrued_no_position() {
    let (_env, _admin, client) = setup();
    let accrued = client.get_yield_accrued(&1);
    assert_eq!(accrued, 0);
}

#[test]
fn test_get_admin() {
    let (_env, admin, client) = setup();
    let stored = client.get_admin();
    assert_eq!(stored, Some(admin));
}

#[test]
fn test_get_owner_splits_empty() {
    let (_env, _admin, client) = setup();
    let owner = Address::generate(&_env);
    let splits = client.get_owner_splits(&owner);
    assert_eq!(splits.len(), 0);
}
