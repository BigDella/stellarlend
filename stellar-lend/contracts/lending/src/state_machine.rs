// state_machine.rs
// Automated state machine exploration for invariant testing
//
// This module provides systematic exploration of protocol states to ensure
// invariants hold across all possible state transitions.

#![allow(unused_imports)]

use soroban_sdk::{Address, Env};
use crate::invariants::{
    InvariantViolation, ExemptionFlags, assert_all_for_user,
    check_inv_001_solvency, check_inv_002_collateral_non_negative,
    check_inv_003_debt_non_negative, check_inv_004_liquidation_eligible,
    check_inv_008_health_factor_consistency, check_inv_009_collateral_covers_debt,
};
use crate::data_store::{get_total_assets, get_protocol_reserves};
use crate::borrow::{get_interest_index, get_admin};

// ─────────────────────────────────────────────
// State transition types
// ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum StateAction {
    Deposit { user: Address, asset: Address, amount: i128 },
    Withdraw { user: Address, asset: Address, amount: i128 },
    Borrow { user: Address, debt_asset: Address, collateral_asset: Address, amount: i128 },
    Repay { user: Address, asset: Address, amount: i128 },
    DepositCollateral { user: Address, asset: Address, amount: i128 },
    SetPause { pause_type: u8, paused: bool },
    SetLiquidationThreshold { bps: i128 },
    SetOraclePrice { asset: Address, price: i128 },
    AdvanceTime { delta: u64 },
}

#[derive(Debug, Clone)]
pub struct StateTransition {
    pub action: StateAction,
    pub pre_state: ProtocolState,
    pub post_state: ProtocolState,
    pub violations: Vec<InvariantViolation>,
}

#[derive(Debug, Clone, Default)]
pub struct ProtocolState {
    pub total_assets: i128,
    pub protocol_reserves: i128,
    pub interest_index: i128,
    pub admin: Option<Address>,
    pub user_positions: std::vec::Vec<(Address, UserState)>,
}

#[derive(Debug, Clone, Default)]
pub struct UserState {
    pub collateral_balance: i128,
    pub debt_balance: i128,
    pub health_factor: i128,
    pub collateral_value: i128,
    pub debt_value: i128,
}

// ─────────────────────────────────────────────
// State machine explorer
// ─────────────────────────────────────────────

pub struct StateMachineExplorer {
    env: Env,
    users: std::vec::Vec<Address>,
    assets: std::vec::Vec<Address>,
    max_depth: usize,
    execution_count: u64,
    confidence_threshold: u64,
}

impl StateMachineExplorer {
    pub fn new(env: Env, max_depth: usize, confidence_threshold: u64) -> Self {
        Self {
            env,
            users: std::vec::Vec::new(),
            assets: std::vec::Vec::new(),
            max_depth,
            execution_count: 0,
            confidence_threshold,
        }
    }

    pub fn add_user(&mut self, user: Address) {
        self.users.push(user);
    }

    pub fn add_asset(&mut self, asset: Address) {
        self.assets.push(asset);
    }

    /// Execute systematic state exploration
    pub fn explore(&mut self) -> std::vec::Vec<StateTransition> {
        let mut all_transitions = std::vec::Vec::new();
        let initial_state = self.capture_state();

        // Generate action sequences up to max_depth
        for depth in 1..=self.max_depth {
            let action_sequences = self.generate_action_sequences(depth);
            
            for sequence in action_sequences {
                let transitions = self.execute_action_sequence(&sequence, &initial_state);
                all_transitions.extend(transitions);
                
                self.execution_count += 1;
                if self.execution_count >= self.confidence_threshold {
                    break;
                }
            }
            
            if self.execution_count >= self.confidence_threshold {
                break;
            }
        }

        all_transitions
    }

    fn generate_action_sequences(&self, depth: usize) -> std::vec::Vec<std::vec::Vec<StateAction>> {
        let mut sequences = std::vec::Vec::new();
        self.generate_sequences_recursive(depth, 0, std::vec::Vec::new(), &mut sequences);
        sequences
    }

    fn generate_sequences_recursive(
        &self,
        max_depth: usize,
        current_depth: usize,
        current_sequence: std::vec::Vec<StateAction>,
        sequences: &mut std::vec::Vec<std::vec::Vec<StateAction>>,
    ) {
        if current_depth == max_depth {
            if !current_sequence.is_empty() {
                sequences.push(current_sequence.clone());
            }
            return;
        }

        // Generate all possible actions
        let actions = self.generate_all_actions();
        
        for action in actions {
            let mut new_sequence = current_sequence.clone();
            new_sequence.push(action);
            self.generate_sequences_recursive(max_depth, current_depth + 1, new_sequence, sequences);
        }
    }

    fn generate_all_actions(&self) -> std::vec::Vec<StateAction> {
        let mut actions = std::vec::Vec::new();

        if self.users.is_empty() || self.assets.is_empty() {
            return actions;
        }

        // Deposit actions
        for user in &self.users {
            for asset in &self.assets {
                for amount in [1000, 10000, 100000] {
                    actions.push(StateAction::Deposit {
                        user: user.clone(),
                        asset: asset.clone(),
                        amount,
                    });
                }
            }
        }

        // Withdraw actions
        for user in &self.users {
            for asset in &self.assets {
                for amount in [1000, 10000, 100000] {
                    actions.push(StateAction::Withdraw {
                        user: user.clone(),
                        asset: asset.clone(),
                        amount,
                    });
                }
            }
        }

        // Borrow actions
        for user in &self.users {
            for debt_asset in &self.assets {
                for collateral_asset in &self.assets {
                    if debt_asset != collateral_asset {
                        for amount in [1000, 10000, 100000] {
                            actions.push(StateAction::Borrow {
                                user: user.clone(),
                                debt_asset: debt_asset.clone(),
                                collateral_asset: collateral_asset.clone(),
                                amount,
                            });
                        }
                    }
                }
            }
        }

        // Repay actions
        for user in &self.users {
            for asset in &self.assets {
                for amount in [1000, 10000, 100000] {
                    actions.push(StateAction::Repay {
                        user: user.clone(),
                        asset: asset.clone(),
                        amount,
                    });
                }
            }
        }

        // Time advance actions
        for delta in [100, 1000, 10000] {
            actions.push(StateAction::AdvanceTime { delta });
        }

        actions
    }

    fn execute_action_sequence(
        &self,
        sequence: &[StateAction],
        initial_state: &ProtocolState,
    ) -> std::vec::Vec<StateTransition> {
        let mut transitions = std::vec::Vec::new();
        let mut current_state = initial_state.clone();

        for action in sequence {
            let pre_state = current_state.clone();
            
            // Execute action (simplified - in real implementation would call contract)
            let post_state = self.apply_action(&current_state, action);
            
            // Check invariants
            let violations = self.check_invariants(&post_state);
            
            transitions.push(StateTransition {
                action: action.clone(),
                pre_state,
                post_state: post_state.clone(),
                violations,
            });
            
            current_state = post_state;
        }

        transitions
    }

    fn apply_action(&self, state: &ProtocolState, action: &StateAction) -> ProtocolState {
        let mut new_state = state.clone();
        
        // Simplified state transition logic
        // In real implementation, this would execute the actual contract action
        match action {
            StateAction::Deposit { amount, .. } => {
                new_state.total_assets += amount;
            }
            StateAction::Withdraw { amount, .. } => {
                new_state.total_assets -= amount;
            }
            StateAction::Borrow { amount, .. } => {
                // Borrow doesn't change total assets (conservation of money)
            }
            StateAction::Repay { amount, .. } => {
                new_state.total_assets += amount;
            }
            StateAction::AdvanceTime { delta, .. } => {
                // Simulate interest accrual
                new_state.interest_index += (*delta as i128);
            }
            _ => {}
        }
        
        new_state
    }

    fn check_invariants(&self, state: &ProtocolState) -> std::vec::Vec<InvariantViolation> {
        let mut violations = std::vec::Vec::new();
        
        // Check protocol-level invariants
        if state.total_assets < 0 {
            violations.push(InvariantViolation {
                invariant_id: "INV-002",
                message: "Total assets negative",
                detail: format!("total_assets: {}", state.total_assets),
            });
        }
        
        if state.protocol_reserves < 0 {
            violations.push(InvariantViolation {
                invariant_id: "INV-003",
                message: "Protocol reserves negative",
                detail: format!("reserves: {}", state.protocol_reserves),
            });
        }
        
        // Check user-level invariants
        for (user, user_state) in &state.user_positions {
            // INV-001: Solvency
            if user_state.health_factor < 10_000 && user_state.debt_balance > 0 {
                violations.push(InvariantViolation {
                    invariant_id: "INV-001",
                    message: "User undercollateralized",
                    detail: format!("user: {:?}, health_factor: {}", user, user_state.health_factor),
                });
            }
            
            // INV-002: Collateral non-negative
            if user_state.collateral_balance < 0 {
                violations.push(InvariantViolation {
                    invariant_id: "INV-002",
                    message: "Collateral balance negative",
                    detail: format!("user: {:?}, balance: {}", user, user_state.collateral_balance),
                });
            }
            
            // INV-003: Debt non-negative
            if user_state.debt_balance < 0 {
                violations.push(InvariantViolation {
                    invariant_id: "INV-003",
                    message: "Debt balance negative",
                    detail: format!("user: {:?}, balance: {}", user, user_state.debt_balance),
                });
            }
        }
        
        violations
    }

    fn capture_state(&self) -> ProtocolState {
        ProtocolState {
            total_assets: get_total_assets(&self.env),
            protocol_reserves: get_protocol_reserves(&self.env),
            interest_index: get_interest_index(&self.env),
            admin: get_admin(&self.env),
            user_positions: std::vec::Vec::new(), // Simplified
        }
    }

    pub fn get_execution_count(&self) -> u64 {
        self.execution_count
    }

    pub fn has_reached_confidence(&self) -> bool {
        self.execution_count >= self.confidence_threshold
    }
}

// ─────────────────────────────────────────────
// Violation reproduction utilities
// ─────────────────────────────────────────────

pub fn reproduce_violation(
    env: &Env,
    transition: &StateTransition,
) -> Result<(), InvariantViolation> {
    // Reset to pre-state
    reset_to_state(env, &transition.pre_state)?;
    
    // Execute the action
    execute_action(env, &transition.action)?;
    
    // Verify violations occur
    let current_state = capture_current_state(env);
    let violations = check_state_invariants(&current_state);
    
    if violations.is_empty() {
        return Err(InvariantViolation {
            invariant_id: "REPRO-001",
            message: "Failed to reproduce violation",
            detail: "No violations found after reproduction".to_string(),
        });
    }
    
    Ok(())
}

fn reset_to_state(_env: &Env, _state: &ProtocolState) -> Result<(), InvariantViolation> {
    // Implementation would reset contract state to match the given state
    Ok(())
}

fn execute_action(_env: &Env, _action: &StateAction) -> Result<(), InvariantViolation> {
    // Implementation would execute the specific action
    Ok(())
}

fn capture_current_state(_env: &Env) -> ProtocolState {
    // Implementation would capture current contract state
    ProtocolState::default()
}

fn check_state_invariants(state: &ProtocolState) -> std::vec::Vec<InvariantViolation> {
    let mut violations = std::vec::Vec::new();
    
    // Basic state validation
    if state.total_assets < 0 {
        violations.push(InvariantViolation {
            invariant_id: "INV-002",
            message: "Total assets negative",
            detail: format!("total_assets: {}", state.total_assets),
        });
    }
    
    violations
}

// ─────────────────────────────────────────────
// Confidence metrics
// ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ConfidenceMetrics {
    pub total_executions: u64,
    pub unique_states_visited: u64,
    pub violations_found: u64,
    pub coverage_percentage: f64,
}

impl ConfidenceMetrics {
    pub fn calculate_confidence(&self) -> f64 {
        if self.total_executions == 0 {
            return 0.0;
        }
        
        // Simple confidence calculation based on execution count and coverage
        let execution_confidence = (self.total_executions as f64 / 10000.0).min(1.0);
        let coverage_confidence = self.coverage_percentage;
        
        (execution_confidence + coverage_confidence) / 2.0
    }
    
    pub fn is_sufficient(&self, threshold: f64) -> bool {
        self.calculate_confidence() >= threshold && self.violations_found == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_state_machine_explorer_creation() {
        let env = Env::default();
        let explorer = StateMachineExplorer::new(env, 3, 1000);
        assert_eq!(explorer.max_depth, 3);
        assert_eq!(explorer.confidence_threshold, 1000);
    }

    #[test]
    fn test_action_generation() {
        let env = Env::default();
        let mut explorer = StateMachineExplorer::new(env, 2, 100);
        
        let user = Address::generate(&env);
        let asset = Address::generate(&env);
        
        explorer.add_user(user.clone());
        explorer.add_asset(asset.clone());
        
        let actions = explorer.generate_all_actions();
        assert!(!actions.is_empty());
    }

    #[test]
    fn test_confidence_metrics() {
        let metrics = ConfidenceMetrics {
            total_executions: 1000,
            unique_states_visited: 500,
            violations_found: 0,
            coverage_percentage: 0.8,
        };
        
        assert!(metrics.calculate_confidence() > 0.5);
        assert!(metrics.is_sufficient(0.6));
    }
}
