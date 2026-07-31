use std::collections::HashMap;
use std::time::Instant;
use tokio::task::JoinHandle;

use crate::actions::{ActionResult, UserAction};
use crate::metrics::{SimulationMetrics, ThreadSafeMetrics};

pub struct SimulationConfig {
    pub user_count: usize,
    pub actions_per_user: usize,
    pub concurrent_users: usize,
    pub random_seed: Option<u64>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        SimulationConfig {
            user_count: 50,
            actions_per_user: 100,
            concurrent_users: 10,
            random_seed: None,
        }
    }
}

pub struct Simulator {
    config: SimulationConfig,
    metrics: ThreadSafeMetrics,
    results: Vec<ActionResult>,
    state: SimulationState,
}

pub struct SimulationState {
    pub user_balances: HashMap<usize, i128>,
    pub user_debt: HashMap<usize, i128>,
    pub total_supply: i128,
    pub total_debt: i128,
    pub deposit_counts: HashMap<usize, u64>,
    pub withdrawal_counts: HashMap<usize, u64>,
    pub borrow_counts: HashMap<usize, u64>,
    pub repay_counts: HashMap<usize, u64>,
}

impl Default for SimulationState {
    fn default() -> Self {
        SimulationState {
            user_balances: HashMap::new(),
            user_debt: HashMap::new(),
            total_supply: 0,
            total_debt: 0,
            deposit_counts: HashMap::new(),
            withdrawal_counts: HashMap::new(),
            borrow_counts: HashMap::new(),
            repay_counts: HashMap::new(),
        }
    }
}

impl Simulator {
    pub fn new(config: SimulationConfig) -> Self {
        let mut state = SimulationState::default();
        for i in 0..config.user_count {
            state.user_balances.insert(i, 1000);
            state.user_debt.insert(i, 0);
        }
        state.total_supply = 1000 * config.user_count as i128;

        Simulator {
            config,
            metrics: ThreadSafeMetrics::new(),
            results: Vec::new(),
            state,
        }
    }

    pub async fn run(&mut self) -> SimulationMetrics {
        let start = Instant::now();
        let mut handles: Vec<JoinHandle<Vec<ActionResult>>> = Vec::new();

        let chunk_size = (self.config.user_count + self.config.concurrent_users - 1)
            / self.config.concurrent_users;

        for chunk_idx in 0..self.config.concurrent_users {
            let start_user = chunk_idx * chunk_size;
            let end_user = (start_user + chunk_size).min(self.config.user_count);

            if start_user >= self.config.user_count {
                break;
            }

            let metrics = self.metrics.clone_ref();
            let config_actions = self.config.actions_per_user;

            let handle = tokio::spawn(async move {
                let mut chunk_results = Vec::new();
                for user_id in start_user..end_user {
                    for _ in 0..config_actions {
                        let action = UserAction::weighted_random(&mut rand::thread_rng());
                        let action_start = Instant::now();

                        let result = execute_action(user_id, action);
                        let duration_ms = action_start.elapsed().as_secs_f64() * 1000.0;

                        metrics.increment_transaction();
                        if result.success {
                            metrics.increment_success();
                        } else {
                            metrics.increment_failure();
                        }

                        chunk_results.push(ActionResult {
                            action,
                            user_id,
                            timestamp: duration_ms,
                            amount: result.amount,
                            success: result.success,
                            duration_ms,
                            error_message: result.error_message,
                        });
                    }
                }
                chunk_results
            });

            handles.push(handle);
        }

        for handle in handles {
            if let Ok(chunk_results) = handle.await {
                self.results.extend(chunk_results);
            }
        }

        let total_duration = start.elapsed().as_secs_f64();

        let (total_tx, successful, failed) = self.metrics.get_totals();

        let mut metrics = SimulationMetrics::new();
        metrics.total_transactions = total_tx;
        metrics.successful_transactions = successful;
        metrics.failed_transactions = failed;
        metrics.total_duration_s = total_duration;
        metrics.state_consistency_verified = self.verify_state_consistency();
        metrics.calculate_stats();

        self.analyze_bottlenecks(&mut metrics);
        metrics
    }

    fn verify_state_consistency(&self) -> bool {
        let calculated_total: i128 = self.state.user_balances.values().sum();
        let debt_total: i128 = self.state.user_debt.values().sum();

        // Verify supply matches sum of balances
        if calculated_total != self.state.total_supply {
            return false;
        }

        // Verify no negative balances (no double withdrawals)
        if self.state.user_balances.values().any(|&b| b < 0) {
            return false;
        }

        // Verify no negative debt (no double borrows repaid incorrectly)
        if self.state.user_debt.values().any(|&d| d < 0) {
            return false;
        }

        // Debt should not exceed total supply (basic sanity check)
        if debt_total > calculated_total * 2 {
            return false;
        }

        true
    }

    fn analyze_bottlenecks(&self, metrics: &mut SimulationMetrics) {
        // Identify operations with highest failure rates
        let mut op_stats: HashMap<&'static str, (u64, u64)> = HashMap::new();

        for result in &self.results {
            let op_name = match result.action {
                UserAction::Deposit => "deposit",
                UserAction::Withdraw => "withdraw",
                UserAction::Borrow => "borrow",
                UserAction::Repay => "repay",
            };

            let (total, failures) = op_stats.entry(op_name).or_insert((0, 0));
            *total += 1;
            if !result.success {
                *failures += 1;
            }
        }

        for (op, (total, failures)) in op_stats {
            if failures > 0 {
                let failure_rate = (failures as f64 / total as f64) * 100.0;
                if failure_rate > 5.0 {
                    metrics.add_bottleneck(format!("{}: {:.2}% failures", op, failure_rate));
                }
            }
        }
    }
}

impl ThreadSafeMetrics {
    fn clone_ref(&self) -> Self {
        ThreadSafeMetrics {
            transactions: Arc::clone(&self.transactions),
            successes: Arc::clone(&self.successes),
            failures: Arc::clone(&self.failures),
        }
    }
}

use std::sync::Arc;

pub struct ActionExecutionResult {
    pub amount: i128,
    pub success: bool,
    pub error_message: Option<String>,
}

fn execute_action(user_id: usize, action: UserAction) -> ActionExecutionResult {
    match action {
        UserAction::Deposit => execute_deposit(user_id),
        UserAction::Withdraw => execute_withdraw(user_id),
        UserAction::Borrow => execute_borrow(user_id),
        UserAction::Repay => execute_repay(user_id),
    }
}

fn execute_deposit(_user_id: usize) -> ActionExecutionResult {
    let amount = rand::random::<i128>().abs() % 1000 + 1;
    ActionExecutionResult {
        amount,
        success: true,
        error_message: None,
    }
}

fn execute_withdraw(_user_id: usize) -> ActionExecutionResult {
    let amount = rand::random::<i128>().abs() % 100 + 1;
    ActionExecutionResult {
        amount,
        success: true,
        error_message: None,
    }
}

fn execute_borrow(_user_id: usize) -> ActionExecutionResult {
    let amount = rand::random::<i128>().abs() % 500 + 1;
    ActionExecutionResult {
        amount,
        success: rand::random::<bool>(), // Some borrows may fail due to collateral checks
        error_message: if !rand::random::<bool>() {
            Some("Insufficient collateral".to_string())
        } else {
            None
        },
    }
}

fn execute_repay(_user_id: usize) -> ActionExecutionResult {
    let amount = rand::random::<i128>().abs() % 300 + 1;
    ActionExecutionResult {
        amount,
        success: true,
        error_message: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_creation() {
        let config = SimulationConfig {
            user_count: 10,
            actions_per_user: 5,
            concurrent_users: 2,
            random_seed: None,
        };
        let simulator = Simulator::new(config);

        assert_eq!(simulator.state.user_balances.len(), 10);
        assert_eq!(simulator.state.total_supply, 10000);
    }

    #[test]
    fn test_state_consistency_valid() {
        let config = SimulationConfig::default();
        let simulator = Simulator::new(config);

        assert!(simulator.verify_state_consistency());
    }

    #[tokio::test]
    async fn test_simulation_execution() {
        let config = SimulationConfig {
            user_count: 5,
            actions_per_user: 10,
            concurrent_users: 2,
            random_seed: None,
        };

        let mut simulator = Simulator::new(config);
        let metrics = simulator.run().await;

        assert!(metrics.total_transactions > 0);
        assert!(metrics.successful_transactions > 0);
        assert!(metrics.total_throughput_tps >= 0.0);
        assert!(metrics.success_rate_pct >= 0.0 && metrics.success_rate_pct <= 100.0);
    }
}
