//! Tests for conditional borrow commitments (oracle triggers, cancel, execute).

use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, Vec};

use crate::{
    CommitmentStatus, LendingContract, LendingContractClient, PriceTrigger, TriggerCombiner,
};

/// Returns 1.20 USD (8 decimals) for any asset — sufficient to satisfy `>= 1.00` triggers.
#[contract]
pub struct CommitmentTestOracle;

#[contractimpl]
impl CommitmentTestOracle {
    pub fn price(_env: Env, _asset: Address) -> i128 {
        120_000_000
    }
}

#[test]
fn commitment_create_rejects_expired_deadline() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &10_000_000_000, &1000);

    let owner = Address::generate(&env);
    let asset = Address::generate(&env);

    let mut triggers = Vec::new(&env);
    triggers.push_back(PriceTrigger {
        asset: asset.clone(),
        trigger_price: 99_000_000,
        fire_when_at_or_above: true,
    });

    let now = env.ledger().timestamp();

    let err = client.try_create_borrow_commitment(
        &owner,
        &triggers,
        &TriggerCombiner::Any,
        &asset,
        &asset,
        &100_000,
        &160_000,
        &9000_u32,
        &(now.saturating_sub(1)),
    );
    assert!(err.is_err());
}

#[test]
fn commitment_cancel_by_owner() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &10_000_000_000, &1000);

    let owner = Address::generate(&env);
    let borrow_asset = Address::generate(&env);
    let collateral_asset = Address::generate(&env);

    let mut triggers = Vec::new(&env);
    triggers.push_back(PriceTrigger {
        asset: borrow_asset.clone(),
        trigger_price: 100,
        fire_when_at_or_above: true,
    });

    let now = env.ledger().timestamp();
    let id = client.create_borrow_commitment(
        &owner,
        &triggers,
        &TriggerCombiner::Any,
        &borrow_asset,
        &collateral_asset,
        &100_000,
        &160_000,
        &5000_u32,
        &(now + 86400),
    );

    client.cancel_borrow_commitment(&owner, &id);

    let c = client.get_borrow_commitment(&id).unwrap();
    assert_eq!(c.status, CommitmentStatus::Cancelled);
}

#[test]
fn commitment_execute_or_trigger_borrows() {
    let env = Env::default();
    env.mock_all_auths();

    let lending_id = env.register(LendingContract, ());
    let lending = LendingContractClient::new(&env, &lending_id);

    let oracle_id = env.register(CommitmentTestOracle, ());

    let admin = Address::generate(&env);
    lending.initialize(&admin, &10_000_000_000, &1000);
    lending.try_set_oracle(&admin, &oracle_id).unwrap().unwrap();

    let owner = Address::generate(&env);
    let borrow_asset = Address::generate(&env);
    let collateral_asset = Address::generate(&env);

    let mut triggers = Vec::new(&env);
    triggers.push_back(PriceTrigger {
        asset: borrow_asset.clone(),
        trigger_price: 100_000_000,
        fire_when_at_or_above: true,
    });

    let now = env.ledger().timestamp();
    let id = lending.create_borrow_commitment(
        &owner,
        &triggers,
        &TriggerCombiner::Any,
        &borrow_asset,
        &collateral_asset,
        &100_000,
        &160_000,
        &5000_u32,
        &(now + 86400),
    );

    lending.try_execute_borrow_commitment(&id).unwrap().unwrap();

    let c = lending.get_borrow_commitment(&id).unwrap();
    assert_eq!(c.status, CommitmentStatus::Executed);

    let debt = lending.get_user_debt(&owner);
    assert_eq!(debt.borrowed_amount, 100_000);
}
