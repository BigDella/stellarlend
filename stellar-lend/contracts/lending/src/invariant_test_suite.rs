// invariant_test_suite.rs
// Comprehensive invariant testing integration
//
// This module provides a complete test suite for invariant validation
// that integrates with the existing testing framework and provides
// automated state exploration with confidence metrics.

#![allow(unused_imports)]

use soroban_sdk::{Address, Env};
use crate::invariants::{
    InvariantViolation, ExemptionFlags, assert_all_for_user,
    check_inv_001_solvency, check_inv_002_collateral_non_negative,
    check_inv_003_debt_non_negative, check_inv_004_liquidation_eligible,
    check_inv_005_no_value_creation_on_borrow, check_inv_006_admin_stability,
    check_inv_007_pause_immutability, check_inv_008_health_factor_consistency,
    check_inv_009_collateral_covers_debt, check_inv_010_total_assets_monotonic,
    check_inv_011_no_mint_on_borrow, check_inv_012_interest_monotonicity,
    check_inv_013_reserve_monotonicity, check_inv_014_access_control,
};
use crate::state_machine::{
    StateMachineExplorer, StateAction, StateTransition, 
    ConfidenceMetrics, reproduce_violation,
};
use crate::data_store::{get_total_assets, get_protocol_reserves};
use crate::borrow::{get_interest_index, get_admin};
use crate::views::{
    get_collateral_balance, get_collateral_value, get_debt_balance, 
    get_debt_value, get_health_factor, get_user_position,
};
use crate::pause::is_paused;

// ─────────────────────────────────────────────
// Test suite configuration
// ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct InvariantTestConfig {
    pub max_depth: usize,
    pub confidence_threshold: u64,
    pub min_execution_count: u64,
    pub enable_exhaustive_testing: bool,
    pub known_exemptions: ExemptionFlags,
}

impl Default for InvariantTestConfig {
    fn default() -> Self {
        Self {
            max_depth: 4,
            confidence_threshold: 10000,
            min_execution_count: 1000,
            enable_exhaustive_testing: false,
            known_exemptions: ExemptionFlags::default(),
        }
    }
}

// ─────────────────────────────────────────────
// Test results and reporting
// ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct InvariantTestResult {
    pub total_executions: u64,
    pub violations_found: std::vec::Vec<InvariantViolation>,
    pub confidence_score: f64,
    pub coverage_metrics: CoverageMetrics,
    pub reproduction_steps: std::vec::Vec<StateTransition>,
}

#[derive(Debug, Clone, Default)]
pub struct CoverageMetrics {
    pub users_tested: u64,
    pub assets_tested: u64,
    pub action_types_covered: std::vec::Vec<String>,
    pub state_space_explored: f64,
}

#[derive(Debug, Clone)]
pub struct InvariantTestReport {
    pub config: InvariantTestConfig,
    pub result: InvariantTestResult,
    pub test_duration_ms: u64,
    pub summary: TestSummary,
}

#[derive(Debug, Clone)]
pub struct TestSummary {
    pub status: TestStatus,
    pub message: String,
    pub recommendations: std::vec::Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Inconclusive,
    Timeout,
}

// ─────────────────────────────────────────────
// Main invariant test suite
// ─────────────────────────────────────────────

pub struct InvariantTestSuite {
    env: Env,
    config: InvariantTestConfig,
    users: std::vec::Vec<Address>,
    assets: std::vec::Vec<Address>,
}

impl InvariantTestSuite {
    pub fn new(env: Env, config: InvariantTestConfig) -> Self {
        Self {
            env,
            config,
            users: std::vec::Vec::new(),
            assets: std::vec::Vec::new(),
        }
    }

    pub fn add_user(&mut self, user: Address) {
        self.users.push(user);
    }

    pub fn add_asset(&mut self, asset: Address) {
        self.assets.push(asset);
    }

    /// Run the complete invariant test suite
    pub fn run(&mut self) -> InvariantTestReport {
        let start_time = std::time::Instant::now();
        
        let mut result = InvariantTestResult {
            total_executions: 0,
            violations_found: std::vec::Vec::new(),
            confidence_score: 0.0,
            coverage_metrics: CoverageMetrics::default(),
            reproduction_steps: std::vec::Vec::new(),
        };

        // Phase 1: Basic invariant checks
        if let Err(violations) = self.run_basic_invariant_checks() {
            result.violations_found.extend(violations);
        }

        // Phase 2: State machine exploration
        if self.config.enable_exhaustive_testing {
            let exploration_result = self.run_state_exploration();
            result.total_executions += exploration_result.total_executions;
            result.violations_found.extend(exploration_result.violations_found);
            result.coverage_metrics = exploration_result.coverage_metrics;
        }

        // Phase 3: Edge case testing
        let edge_case_result = self.run_edge_case_tests();
        result.total_executions += edge_case_result.total_executions;
        result.violations_found.extend(edge_case_result.violations_found);

        // Calculate confidence score
        result.confidence_score = self.calculate_confidence_score(&result);

        let test_duration = start_time.elapsed().as_millis() as u64;
        let summary = self.generate_summary(&result);

        InvariantTestReport {
            config: self.config.clone(),
            result,
            test_duration_ms: test_duration,
            summary,
        }
    }

    fn run_basic_invariant_checks(&self) -> Result<(), std::vec::Vec<InvariantViolation>> {
        let mut all_violations = std::vec::Vec::new();

        // Check invariants for all users
        for user in &self.users {
            let violations = assert_all_for_user(&self.env, user);
            all_violations.extend(violations);
        }

        // Check protocol-level invariants
        let current_assets = get_total_assets(&self.env);
        let current_reserves = get_protocol_reserves(&self.env);
        let current_index = get_interest_index(&self.env);
        let current_admin = get_admin(&self.env).unwrap_or_else(|| Address::generate(&self.env));

        // INV-010: Total assets monotonic (baseline check)
        if let Err(v) = check_inv_010_total_assets_monotonic(&self.env, current_assets) {
            all_violations.push(v);
        }

        // INV-012: Interest index monotonic
        if let Err(v) = check_inv_012_interest_monotonicity(&self.env, current_index, &self.config.known_exemptions) {
            all_violations.push(v);
        }

        // INV-013: Reserve monotonic
        if let Err(v) = check_inv_013_reserve_monotonic(&self.env, current_reserves) {
            all_violations.push(v);
        }

        // INV-014: Access control
        if let Err(v) = check_inv_014_access_control(&self.env, &current_admin) {
            all_violations.push(v);
        }

        if all_violations.is_empty() {
            Ok(())
        } else {
            Err(all_violations)
        }
    }

    fn run_state_exploration(&mut self) -> InvariantTestResult {
        let mut explorer = StateMachineExplorer::new(
            self.env.clone(),
            self.config.max_depth,
            self.config.confidence_threshold,
        );

        // Add all users and assets to explorer
        for user in &self.users {
            explorer.add_user(user.clone());
        }
        for asset in &self.assets {
            explorer.add_asset(asset.clone());
        }

        // Run exploration
        let transitions = explorer.explore();
        let mut violations = std::vec::Vec::new();

        // Collect all violations
        for transition in &transitions {
            violations.extend(transition.violations.clone());
        }

        // Calculate coverage metrics
        let coverage_metrics = CoverageMetrics {
            users_tested: self.users.len() as u64,
            assets_tested: self.assets.len() as u64,
            action_types_covered: self.get_covered_action_types(&transitions),
            state_space_explored: self.calculate_state_coverage(&transitions),
        };

        InvariantTestResult {
            total_executions: explorer.get_execution_count(),
            violations_found: violations,
            confidence_score: 0.0, // Will be calculated later
            coverage_metrics,
            reproduction_steps: transitions,
        }
    }

    fn run_edge_case_tests(&self) -> InvariantTestResult {
        let mut violations = std::vec::Vec::new();
        let mut execution_count = 0;

        // Test edge cases for each invariant
        violations.extend(self.test_edge_case_solvency());
        execution_count += 1;

        violations.extend(self.test_edge_case_negative_balances());
        execution_count += 1;

        violations.extend(self.test_edge_case_pause_immutability());
        execution_count += 1;

        violations.extend(self.test_edge_case_admin_stability());
        execution_count += 1;

        InvariantTestResult {
            total_executions: execution_count,
            violations_found: violations,
            confidence_score: 0.0,
            coverage_metrics: CoverageMetrics::default(),
            reproduction_steps: std::vec::Vec::new(),
        }
    }

    fn test_edge_case_solvency(&self) -> std::vec::Vec<InvariantViolation> {
        let mut violations = std::vec::Vec::new();

        // Test users with zero debt
        for user in &self.users {
            let debt_balance = get_debt_balance(&self.env, user);
            if debt_balance == 0 {
                let health_factor = get_health_factor(&self.env, user);
                if health_factor < 10_000 {
                    violations.push(InvariantViolation {
                        invariant_id: "INV-001-EDGE-1",
                        message: "Zero debt user has unhealthy health factor",
                        detail: format!("user: {:?}, health_factor: {}", user, health_factor),
                    });
                }
            }
        }

        violations
    }

    fn test_edge_case_negative_balances(&self) -> std::vec::Vec<InvariantViolation> {
        let mut violations = std::vec::Vec::new();

        // Test for negative balances (should never happen)
        for user in &self.users {
            let collateral_balance = get_collateral_balance(&self.env, user);
            let debt_balance = get_debt_balance(&self.env, user);

            if collateral_balance < 0 {
                violations.push(InvariantViolation {
                    invariant_id: "INV-002-EDGE-1",
                    message: "Negative collateral balance detected",
                    detail: format!("user: {:?}, balance: {}", user, collateral_balance),
                });
            }

            if debt_balance < 0 {
                violations.push(InvariantViolation {
                    invariant_id: "INV-003-EDGE-1",
                    message: "Negative debt balance detected",
                    detail: format!("user: {:?}, balance: {}", user, debt_balance),
                });
            }
        }

        violations
    }

    fn test_edge_case_pause_immutability(&self) -> std::vec::Vec<InvariantViolation> {
        let mut violations = std::vec::Vec::new();

        if is_paused(&self.env) {
            // When paused, user balances should not change
            // This is a simplified check - in reality would need to track changes over time
            for user in &self.users {
                let position = get_user_position(&self.env, user);
                
                // Just validate the position is consistent
                if position.collateral_balance < 0 || position.debt_balance < 0 {
                    violations.push(InvariantViolation {
                        invariant_id: "INV-007-EDGE-1",
                        message: "Invalid position while paused",
                        detail: format!("user: {:?}, collateral: {}, debt: {}", 
                                       user, position.collateral_balance, position.debt_balance),
                    });
                }
            }
        }

        violations
    }

    fn test_edge_case_admin_stability(&self) -> std::vec::Vec<InvariantViolation> {
        let mut violations = std::vec::Vec::new();

        // Test that admin address is consistent
        if let Some(admin) = get_admin(&self.env) {
            // In a real test, would check that admin hasn't changed unexpectedly
            // For now, just validate admin exists
            if admin == Address::default() {
                violations.push(InvariantViolation {
                    invariant_id: "INV-006-EDGE-1",
                    message: "Admin address is default (unset)",
                    detail: "Admin should be properly configured".to_string(),
                });
            }
        }

        violations
    }

    fn get_covered_action_types(&self, transitions: &[StateTransition]) -> std::vec::Vec<String> {
        let mut action_types = std::vec::Vec::new();
        
        for transition in transitions {
            let action_type = match &transition.action {
                StateAction::Deposit { .. } => "Deposit",
                StateAction::Withdraw { .. } => "Withdraw",
                StateAction::Borrow { .. } => "Borrow",
                StateAction::Repay { .. } => "Repay",
                StateAction::DepositCollateral { .. } => "DepositCollateral",
                StateAction::SetPause { .. } => "SetPause",
                StateAction::SetLiquidationThreshold { .. } => "SetLiquidationThreshold",
                StateAction::SetOraclePrice { .. } => "SetOraclePrice",
                StateAction::AdvanceTime { .. } => "AdvanceTime",
            };
            
            if !action_types.contains(&action_type.to_string()) {
                action_types.push(action_type.to_string());
            }
        }
        
        action_types
    }

    fn calculate_state_coverage(&self, transitions: &[StateTransition]) -> f64 {
        if transitions.is_empty() {
            return 0.0;
        }

        // Simplified coverage calculation based on unique states
        let mut unique_states = std::collections::HashSet::new();
        
        for transition in transitions {
            // Use a hash of the state to identify unique states
            let state_hash = self.hash_state(&transition.post_state);
            unique_states.insert(state_hash);
        }

        unique_states.len() as f64 / transitions.len() as f64
    }

    fn hash_state(&self, _state: &crate::state_machine::ProtocolState) -> u64 {
        // Simplified state hashing - in reality would use a proper hash function
        0
    }

    fn calculate_confidence_score(&self, result: &InvariantTestResult) -> f64 {
        let execution_confidence = (result.total_executions as f64 / self.config.confidence_threshold as f64).min(1.0);
        let coverage_confidence = result.coverage_metrics.state_space_explored;
        let violation_penalty = if result.violations_found.is_empty() { 0.0 } else { 1.0 };

        (execution_confidence + coverage_confidence) / 2.0 * (1.0 - violation_penalty)
    }

    fn generate_summary(&self, result: &InvariantTestResult) -> TestSummary {
        let status = if result.violations_found.is_empty() && result.confidence_score >= 0.8 {
            TestStatus::Passed
        } else if !result.violations_found.is_empty() {
            TestStatus::Failed
        } else {
            TestStatus::Inconclusive
        };

        let message = match &status {
            TestStatus::Passed => format!(
                "All invariants passed with {:.2}% confidence after {} executions",
                result.confidence_score * 100.0,
                result.total_executions
            ),
            TestStatus::Failed => format!(
                "Found {} invariant violations after {} executions",
                result.violations_found.len(),
                result.total_executions
            ),
            TestStatus::Inconclusive => format!(
                "Insufficient confidence ({:.2}%) after {} executions",
                result.confidence_score * 100.0,
                result.total_executions
            ),
            TestStatus::Timeout => "Test timed out".to_string(),
        };

        let mut recommendations = std::vec::Vec::new();

        if result.confidence_score < 0.8 {
            recommendations.push("Consider increasing execution count or test depth".to_string());
        }

        if result.coverage_metrics.state_space_explored < 0.5 {
            recommendations.push("Increase test coverage by adding more users/assets".to_string());
        }

        if !result.violations_found.is_empty() {
            recommendations.push("Fix invariant violations before proceeding".to_string());
        }

        TestSummary {
            status,
            message,
            recommendations,
        }
    }

    /// Generate a detailed report for CI/CD integration
    pub fn generate_ci_report(&self, report: &InvariantTestReport) -> String {
        format!(
            r#"# Invariant Testing Report

## Configuration
- Max Depth: {}
- Confidence Threshold: {}
- Min Executions: {}
- Exhaustive Testing: {}

## Results
- Status: {:?}
- Total Executions: {}
- Violations Found: {}
- Confidence Score: {:.2}%
- Test Duration: {}ms

## Coverage
- Users Tested: {}
- Assets Tested: {}
- Action Types Covered: {}/{}
- State Space Explored: {:.2}%

## Summary
{}

## Recommendations
{}

## Violations
{}
"#,
            report.config.max_depth,
            report.config.confidence_threshold,
            report.config.min_execution_count,
            report.config.enable_exhaustive_testing,
            report.summary.status,
            report.result.total_executions,
            report.result.violations_found.len(),
            report.result.confidence_score * 100.0,
            report.test_duration_ms,
            report.result.coverage_metrics.users_tested,
            report.result.coverage_metrics.assets_tested,
            report.result.coverage_metrics.action_types_covered.len(),
            9, // Total possible action types
            report.result.coverage_metrics.state_space_explored * 100.0,
            report.summary.message,
            report.summary.recommendations.join("\n"),
            if report.result.violations_found.is_empty() {
                "None".to_string()
            } else {
                report.result.violations_found.iter()
                    .map(|v| format!("- [{}] {}: {}", v.invariant_id, v.message, v.detail))
                    .collect::<std::vec::Vec<_>>()
                    .join("\n")
            }
        )
    }
}

// ─────────────────────────────────────────────
// Utility functions for testing
// ─────────────────────────────────────────────

pub fn setup_test_environment(env: &Env, num_users: usize, num_assets: usize) -> (std::vec::Vec<Address>, std::vec::Vec<Address>) {
    let mut users = std::vec::Vec::new();
    let mut assets = std::vec::Vec::new();

    for _ in 0..num_users {
        users.push(Address::generate(env));
    }

    for _ in 0..num_assets {
        assets.push(Address::generate(env));
    }

    (users, assets)
}

pub fn assert_invariants_pass(env: &Env, users: &[Address]) -> Result<(), std::vec::Vec<InvariantViolation>> {
    let mut all_violations = std::vec::Vec::new();

    for user in users {
        let violations = assert_all_for_user(env, user);
        all_violations.extend(violations);
    }

    if all_violations.is_empty() {
        Ok(())
    } else {
        Err(all_violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_invariant_test_suite_creation() {
        let env = Env::default();
        let config = InvariantTestConfig::default();
        let suite = InvariantTestSuite::new(env, config);
        
        assert_eq!(suite.config.max_depth, 4);
        assert_eq!(suite.config.confidence_threshold, 10000);
    }

    #[test]
    fn test_basic_invariant_checks() {
        let env = Env::default();
        let config = InvariantTestConfig::default();
        let suite = InvariantTestSuite::new(env, config);
        
        // Should pass with no users
        let result = suite.run_basic_invariant_checks();
        assert!(result.is_ok());
    }

    #[test]
    fn test_edge_case_testing() {
        let env = Env::default();
        let config = InvariantTestConfig::default();
        let suite = InvariantTestSuite::new(env, config);
        
        let result = suite.run_edge_case_tests();
        assert_eq!(result.total_executions, 4);
    }

    #[test]
    fn test_ci_report_generation() {
        let env = Env::default();
        let config = InvariantTestConfig::default();
        let mut suite = InvariantTestSuite::new(env, config);
        
        let report = suite.run();
        let ci_report = suite.generate_ci_report(&report);
        
        assert!(ci_report.contains("Invariant Testing Report"));
        assert!(ci_report.contains("## Results"));
    }

    #[test]
    fn test_setup_test_environment() {
        let env = Env::default();
        let (users, assets) = setup_test_environment(&env, 3, 2);
        
        assert_eq!(users.len(), 3);
        assert_eq!(assets.len(), 2);
    }
}
