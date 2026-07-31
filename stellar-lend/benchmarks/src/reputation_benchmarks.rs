//! # Reputation System Gas Benchmarks
//!
//! Measures instruction counts for all public functions in the
//! `stellarlend-reputation` contract, covering:
//! - Initialization
//! - Repayment recording (cold / warm storage)
//! - Default recording
//! - Reputation queries
//! - Tier queries and benefits
//! - Inactivity decay

use crate::framework::{
    fresh_env, get_budget, measure_instructions, BenchmarkResult, BenchmarkSuite, RunConfig,
};
use soroban_sdk::{testutils::Address as _, Address, Env};
use stellarlend_reputation::{
    ReputationConfig, ReputationContract, ReputationContractClient, ReputationTier, TierBenefits,
};

const CONTRACT: &str = "reputation";

pub fn register(suite: &mut BenchmarkSuite) {
    suite.register_group("Reputation System Contract", run_all);
}

fn run_all(config: &RunConfig) -> Vec<BenchmarkResult> {
    vec![
        bench_initialize(config),
        bench_record_repayment_cold(config),
        bench_record_repayment_warm(config),
        bench_record_default(config),
        bench_get_reputation(config),
        bench_get_tier(config),
        bench_get_tier_benefits(config),
        bench_apply_decay(config),
    ]
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn default_config(env: &Env, admin: &Address) -> ReputationConfig {
    ReputationConfig {
        admin: admin.clone(),
        silver_threshold: 250,
        gold_threshold: 500,
        platinum_threshold: 750,
        bronze_benefits: TierBenefits {
            interest_rate_discount_bps: 0,
            borrowing_limit_multiplier_bps: 10_000,
            collateral_reduction_bps: 0,
        },
        silver_benefits: TierBenefits {
            interest_rate_discount_bps: 25,
            borrowing_limit_multiplier_bps: 11_000,
            collateral_reduction_bps: 100,
        },
        gold_benefits: TierBenefits {
            interest_rate_discount_bps: 50,
            borrowing_limit_multiplier_bps: 12_500,
            collateral_reduction_bps: 200,
        },
        platinum_benefits: TierBenefits {
            interest_rate_discount_bps: 100,
            borrowing_limit_multiplier_bps: 15_000,
            collateral_reduction_bps: 300,
        },
        decay_rate: 10,
        decay_interval: 604_800, // 1 week in seconds
    }
}

fn setup_initialized(env: &Env) -> (ReputationContractClient<'static>, Address) {
    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let cfg = default_config(env, &admin);
    client.initialize(&admin, &cfg).unwrap();
    (client, admin)
}

fn setup_with_repayment(env: &Env) -> (ReputationContractClient<'static>, Address, Address) {
    let (client, admin) = setup_initialized(env);
    let borrower = Address::generate(env);
    client
        .record_repayment(&admin, &borrower, &100_000i128, &true)
        .unwrap();
    (client, admin, borrower)
}

// ── Initialize ────────────────────────────────────────────────────────────────

fn bench_initialize(config: &RunConfig) -> BenchmarkResult {
    let op = "reputation::initialize";
    let env = fresh_env();
    let contract_id = env.register(ReputationContract, ());
    let client = ReputationContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let cfg = default_config(&env, &admin);

    let (insns, mem) = measure_instructions(&env, || {
        client.initialize(&admin, &cfg).unwrap();
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Initialize reputation system with admin and tier configuration",
        insns,
        mem,
        0,
        1,
        true,
        get_budget(config, op),
        vec!["admin".into(), "init".into()],
    )
}

// ── Record repayment ──────────────────────────────────────────────────────────

fn bench_record_repayment_cold(config: &RunConfig) -> BenchmarkResult {
    let op = "reputation::record_repayment";
    let env = fresh_env();
    let (client, admin) = setup_initialized(&env);
    let borrower = Address::generate(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client
            .record_repayment(&admin, &borrower, &100_000i128, &true)
            .unwrap();
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Record first repayment — cold persistent storage write",
        insns,
        mem,
        1,
        1,
        true,
        get_budget(config, op),
        vec!["repayment".into(), "cold".into()],
    )
}

fn bench_record_repayment_warm(config: &RunConfig) -> BenchmarkResult {
    let op = "reputation::record_repayment_warm";
    let env = fresh_env();
    let (client, admin, borrower) = setup_with_repayment(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client
            .record_repayment(&admin, &borrower, &50_000i128, &true)
            .unwrap();
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Record subsequent repayment — warm persistent storage update",
        insns,
        mem,
        1,
        1,
        false,
        get_budget(config, "reputation::record_repayment"),
        vec!["repayment".into(), "warm".into()],
    )
}

// ── Record default ────────────────────────────────────────────────────────────

fn bench_record_default(config: &RunConfig) -> BenchmarkResult {
    let op = "reputation::record_default";
    let env = fresh_env();
    let (client, admin, borrower) = setup_with_repayment(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.record_default(&admin, &borrower).unwrap();
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Record borrower default — score penalty + persistent write",
        insns,
        mem,
        1,
        1,
        false,
        get_budget(config, op),
        vec!["default".into()],
    )
}

// ── Queries ───────────────────────────────────────────────────────────────────

fn bench_get_reputation(config: &RunConfig) -> BenchmarkResult {
    let op = "reputation::get_reputation";
    let env = fresh_env();
    let (client, _, borrower) = setup_with_repayment(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.get_reputation(&borrower).unwrap();
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Get reputation score — single persistent storage read",
        insns,
        mem,
        1,
        0,
        false,
        get_budget(config, op),
        vec!["query".into(), "score".into()],
    )
}

fn bench_get_tier(config: &RunConfig) -> BenchmarkResult {
    let op = "reputation::get_tier";
    let env = fresh_env();
    let (client, _, borrower) = setup_with_repayment(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.get_tier(&borrower).unwrap();
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Get tier — persistent read + tier derivation",
        insns,
        mem,
        1,
        0,
        false,
        get_budget(config, op),
        vec!["query".into(), "tier".into()],
    )
}

fn bench_get_tier_benefits(config: &RunConfig) -> BenchmarkResult {
    let op = "reputation::get_tier_benefits";
    let env = fresh_env();
    let (client, _) = setup_initialized(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.get_tier_benefits(&ReputationTier::Gold).unwrap();
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Get tier benefits — instance config read + tier match",
        insns,
        mem,
        1,
        0,
        false,
        get_budget(config, op),
        vec!["query".into(), "benefits".into()],
    )
}

// ── Decay ─────────────────────────────────────────────────────────────────────

fn bench_apply_decay(config: &RunConfig) -> BenchmarkResult {
    let op = "reputation::apply_decay";
    let env = fresh_env();
    let (client, _, borrower) = setup_with_repayment(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.apply_decay(&borrower).unwrap();
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Apply inactivity decay — config read + score update + persistent write",
        insns,
        mem,
        2,
        1,
        false,
        get_budget(config, op),
        vec!["decay".into()],
    )
}
