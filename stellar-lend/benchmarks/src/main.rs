//! # StellarLend Gas Benchmark Runner
//!
//! Entry point for the comprehensive gas benchmark suite.
//! Runs all contract benchmarks, compares against baselines,
//! and outputs results in JSON format for CI integration.
//!
//! ## Usage
//! ```
//! cargo run --bin run_benchmarks
//! cargo run --bin run_benchmarks -- --output results.json
//! cargo run --bin run_benchmarks -- --compare baseline.json
//! ```

mod amm_benchmarks;
mod bridge_benchmarks;
mod framework;
mod hello_world_benchmarks;
mod lending_benchmarks;
mod pool_factory_benchmarks;
mod report;
mod reputation_benchmarks;

use framework::{BenchmarkSuite, RunConfig};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = RunConfig::from_args(&args);

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║       StellarLend Gas Benchmark Suite v0.1.0             ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    let mut suite = BenchmarkSuite::new(config.clone());

    // Register all contract benchmark modules
    lending_benchmarks::register(&mut suite);
    hello_world_benchmarks::register(&mut suite);
    amm_benchmarks::register(&mut suite);
    bridge_benchmarks::register(&mut suite);
    pool_factory_benchmarks::register(&mut suite);
    reputation_benchmarks::register(&mut suite);

    // Run all benchmarks
    let results = suite.run_all();

    // Print summary table
    report::print_summary(&results);

    // Compare against baseline if provided
    if let Some(ref baseline_path) = config.compare_baseline {
        let regressions =
            report::compare_baseline(&results, baseline_path, config.regression_threshold);
        if !regressions.is_empty() {
            eprintln!("\n[REGRESSION DETECTED] The following operations exceeded gas budgets:");
            for r in &regressions {
                eprintln!(
                    "  ✗ {} — used {} instructions (budget: {}, delta: +{})",
                    r.operation, r.actual, r.budget, r.delta
                );
            }
            std::process::exit(1);
        } else {
            println!("\n[OK] All operations within gas budgets.");
        }
    }

    // Write output JSON
    let output_path = config
        .output_file
        .as_deref()
        .unwrap_or("benchmark-results.json");
    report::write_json(&results, output_path);
    println!("\nResults written to: {}", output_path);

    // Write Markdown report
    let markdown_path = output_path.replace(".json", ".md");
    report::write_markdown(&results, &markdown_path);
    println!("Markdown report written to: {}", markdown_path);

    // Append to historical trend storage
    let history_path = "benchmarks/history.json";
    report::append_to_history(&results, history_path);
    println!("Historical data appended to: {}", history_path);
}
