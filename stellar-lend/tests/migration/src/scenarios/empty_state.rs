use crate::gas_profiler::profile_migration;
use crate::integrity_checker::verify_state_integrity;
use crate::rollback_validator::verify_rollback;
use crate::scenarios::ScenarioResult;
use crate::storage::StorageContext;
use soroban_sdk::{Bytes, String as SorobanString};

pub fn run(ctx: &StorageContext) -> ScenarioResult {
    ctx.bump_schema_version(1).ok();
    let pre_version = 1u32;
    let keys: Vec<SorobanString> = vec![];
    let pre_values: Vec<Bytes> = vec![];

    let gas_forward = profile_migration(&ctx.env, "empty_state_forward", || {
        let _ = ctx.bump_schema_version(2);
    });

    let integrity = verify_state_integrity(ctx, 2, &keys, &pre_values);

    let gas_rollback = profile_migration(&ctx.env, "empty_state_rollback", || {
        ctx.set_schema_version(pre_version);
    });

    let rollback = verify_rollback(ctx, pre_version, &keys, &pre_values);
    let passed = rollback.lossless_round_trip && integrity.all_passed();

    ScenarioResult {
        name: "empty_state",
        integrity,
        rollback,
        gas_profiles: vec![gas_forward, gas_rollback],
        passed,
    }
}
