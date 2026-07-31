use crate::gas_profiler::{gas_report_to_json, GasReport};
use crate::scenarios::ScenarioResult;

pub struct TestReport {
    pub scenarios: Vec<ScenarioResult>,
    pub fuzz_report: Option<crate::fuzz_runner::FuzzReport>,
    pub overall_gas: GasReport,
}

pub fn print_summary(report: &TestReport) {
    println!();
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║    Migration Test Framework — Results Summary            ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    let mut total = 0;
    let mut passed = 0;

    for scenario in &report.scenarios {
        total += 1;
        let status = if scenario.passed { "✓ PASS" } else { "✗ FAIL" };
        println!("  {}  {}", status, scenario.name);
        if !scenario.integrity.all_passed() {
            println!("        integrity: {} errors", scenario.integrity.errors.len());
        }
        if !scenario.rollback.lossless_round_trip {
            println!("        rollback not lossless");
        }
        if scenario.passed {
            passed += 1;
        }
    }

    if let Some(ref fuzz) = report.fuzz_report {
        println!();
        println!("  Fuzz Suite: {} iterations", fuzz.total_iterations);
        println!("    Passed: {}, Failed: {}", fuzz.passed, fuzz.failed);
        total += 1;
        if fuzz.failed == 0 {
            passed += 1;
        }
    }

    println!();
    println!("  ─────────────────────────────────────────────");
    println!("  Total: {}, Passed: {}, Failed: {}", total, passed, total - passed);
    println!();

    println!("  Gas Profile:");
    for profile in &report.overall_gas.profiles {
        println!(
            "    {:30} {} instructions, {} memory bytes",
            profile.operation, profile.instructions, profile.memory_bytes
        );
    }
    println!(
        "    {:30} {} instructions (avg)",
        "Average:",
        report.overall_gas.average_instructions as u64
    );
    println!();
}

pub fn report_to_json(report: &TestReport) -> String {
    let scenario_entries: Vec<String> = report
        .scenarios
        .iter()
        .map(|s| {
            format!(
                r#"{{"name":"{}","passed":{},"integrity_errors":{},"rollback_lossless":{}}}"#,
                s.name,
                s.passed,
                s.integrity.errors.len(),
                s.rollback.lossless_round_trip
            )
        })
        .collect();

    let scenarios_part = format!(r#""scenarios":[{}]"#, scenario_entries.join(","));

    let fuzz_part = match &report.fuzz_report {
        Some(fuzz) => format!(
            r#","fuzz_suite":{{"total_iterations":{},"passed":{},"failed":{}}}"#,
            fuzz.total_iterations, fuzz.passed, fuzz.failed
        ),
        None => String::new(),
    };

    let gas_part = format!(r#","gas_profile":{}"#, gas_report_to_json(&report.overall_gas));

    format!("{{{}{}{}}}", scenarios_part, fuzz_part, gas_part)
}
