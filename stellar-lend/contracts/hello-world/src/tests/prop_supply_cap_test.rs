//! # Property-Based Supply Cap Tests
//!
//! Invariants for supply cap enforcement across all valid inputs.
//!
//! Run:  cargo test -p hello-world prop_supply_cap -- --test-threads=1

#![cfg(test)]

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env};
use crate::cross_asset::AssetConfig;
use crate::{HelloContract, HelloContractClient};

fn create_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn make_config(env: &Env, asset: Option<Address>, max_supply: i128) -> AssetConfig {
    AssetConfig {
        asset: asset.clone(),
        collateral_factor: 8000,
        liquidation_threshold: 8500,
        reserve_factor: 500,
        max_supply,
        max_borrow: 0,
        can_collateralize: true,
        can_borrow: false,
        price: 1_0000000,
        price_updated_at: env.ledger().timestamp(),
        is_isolated: false,
        is_frozen: false,
    }
}

fn setup_with_cap<'a>(
    env: &'a Env,
    admin: &'a Address,
    asset: Option<Address>,
    max_supply: i128,
) -> HelloContractClient<'a> {
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(env, &contract_id);
    client.initialize(admin);
    client.initialize_ca(admin);
    client.initialize_asset(&asset, &make_config(env, asset.clone(), max_supply));
    client
}

// ── PROPERTY 1 – any deposit <= cap succeeds (fresh pool) ────────────────────

proptest! {
    #[test]
    fn prop_deposit_within_cap_succeeds(
        cap    in 1i128..=1_000_000i128,
        amount in 1i128..=1_000_000i128,
    ) {
        prop_assume!(amount <= cap);
        let env = create_env();
        let admin = Address::generate(&env);
        let user  = Address::generate(&env);
        let asset = Address::generate(&env);

        let client = setup_with_cap(&env, &admin, Some(asset.clone()), cap);
        let result = client.try_cross_asset_deposit(&user, &Some(asset), &amount);
        prop_assert!(result.is_ok(),
            "deposit within cap must succeed; cap={cap}, amount={amount}");
    }
}

// ── PROPERTY 2 – deposit that pushes total over cap must fail ─────────────────

proptest! {
    #[test]
    fn prop_deposit_over_cap_fails(
        cap    in 100i128..=100_000i128,
        first  in 1i128..=100_000i128,
        second in 1i128..=100_000i128,
    ) {
        prop_assume!(first <= cap);
        prop_assume!(first + second > cap);

        let env = create_env();
        let admin = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let asset = Address::generate(&env);

        let client = setup_with_cap(&env, &admin, Some(asset.clone()), cap);
        client.cross_asset_deposit(&user1, &Some(asset.clone()), &first);

        let result = client.try_cross_asset_deposit(&user2, &Some(asset), &second);
        prop_assert!(result.is_err(),
            "deposit pushing total over cap must fail; \
             cap={cap}, first={first}, second={second}");
    }
}

// ── PROPERTY 3 – headroom is always accurate and non-negative ─────────────────

proptest! {
    #[test]
    fn prop_headroom_accurate(
        cap    in 100i128..=1_000_000i128,
        amount in 1i128..=100_000i128,
    ) {
        prop_assume!(amount <= cap);
        let env = create_env();
        let admin = Address::generate(&env);
        let user  = Address::generate(&env);
        let asset = Address::generate(&env);

        let client = setup_with_cap(&env, &admin, Some(asset.clone()), cap);
        client.cross_asset_deposit(&user, &Some(asset.clone()), &amount);

        let (avail, reported_cap, current) = client.get_supply_headroom(&Some(asset));

        prop_assert_eq!(reported_cap, cap,
            "reported cap must match configured cap; cap={cap}");
        prop_assert_eq!(current, amount,
            "current must equal deposited amount; amount={amount}");
        prop_assert_eq!(avail, cap - amount,
            "headroom must equal cap - current; cap={cap}, current={current}");
        prop_assert!(avail >= 0, "headroom must never be negative");
    }
}

// ── PROPERTY 4 – raising cap allows previously-rejected deposits ──────────────

proptest! {
    #[test]
    fn prop_raised_cap_allows_more_deposits(
        initial_cap in 100i128..=10_000i128,
        amount      in 1i128..=10_000i128,
        raise_by    in 1i128..=100_000i128,
    ) {
        prop_assume!(amount > initial_cap);
        let new_cap = initial_cap + raise_by;
        prop_assume!(amount <= new_cap);

        let env = create_env();
        let admin = Address::generate(&env);
        let user  = Address::generate(&env);
        let asset = Address::generate(&env);

        let client = setup_with_cap(&env, &admin, Some(asset.clone()), initial_cap);

        let before = client.try_cross_asset_deposit(&user, &Some(asset.clone()), &amount);
        prop_assert!(before.is_err(),
            "deposit over initial cap must fail; initial_cap={initial_cap}, amount={amount}");

        client.update_ca_config(
            &Some(asset.clone()),
            &None, &None, &Some(new_cap), &None, &None, &None,
        );

        let after = client.try_cross_asset_deposit(&user, &Some(asset), &amount);
        prop_assert!(after.is_ok(),
            "deposit within raised cap must succeed; new_cap={new_cap}, amount={amount}");
    }
}
