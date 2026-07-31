use crate::proptest_helpers::{make_harness, MAX_AMOUNT, MIN_AMOUNT};
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address};

proptest! {
    /// PROP-DEP-01: valid deposit returns exact amount
    #[test]
    fn prop_deposit_returns_amount(amount in MIN_AMOUNT..=MAX_AMOUNT) {
        let (env, client, _admin, user, asset, _col) = make_harness();
        let result = client.deposit(&user, &asset, &amount);
        prop_assert_eq!(result, amount);
    }

    /// PROP-DEP-02: balance is always non-negative
    #[test]
    fn prop_deposit_balance_non_negative(
        amounts in prop::collection::vec(MIN_AMOUNT..=MAX_AMOUNT, 1..6)
    ) {
        let (env, client, _admin, user, asset, _col) = make_harness();
        for a in &amounts {
            let _ = client.deposit(&user, &asset, a);
            prop_assert!(client.get_collateral_balance(&user) >= 0);
        }
    }

    /// PROP-DEP-03: deposits are additive
    #[test]
    fn prop_deposit_additive(a in MIN_AMOUNT..=MAX_AMOUNT, b in MIN_AMOUNT..=MAX_AMOUNT) {
        let (env, client, _admin, user, asset, _col) = make_harness();
        client.deposit(&user, &asset, &a);
        let before = client.get_collateral_balance(&user);
        client.deposit(&user, &asset, &b);
        let after = client.get_collateral_balance(&user);
        prop_assert_eq!(after, before + b);
    }

    /// PROP-DEP-04: zero/negative amounts rejected
    #[test]
    fn prop_deposit_rejects_zero(amount in i128::MIN..=0_i128) {
        let (env, client, _admin, user, asset, _col) = make_harness();
        prop_assert!(client.try_deposit(&user, &asset, &amount).is_err());
    }

    /// PROP-DEP-05: user isolation
    #[test]
    fn prop_deposit_user_isolation(a in MIN_AMOUNT..=MAX_AMOUNT, b in MIN_AMOUNT..=MAX_AMOUNT) {
        let (env, client, _admin, _u, asset, _col) = make_harness();
        let user_a = Address::generate(&env);
        let user_b = Address::generate(&env);
        client.deposit(&user_a, &asset, &a);
        client.deposit(&user_b, &asset, &b);
        prop_assert_eq!(client.get_collateral_balance(&user_a), a);
        prop_assert_eq!(client.get_collateral_balance(&user_b), b);
    }
}
