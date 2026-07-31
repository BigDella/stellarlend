//! Rate limiter integration tests.
//!
//! Validates that borrow/liquidate entrypoints enforce per-user and global-per-pool
//! limits, including burst/grace and admin bypass.

#![cfg(test)]

use crate::rate_limiter::{CongestionConfig, RateLimitConfig};
use crate::{HelloContract, HelloContractClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env, Symbol};

const BPS: i128 = 10_000;

fn adaptive_config(enabled: bool) -> CongestionConfig {
    CongestionConfig {
        enabled,
        baseline_secs_per_ledger: 5,
        report_ttl_seconds: 300,
        min_factor_bps: 2_500,
        max_factor_bps: 10_000,
    }
}

fn setup(env: &Env) -> (Address, Address, HelloContractClient<'_>) {
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (contract_id, admin, client)
}

#[test]
fn test_borrow_rate_limited_per_user() {
    let env = Env::default();
    env.mock_all_auths();
    let (_cid, admin, client) = setup(&env);
    let user = Address::generate(&env);

    // Large collateral so we don't hit collateral ratio limits first.
    client.deposit_collateral(&user, &None, &1_000_000_000);

    // Tight limit: 2 calls per 60s, no burst.
    client.configure_rate_limit_operation(
        &admin,
        &Symbol::new(&env, "borrow"),
        &RateLimitConfig {
            window_seconds: 60,
            max_calls_per_window: 2,
            burst_calls: 0,
            grace_burst_calls: 0,
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 1);
    assert!(client.borrow_asset(&user, &None, &1).is_ok());
    assert!(client.borrow_asset(&user, &None, &1).is_ok());

    // Third call in same window should be blocked.
    let res = client.try_borrow_asset(&user, &None, &1);
    assert!(res.is_err());
}

#[test]
fn test_borrow_global_pool_rate_limited() {
    let env = Env::default();
    env.mock_all_auths();
    let (_cid, admin, client) = setup(&env);
    let u1 = Address::generate(&env);
    let u2 = Address::generate(&env);

    client.deposit_collateral(&u1, &None, &1_000_000_000);
    client.deposit_collateral(&u2, &None, &1_000_000_000);

    // Global-per-pool limit uses the same config; set max=2 and observe u2 blocked.
    client.configure_rate_limit_operation(
        &admin,
        &Symbol::new(&env, "borrow"),
        &RateLimitConfig {
            window_seconds: 60,
            max_calls_per_window: 2,
            burst_calls: 0,
            grace_burst_calls: 0,
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 1);
    assert!(client.borrow_asset(&u1, &None, &1).is_ok());
    assert!(client.borrow_asset(&u1, &None, &1).is_ok());

    // Global bucket should be empty now, so u2 fails even though its user bucket is fresh.
    let res = client.try_borrow_asset(&u2, &None, &1);
    assert!(res.is_err());
}

#[test]
fn test_grace_burst_allows_extra_borrows() {
    let env = Env::default();
    env.mock_all_auths();
    let (_cid, admin, client) = setup(&env);
    let user = Address::generate(&env);
    client.deposit_collateral(&user, &None, &1_000_000_000);

    client.configure_rate_limit_operation(
        &admin,
        &Symbol::new(&env, "borrow"),
        &RateLimitConfig {
            window_seconds: 60,
            max_calls_per_window: 1,
            burst_calls: 0,
            grace_burst_calls: 2,
        },
    );

    // Enable grace for user.
    client.set_user_rate_limit_grace(&admin, &user, &Symbol::new(&env, "borrow"), &true);

    env.ledger().with_mut(|li| li.timestamp = 1);
    assert!(client.borrow_asset(&user, &None, &1).is_ok());
    assert!(client.borrow_asset(&user, &None, &1).is_ok());
    assert!(client.borrow_asset(&user, &None, &1).is_ok());

    // Fourth should fail (1 + grace_burst_calls(2) = 3 capacity).
    let res = client.try_borrow_asset(&user, &None, &1);
    assert!(res.is_err());
}

#[test]
fn test_admin_bypass() {
    let env = Env::default();
    env.mock_all_auths();
    let (_cid, admin, client) = setup(&env);

    // Configure absurdly small limits.
    client.configure_rate_limit_operation(
        &admin,
        &Symbol::new(&env, "borrow"),
        &RateLimitConfig {
            window_seconds: 60,
            max_calls_per_window: 1,
            burst_calls: 0,
            grace_burst_calls: 0,
        },
    );

    // Admin borrow call should bypass limits (even if it would otherwise be rate limited).
    client.deposit_collateral(&admin, &None, &1_000_000_000);
    env.ledger().with_mut(|li| li.timestamp = 1);
    assert!(client.borrow_asset(&admin, &None, &1).is_ok());
    assert!(client.borrow_asset(&admin, &None, &1).is_ok());
}


// ── Issue #667: congestion-adaptive rate limiting ──────────────────────────────

#[test]
fn test_congestion_disabled_by_default() {
    let env = Env::default();
    env.mock_all_auths();
    let (_cid, _admin, client) = setup(&env);

    let state = client.get_rate_limit_congestion_state();
    assert!(!state.config.enabled);
    assert_eq!(state.factor_bps, BPS);
}

#[test]
fn test_congestion_report_scales_down_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (_cid, admin, client) = setup(&env);
    let user = Address::generate(&env);

    client.deposit_collateral(&user, &None, &1_000_000_000);
    client.configure_rate_limit_operation(
        &admin,
        &Symbol::new(&env, "borrow"),
        &RateLimitConfig {
            window_seconds: 60,
            max_calls_per_window: 4,
            burst_calls: 0,
            grace_burst_calls: 0,
        },
    );

    // Enable congestion adaptation and report double-normal congestion (20_000 bps).
    // factor = 10_000 * 10_000 / 20_000 = 5_000 bps -> max_calls scales 4 -> 2.
    client.configure_rate_limit_congestion(&admin, &adaptive_config(true));
    client.report_network_congestion(&admin, &20_000i128);

    let state = client.get_rate_limit_congestion_state();
    assert_eq!(state.congestion_bps, 20_000i128);
    assert_eq!(state.factor_bps, 5_000i128);

    env.ledger().with_mut(|li| li.timestamp = 1);
    assert!(client.borrow_asset(&user, &None, &1).is_ok());
    assert!(client.borrow_asset(&user, &None, &1).is_ok());

    // Third call would have been allowed under the unscaled limit of 4, but the
    // congestion-scaled effective limit is 2.
    let res = client.try_borrow_asset(&user, &None, &1);
    assert!(res.is_err());
}

#[test]
fn test_congestion_factor_never_starves_users() {
    let env = Env::default();
    env.mock_all_auths();
    let (_cid, admin, client) = setup(&env);
    let user = Address::generate(&env);

    client.deposit_collateral(&user, &None, &1_000_000_000);
    client.configure_rate_limit_operation(
        &admin,
        &Symbol::new(&env, "borrow"),
        &RateLimitConfig {
            window_seconds: 60,
            max_calls_per_window: 1,
            burst_calls: 0,
            grace_burst_calls: 0,
        },
    );

    // Extreme congestion report, but min_factor_bps floors the scaling at 25%, and
    // max_calls_per_window itself is floored at 1 regardless of scaling — so an
    // already-configured operation is throttled, never fully denied.
    client.configure_rate_limit_congestion(&admin, &adaptive_config(true));
    client.report_network_congestion(&admin, &1_000_000i128);

    env.ledger().with_mut(|li| li.timestamp = 1);
    assert!(client.borrow_asset(&user, &None, &1).is_ok());
}

#[test]
fn test_congestion_report_expires_and_falls_back_to_normal() {
    let env = Env::default();
    env.mock_all_auths();
    let (_cid, admin, client) = setup(&env);

    client.configure_rate_limit_congestion(&admin, &adaptive_config(true));
    client.report_network_congestion(&admin, &50_000i128);

    let state = client.get_rate_limit_congestion_state();
    assert_eq!(state.congestion_bps, 50_000i128);

    // Advance past the 300s report TTL with no ledger-interval sample recorded yet;
    // the resolved congestion index must fall back to normal (BPS), not stay pinned.
    env.ledger().with_mut(|li| li.timestamp = 301);
    let state_after = client.get_rate_limit_congestion_state();
    assert_eq!(state_after.congestion_bps, BPS);
    assert_eq!(state_after.source, Symbol::new(&env, "none"));
}

#[test]
fn test_non_admin_non_reporter_cannot_report_congestion() {
    let env = Env::default();
    env.mock_all_auths();
    let (_cid, admin, client) = setup(&env);
    let impostor = Address::generate(&env);

    client.configure_rate_limit_congestion(&admin, &adaptive_config(true));
    let res = client.try_report_network_congestion(&impostor, &20_000i128);
    assert!(res.is_err());
}

#[test]
fn test_congestion_reporter_role_can_report() {
    let env = Env::default();
    env.mock_all_auths();
    let (_cid, admin, client) = setup(&env);
    let reporter = Address::generate(&env);

    client.configure_rate_limit_congestion(&admin, &adaptive_config(true));

    // Grant the congestion_reporter role directly via the internal admin module
    // (no public grant_role entrypoint is wired for this contract yet); mirrors
    // the pattern used in access_control_regression_test.rs.
    crate::admin::grant_role(
        &env,
        admin.clone(),
        Symbol::new(&env, "congestion_reporter"),
        reporter.clone(),
    )
    .unwrap();

    assert!(client
        .try_report_network_congestion(&reporter, &20_000i128)
        .is_ok());
    let state = client.get_rate_limit_congestion_state();
    assert_eq!(state.congestion_bps, 20_000i128);
}
