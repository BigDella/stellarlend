use crate::proptest_helpers::{make_harness, MAX_AMOUNT, MIN_AMOUNT};
use proptest::prelude::*;

proptest! {
    /// PROP-WITH-01: balance conservation after withdraw
    #[test]
    fn prop_withdraw_conservation(
        deposit in MIN_AMOUNT..=MAX_AMOUNT,
        withdraw in MIN_AMOUNT..=MIN_AMOUNT,
    ) {
        let (_env, client, _admin, user, asset, _col) = make_harness();
        // deposit() returns i128 — no .expect()
        client.deposit(&user, &asset, &deposit);
        let w = withdraw.min(deposit);
        // withdraw() returns i128 — no .expect()
        let remaining = client.withdraw(&user, &asset, &w);
        prop_assert_eq!(remaining, deposit - w);
    }

    /// PROP-WITH-02: cannot withdraw more than balance
    #[test]
    fn prop_withdraw_exceeds_balance_fails(
        deposit in MIN_AMOUNT..=MAX_AMOUNT / 2,
        extra  in 1_i128..=MAX_AMOUNT / 2,
    ) {
        let (_env, client, _admin, user, asset, _col) = make_harness();
        client.deposit(&user, &asset, &deposit);
        prop_assert!(client.try_withdraw(&user, &asset, &(deposit + extra)).is_err());
    }

    /// PROP-WITH-03: full withdraw leaves zero balance
    #[test]
    fn prop_full_withdraw_zero(deposit in MIN_AMOUNT..=MAX_AMOUNT) {
        let (_env, client, _admin, user, asset, _col) = make_harness();
        client.deposit(&user, &asset, &deposit);
        let remaining = client.withdraw(&user, &asset, &deposit);
        prop_assert_eq!(remaining, 0_i128);
    }

    /// PROP-WITH-04: zero withdraw rejected
    #[test]
    fn prop_withdraw_zero_rejected(deposit in MIN_AMOUNT..=MAX_AMOUNT) {
        let (_env, client, _admin, user, asset, _col) = make_harness();
        client.deposit(&user, &asset, &deposit);
        prop_assert!(client.try_withdraw(&user, &asset, &0).is_err());
    }
}
