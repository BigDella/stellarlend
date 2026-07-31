//! Tests for issue #666's minimal auto-compounding slice
//! (`amm::compound_lp_fees` / `HelloContract::amm_compound_lp_fees`).
//!
//! No prior test file covered the amm-lending module (record_lp_fees,
//! wrap_deposit_to_lp, etc.) at all, so this is a fresh file rather than an
//! extension of an existing one.

use crate::{HelloContract, HelloContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn create_test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

#[test]
fn test_compound_lp_fees_reinvests_and_zeroes_accrued() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let asset = Address::generate(&env);
    let amm_protocol = Address::generate(&env);

    client.initialize_amm_lending(&admin);
    client.amm_wrap_deposit_to_lp(&admin, &asset, &1_000_000i128, &amm_protocol);
    client.amm_record_lp_fees(&admin, &asset, &50_000i128);

    assert_eq!(client.amm_get_accrued_lp_fees(&asset), 50_000i128);
    assert_eq!(client.amm_get_lp_token_balance(&asset), 1_000_000i128);

    let compounded = client.amm_compound_lp_fees(&admin, &asset);
    assert_eq!(compounded, 50_000i128);

    // Fees are reinvested into the LP balance and the accrued counter resets.
    assert_eq!(client.amm_get_lp_token_balance(&asset), 1_050_000i128);
    assert_eq!(client.amm_get_accrued_lp_fees(&asset), 0i128);
}

#[test]
fn test_compound_lp_fees_is_a_noop_when_nothing_accrued() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let asset = Address::generate(&env);
    let amm_protocol = Address::generate(&env);

    client.initialize_amm_lending(&admin);
    client.amm_wrap_deposit_to_lp(&admin, &asset, &1_000_000i128, &amm_protocol);

    // Nothing accrued yet — compounding is a legitimate no-op, not an error.
    let compounded = client.amm_compound_lp_fees(&admin, &asset);
    assert_eq!(compounded, 0i128);
    assert_eq!(client.amm_get_lp_token_balance(&asset), 1_000_000i128);
}

#[test]
fn test_compound_lp_fees_requires_admin() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let impostor = Address::generate(&env);
    let asset = Address::generate(&env);
    let amm_protocol = Address::generate(&env);

    client.initialize_amm_lending(&admin);
    client.amm_wrap_deposit_to_lp(&admin, &asset, &1_000_000i128, &amm_protocol);
    client.amm_record_lp_fees(&admin, &asset, &50_000i128);

    let result = client.try_amm_compound_lp_fees(&impostor, &asset);
    assert!(result.is_err());
}

#[test]
fn test_compounding_twice_only_reinvests_new_fees() {
    let env = create_test_env();
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let asset = Address::generate(&env);
    let amm_protocol = Address::generate(&env);

    client.initialize_amm_lending(&admin);
    client.amm_wrap_deposit_to_lp(&admin, &asset, &1_000_000i128, &amm_protocol);
    client.amm_record_lp_fees(&admin, &asset, &10_000i128);
    client.amm_compound_lp_fees(&admin, &asset);

    // A second compound with nothing newly accrued must not double-count.
    let second = client.amm_compound_lp_fees(&admin, &asset);
    assert_eq!(second, 0i128);
    assert_eq!(client.amm_get_lp_token_balance(&asset), 1_010_000i128);

    // New fees accrue and compound independently of the earlier round.
    client.amm_record_lp_fees(&admin, &asset, &5_000i128);
    let third = client.amm_compound_lp_fees(&admin, &asset);
    assert_eq!(third, 5_000i128);
    assert_eq!(client.amm_get_lp_token_balance(&asset), 1_015_000i128);
}
