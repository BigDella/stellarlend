use soroban_sdk::Env;

#[derive(Clone)]
pub struct GasProfile {
    pub operation: String,
    pub instructions: u64,
    pub memory_bytes: u64,
}

pub struct GasReport {
    pub profiles: Vec<GasProfile>,
    pub total_instructions: u64,
    pub total_memory_bytes: u64,
    pub average_instructions: f64,
    pub max_instructions: u64,
    pub min_instructions: u64,
    pub sample_count: usize,
}

pub fn measure<F>(env: &Env, f: F) -> (u64, u64)
where
    F: FnOnce(),
{
    env.cost_estimate().budget().reset_unlimited();
    f();
    let instructions = env.cost_estimate().budget().cpu_instruction_cost();
    let memory_bytes = env.cost_estimate().budget().memory_bytes_cost();
    (instructions, memory_bytes)
}

pub fn profile_migration<F>(env: &Env, operation: &str, f: F) -> GasProfile
where
    F: FnOnce(),
{
    let (instructions, memory_bytes) = measure(env, f);
    GasProfile {
        operation: operation.to_string(),
        instructions,
        memory_bytes,
    }
}

pub fn generate_gas_report(profiles: Vec<GasProfile>) -> GasReport {
    let total_instructions: u64 = profiles.iter().map(|p| p.instructions).sum();
    let total_memory_bytes: u64 = profiles.iter().map(|p| p.memory_bytes).sum();
    let sample_count = profiles.len();

    GasReport {
        average_instructions: if sample_count > 0 {
            total_instructions as f64 / sample_count as f64
        } else {
            0.0
        },
        max_instructions: profiles.iter().map(|p| p.instructions).max().unwrap_or(0),
        min_instructions: profiles.iter().map(|p| p.instructions).min().unwrap_or(0),
        total_instructions,
        total_memory_bytes,
        profiles,
        sample_count,
    }
}

pub fn gas_report_to_json(report: &GasReport) -> String {
    let entries: Vec<String> = report
        .profiles
        .iter()
        .map(|p| {
            format!(
                r#"{{"operation":"{}","instructions":{},"memory_bytes":{}}}"#,
                p.operation, p.instructions, p.memory_bytes
            )
        })
        .collect();

    format!(
        r#"{{"total_instructions":{},"total_memory_bytes":{},"average_instructions":{:.2},"max_instructions":{},"min_instructions":{},"sample_count":{},"profiles":[{}]}}"#,
        report.total_instructions,
        report.total_memory_bytes,
        report.average_instructions,
        report.max_instructions,
        report.min_instructions,
        report.sample_count,
        entries.join(",")
    )
}
