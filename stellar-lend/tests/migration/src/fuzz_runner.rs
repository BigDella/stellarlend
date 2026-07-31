use crate::gas_profiler::{profile_migration, GasProfile};
use crate::integrity_checker::{verify_state_integrity, IntegrityReport};
use crate::rollback_validator::{verify_rollback, RollbackReport};
use crate::state_generator::StateGenerator;
use crate::storage::StorageContext;
use rand::Rng;
use soroban_sdk::Env;

pub struct FuzzConfig {
    pub iterations: u32,
    pub min_keys: usize,
    pub max_keys: usize,
    pub seed: u64,
    pub schema_old: u32,
    pub schema_new: u32,
}

impl Default for FuzzConfig {
    fn default() -> Self {
        Self {
            iterations: 100,
            min_keys: 0,
            max_keys: 100,
            seed: 42,
            schema_old: 0,
            schema_new: 1,
        }
    }
}

pub struct FuzzReport {
    pub total_iterations: u32,
    pub passed: u32,
    pub failed: u32,
}

pub struct CycleOutput {
    pub integrity: IntegrityReport,
    pub rollback: RollbackReport,
    pub gas_profiles: Vec<GasProfile>,
}

pub fn run_fuzz_cycle(
    ctx: &StorageContext,
    generator: &mut StateGenerator,
    num_keys: usize,
    schema_old: u32,
    schema_new: u32,
) -> CycleOutput {
    use soroban_sdk::Bytes;
    use soroban_sdk::String as SorobanString;

    let mut keys: Vec<SorobanString> = Vec::new();
    let mut values: Vec<Bytes> = Vec::new();
    generator.generate_state(ctx, &mut keys, &mut values, num_keys / 2, 3);

    for (i, key) in keys.iter().enumerate() {
        let val = if i < values.len() {
            values[i].clone()
        } else {
            Bytes::new(&ctx.env)
        };
        ctx.save_with_count(key, &val);
    }

    ctx.set_schema_version(schema_old);
    let pre_version = ctx.schema_version();
    let pre_values: Vec<Bytes> = keys.iter().map(|k| ctx.data_load(k)).collect();

    let gas_forward = profile_migration(&ctx.env, "fuzz_migration_forward", || {
        let _ = ctx.bump_schema_version(schema_new);
    });

    let integrity = verify_state_integrity(ctx, schema_new, &keys, &pre_values);

    let gas_rollback = profile_migration(&ctx.env, "fuzz_migration_rollback", || {
        ctx.set_schema_version(pre_version);
    });

    let rollback = verify_rollback(ctx, pre_version, &keys, &pre_values);

    CycleOutput {
        integrity,
        rollback,
        gas_profiles: vec![gas_forward, gas_rollback],
    }
}

pub fn run_fuzz_suite(config: FuzzConfig) -> (FuzzReport, Vec<GasProfile>) {
    let mut all_gas_profiles = Vec::new();
    let mut passed = 0u32;
    let mut failed = 0u32;

    for iteration in 0..config.iterations {
        let env = Env::default();
        let ctx = StorageContext::new(&env);
        let mut generator = StateGenerator::new(config.seed + iteration as u64);
        let num_keys = generator.rng.gen_range(config.min_keys..=config.max_keys);

        let output = run_fuzz_cycle(&ctx, &mut generator, num_keys, config.schema_old, config.schema_new);

        all_gas_profiles.extend(output.gas_profiles);

        if output.integrity.all_passed() && output.rollback.lossless_round_trip {
            passed += 1;
        } else {
            failed += 1;
        }
    }

    (
        FuzzReport {
            total_iterations: config.iterations,
            passed,
            failed,
        },
        all_gas_profiles,
    )
}
