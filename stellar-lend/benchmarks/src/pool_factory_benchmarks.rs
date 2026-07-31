//! # Pool Factory Gas Benchmarks
//!
//! Measures instruction counts for all public functions in the
//! `stellarlend-pool-factory` contract, covering:
//! - Initialization
//! - Pool creation (cold / warm storage)
//! - Pool queries (count, list, by-index)
//! - Config updates

use crate::framework::{
    fresh_env, get_budget, measure_instructions, BenchmarkResult, BenchmarkSuite, RunConfig,
};
use soroban_sdk::{testutils::Address as _, Address, Env};
use stellarlend_pool_factory::{PoolConfig, PoolFactory, PoolFactoryClient};

const CONTRACT: &str = "pool_factory";

pub fn register(suite: &mut BenchmarkSuite) {
    suite.register_group("Pool Factory Contract", run_all);
}

fn run_all(config: &RunConfig) -> Vec<BenchmarkResult> {
    vec![
        bench_initialize(config),
        bench_create_pool_cold(config),
        bench_create_pool_warm(config),
        bench_get_pool_count(config),
        bench_get_pools(config),
        bench_get_pool_by_index(config),
        bench_update_pool_config(config),
    ]
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_pool_config(env: &Env) -> PoolConfig {
    PoolConfig {
        asset: Address::generate(env),
        oracle: Address::generate(env),
        ltv_bps: 7500,
        liquidation_threshold_bps: 8500,
        interest_model: Address::generate(env),
    }
}

fn setup_initialized(env: &Env) -> (PoolFactoryClient<'static>, Address) {
    let contract_id = env.register(PoolFactory, ());
    let client = PoolFactoryClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin)
}

fn setup_with_pool(env: &Env) -> PoolFactoryClient<'static> {
    let (client, _) = setup_initialized(env);
    let cfg = make_pool_config(env);
    client.create_pool(
        &cfg.asset,
        &cfg.oracle,
        &cfg.ltv_bps,
        &cfg.liquidation_threshold_bps,
        &cfg.interest_model,
    );
    client
}

// ── Initialize ────────────────────────────────────────────────────────────────

fn bench_initialize(config: &RunConfig) -> BenchmarkResult {
    let op = "pool_factory::initialize";
    let env = fresh_env();
    let contract_id = env.register(PoolFactory, ());
    let client = PoolFactoryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.initialize(&admin);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Initialize pool factory with admin address",
        insns,
        mem,
        0,
        2,
        true,
        get_budget(config, op),
        vec!["admin".into(), "init".into()],
    )
}

// ── Create pool ───────────────────────────────────────────────────────────────

fn bench_create_pool_cold(config: &RunConfig) -> BenchmarkResult {
    let op = "pool_factory::create_pool";
    let env = fresh_env();
    let (client, _) = setup_initialized(&env);
    let cfg = make_pool_config(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.create_pool(
            &cfg.asset,
            &cfg.oracle,
            &cfg.ltv_bps,
            &cfg.liquidation_threshold_bps,
            &cfg.interest_model,
        );
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Create pool — first pool (cold: pool list write + count write)",
        insns,
        mem,
        1,
        2,
        true,
        get_budget(config, op),
        vec!["create_pool".into(), "cold".into()],
    )
}

fn bench_create_pool_warm(config: &RunConfig) -> BenchmarkResult {
    let op = "pool_factory::create_pool_warm";
    let env = fresh_env();
    let client = setup_with_pool(&env); // first pool warms pool-list storage

    let cfg2 = make_pool_config(&env);
    let (insns, mem) = measure_instructions(&env, || {
        client.create_pool(
            &cfg2.asset,
            &cfg2.oracle,
            &cfg2.ltv_bps,
            &cfg2.liquidation_threshold_bps,
            &cfg2.interest_model,
        );
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Create pool — subsequent pool (warm: pool list append)",
        insns,
        mem,
        1,
        1,
        false,
        get_budget(config, "pool_factory::create_pool"),
        vec!["create_pool".into(), "warm".into()],
    )
}

// ── Queries ───────────────────────────────────────────────────────────────────

fn bench_get_pool_count(config: &RunConfig) -> BenchmarkResult {
    let op = "pool_factory::get_pool_count";
    let env = fresh_env();
    let client = setup_with_pool(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.get_pool_count();
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Get pool count — single storage read",
        insns,
        mem,
        1,
        0,
        false,
        get_budget(config, op),
        vec!["query".into(), "count".into()],
    )
}

fn bench_get_pools(config: &RunConfig) -> BenchmarkResult {
    let op = "pool_factory::get_pools";
    let env = fresh_env();
    let (client, _) = setup_initialized(&env);

    // Pre-populate 5 pools so the list read is non-trivial
    for _ in 0..5 {
        let cfg = make_pool_config(&env);
        client.create_pool(
            &cfg.asset,
            &cfg.oracle,
            &cfg.ltv_bps,
            &cfg.liquidation_threshold_bps,
            &cfg.interest_model,
        );
    }

    let (insns, mem) = measure_instructions(&env, || {
        client.get_pools();
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Get all pools — full list read (5 pools)",
        insns,
        mem,
        1,
        0,
        false,
        get_budget(config, op),
        vec!["query".into(), "list".into()],
    )
}

fn bench_get_pool_by_index(config: &RunConfig) -> BenchmarkResult {
    let op = "pool_factory::get_pool_by_index";
    let env = fresh_env();
    let client = setup_with_pool(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.get_pool_by_index(&0u32);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Get pool by index — list read + element access",
        insns,
        mem,
        1,
        0,
        false,
        get_budget(config, op),
        vec!["query".into(), "index".into()],
    )
}

// ── Admin mutations ───────────────────────────────────────────────────────────

fn bench_update_pool_config(config: &RunConfig) -> BenchmarkResult {
    let op = "pool_factory::update_pool_config";
    let env = fresh_env();
    let client = setup_with_pool(&env);

    let new_cfg = make_pool_config(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.update_pool_config(&0u32, &new_cfg);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Update pool config — admin auth + list read + write",
        insns,
        mem,
        1,
        1,
        true,
        get_budget(config, op),
        vec!["admin".into(), "update".into()],
    )
}
