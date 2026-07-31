#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup(env: &Env) -> (LendingContractClient<'_>, Address, Address, Address) {
    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let asset = Address::generate(env);
    // Initialize lending contract (required before insurance)
    client.initialize(&admin, &1_000_000_000, &1000);
    client.insurance_initialize(&admin);
    (client, admin, asset, contract_id)
}

// ─── Initialization ───────────────────────────────────────────────────────────

#[test]
fn test_initialize_ok() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000, &1000);
    client.insurance_initialize(&admin);
    // Analytics should start at zero
    let analytics = client.insurance_get_analytics();
    assert_eq!(analytics.pool_balance, 0);
}

#[test]
fn test_initialize_twice_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let result = client.try_insurance_initialize(&admin);
    assert_eq!(result, Err(Ok(InsuranceError::AlreadyInitialized)));
}

// ─── Fund pool ────────────────────────────────────────────────────────────────

#[test]
fn test_fund_pool_increases_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _, _) = setup(&env);
    client.insurance_fund_pool(&1_000_000);
    let analytics = client.insurance_get_analytics();
    assert_eq!(analytics.pool_balance, 1_000_000);
    assert_eq!(analytics.total_fee_contributions, 1_000_000);
}

#[test]
fn test_fund_pool_allocates_emergency_fund() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _, _) = setup(&env);
    client.insurance_fund_pool(&10_000);
    let analytics = client.insurance_get_analytics();
    // 20% of 10_000 = 2_000
    assert_eq!(analytics.emergency_fund, 2_000);
}

#[test]
fn test_fund_pool_zero_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _, _) = setup(&env);
    let result = client.try_insurance_fund_pool(&0);
    assert_eq!(result, Err(Ok(InsuranceError::InvalidAmount)));
}

#[test]
fn test_fund_pool_negative_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _, _) = setup(&env);
    let result = client.try_insurance_fund_pool(&-1);
    assert_eq!(result, Err(Ok(InsuranceError::InvalidAmount)));
}

// ─── Coverage limits ─────────────────────────────────────────────────────────

#[test]
fn test_set_coverage_limit_ok() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, asset, _) = setup(&env);
    client.insurance_set_coverage_limit(&admin, &asset, &3000);
    assert_eq!(client.insurance_get_coverage_limit(&asset), 3000);
}

#[test]
fn test_set_coverage_limit_exceeds_max_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, asset, _) = setup(&env);
    let result = client.try_insurance_set_coverage_limit(&admin, &asset, &5001);
    assert_eq!(result, Err(Ok(InsuranceError::InvalidCoverageLimit)));
}

#[test]
fn test_set_coverage_limit_unauthorized_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, asset, _) = setup(&env);
    let not_admin = Address::generate(&env);
    let result = client.try_insurance_set_coverage_limit(&not_admin, &asset, &1000);
    assert_eq!(result, Err(Ok(InsuranceError::Unauthorized)));
}

// ─── Premium collection ───────────────────────────────────────────────────────

#[test]
fn test_collect_premium_updates_pool() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, asset, _) = setup(&env);
    client.insurance_fund_pool(&1_000_000);
    let payer = Address::generate(&env);
    let premium = client.insurance_collect_premium(&payer, &asset, &100_000);
    assert!(premium > 0);
    let analytics = client.insurance_get_analytics();
    assert_eq!(analytics.total_premiums_collected, premium);
    assert_eq!(analytics.pool_balance, 1_000_000 + premium);
}

#[test]
fn test_collect_premium_zero_coverage_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, asset, _) = setup(&env);
    let payer = Address::generate(&env);
    let result = client.try_insurance_collect_premium(&payer, &asset, &0);
    assert_eq!(result, Err(Ok(InsuranceError::InvalidAmount)));
}

#[test]
fn test_premium_rate_is_dynamic() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, asset, _) = setup(&env);
    client.insurance_fund_pool(&1_000_000);
    let payer = Address::generate(&env);
    client.insurance_collect_premium(&payer, &asset, &100_000);
    let rate = client.insurance_get_premium_rate(&asset);
    assert!(rate >= 10 && rate <= 500);
}

// ─── Claim submission ─────────────────────────────────────────────────────────

#[test]
fn test_submit_claim_ok() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, asset, _) = setup(&env);
    client.insurance_fund_pool(&1_000_000);
    let claimant = Address::generate(&env);
    let id = client.insurance_submit_claim(&claimant, &asset, &100_000);
    assert_eq!(id, 1);
    let claim = client.insurance_get_claim(&id).unwrap();
    assert_eq!(claim.amount, 100_000);
    assert_eq!(claim.claimant, claimant);
}

#[test]
fn test_submit_claim_exceeds_limit_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, asset, _) = setup(&env);
    client.insurance_fund_pool(&1_000_000);
    let claimant = Address::generate(&env);
    // Default limit 50% = 500_000; claim 600_000 should fail
    let result = client.try_insurance_submit_claim(&claimant, &asset, &600_000);
    assert_eq!(result, Err(Ok(InsuranceError::ExceedsCoverageLimit)));
}

#[test]
fn test_submit_claim_zero_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, asset, _) = setup(&env);
    let claimant = Address::generate(&env);
    let result = client.try_insurance_submit_claim(&claimant, &asset, &0);
    assert_eq!(result, Err(Ok(InsuranceError::InvalidAmount)));
}

#[test]
fn test_submit_claim_increments_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, asset, _) = setup(&env);
    client.insurance_fund_pool(&1_000_000);
    let claimant = Address::generate(&env);
    let id1 = client.insurance_submit_claim(&claimant, &asset, &10_000);
    let id2 = client.insurance_submit_claim(&claimant, &asset, &10_000);
    assert_eq!(id2, id1 + 1);
}

// ─── Claim evaluation ─────────────────────────────────────────────────────────

#[test]
fn test_approve_claim_deducts_pool() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, asset, _) = setup(&env);
    client.insurance_fund_pool(&1_000_000);
    let claimant = Address::generate(&env);
    let id = client.insurance_submit_claim(&claimant, &asset, &100_000);
    client.insurance_evaluate_claim(&admin, &id, &true);
    let analytics = client.insurance_get_analytics();
    assert_eq!(analytics.pool_balance, 900_000);
    assert_eq!(analytics.total_claims_paid, 100_000);
    let claim = client.insurance_get_claim(&id).unwrap();
    assert_eq!(claim.status, insurance::ClaimStatus::Approved);
}

#[test]
fn test_reject_claim_does_not_deduct_pool() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, asset, _) = setup(&env);
    client.insurance_fund_pool(&1_000_000);
    let claimant = Address::generate(&env);
    let id = client.insurance_submit_claim(&claimant, &asset, &100_000);
    client.insurance_evaluate_claim(&admin, &id, &false);
    let analytics = client.insurance_get_analytics();
    assert_eq!(analytics.pool_balance, 1_000_000);
    assert_eq!(analytics.total_claims_paid, 0);
    let claim = client.insurance_get_claim(&id).unwrap();
    assert_eq!(claim.status, insurance::ClaimStatus::Rejected);
}

#[test]
fn test_evaluate_nonexistent_claim_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let result = client.try_insurance_evaluate_claim(&admin, &999, &true);
    assert_eq!(result, Err(Ok(InsuranceError::ClaimNotFound)));
}

#[test]
fn test_evaluate_already_resolved_claim_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, asset, _) = setup(&env);
    client.insurance_fund_pool(&1_000_000);
    let claimant = Address::generate(&env);
    let id = client.insurance_submit_claim(&claimant, &asset, &100_000);
    client.insurance_evaluate_claim(&admin, &id, &true);
    let result = client.try_insurance_evaluate_claim(&admin, &id, &true);
    assert_eq!(result, Err(Ok(InsuranceError::ClaimNotPending)));
}

#[test]
fn test_evaluate_unauthorized_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, asset, _) = setup(&env);
    client.insurance_fund_pool(&1_000_000);
    let claimant = Address::generate(&env);
    let id = client.insurance_submit_claim(&claimant, &asset, &100_000);
    let not_admin = Address::generate(&env);
    let result = client.try_insurance_evaluate_claim(&not_admin, &id, &true);
    assert_eq!(result, Err(Ok(InsuranceError::Unauthorized)));
}

#[test]
fn test_approve_claim_protects_emergency_fund() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, asset, _) = setup(&env);
    // Fund 10_000 → emergency = 2_000, available = 8_000
    client.insurance_fund_pool(&10_000);
    // Set limit to 100% so submit_claim passes (we want to test evaluate_claim blocking)
    // Note: max allowed is 5000 bps (50%), so max claimable = 5_000
    // available = 8_000, so claim 4_999 should pass submit but we need to test
    // a claim that exceeds available. Use a fresh pool where emergency > 0.
    // Fund more so available < claim amount when emergency is factored in.
    // Pool=10_000, emergency=2_000, available=8_000
    // Claim exactly 8_001 — exceeds available, should fail at evaluate
    // But submit checks coverage limit (50% of pool = 5_000), so 8_001 > 5_000 fails submit.
    // To test evaluate blocking: set limit to max (5000 bps = 50%), claim 4_999 (passes submit),
    // then drain available by approving first claim, then try second claim.
    let claimant = Address::generate(&env);
    // First claim: 4_999 (within 50% limit of 10_000 pool)
    let id1 = client.insurance_submit_claim(&claimant, &asset, &4_999);
    // Approve it: pool becomes 10_000 - 4_999 = 5_001, emergency still 2_000, available = 3_001
    client.insurance_evaluate_claim(&admin, &id1, &true);
    // Second claim: 3_002 — exceeds available (3_001)
    // Pool is now 5_001, 50% limit = 2_500, so 3_002 > 2_500 → ExceedsCoverageLimit at submit
    // Use a smaller amount that passes submit but fails evaluate
    // Pool=5_001, limit=50%=2_500, claim 2_499 passes submit
    let id2 = client.insurance_submit_claim(&claimant, &asset, &2_499);
    // available = 5_001 - 2_000 = 3_001, so 2_499 < 3_001 → this would pass evaluate too
    // Better approach: make emergency fund larger than available
    // Fund a tiny pool where emergency eats most of it
    // Use a separate fresh scenario: fund 100, emergency=20, available=80
    // claim 81 → passes submit (50% of 100 = 50, 81 > 50 → fails submit)
    // The constraint is: coverage limit caps at 50% of pool, emergency is 20% of pool
    // So max claimable via submit = 50% of pool
    // available = pool - emergency = pool - 20% = 80% of pool
    // 50% < 80%, so any claim that passes submit will also pass the available check
    // The only way to trigger InsufficientPoolFunds is after pool is drained below emergency
    // Approve id2 and verify pool state
    client.insurance_evaluate_claim(&admin, &id2, &true);
    let analytics = client.insurance_get_analytics();
    // Pool drained: 10_000 - 4_999 - 2_499 = 2_502, emergency still 2_000
    // available = 2_502 - 2_000 = 502
    assert_eq!(analytics.pool_balance, 2_502);
    assert_eq!(analytics.emergency_fund, 2_000);
    assert_eq!(analytics.available_balance, 502);
    // Now try to claim 503 — passes submit (50% of 2_502 = 1_251, 503 < 1_251)
    // but available = 502, so evaluate should fail
    let id3 = client.insurance_submit_claim(&claimant, &asset, &503);
    let result = client.try_insurance_evaluate_claim(&admin, &id3, &true);
    assert_eq!(result, Err(Ok(InsuranceError::InsufficientPoolFunds)));
}

// ─── Analytics ────────────────────────────────────────────────────────────────

#[test]
fn test_analytics_available_balance_excludes_emergency_fund() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _, _) = setup(&env);
    client.insurance_fund_pool(&10_000);
    let analytics = client.insurance_get_analytics();
    // available = pool - emergency = 10_000 - 2_000 = 8_000
    assert_eq!(analytics.available_balance, 8_000);
}

#[test]
fn test_analytics_initial_state() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _, _) = setup(&env);
    let analytics = client.insurance_get_analytics();
    assert_eq!(analytics.pool_balance, 0);
    assert_eq!(analytics.emergency_fund, 0);
    assert_eq!(analytics.total_claims_paid, 0);
    assert_eq!(analytics.total_premiums_collected, 0);
    assert_eq!(analytics.total_fee_contributions, 0);
}
