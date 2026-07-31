pub mod storage;

mod fuzz_runner;
mod gas_profiler;
mod integrity_checker;
mod reporter;
mod rollback_validator;
mod scenarios;
mod state_generator;

use fuzz_runner::{run_fuzz_suite, FuzzConfig};
use gas_profiler::generate_gas_report;
use reporter::{print_summary, report_to_json, TestReport};
use soroban_sdk::Env;
use std::env;
use storage::StorageContext;

fn run_scenario_suite(ctx: &StorageContext) -> (Vec<scenarios::ScenarioResult>, Vec<gas_profiler::GasProfile>) {
    let mut results = Vec::new();
    let mut all_gas = Vec::new();

    let empty = scenarios::empty_state::run(ctx);
    all_gas.extend(empty.gas_profiles.clone());
    results.push(empty);

    let full = scenarios::full_state::run(ctx);
    all_gas.extend(full.gas_profiles.clone());
    results.push(full);

    let boundary = scenarios::boundary::run(ctx);
    all_gas.extend(boundary.gas_profiles.clone());
    results.push(boundary);

    (results, all_gas)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let fuzz_iterations: u32 = args
        .iter()
        .position(|a| a == "--fuzz")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let output_json = args.iter().any(|a| a == "--json");
    let skip_fuzz = args.iter().any(|a| a == "--no-fuzz");

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  StellarLend Migration Test Runner v0.1.0               ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    let env = Env::default();
    let ctx = StorageContext::new(&env);

    println!("Running scenario suite...");
    let (scenario_results, scenario_gas) = run_scenario_suite(&ctx);

    let (fuzz_report, fuzz_gas) = if !skip_fuzz {
        println!("Running fuzz suite ({} iterations)...", fuzz_iterations);
        let config = FuzzConfig {
            iterations: fuzz_iterations,
            ..Default::default()
        };
        let (report, gas) = run_fuzz_suite(config);
        (Some(report), gas)
    } else {
        (None, vec![])
    };

    let mut all_gas = scenario_gas;
    all_gas.extend(fuzz_gas);
    let overall_gas = generate_gas_report(all_gas);

    let report = TestReport {
        scenarios: scenario_results,
        fuzz_report,
        overall_gas,
    };

    if output_json {
        println!("{}", report_to_json(&report));
    } else {
        print_summary(&report);
    }

    let failed = report
        .scenarios
        .iter()
        .filter(|s| !s.passed)
        .count()
        + if report
            .fuzz_report
            .as_ref()
            .map(|f| f.failed > 0)
            .unwrap_or(false)
        {
            1
        } else {
            0
        };

    if failed > 0 {
        std::process::exit(1);
    }
}
