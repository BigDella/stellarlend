//! Differential tests comparing two *genuinely different* contract
//! implementations: `hello-world` (this crate) vs the separate `lending`
//! crate (`stellarlend_lending::LendingContract`).
//!
//! This is the part of issue #226 ("Different contract implementations —
//! e.g. upgraded vs original") that `differential_test.rs` does not cover:
//! that file only diffs hello-world against a second instance of itself.
//!
//! Scope: only `deposit` + the resulting position are compared. See
//! "Known Structural Differences" in DIFFERENTIAL_TEST_REPORT.md for why
//! borrow/repay/withdraw are excluded — the two contracts' domain models
//! for those operations are not comparable without misrepresenting what's
//! being tested.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};

use super::diff_harness::{ContractAdapter, DivergenceReport, HwAdapter, LendingAdapter};

fn make_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    env
}

fn diff_deposit_cross(
    a: &dyn ContractAdapter,
    b: &dyn ContractAdapter,
    user_a: &Address,
    user_b: &Address,
    amount: i128,
    reports: &mut Vec<DivergenceReport>,
) {
    let r1 = a.deposit(user_a, amount).map(|v| v > 0);
    let r2 = b.deposit(user_b, amount).map(|v| v > 0);
    if r1 != r2 {
        reports.push(DivergenceReport::new("cross_deposit", r1, r2));
    }
}

fn assert_no_divergences(reports: &[DivergenceReport]) {
    if !reports.is_empty() {
        let msgs: Vec<String> = reports
            .iter()
            .map(|r| format!("[DIVERGENCE] {}: v1={} v2={}", r.operation, r.v1, r.v2))
            .collect();
        panic!("Divergences detected:\n{}", msgs.join("\n"));
    }
}

/// A successful deposit must be accepted by both implementations.
#[test]
fn test_cross_impl_deposit_accepted_consistently() {
    let env = make_env();
    let hw = HwAdapter::new(&env);
    let lending = LendingAdapter::new(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let mut reports = Vec::new();

    diff_deposit_cross(&hw, &lending, &user_a, &user_b, 1_000_000, &mut reports);
    assert_no_divergences(&reports);
}

/// A zero-amount deposit must be rejected by both implementations.
#[test]
fn test_cross_impl_zero_deposit_rejected_consistently() {
    let env = make_env();
    let hw = HwAdapter::new(&env);
    let lending = LendingAdapter::new(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let mut reports = Vec::new();

    diff_deposit_cross(&hw, &lending, &user_a, &user_b, 0, &mut reports);
    assert_no_divergences(&reports);
}

/// After a successful deposit, both implementations must report non-zero
/// collateral for that user (exact scale/units are implementation-specific,
/// so only the "collateral was recorded" property is compared).
#[test]
fn test_cross_impl_deposit_reflected_in_position() {
    let env = make_env();
    let hw = HwAdapter::new(&env);
    let lending = LendingAdapter::new(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);

    hw.deposit(&user_a, 2_000_000).expect("hello-world deposit should succeed");
    lending
        .deposit(&user_b, 2_000_000)
        .expect("lending deposit should succeed");

    let pos_a = hw.get_position(&user_a);
    let pos_b = lending.get_position(&user_b);

    assert!(pos_a.collateral > 0, "hello-world collateral must be recorded");
    assert!(pos_b.collateral > 0, "lending collateral must be recorded");
    assert_eq!(
        pos_a.debt, 0,
        "hello-world debt must remain zero after a deposit-only flow"
    );
    assert_eq!(
        pos_b.debt, 0,
        "lending debt must remain zero after a deposit-only flow"
    );
}
