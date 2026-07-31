use crate::proptest_helpers::{make_harness, MAX_AMOUNT, MIN_AMOUNT};
use proptest::prelude::*;

fn valid_borrow_pair() -> impl Strategy<Value = (i128, i128)> {
    (MIN_AMOUNT..=MAX_AMOUNT / 2).prop_flat_map(|borrow| {
        let min_col = (borrow * 15_000 + 9_999) / 10_000;
        let max_col = MAX_AMOUNT.max(min_col + 1);
        (Just(borrow), min_col..=max_col)
    })
}

proptest! {
    /// PROP-BOR-01: sufficient collateral — debt becomes positive
    #[test]
    fn prop_borrow_sufficient_collateral((borrow, collateral) in valid_borrow_pair()) {
        let (_env, client, _admin, user, asset, col_asset) = make_harness();
        // borrow() returns () — use try_borrow() for the Result check
        prop_assert!(client.try_borrow(&user, &asset, &borrow, &col_asset, &collateral).is_ok());
        prop_assert!(client.get_debt_balance(&user) > 0);
    }

    /// PROP-BOR-02: zero borrow rejected
    #[test]
    fn prop_borrow_zero_rejected(col in MIN_AMOUNT..=MAX_AMOUNT) {
        let (_env, client, _admin, user, asset, col_asset) = make_harness();
        prop_assert!(client.try_borrow(&user, &asset, &0, &col_asset, &col).is_err());
    }

    /// PROP-BOR-03: insufficient collateral rejected
    #[test]
    fn prop_borrow_insufficient_collateral(borrow in MIN_AMOUNT * 2..=MAX_AMOUNT / 2) {
        let (_env, client, _admin, user, asset, col_asset) = make_harness();
        let bad_col = (borrow * 15_000 / 10_000).saturating_sub(1);
        if bad_col > 0 {
            prop_assert!(
                client.try_borrow(&user, &asset, &borrow, &col_asset, &bad_col).is_err()
            );
        }
    }

    /// PROP-BOR-04: debt never goes negative after repay
    #[test]
    fn prop_repay_debt_non_negative((borrow, collateral) in valid_borrow_pair()) {
        let (_env, client, _admin, user, asset, col_asset) = make_harness();
        // borrow() returns () — no .expect()
        client.borrow(&user, &asset, &borrow, &col_asset, &collateral);
        let debt = client.get_debt_balance(&user);
        let _ = client.try_repay(&user, &asset, &(debt + 1_000_000));
        prop_assert!(client.get_debt_balance(&user) >= 0);
    }
}
