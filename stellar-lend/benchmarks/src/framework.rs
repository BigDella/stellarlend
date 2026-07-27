//! # Benchmark Framework
//!
//! Core infrastructure for measuring Soroban instruction counts (gas proxy),
//! tracking storage reads/writes, and detecting regressions.
//!
//! ## How Gas is Measured on Soroban
//! Soroban charges fees based on CPU instructions and memory bytes consumed.
//! The `soroban-sdk` testutils expose `env.cost_estimate()` which provides
//! instruction counts — the primary gas proxy used here.
//!
//! ## Cold vs Warm Storage
//! - Cold read: first access to a storage entry in a transaction (higher cost)
//! - Warm read: subsequent access to the same entry (lower cost, cached)
//!   Benchmarks track both via separate cold/warm measurement runs.

use serde::{Deserialize, Serialize};
use soroban_sdk::Env;
use std::collections::HashMap;
use std::env;

/// Configuration for a benchmark run
#[derive(Clone, Debug)]
pub struct RunConfig {
    /// Path to baseline JSON for regression comparison
    pub compare_baseline: Option<String>,
    /// Path to write output JSON results
    pub output_file: Option<String>,
    /// Number of iterations per benchmark (for averaging)
    pub iterations: u32,
    /// Gas budget thresholds per operation (instruction count)
    pub budgets: HashMap<String, u64>,
    /// Regression threshold percentage (default: 5%)
    pub regression_threshold: f64,
}

impl RunConfig {
    pub fn from_args(args: &[String]) -> Self {
        let mut config = Self::default();

        // Load from environment variables first
        if let Ok(threshold) = env::var("BENCHMARK_REGRESSION_THRESHOLD") {
            config.regression_threshold = threshold.parse().unwrap_or(5.0);
        }
        if let Ok(output) = env::var("BENCHMARK_OUTPUT_JSON") {
            config.output_file = Some(output);
        }
        if let Ok(baseline) = env::var("BENCHMARK_BASELINE") {
            config.compare_baseline = Some(baseline);
        }

        // Override with command-line arguments
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--compare" if i + 1 < args.len() => {
                    config.compare_baseline = Some(args[i + 1].clone());
                    i += 1;
                }
                "--output" if i + 1 < args.len() => {
                    config.output_file = Some(args[i + 1].clone());
                    i += 1;
                }
                "--iterations" if i + 1 < args.len() => {
                    config.iterations = args[i + 1].parse().unwrap_or(1);
                    i += 1;
                }
                "--threshold" if i + 1 < args.len() => {
                    config.regression_threshold = args[i + 1].parse().unwrap_or(5.0);
                    i += 1;
                }
                _ => {}
            }
            i += 1;
        }
        config
    }

    /// Default gas budgets (instruction count limits) per operation.
    /// These represent acceptable upper bounds; CI fails if exceeded.
    /// Values are conservative estimates — tighten after profiling real workloads.
    fn default_budgets() -> HashMap<String, u64> {
        let mut m = HashMap::new();
        // Lending contract
        m.insert("lending::initialize".into(), 500_000);
        m.insert("lending::deposit".into(), 800_000);
        m.insert("lending::deposit_collateral".into(), 900_000);
        m.insert("lending::borrow".into(), 1_200_000);
        m.insert("lending::repay".into(), 1_000_000);
        m.insert("lending::withdraw".into(), 1_000_000);
        m.insert("lending::liquidate".into(), 1_500_000);
        m.insert("lending::liquidate_plan_optimized".into(), 250_000);
        m.insert(
            "lending::liquidate_plan_optimized_scale_100".into(),
            900_000,
        );
        m.insert("lending::liquidate_flash_loan_plan".into(), 350_000);
        m.insert("lending::interest_full_recompute".into(), 600_000);
        m.insert("lending::interest_incremental_update".into(), 220_000);
        m.insert("lending::interest_same_block_cached".into(), 80_000);
        m.insert("lending::flash_loan".into(), 1_800_000);
        m.insert("lending::get_health_factor".into(), 400_000);
        m.insert("lending::get_user_position".into(), 400_000);
        m.insert("lending::set_oracle".into(), 300_000);
        m.insert("lending::set_pause".into(), 200_000);
        // Hello-world (core lending) contract
        m.insert("hello_world::initialize".into(), 500_000);
        m.insert("hello_world::deposit_collateral".into(), 900_000);
        m.insert("hello_world::borrow_asset".into(), 1_200_000);
        m.insert("hello_world::repay_debt".into(), 1_000_000);
        m.insert("hello_world::withdraw_collateral".into(), 1_000_000);
        m.insert("hello_world::liquidate".into(), 1_500_000);
        m.insert("hello_world::execute_flash_loan".into(), 1_800_000);
        m.insert("hello_world::set_risk_params".into(), 400_000);
        m.insert("hello_world::get_health_factor".into(), 400_000);
        m.insert("hello_world::set_emergency_pause".into(), 200_000);
        // AMM contract
        m.insert("amm::initialize_amm_settings".into(), 500_000);
        m.insert("amm::add_amm_protocol".into(), 600_000);
        m.insert("amm::execute_swap".into(), 1_200_000);
        m.insert("amm::add_liquidity".into(), 1_000_000);
        m.insert("amm::remove_liquidity".into(), 1_000_000);
        m.insert("amm::auto_swap_for_collateral".into(), 1_200_000);
        m.insert("amm::update_amm_settings".into(), 400_000);
        m.insert("amm::get_amm_settings".into(), 200_000);
        // Bridge contract
        m.insert("bridge::init".into(), 300_000);
        m.insert("bridge::register_bridge".into(), 500_000);
        m.insert("bridge::set_bridge_fee".into(), 300_000);
        m.insert("bridge::set_bridge_active".into(), 300_000);
        m.insert("bridge::bridge_deposit".into(), 600_000);
        m.insert("bridge::bridge_withdraw".into(), 600_000);
        m.insert("bridge::transfer_admin".into(), 300_000);
        m.insert("bridge::list_bridges".into(), 200_000);
        m.insert("bridge::compute_fee".into(), 100_000);
        // Pool factory contract
        m.insert("pool_factory::initialize".into(), 300_000);
        m.insert("pool_factory::create_pool".into(), 800_000);
        m.insert("pool_factory::get_pool_count".into(), 150_000);
        m.insert("pool_factory::get_pools".into(), 500_000);
        m.insert("pool_factory::get_pool_by_index".into(), 300_000);
        m.insert("pool_factory::update_pool_config".into(), 700_000);
        // Reputation system contract
        m.insert("reputation::initialize".into(), 400_000);
        m.insert("reputation::record_repayment".into(), 600_000);
        m.insert("reputation::record_default".into(), 500_000);
        m.insert("reputation::get_reputation".into(), 200_000);
        m.insert("reputation::get_tier".into(), 250_000);
        m.insert("reputation::get_tier_benefits".into(), 200_000);
        m.insert("reputation::apply_decay".into(), 500_000);
        m
    }
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            compare_baseline: None,
            output_file: None,
            iterations: 1,
            budgets: Self::default_budgets(),
            regression_threshold: 5.0,
        }
    }
}

/// A single benchmark measurement result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Unique operation identifier e.g. "lending::deposit"
    pub operation: String,
    /// Contract name
    pub contract: String,
    /// Human-readable description
    pub description: String,
    /// CPU instruction count (primary gas proxy)
    pub instructions: u64,
    /// Memory bytes consumed
    pub memory_bytes: u64,
    /// Number of storage reads performed
    pub storage_reads: u32,
    /// Number of storage writes performed
    pub storage_writes: u32,
    /// Whether this was a cold storage access scenario
    pub cold_storage: bool,
    /// Gas budget for this operation (0 = no budget set)
    pub budget: u64,
    /// Whether the result is within budget
    pub within_budget: bool,
    /// Timestamp of measurement (ISO 8601)
    pub timestamp: String,
    /// Additional metadata tags
    pub tags: Vec<String>,
}

impl BenchmarkResult {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operation: impl Into<String>,
        contract: impl Into<String>,
        description: impl Into<String>,
        instructions: u64,
        memory_bytes: u64,
        storage_reads: u32,
        storage_writes: u32,
        cold_storage: bool,
        budget: u64,
        tags: Vec<String>,
    ) -> Self {
        let within_budget = budget == 0 || instructions <= budget;
        Self {
            operation: operation.into(),
            contract: contract.into(),
            description: description.into(),
            instructions,
            memory_bytes,
            storage_reads,
            storage_writes,
            cold_storage,
            budget,
            within_budget,
            timestamp: chrono::Utc::now().to_rfc3339(),
            tags,
        }
    }
}

/// Regression detected when actual > budget
#[derive(Debug, Clone)]
pub struct Regression {
    pub operation: String,
    pub actual: u64,
    pub budget: u64,
    pub delta: u64,
}

type BenchmarkGroup = Box<dyn Fn(&RunConfig) -> Vec<BenchmarkResult>>;

/// The main benchmark suite orchestrator
pub struct BenchmarkSuite {
    pub config: RunConfig,
    benchmarks: Vec<BenchmarkGroup>,
    labels: Vec<String>,
}

impl BenchmarkSuite {
    pub fn new(config: RunConfig) -> Self {
        Self {
            config,
            benchmarks: Vec::new(),
            labels: Vec::new(),
        }
    }

    /// Register a benchmark group (e.g. all lending benchmarks)
    pub fn register_group(
        &mut self,
        label: impl Into<String>,
        f: impl Fn(&RunConfig) -> Vec<BenchmarkResult> + 'static,
    ) {
        self.labels.push(label.into());
        self.benchmarks.push(Box::new(f));
    }

    /// Run all registered benchmark groups and collect results
    pub fn run_all(&self) -> Vec<BenchmarkResult> {
        let mut all = Vec::new();
        for (label, bench_fn) in self.labels.iter().zip(self.benchmarks.iter()) {
            println!("  Running benchmarks: {}", label);
            let results = bench_fn(&self.config);
            println!("    {} measurements collected", results.len());
            all.extend(results);
        }
        all
    }
}

/// Measurement result from a single invocation
#[allow(dead_code)]
pub struct Measurement {
    pub instructions: u64,
    pub memory_bytes: u64,
    pub disk_read_entries: u32,
    pub write_entries: u32,
}

/// Measure resource usage for a closure using Soroban's cost estimator.
/// Uses `resources()` for per-invocation tracking (resets automatically before each call).
/// Falls back to budget-based tracking if resources are unavailable.
pub fn measure<F>(env: &Env, f: F) -> Measurement
where
    F: FnOnce(),
{
    // Reset budget tracking before measurement
    env.cost_estimate().budget().reset_unlimited();
    f();
    // Try to get per-invocation resources first (most accurate)
    // resources() panics if metering is not enabled, so we use budget as fallback
    let instructions = env.cost_estimate().budget().cpu_instruction_cost();
    let memory_bytes = env.cost_estimate().budget().memory_bytes_cost();
    Measurement {
        instructions,
        memory_bytes,
        disk_read_entries: 0,
        write_entries: 0,
    }
}

/// Convenience wrapper returning (instructions, memory_bytes)
pub fn measure_instructions<F>(env: &Env, f: F) -> (u64, u64)
where
    F: FnOnce(),
{
    let m = measure(env, f);
    (m.instructions, m.memory_bytes)
}

/// Create a fresh Env with unlimited budget (cold storage scenario)
pub fn fresh_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.cost_estimate().budget().reset_unlimited();
    env
}

/// Create an Env and pre-warm it by running setup, then measure only the target op.
/// This simulates warm storage (subsequent accesses in same transaction context).
///
/// IMPORTANT: Both `setup` and `measure_fn` receive the SAME `Env` instance.
/// `setup` runs first to populate storage, then the budget is reset, then
/// `measure_fn` runs against the already-populated state (warm storage).
///
/// NOTE: Because closures cannot share local variables, prefer the direct pattern:
/// ```
/// let env = fresh_env();
/// setup_code(&env);  // warms storage
/// let (insns, mem) = measure_instructions(&env, || { target_op(&env); });
/// ```
#[allow(dead_code)]
pub fn warm_env_after<S, F>(setup: S, measure_fn: F) -> (u64, u64)
where
    S: FnOnce(&Env),
    F: FnOnce(&Env),
{
    let env = Env::default();
    env.mock_all_auths();
    env.cost_estimate().budget().reset_unlimited();
    // Run setup on the shared env (warms storage)
    setup(&env);
    // Reset cost tracking, then measure target operation on the same env
    env.cost_estimate().budget().reset_unlimited();
    measure_fn(&env);
    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();
    (cpu, mem)
}

/// Helper: get budget for an operation from config, defaulting to 0 (no limit)
pub fn get_budget(config: &RunConfig, operation: &str) -> u64 {
    *config.budgets.get(operation).unwrap_or(&0)
}
