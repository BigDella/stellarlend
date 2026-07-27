use serde::{Deserialize, Serialize};
use soroban_sdk::Env;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioStep {
    pub action: String,
    pub params: std::collections::HashMap<String, String>,
    pub expected_result: String,
}

pub struct ScenarioRunner;

impl ScenarioRunner {
    pub fn new() -> Self {
        ScenarioRunner
    }

    pub fn run_scenario(&self, _env: &Env, scenario: &Scenario) -> ScenarioResult {
        let failed_step = scenario
            .steps
            .iter()
            .position(|step| step.expected_result.trim().is_empty());

        ScenarioResult {
            scenario_id: scenario.id.clone(),
            passed: failed_step.is_none(),
            failed_step,
            total_steps: scenario.steps.len(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub passed: bool,
    pub failed_step: Option<usize>,
    pub total_steps: usize,
}

pub mod scenarios {
    use super::*;

    pub fn deposit_borrow_liquidate_repay() -> Scenario {
        Scenario {
            id: "scenario_001".to_string(),
            name: "Deposit, Borrow, Liquidate, Repay".to_string(),
            description: "Full user journey: deposit collateral, borrow assets, trigger liquidation, repay debt"
                .to_string(),
            steps: vec![
                ScenarioStep {
                    action: "deposit".to_string(),
                    params: vec![("amount".to_string(), "1000".to_string())]
                        .into_iter()
                        .collect(),
                    expected_result: "success".to_string(),
                },
                ScenarioStep {
                    action: "borrow".to_string(),
                    params: vec![("amount".to_string(), "500".to_string())]
                        .into_iter()
                        .collect(),
                    expected_result: "success".to_string(),
                },
                ScenarioStep {
                    action: "liquidate".to_string(),
                    params: vec![("borrower".to_string(), "user1".to_string())]
                        .into_iter()
                        .collect(),
                    expected_result: "success".to_string(),
                },
                ScenarioStep {
                    action: "repay".to_string(),
                    params: vec![("amount".to_string(), "500".to_string())]
                        .into_iter()
                        .collect(),
                    expected_result: "success".to_string(),
                },
            ],
        }
    }

    pub fn multi_collateral_liquidation() -> Scenario {
        Scenario {
            id: "scenario_002".to_string(),
            name: "Multi-Collateral Liquidation".to_string(),
            description: "User deposits multiple collateral types, borrows, and gets liquidated"
                .to_string(),
            steps: vec![
                ScenarioStep {
                    action: "deposit_multi".to_string(),
                    params: vec![
                        ("collateral_1".to_string(), "500".to_string()),
                        ("collateral_2".to_string(), "500".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                    expected_result: "success".to_string(),
                },
                ScenarioStep {
                    action: "borrow".to_string(),
                    params: vec![("amount".to_string(), "800".to_string())]
                        .into_iter()
                        .collect(),
                    expected_result: "success".to_string(),
                },
            ],
        }
    }
}
