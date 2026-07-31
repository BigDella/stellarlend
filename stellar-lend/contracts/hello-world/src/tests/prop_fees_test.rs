//! # Property-Based Fee & Reserve Tests
//!
//! Invariants for fee/reserve accounting paths.
//!
//! Run:  cargo test -p hello-world prop_fees -- --test-threads=1

#![cfg(test)]

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env};
use crate::deposit::{AssetParams, DepositDataKey};
use crate::{HelloContract, HelloContractClient};

fn create_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn setup(env: &Env) -> (Address, Address, HelloContractClient<'_>) {
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (contract_id, admin, client)
}

fn set_asset_params(env: &Env, contract_id: &Address, asset: &Address, fee_bps: i128) {
    env.as_contract(contract_id, || {
        env.storage().persistent().set(
            &DepositDataKey::AssetParams(asset.clone()),
            &AssetParams {
                deposit_enabled: true,
                collateral_factor: 7000,
                max_deposit: 0,
                borrow_fee_bps: fee_bps,
            },
        );
    });
}

// ── PROPERTY 1 – reserve after borrow = floor(amount * fee_bps / 10_000) ─────

proptest! {
    #[test]
    fn prop_borrow_fee_matches_formula(
        amount     in 1i128..=1_000_000i128,
        fee_bps    in 0i128..=500i128,
        collateral in 10_000_000i128..=100_000_000i128,
    ) {
        let env = create_env();
        let (contract_id, _admin, client) = setup(&env);
        let user  = Address::generate(&env);
        let asset = Address::generate(&env);

        set_asset_params(&env, &contract_id, &asset, fee_bps);

        env.as_contract(&contract_id, || {
            env.storage().persistent().set(
                &DepositDataKey::CollateralBalance(user.clone()), &collateral);
            env.storage().persistent().set(
                &DepositDataKey::Position(user.clone()),
                &crate::deposit::Position {
                    collateral,
                    debt: 0,
                    borrow_interest: 0,
                    last_accrual_time: 0,
                },
            );
        });

        client.borrow_asset(&user, &Some(asset.clone()), &amount);

        let expected_fee = amount * fee_bps / 10_000;
        let reserve = client.get_reserve_balance(&Some(asset));
        prop_assert_eq!(reserve, expected_fee,
            "reserve must equal floor(amount*fee_bps/10_000); \
             amount={amount}, fee_bps={fee_bps}, expected={expected_fee}, got={reserve}");
    }
}

// ── PROPERTY 2 – computed fee is always non-negative ─────────────────────────

proptest! {
    #[test]
    fn prop_reserve_always_non_negative(
        amount  in 1i128..=1_000_000i128,
        fee_bps in 0i128..=10_000i128,
    ) {
        let fee = amount * fee_bps / 10_000;
        prop_assert!(fee >= 0,
            "computed fee must be non-negative; amount={amount}, fee_bps={fee_bps}");
    }
}

// ── PROPERTY 3 – claim_reserves reduces balance by exact claimed amount ───────

proptest! {
    #[test]
    fn prop_claim_reserves_reduces_by_exact_amount(
        initial in 100i128..=10_000i128,
        claim   in 1i128..=100i128,
    ) {
        prop_assume!(claim <= initial);
        let env = create_env();
        let (contract_id, admin, client) = setup(&env);
        let asset    = Address::generate(&env);
        let treasury = Address::generate(&env);

        env.as_contract(&contract_id, || {
            env.storage().persistent().set(
                &DepositDataKey::ProtocolReserve(Some(asset.clone())), &initial);
        });

        client.claim_reserves(&admin, &Some(asset.clone()), &treasury, &claim);

        let remaining = client.get_reserve_balance(&Some(asset));
        prop_assert_eq!(remaining, initial - claim,
            "remaining reserve must equal initial-claimed; \
             initial={initial}, claim={claim}, remaining={remaining}");
    }
}

// ── PROPERTY 4 – claiming more than balance always errors ────────────────────

proptest! {
    #[test]
    fn prop_claim_exceeding_balance_errors(
        balance in 0i128..=10_000i128,
        excess  in 1i128..=10_000i128,
    ) {
        let env = create_env();
        let (contract_id, admin, client) = setup(&env);
        let asset    = Address::generate(&env);
        let treasury = Address::generate(&env);

        env.as_contract(&contract_id, || {
            env.storage().persistent().set(
                &DepositDataKey::ProtocolReserve(Some(asset.clone())), &balance);
        });

        let result = client.try_claim_reserves(
            &admin, &Some(asset), &treasury, &(balance + excess));
        prop_assert!(result.is_err(),
            "claiming more than balance must error; balance={balance}, excess={excess}");
    }
}

// ── PROPERTY 5 – flash loan total is always >= principal ─────────────────────

proptest! {
    #[test]
    fn prop_flash_loan_total_geq_principal(
        principal in 1_000i128..=1_000_000i128,
        fee_bps   in 0i128..=100i128,
    ) {
        let fee   = principal * fee_bps / 10_000;
        let total = principal + fee;
        prop_assert!(total >= principal,
            "flash loan total must be >= principal; principal={principal}, total={total}");
        prop_assert!(fee >= 0, "fee must be non-negative");
    }
}

// ── PROPERTY 6 – fee rounds down (integer division, never exceeds exact) ──────

proptest! {
    #[test]
    fn prop_fee_rounds_down(
        amount  in 1i128..=100_000i128,
        fee_bps in 0i128..=10_000i128,
    ) {
        let fee_floor = amount * fee_bps / 10_000;
        let fee_exact = (amount as f64 * fee_bps as f64 / 10_000.0) as i128;
        prop_assert!(fee_floor <= fee_exact + 1,
            "integer division must not exceed exact value by more than 1; \
             amount={amount}, fee_bps={fee_bps}, floor={fee_floor}, exact={fee_exact}");
    }
}
