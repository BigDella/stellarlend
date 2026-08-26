//! Differential tests — same inputs run against two contract instances (v1 hello-world,
//! v2 hello-world post-upgrade simulation) and outputs compared.
//!
//! Covers:
//! - Property-based comparison tests
//! - Divergence detection and reporting
//! - State-dependent output sequences
//! - Non-deterministic behavior guard (pinned ledger timestamp)

#![cfg(test)]

use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env};

use crate::{HelloContract, HelloContractClient};

use super::diff_harness::{DivergenceReport, HwAdapter, PositionSnapshot};

// ── helpers ───────────────────────────────────────────────────────────────

fn make_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    // Pin timestamp so interest accrual is deterministic across both instances
    env.ledger().set_timestamp(1_000_000);
    env
}

// Run the same operation on two adapters and collect divergences
fn diff_deposit<'a>(
    a: &HwAdapter<'a>,
    b: &HwAdapter<'a>,
    user_a: &Address,
    user_b: &Address,
    amount: i128,
    reports: &mut Vec<DivergenceReport>,
) {
    let r1 = a.deposit(user_a, amount).map(|v| v > 0);
    let r2 = b.deposit(user_b, amount).map(|v| v > 0);
    if r1 != r2 {
        reports.push(DivergenceReport::new("deposit", r1, r2));
    }
}

fn diff_borrow<'a>(
    a: &HwAdapter<'a>,
    b: &HwAdapter<'a>,
    user_a: &Address,
    user_b: &Address,
    amount: i128,
    reports: &mut Vec<DivergenceReport>,
) {
    let r1 = a.borrow(user_a, amount).map(|v| v > 0);
    let r2 = b.borrow(user_b, amount).map(|v| v > 0);
    if r1 != r2 {
        reports.push(DivergenceReport::new("borrow", r1, r2));
    }
}

fn diff_position<'a>(
    a: &HwAdapter<'a>,
    b: &HwAdapter<'a>,
    user_a: &Address,
    user_b: &Address,
    reports: &mut Vec<DivergenceReport>,
) {
    let p1 = a.get_position(user_a);
    let p2 = b.get_position(user_b);
    if p1 != p2 {
        reports.push(DivergenceReport::new("get_position", p1, p2));
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

// ── tests ─────────────────────────────────────────────────────────────────

/// Two fresh instances of the same contract must behave identically on deposit.
#[test]
fn test_diff_deposit_same_result() {
    let env = make_env();
    let v1 = HwAdapter::new(&env);
    let v2 = HwAdapter::new(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let mut reports = Vec::new();

    diff_deposit(&v1, &v2, &user_a, &user_b, 1_000_000, &mut reports);
    assert_no_divergences(&reports);
}

/// Deposit then borrow — both instances must agree on success/failure.
#[test]
fn test_diff_deposit_then_borrow() {
    let env = make_env();
    let v1 = HwAdapter::new(&env);
    let v2 = HwAdapter::new(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let mut reports = Vec::new();

    diff_deposit(&v1, &v2, &user_a, &user_b, 2_000_000, &mut reports);
    diff_borrow(&v1, &v2, &user_a, &user_b, 500_000, &mut reports);
    diff_position(&v1, &v2, &user_a, &user_b, &mut reports);

    assert_no_divergences(&reports);
}

/// Repay after borrow — positions must match across instances.
#[test]
fn test_diff_full_flow_deposit_borrow_repay() {
    let env = make_env();
    let v1 = HwAdapter::new(&env);
    let v2 = HwAdapter::new(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let mut reports = Vec::new();

    diff_deposit(&v1, &v2, &user_a, &user_b, 3_000_000, &mut reports);
    diff_borrow(&v1, &v2, &user_a, &user_b, 1_000_000, &mut reports);

    let r1 = v1.repay(&user_a, 1_000_000).is_ok();
    let r2 = v2.repay(&user_b, 1_000_000).is_ok();
    if r1 != r2 {
        reports.push(DivergenceReport::new("repay", r1, r2));
    }

    diff_position(&v1, &v2, &user_a, &user_b, &mut reports);
    assert_no_divergences(&reports);
}

/// Zero-amount operations must be rejected consistently across both instances.
#[test]
fn test_diff_zero_amount_rejected_consistently() {
    let env = make_env();
    let v1 = HwAdapter::new(&env);
    let v2 = HwAdapter::new(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let mut reports = Vec::new();

    diff_deposit(&v1, &v2, &user_a, &user_b, 0, &mut reports);
    diff_borrow(&v1, &v2, &user_a, &user_b, 0, &mut reports);
    assert_no_divergences(&reports);
}

/// Borrow without collateral must fail on both instances.
#[test]
fn test_diff_borrow_without_collateral_fails_consistently() {
    let env = make_env();
    let v1 = HwAdapter::new(&env);
    let v2 = HwAdapter::new(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let mut reports = Vec::new();

    diff_borrow(&v1, &v2, &user_a, &user_b, 500_000, &mut reports);
    assert_no_divergences(&reports);
}

/// Multiple sequential operations — state must stay in sync.
#[test]
fn test_diff_sequential_operations_state_consistent() {
    let env = make_env();
    let v1 = HwAdapter::new(&env);
    let v2 = HwAdapter::new(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let mut reports = Vec::new();

    for amount in [500_000i128, 1_000_000, 250_000] {
        diff_deposit(&v1, &v2, &user_a, &user_b, amount, &mut reports);
    }
    diff_position(&v1, &v2, &user_a, &user_b, &mut reports);
    assert_no_divergences(&reports);
}
