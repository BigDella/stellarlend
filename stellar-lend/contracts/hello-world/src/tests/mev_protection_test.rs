use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env};

use crate::mev_protection::{
    create_commit, execution_hint, get_commit, get_ordering_stats, reveal_borrow, user_guidance,
    MevProtectionConfig, MevProtectionError, SensitiveOperation, TxOrderingHint,
};
use crate::{HelloContract, HelloContractClient};

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
fn test_auction_bid_rejected_after_deadline() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup_contract(&env);
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let bidder = Address::generate(&env);
    let borrower = Address::generate(&env);

    client.initialize(&admin);
    client
        .configure_mev_protection(
            &admin,
            &MevProtectionConfig {
                commit_delay_secs: 30,
                commit_expiry_secs: 300,
                suspicious_window_secs: 60,
                fee_smoothing_bps: 100,
                base_protection_fee_bps: 50,
                surge_protection_fee_bps: 100,
                sandwich_threshold_bps: 250,
                private_mempool_enabled: true,
                batching_enabled: true,
                batch_window_secs: 60,
                default_max_slippage_bps: 200,
            },
        )
        .unwrap();

    env.ledger().with_mut(|li| li.timestamp = 10);
    let bid_deadline = 20;
    let slot_id = client
        .place_auction_bid(&bidder, &borrower, &1_000, &900, &100, &bid_deadline)
        .unwrap();

    env.ledger().with_mut(|li| li.timestamp = 21);
    let result = client.try_place_auction_bid(&bidder, &borrower, &1_000, &900, &100, &bid_deadline);
    assert!(result.is_err());

    let bids = client.get_auction_bids(&slot_id);
    assert_eq!(bids.len(), 1);
    assert_eq!(bids.get(0).unwrap().deadline, bid_deadline);
}

#[test]
fn test_auction_bid_rejected_at_deadline_boundary() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup_contract(&env);
    let client = HelloContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let bidder = Address::generate(&env);
    let borrower = Address::generate(&env);

    client.initialize(&admin);
    client
        .configure_mev_protection(
            &admin,
            &MevProtectionConfig {
                commit_delay_secs: 30,
                commit_expiry_secs: 300,
                suspicious_window_secs: 60,
                fee_smoothing_bps: 100,
                base_protection_fee_bps: 50,
                surge_protection_fee_bps: 100,
                sandwich_threshold_bps: 250,
                private_mempool_enabled: true,
                batching_enabled: true,
                batch_window_secs: 60,
                default_max_slippage_bps: 200,
            },
        )
        .unwrap();

    env.ledger().with_mut(|li| li.timestamp = 10);
    let bid_deadline = 20;
    client
        .place_auction_bid(&bidder, &borrower, &1_000, &900, &100, &bid_deadline)
        .unwrap();

    env.ledger().with_mut(|li| li.timestamp = 20);
    let result = client.try_place_auction_bid(&bidder, &borrower, &1_000, &900, &100, &bid_deadline);
    assert!(result.is_err());
}
