use serde::{Deserialize, Serialize};
use soroban_sdk::Env;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GasBenchmark {
    pub operation: String,
    pub baseline_gas: u64,
    pub threshold_increase_bps: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GasReport {
    pub operation: String,
    pub actual_gas: u64,
    pub expected_gas: u64,
    pub difference_bps: i32,
    pub passed: bool,
}

pub struct GasMetrics {
    pub benchmarks: Vec<GasBenchmark>,
}

impl GasMetrics {
    pub fn new() -> Self {
        GasMetrics {
            benchmarks: Vec::new(),
        }
    }

    pub fn add_benchmark(&mut self, operation: &str, baseline_gas: u64, threshold_bps: u32) {
        self.benchmarks.push(GasBenchmark {
            operation: operation.to_string(),
            baseline_gas,
            threshold_increase_bps: threshold_bps,
        });
    }

    pub fn check_gas(_env: &Env, operation: &str, actual_gas: u64) -> GasReport {
        let baseline = match operation {
            "deposit" => 100_000,
            "borrow" => 150_000,
            "liquidate" => 200_000,
            "repay" => 120_000,
            _ => 50_000,
        };

        let threshold_bps = 500;
        let max_gas = (baseline as i128 * (10_000 + threshold_bps as i128)) / 10_000;
        let passed = (actual_gas as i128) <= max_gas;

        let difference_bps = if baseline > 0 {
            ((actual_gas as i128 - baseline as i128) * 10_000 / baseline as i128) as i32
        } else {
            0
        };

        GasReport {
            operation: operation.to_string(),
            actual_gas,
            expected_gas: baseline,
            difference_bps,
            passed,
        }
    }
}

impl Default for GasMetrics {
    fn default() -> Self {
        Self::new()
    }
}

pub fn benchmark_operation<F>(env: &Env, operation: &str, f: F) -> GasReport
where
    F: FnOnce(&Env) -> u64,
{
    let gas_used = f(env);
    GasMetrics::check_gas(env, operation, gas_used)
}
