extern crate std;
use crate::proptest_helpers::{LARGE_CEILING, MAX_AMOUNT, MIN_AMOUNT};
use crate::{LendingContract, LendingContractClient};
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[derive(Clone, Debug)]
enum Action {
    Deposit {
        user: u8,
        amount: i128,
    },
    Withdraw {
        user: u8,
        amount: i128,
    },
    Borrow {
        user: u8,
        borrow: i128,
        collateral: i128,
    },
    Repay {
        user: u8,
        amount: i128,
    },
}

fn action_strategy() -> impl Strategy<Value = Action> {
    prop_oneof![
        (0u8..=2, MIN_AMOUNT..=MAX_AMOUNT / 4)
            .prop_map(|(u, a)| Action::Deposit { user: u, amount: a }),
        (0u8..=2, MIN_AMOUNT..=MAX_AMOUNT / 8)
            .prop_map(|(u, a)| Action::Withdraw { user: u, amount: a }),
        (0u8..=2, MIN_AMOUNT..=MAX_AMOUNT / 4).prop_flat_map(|(u, b)| {
            let min_c = (b * 15_000 + 9_999) / 10_000;
            (Just(u), Just(b), min_c..=MAX_AMOUNT / 2).prop_map(|(u, b, c)| Action::Borrow {
                user: u,
                borrow: b,
                collateral: c,
            })
        }),
        (0u8..=2, MIN_AMOUNT..=MAX_AMOUNT / 4)
            .prop_map(|(u, a)| Action::Repay { user: u, amount: a }),
    ]
}

proptest! {
    /// PROP-INV-01: balances always non-negative across any action sequence
    #[test]
    fn prop_balances_always_non_negative(
        actions in prop::collection::vec(action_strategy(), 1..8)
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(LendingContract, ());
        let client = LendingContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let mut users = std::vec::Vec::new();
        for _ in 0..3 { users.push(Address::generate(&env)); }
        let asset     = Address::generate(&env);
        let col_asset = Address::generate(&env);

        client.initialize(&admin, &LARGE_CEILING, &MIN_AMOUNT);
        client.initialize_deposit_settings(&LARGE_CEILING, &MIN_AMOUNT);
        client.initialize_withdraw_settings(&MIN_AMOUNT);

        for action in &actions {
            let idx = match action {
                Action::Deposit{user,..}|Action::Withdraw{user,..}
                |Action::Borrow{user,..}|Action::Repay{user,..} => (*user as usize) % 3,
            };
            let user = &users[idx];
            match action {
                Action::Deposit{amount,..} => {
                    let _ = client.try_deposit(user, &asset, amount);
                }
                Action::Withdraw{amount,..} => {
                    let _ = client.try_withdraw(user, &asset, amount);
                }
                Action::Borrow{borrow,collateral,..} => {
                    let _ = client.try_borrow(user, &asset, borrow, &col_asset, collateral);
                }
                Action::Repay{amount,..} => {
                    let _ = client.try_repay(user, &asset, amount);
                }
            }
            for u in &users {
                prop_assert!(client.get_collateral_balance(u) >= 0);
                prop_assert!(client.get_debt_balance(u)       >= 0);
            }
        }
    }

    /// PROP-INV-02: deposit + full withdraw = no-op
    #[test]
    fn prop_deposit_withdraw_noop(amount in MIN_AMOUNT..=MAX_AMOUNT) {
        let (_env, client, _admin, user, asset, _col) =
            crate::proptest_helpers::make_harness();
        client.deposit(&user, &asset, &amount);
        client.withdraw(&user, &asset, &amount);
        prop_assert_eq!(client.get_collateral_balance(&user), 0);
    }

    /// PROP-INV-03: user isolation — B's balance unaffected by A's operations
    #[test]
    fn prop_user_isolation(
        dep_a in MIN_AMOUNT..=MAX_AMOUNT/2,
        dep_b in MIN_AMOUNT..=MAX_AMOUNT/2,
    ) {
        let (env, client, _admin, _u, asset, _col) =
            crate::proptest_helpers::make_harness();
        let user_a = Address::generate(&env);
        let user_b = Address::generate(&env);
        client.deposit(&user_a, &asset, &dep_a);
        client.deposit(&user_b, &asset, &dep_b);
        let bal_b = client.get_collateral_balance(&user_b);
        let _ = client.try_withdraw(&user_a, &asset, &(dep_a / 2 + 1));
        prop_assert_eq!(client.get_collateral_balance(&user_b), bal_b);
    }
}
