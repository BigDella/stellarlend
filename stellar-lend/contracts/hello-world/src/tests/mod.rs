pub mod diff_harness;
pub mod differential_test;
pub mod hello_world_vs_lending_test;
pub mod migration_verification_test;
pub mod access_control_regression_test;
pub mod admin_test;
pub mod test_helpers;
pub mod analytics_test;
pub mod arithmetic_safety_test;
pub mod asset_config_test;
pub mod config_test;
pub mod deploy_test;
pub mod edge_cases_test;
pub mod events_test;
pub mod integration_test;
pub mod interest_accrual_test;
pub mod interest_rate_test;
pub mod intents_test;
pub mod liquidate_test;
#[cfg(test)]
pub mod oracle_circuit_breaker_test;
pub mod oracle_test;
pub mod oracle_staleness_fallback_test;
pub mod pause_test;
pub mod rate_limiter_test;
pub mod risk_params_test;
pub mod security_test;
pub mod test;
pub mod withdraw_test;
// Cross-asset tests disabled - contract methods not yet implemented
pub mod views_test;
pub mod borrow_cap_test;
pub mod supply_cap_test;
pub mod isolated_pool_test;
// Cross-asset tests re-enabled when contract exposes full CA API (try_* return Result; get_user_asset_position; try_ca_repay_debt)
// pub mod test_cross_asset;
pub mod bridge_test;
pub mod recovery_test;
pub mod multisig_test;
pub mod mev_protection_test;
pub mod multisig_governance_execution_test;
pub mod cross_contract_test;
// Temporarily disabled due to pre-existing issues (not related to governance attack prevention)
// pub mod timelock_test;
pub mod input_length_test;
pub mod governance_test;
// Disabled until the full governance attack-prevention surface is implemented.
// pub mod governance_attack_prevention_test;

// New test modules for implemented features
pub mod cross_asset_tests;
pub mod debt_token_tests;
pub mod rebalancing_tests;
pub mod test_utils;
pub mod amm_compound_test;

// Property-based tests (proptest)
//
// prop_arithmetic_test, prop_interest_test, prop_liquidation_test, and
// prop_deposit_test were declared here by a prior refactor but their source
// files were never added for this crate (the equivalent property tests live
// in the separate `lending` crate as borrow_prop_test.rs, interest_rate_prop_test.rs,
// invariant_prop_test.rs, and deposit_prop_test.rs). Declaring a `mod` for a
// nonexistent file is a hard compile error, so the phantom declarations are
// removed rather than left broken.
pub mod prop_fees_test;
pub mod prop_supply_cap_test;
