use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env, Symbol};

use crate::mev_protection::{
    create_commit, create_guarded_commit, create_liquidation_auction_commit, execution_hint,
    get_commit, get_monitoring_dashboard, get_ordering_stats, open_liquidation_auction,
    record_gas_bid_sample, record_private_execution, register_private_route, reveal_borrow,
    reveal_liquidation, reveal_liquidation_with_output, settle_liquidation_auction,
    submit_liquidation_bid, user_guidance, AuctionStatus, ExecutionGuard, MevProtectionError,
    SensitiveOperation, TxOrderingHint,
};
use crate::HelloContract;

fn setup_contract(env: &Env) -> Address {
    env.register(HelloContract, ())
}

#[test]
fn test_commit_reveal_requires_delay() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup_contract(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let commit_id = env.as_contract(&contract_id, || {
        create_commit(
            &env,
            user.clone(),
            SensitiveOperation::Borrow,
            Some(asset),
            None,
            None,
            500,
            100,
            TxOrderingHint::Default,
        )
        .unwrap()
    });

    let err = env
        .as_contract(&contract_id, || reveal_borrow(&env, user, commit_id))
        .unwrap_err();
    assert_eq!(err, MevProtectionError::CommitNotReady);
}

#[test]
fn test_commit_expires_after_window() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup_contract(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let commit_id = env.as_contract(&contract_id, || {
        create_commit(
            &env,
            user.clone(),
            SensitiveOperation::Borrow,
            Some(asset),
            None,
            None,
            500,
            100,
            TxOrderingHint::PrivateMempool,
        )
        .unwrap()
    });

    env.ledger().with_mut(|li| li.timestamp = 301);

    let err = env
        .as_contract(&contract_id, || reveal_borrow(&env, user, commit_id))
        .unwrap_err();
    assert_eq!(err, MevProtectionError::CommitExpired);
}

#[test]
fn test_fee_cap_blocks_surge_execution() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup_contract(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let asset = Address::generate(&env);

    let first = env.as_contract(&contract_id, || {
        create_commit(
            &env,
            user_a.clone(),
            SensitiveOperation::Borrow,
            Some(asset.clone()),
            None,
            None,
            1_000,
            100,
            TxOrderingHint::Default,
        )
        .unwrap()
    });
    let second = env.as_contract(&contract_id, || {
        create_commit(
            &env,
            user_b.clone(),
            SensitiveOperation::Borrow,
            Some(asset),
            None,
            None,
            1_000,
            5,
            TxOrderingHint::Default,
        )
        .unwrap()
    });

    env.ledger().with_mut(|li| li.timestamp = 31);
    env.as_contract(&contract_id, || reveal_borrow(&env, user_a, first))
        .unwrap();
    env.ledger().with_mut(|li| li.timestamp = 32);

    let err = env
        .as_contract(&contract_id, || reveal_borrow(&env, user_b, second))
        .unwrap_err();
    assert_eq!(err, MevProtectionError::FeeCapExceeded);
}

#[test]
fn test_sandwich_pattern_updates_monitoring_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup_contract(&env);
    let attacker = Address::generate(&env);
    let victim = Address::generate(&env);
    let asset = Address::generate(&env);

    let first = env.as_contract(&contract_id, || {
        create_commit(
            &env,
            attacker.clone(),
            SensitiveOperation::Borrow,
            Some(asset.clone()),
            None,
            None,
            2_000,
            100,
            TxOrderingHint::PrivateMempool,
        )
        .unwrap()
    });
    let middle = env.as_contract(&contract_id, || {
        create_commit(
            &env,
            victim.clone(),
            SensitiveOperation::Borrow,
            Some(asset.clone()),
            None,
            None,
            2_050,
            100,
            TxOrderingHint::Default,
        )
        .unwrap()
    });
    let last = env.as_contract(&contract_id, || {
        create_commit(
            &env,
            attacker.clone(),
            SensitiveOperation::Borrow,
            Some(asset),
            None,
            None,
            2_010,
            100,
            TxOrderingHint::BatchAuction,
        )
        .unwrap()
    });

    env.ledger().with_mut(|li| li.timestamp = 31);
    env.as_contract(&contract_id, || {
        reveal_borrow(&env, attacker.clone(), first)
    })
    .unwrap();
    env.ledger().with_mut(|li| li.timestamp = 32);
    env.as_contract(&contract_id, || reveal_borrow(&env, victim, middle))
        .unwrap();
    env.ledger().with_mut(|li| li.timestamp = 33);
    env.as_contract(&contract_id, || reveal_borrow(&env, attacker, last))
        .unwrap();

    let stats = env.as_contract(&contract_id, || get_ordering_stats(&env));
    assert!(stats.suspicious_sequences >= 2);
    assert!(stats.sandwich_alerts >= 1);
}

#[test]
fn test_guidance_hint_and_commit_lookup() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup_contract(&env);
    let user = Address::generate(&env);

    let hint = env.as_contract(&contract_id, || {
        execution_hint(&env, TxOrderingHint::Default)
    });
    assert_eq!(hint, TxOrderingHint::PrivateMempool);

    let msg = env.as_contract(&contract_id, || {
        user_guidance(&env, SensitiveOperation::Liquidate)
    });
    assert!(!msg.is_empty());

    let commit_id = env.as_contract(&contract_id, || {
        create_commit(
            &env,
            user.clone(),
            SensitiveOperation::Withdraw,
            None,
            None,
            None,
            100,
            100,
            TxOrderingHint::DelayedReveal,
        )
        .unwrap()
    });
    let commit = env
        .as_contract(&contract_id, || get_commit(&env, commit_id))
        .unwrap();
    assert_eq!(commit.owner, user);
}

#[test]
fn test_guarded_liquidation_enforces_deadline_and_slippage() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup_contract(&env);
    let liquidator = Address::generate(&env);
    let borrower = Address::generate(&env);
    let asset = Address::generate(&env);

    let guard = ExecutionGuard {
        quoted_output_amount: 1_100,
        min_output_amount: 1_050,
        max_slippage_bps: 500,
        deadline: 90,
    };
    let low_output_commit = env.as_contract(&contract_id, || {
        create_guarded_commit(
            &env,
            liquidator.clone(),
            SensitiveOperation::Liquidate,
            Some(asset.clone()),
            Some(asset.clone()),
            Some(borrower.clone()),
            1_000,
            100,
            TxOrderingHint::DelayedReveal,
            guard.clone(),
            None,
            None,
        )
        .unwrap()
    });

    env.ledger().with_mut(|li| li.timestamp = 31);
    let err = env
        .as_contract(&contract_id, || {
            reveal_liquidation_with_output(&env, liquidator.clone(), low_output_commit, 1_000)
        })
        .unwrap_err();
    assert_eq!(err, MevProtectionError::SlippageExceeded);

    let expired_commit = env.as_contract(&contract_id, || {
        create_guarded_commit(
            &env,
            liquidator.clone(),
            SensitiveOperation::Liquidate,
            Some(asset.clone()),
            Some(asset),
            Some(borrower),
            1_000,
            100,
            TxOrderingHint::DelayedReveal,
            guard,
            None,
            None,
        )
        .unwrap()
    });
    env.ledger().with_mut(|li| li.timestamp = 91);
    let err = env
        .as_contract(&contract_id, || {
            reveal_liquidation_with_output(&env, liquidator, expired_commit, 1_100)
        })
        .unwrap_err();
    assert_eq!(err, MevProtectionError::SlippageExpired);
}

#[test]
fn test_private_route_receipt_required_for_private_commit() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup_contract(&env);
    let liquidator = Address::generate(&env);
    let borrower = Address::generate(&env);
    let relay = Address::generate(&env);
    let asset = Address::generate(&env);
    let route_id = Symbol::new(&env, "flashbots");

    env.as_contract(&contract_id, || {
        register_private_route(&env, relay.clone(), route_id.clone(), relay.clone(), 120).unwrap()
    });
    let commit_id = env.as_contract(&contract_id, || {
        create_guarded_commit(
            &env,
            liquidator.clone(),
            SensitiveOperation::Liquidate,
            Some(asset.clone()),
            Some(asset),
            Some(borrower),
            1_000,
            100,
            TxOrderingHint::PrivateMempool,
            ExecutionGuard {
                quoted_output_amount: 1_100,
                min_output_amount: 1_000,
                max_slippage_bps: 1_000,
                deadline: 200,
            },
            Some(route_id.clone()),
            None,
        )
        .unwrap()
    });

    env.ledger().with_mut(|li| li.timestamp = 31);
    let err = env
        .as_contract(&contract_id, || {
            reveal_liquidation_with_output(&env, liquidator.clone(), commit_id, 1_100)
        })
        .unwrap_err();
    assert_eq!(err, MevProtectionError::PrivateRouteRequired);

    env.as_contract(&contract_id, || {
        record_private_execution(&env, relay, commit_id, route_id).unwrap()
    });
    let revealed = env
        .as_contract(&contract_id, || {
            reveal_liquidation_with_output(&env, liquidator, commit_id, 1_100)
        })
        .unwrap();
    assert_eq!(revealed.3, 1_000);
}

#[test]
fn test_liquidation_batch_auction_picks_best_bid_and_commit() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup_contract(&env);
    let opener = Address::generate(&env);
    let borrower = Address::generate(&env);
    let bidder_a = Address::generate(&env);
    let bidder_b = Address::generate(&env);
    let asset = Address::generate(&env);

    let auction_id = env.as_contract(&contract_id, || {
        open_liquidation_auction(
            &env,
            opener,
            borrower,
            Some(asset.clone()),
            Some(asset),
            5_000,
            25,
            20,
        )
        .unwrap()
    });

    env.as_contract(&contract_id, || {
        submit_liquidation_bid(&env, bidder_a, auction_id, 4_500, 50, 100, 4_900, None).unwrap()
    });
    let best_bid_id = env.as_contract(&contract_id, || {
        submit_liquidation_bid(
            &env,
            bidder_b.clone(),
            auction_id,
            5_000,
            75,
            100,
            5_200,
            None,
        )
        .unwrap()
    });

    env.ledger().with_mut(|li| li.timestamp = 21);
    let best = env
        .as_contract(&contract_id, || {
            settle_liquidation_auction(&env, bidder_b.clone(), auction_id)
        })
        .unwrap();
    assert_eq!(best.id, best_bid_id);
    assert_eq!(best.liquidator, bidder_b);

    let commit_id = env.as_contract(&contract_id, || {
        create_liquidation_auction_commit(
            &env,
            best.liquidator.clone(),
            auction_id,
            ExecutionGuard {
                quoted_output_amount: 5_500,
                min_output_amount: 5_200,
                max_slippage_bps: 600,
                deadline: 100,
            },
            None,
        )
        .unwrap()
    });
    let auction = env
        .as_contract(&contract_id, || get_commit(&env, commit_id))
        .unwrap();
    assert_eq!(auction.auction_id, Some(auction_id));
    assert_eq!(auction.hint, TxOrderingHint::BatchAuction);

    let stored = env
        .as_contract(&contract_id, || {
            crate::mev_protection::get_liquidation_auction(&env, auction_id)
        })
        .unwrap();
    assert_eq!(stored.status, AuctionStatus::Settled);
}

#[test]
fn test_gas_bid_dashboard_tracks_analysis() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup_contract(&env);
    let reporter = Address::generate(&env);
    let asset = Address::generate(&env);

    let stats = env
        .as_contract(&contract_id, || {
            record_gas_bid_sample(
                &env,
                reporter.clone(),
                SensitiveOperation::Liquidate,
                Some(asset.clone()),
                200,
                3,
            )
        })
        .unwrap();
    assert_eq!(stats.samples, 1);

    env.as_contract(&contract_id, || {
        record_gas_bid_sample(
            &env,
            reporter,
            SensitiveOperation::Liquidate,
            Some(asset.clone()),
            400,
            5,
        )
        .unwrap()
    });

    let dashboard = env.as_contract(&contract_id, || {
        get_monitoring_dashboard(&env, SensitiveOperation::Liquidate, Some(asset), 1_000)
    });
    assert_eq!(dashboard.gas_bids.samples, 2);
    assert_eq!(dashboard.gas_bids.avg_bid_microlumens, 300);
    assert_eq!(dashboard.recommended_hint, TxOrderingHint::PrivateMempool);
}

#[test]
fn test_guarded_liquidation_cannot_use_unguarded_reveal() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup_contract(&env);
    let liquidator = Address::generate(&env);
    let borrower = Address::generate(&env);

    let commit_id = env.as_contract(&contract_id, || {
        create_guarded_commit(
            &env,
            liquidator.clone(),
            SensitiveOperation::Liquidate,
            None,
            None,
            Some(borrower),
            1_000,
            100,
            TxOrderingHint::DelayedReveal,
            ExecutionGuard {
                quoted_output_amount: 1_000,
                min_output_amount: 950,
                max_slippage_bps: 500,
                deadline: 100,
            },
            None,
            None,
        )
        .unwrap()
    });

    env.ledger().with_mut(|li| li.timestamp = 31);
    let err = env
        .as_contract(&contract_id, || {
            reveal_liquidation(&env, liquidator, commit_id)
        })
        .unwrap_err();
    assert_eq!(err, MevProtectionError::InvalidOperation);
}
