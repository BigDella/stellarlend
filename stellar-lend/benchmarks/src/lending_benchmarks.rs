//! # Lending Contract Gas Benchmarks
//!
//! Measures instruction counts for all public functions in the
//! `stellarlend-lending` contract, covering:
//! - Core operations: deposit, borrow, repay, withdraw, liquidate
//! - Flash loans
//! - Admin operations: initialize, set_oracle, set_pause
//! - Query operations: get_health_factor, get_user_position
//! - Cold vs warm storage access patterns

use crate::framework::{
    fresh_env, get_budget, measure_instructions, BenchmarkResult, BenchmarkSuite, RunConfig,
};
use soroban_sdk::{contract, contractimpl, testutils::Address as _, token, Address, Bytes, Env};
use stellarlend_lending::{
    interest::{advance_index, interest_for, INDEX_SCALE},
    liquidation::{plan_liquidation, PositionSnapshot},
    LendingContract, LendingContractClient, PauseType,
};

const CONTRACT: &str = "lending";

#[contract]
pub struct BenchmarkFlashLoanReceiver;

#[contractimpl]
impl BenchmarkFlashLoanReceiver {
    pub fn on_flash_loan(
        env: Env,
        initiator: Address,
        asset: Address,
        amount: i128,
        fee: i128,
        _params: Bytes,
    ) -> bool {
        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&env.current_contract_address(), &initiator, &(amount + fee));
        true
    }
}

/// Register all lending benchmarks into the suite
pub fn register(suite: &mut BenchmarkSuite) {
    suite.register_group("Lending Contract", run_all);
}

fn run_all(config: &RunConfig) -> Vec<BenchmarkResult> {
    vec![
        bench_initialize(config),
        bench_initialize_deposit_settings(config),
        bench_deposit_cold(config),
        bench_deposit_warm(config),
        bench_deposit_collateral_cold(config),
        bench_deposit_collateral_warm(config),
        bench_borrow_cold(config),
        bench_repay_cold(config),
        bench_repay_warm(config),
        bench_withdraw_cold(config),
        bench_withdraw_warm(config),
        bench_liquidate(config),
        bench_liquidation_plan_optimized(config),
        bench_liquidation_plan_scale_100(config),
        bench_liquidation_flash_loan_plan(config),
        bench_interest_full_recompute(config),
        bench_interest_incremental_update(config),
        bench_interest_same_block_cached(config),
        bench_flash_loan(config),
        bench_get_health_factor(config),
        bench_get_user_position(config),
        bench_get_user_debt(config),
        bench_get_collateral_balance(config),
        bench_set_oracle(config),
        bench_set_pause(config),
        bench_set_flash_loan_fee(config),
        bench_set_liquidation_threshold(config),
        bench_deposit_multiple_assets_storage(config),
    ]
}

// ─── Setup helpers ────────────────────────────────────────────────────────────

fn setup_initialized(env: &Env) -> (LendingContractClient<'static>, Address, Address) {
    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(env, &contract_id);
    let user = Address::generate(env);
    let asset = Address::generate(env);
    client.initialize_deposit_settings(&1_000_000_000, &100);
    (client, user, asset)
}

fn setup_with_deposit(env: &Env) -> (LendingContractClient<'static>, Address, Address) {
    let (client, user, asset) = setup_initialized(env);
    client.deposit(&user, &asset, &100_000);
    (client, user, asset)
}

fn setup_with_borrow(env: &Env) -> (LendingContractClient<'static>, Address, Address) {
    let (client, user, asset) = setup_initialized(env);
    client.initialize_borrow_settings(&1_000_000_000, &100);
    client.deposit_collateral(&user, &asset, &200_000);
    let _ = client.try_borrow(&user, &asset, &50_000, &asset, &200_000);
    (client, user, asset)
}

// ─── Initialize ───────────────────────────────────────────────────────────────

fn setup_admin_initialized(env: &Env) -> (LendingContractClient<'static>, Address) {
    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin, &1_000_000_000, &100);
    (client, admin)
}

fn bench_initialize(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::initialize";
    let env = fresh_env();
    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let (insns, mem) = measure_instructions(&env, || {
        let _ = client.try_initialize(&admin, &1_000_000_000, &100);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Initialize lending contract with admin, debt ceiling, min borrow",
        insns,
        mem,
        0,
        1,
        true,
        get_budget(config, op),
        vec!["admin".into(), "init".into()],
    )
}

fn bench_initialize_deposit_settings(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::initialize_deposit_settings";
    let env = fresh_env();
    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);

    let (insns, mem) = measure_instructions(&env, || {
        client.initialize_deposit_settings(&1_000_000_000, &100);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Initialize deposit settings (cap + min amount)",
        insns,
        mem,
        0,
        1,
        true,
        get_budget(config, op),
        vec!["admin".into(), "settings".into()],
    )
}

// ─── Deposit ──────────────────────────────────────────────────────────────────

fn bench_deposit_cold(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::deposit";
    let env = fresh_env();
    let (client, user, asset) = setup_initialized(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.deposit(&user, &asset, &10_000);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Deposit asset — first deposit (cold storage write)",
        insns,
        mem,
        1,
        2,
        true,
        get_budget(config, op),
        vec!["deposit".into(), "cold".into()],
    )
}

fn bench_deposit_warm(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::deposit_warm";
    // Use a single env: setup does first deposit, measure does second deposit on same contract
    let env = fresh_env();
    let (client, user, asset) = setup_initialized(&env);
    client.deposit(&user, &asset, &10_000); // first deposit — warms storage
                                            // Reset budget, then measure the second deposit (warm path)
    let (insns, mem) = measure_instructions(&env, || {
        client.deposit(&user, &asset, &5_000);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Deposit asset — subsequent deposit (warm storage update)",
        insns,
        mem,
        1,
        1,
        false,
        get_budget(config, "lending::deposit"),
        vec!["deposit".into(), "warm".into()],
    )
}

fn bench_deposit_collateral_cold(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::deposit_collateral";
    let env = fresh_env();
    let (client, user, asset) = setup_initialized(&env);
    client.initialize_borrow_settings(&1_000_000_000, &100);

    let (insns, mem) = measure_instructions(&env, || {
        client.deposit_collateral(&user, &asset, &50_000);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Deposit collateral — first deposit (cold storage write)",
        insns,
        mem,
        1,
        2,
        true,
        get_budget(config, op),
        vec!["collateral".into(), "cold".into()],
    )
}

fn bench_deposit_collateral_warm(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::deposit_collateral_warm";
    let env = fresh_env();
    let (client, user, asset) = setup_initialized(&env);
    client.initialize_borrow_settings(&1_000_000_000, &100);
    client.deposit_collateral(&user, &asset, &50_000); // first deposit — warms storage
    let (insns, mem) = measure_instructions(&env, || {
        client.deposit_collateral(&user, &asset, &10_000); // warm update
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Deposit collateral — subsequent deposit (warm storage update)",
        insns,
        mem,
        1,
        1,
        false,
        get_budget(config, "lending::deposit_collateral"),
        vec!["collateral".into(), "warm".into()],
    )
}

// ─── Borrow ───────────────────────────────────────────────────────────────────

fn bench_borrow_cold(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::borrow";
    let env = fresh_env();
    let (client, user, asset) = setup_initialized(&env);
    client.initialize_borrow_settings(&1_000_000_000, &100);
    client.deposit_collateral(&user, &asset, &200_000);

    let (insns, mem) = measure_instructions(&env, || {
        let _ = client.try_borrow(&user, &asset, &50_000, &asset, &200_000);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Borrow asset — first borrow (cold: collateral check + debt write)",
        insns,
        mem,
        2,
        2,
        true,
        get_budget(config, op),
        vec!["borrow".into(), "cold".into()],
    )
}

// ─── Repay ────────────────────────────────────────────────────────────────────

fn bench_repay_cold(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::repay";
    let env = fresh_env();
    let (client, user, asset) = setup_with_borrow(&env);

    let (insns, mem) = measure_instructions(&env, || {
        let _ = client.try_repay(&user, &asset, &10_000);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Repay debt — partial repayment (cold: debt read + write)",
        insns,
        mem,
        2,
        2,
        true,
        get_budget(config, op),
        vec!["repay".into(), "cold".into()],
    )
}

fn bench_repay_warm(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::repay_warm";
    let env = fresh_env();
    let (client, user, asset) = setup_with_borrow(&env);
    let _ = client.try_repay(&user, &asset, &5_000); // first repay — warms storage
    let (insns, mem) = measure_instructions(&env, || {
        let _ = client.try_repay(&user, &asset, &5_000); // warm: debt position cached
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Repay debt — subsequent repayment (warm storage)",
        insns,
        mem,
        1,
        1,
        false,
        get_budget(config, "lending::repay"),
        vec!["repay".into(), "warm".into()],
    )
}

// ─── Withdraw ─────────────────────────────────────────────────────────────────

fn bench_withdraw_cold(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::withdraw";
    let env = fresh_env();
    let (client, user, asset) = setup_with_deposit(&env);
    client.initialize_withdraw_settings(&100);

    let (insns, mem) = measure_instructions(&env, || {
        let _ = client.try_withdraw(&user, &asset, &5_000);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Withdraw deposited asset — cold storage read + write",
        insns,
        mem,
        2,
        2,
        true,
        get_budget(config, op),
        vec!["withdraw".into(), "cold".into()],
    )
}

fn bench_withdraw_warm(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::withdraw_warm";
    let env = fresh_env();
    let (client, user, asset) = setup_with_deposit(&env);
    client.initialize_withdraw_settings(&100);
    let _ = client.try_withdraw(&user, &asset, &5_000); // first withdraw — warms storage
    let (insns, mem) = measure_instructions(&env, || {
        let _ = client.try_withdraw(&user, &asset, &3_000); // warm
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Withdraw deposited asset — warm storage",
        insns,
        mem,
        1,
        1,
        false,
        get_budget(config, "lending::withdraw"),
        vec!["withdraw".into(), "warm".into()],
    )
}

// ─── Liquidate ────────────────────────────────────────────────────────────────

fn bench_liquidate(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::liquidate";
    let env = fresh_env();
    let (client, user, asset) = setup_with_borrow(&env);
    let liquidator = Address::generate(&env);
    let collateral_asset = asset.clone();

    let (insns, mem) = measure_instructions(&env, || {
        let _ = client.try_liquidate(&liquidator, &user, &asset, &collateral_asset, &10_000);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Liquidate undercollateralized position (health factor check + state update)",
        insns,
        mem,
        3,
        3,
        true,
        get_budget(config, op),
        vec!["liquidate".into(), "cold".into()],
    )
}

fn liquidation_snapshot(position_size: i128) -> PositionSnapshot {
    PositionSnapshot {
        collateral_value: position_size * 80 / 100,
        debt_value: position_size,
        liquidation_threshold_bps: 8_000,
        close_factor_bps: 5_000,
        liquidation_incentive_bps: 1_000,
        oracle_timestamp: 1_000,
    }
}

fn bench_liquidation_plan_optimized(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::liquidate_plan_optimized";
    let env = fresh_env();
    let snapshot = liquidation_snapshot(1_000_000);

    let (insns, mem) = measure_instructions(&env, || {
        let plan = plan_liquidation(&snapshot, 250_000, 1_030, 60, 1_000).unwrap();
        assert_eq!(plan.repay_value, 250_000);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Optimized liquidation planning only: batched snapshot, early checks, gas-profit guard",
        insns,
        mem,
        1,
        0,
        false,
        get_budget(config, op),
        vec!["liquidate".into(), "optimized".into(), "early_exit".into()],
    )
}

fn bench_liquidation_plan_scale_100(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::liquidate_plan_optimized_scale_100";
    let env = fresh_env();
    let positions: std::vec::Vec<PositionSnapshot> = (1..=100)
        .map(|n| liquidation_snapshot(100_000 * n))
        .collect();

    let (insns, mem) = measure_instructions(&env, || {
        let mut total_bonus = 0;
        for snapshot in positions.iter() {
            let plan =
                plan_liquidation(snapshot, snapshot.debt_value / 4, 1_030, 60, 1_000).unwrap();
            total_bonus += plan.bonus_value;
        }
        assert!(total_bonus > 0);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Optimized liquidation planning across 100 positions for scale regression detection",
        insns,
        mem,
        100,
        0,
        false,
        get_budget(config, op),
        vec!["liquidate".into(), "optimized".into(), "scale_100".into()],
    )
}

fn bench_liquidation_flash_loan_plan(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::liquidate_flash_loan_plan";
    let env = fresh_env();
    let snapshot = liquidation_snapshot(1_000_000);

    let (insns, mem) = measure_instructions(&env, || {
        let plan = plan_liquidation(&snapshot, 500_000, 1_030, 60, 1_500).unwrap();
        let flash_loan_principal = plan.repay_value;
        let flash_loan_fee = flash_loan_principal * 50 / 10_000;
        assert!(plan.bonus_value > flash_loan_fee);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Flash-loan-assisted liquidation plan: repay sizing plus profitability after loan fee",
        insns,
        mem,
        1,
        0,
        false,
        get_budget(config, op),
        vec![
            "liquidate".into(),
            "flash_loan".into(),
            "profit_guard".into(),
        ],
    )
}

fn bench_interest_full_recompute(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::interest_full_recompute";
    let env = fresh_env();
    let principal = 1_000_000;
    let rate_bps = 650;
    let one_day = 86_400;

    let (insns, mem) = measure_instructions(&env, || {
        let mut total = 0;
        for day in 1..=30 {
            let index = advance_index(INDEX_SCALE, rate_bps, one_day * day).unwrap();
            total += interest_for(principal, INDEX_SCALE, index).unwrap();
        }
        assert!(total > 0);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Baseline full interest recompute over 30 elapsed windows",
        insns,
        mem,
        30,
        0,
        false,
        get_budget(config, op),
        vec![
            "interest".into(),
            "baseline".into(),
            "full_recompute".into(),
        ],
    )
}

fn bench_interest_incremental_update(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::interest_incremental_update";
    let env = fresh_env();
    let rate_bps = 650;
    let one_day = 86_400;

    let (insns, mem) = measure_instructions(&env, || {
        let mut index = INDEX_SCALE;
        for _ in 0..30 {
            index = advance_index(index, rate_bps, one_day).unwrap();
        }
        assert!(index > INDEX_SCALE);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Incremental interest cache update: advance cumulative index by daily deltas",
        insns,
        mem,
        1,
        1,
        false,
        get_budget(config, op),
        vec![
            "interest".into(),
            "incremental".into(),
            "cached_index".into(),
        ],
    )
}

fn bench_interest_same_block_cached(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::interest_same_block_cached";
    let env = fresh_env();

    let (insns, mem) = measure_instructions(&env, || {
        let index = advance_index(INDEX_SCALE, 650, 0).unwrap();
        assert_eq!(index, INDEX_SCALE);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Same-block interest cache no-op: batched operations charge interest once",
        insns,
        mem,
        1,
        0,
        false,
        get_budget(config, op),
        vec!["interest".into(), "same_block".into(), "no_write".into()],
    )
}

// ─── Flash Loan ───────────────────────────────────────────────────────────────

fn bench_flash_loan(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::flash_loan";
    let env = fresh_env();
    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &asset);
    let receiver = env.register(BenchmarkFlashLoanReceiver, ());

    client.initialize(&admin, &1_000_000_000, &100);
    client.set_flash_loan_fee_bps(&50);
    token_admin.mint(&contract_id, &100_000);
    token_admin.mint(&receiver, &1_000);
    let params = Bytes::new(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.flash_loan(&receiver, &asset, &1_000, &params);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Flash loan — borrow + fee calculation + repayment check",
        insns,
        mem,
        2,
        2,
        true,
        get_budget(config, op),
        vec!["flash_loan".into(), "cold".into()],
    )
}

// ─── Query operations ─────────────────────────────────────────────────────────

fn bench_get_health_factor(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::get_health_factor";
    let env = fresh_env();
    let (client, user, _) = setup_with_borrow(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.get_health_factor(&user);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Get health factor — reads collateral + debt positions",
        insns,
        mem,
        2,
        0,
        false,
        get_budget(config, op),
        vec!["query".into(), "health_factor".into()],
    )
}

fn bench_get_user_position(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::get_user_position";
    let env = fresh_env();
    let (client, user, _) = setup_with_borrow(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.get_user_position(&user);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Get full user position summary — multi-storage read",
        insns,
        mem,
        3,
        0,
        false,
        get_budget(config, op),
        vec!["query".into(), "position".into()],
    )
}

fn bench_get_user_debt(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::get_user_debt";
    let env = fresh_env();
    let (client, user, _) = setup_with_borrow(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.get_user_debt(&user);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Get user debt position",
        insns,
        mem,
        1,
        0,
        false,
        get_budget(config, op),
        vec!["query".into(), "debt".into()],
    )
}

fn bench_get_collateral_balance(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::get_collateral_balance";
    let env = fresh_env();
    let (client, user, _) = setup_with_deposit(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.get_collateral_balance(&user);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Get collateral balance — single storage read",
        insns,
        mem,
        1,
        0,
        false,
        get_budget(config, op),
        vec!["query".into(), "collateral".into()],
    )
}

// ─── Admin operations ─────────────────────────────────────────────────────────

fn bench_set_oracle(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::set_oracle";
    let env = fresh_env();
    let (client, admin) = setup_admin_initialized(&env);
    let oracle = Address::generate(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.set_oracle(&admin, &oracle);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Set oracle address — admin auth + storage write",
        insns,
        mem,
        1,
        1,
        true,
        get_budget(config, op),
        vec!["admin".into(), "oracle".into()],
    )
}

fn bench_set_pause(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::set_pause";
    let env = fresh_env();
    let (client, admin) = setup_admin_initialized(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.set_pause(&admin, &PauseType::Deposit, &true);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Set protocol pause — single storage write",
        insns,
        mem,
        0,
        1,
        true,
        get_budget(config, op),
        vec!["admin".into(), "pause".into()],
    )
}

fn bench_set_flash_loan_fee(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::set_flash_loan_fee_bps";
    let env = fresh_env();
    let (client, _) = setup_admin_initialized(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.set_flash_loan_fee_bps(&100);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Set flash loan fee in basis points",
        insns,
        mem,
        0,
        1,
        true,
        get_budget(config, op),
        vec!["admin".into(), "flash_loan".into()],
    )
}

fn bench_set_liquidation_threshold(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::set_liquidation_threshold_bps";
    let env = fresh_env();
    let (client, admin) = setup_admin_initialized(&env);

    let (insns, mem) = measure_instructions(&env, || {
        client.set_liquidation_threshold_bps(&admin, &8000);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Set liquidation threshold in basis points",
        insns,
        mem,
        0,
        1,
        true,
        get_budget(config, op),
        vec!["admin".into(), "liquidation".into()],
    )
}

// ─── Storage pattern benchmarks ───────────────────────────────────────────────

/// Benchmark storage cost growth with multiple assets deposited
fn bench_deposit_multiple_assets_storage(config: &RunConfig) -> BenchmarkResult {
    let op = "lending::deposit_multi_asset_storage";
    let env = fresh_env();
    let (client, user, _) = setup_initialized(&env);

    // Pre-populate with 5 different assets
    for _ in 0..5 {
        let asset = Address::generate(&env);
        client.deposit(&user, &asset, &10_000);
    }

    // Measure the 6th deposit — storage is now populated (warm-ish)
    let new_asset = Address::generate(&env);
    let (insns, mem) = measure_instructions(&env, || {
        client.deposit(&user, &new_asset, &10_000);
    });

    BenchmarkResult::new(
        op,
        CONTRACT,
        "Deposit with 5 existing assets — storage write pattern with populated state",
        insns,
        mem,
        1,
        2,
        false,
        get_budget(config, "lending::deposit"),
        vec![
            "deposit".into(),
            "storage_pattern".into(),
            "multi_asset".into(),
        ],
    )
}
