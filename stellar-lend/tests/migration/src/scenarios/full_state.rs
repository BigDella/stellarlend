use crate::gas_profiler::profile_migration;
use crate::integrity_checker::verify_state_integrity;
use crate::rollback_validator::verify_rollback;
use crate::scenarios::ScenarioResult;
use crate::storage::StorageContext;
use soroban_sdk::{Bytes, String as SorobanString};

pub fn run(ctx: &StorageContext) -> ScenarioResult {
    ctx.bump_schema_version(1).ok();

    let mut keys = Vec::new();
    let mut pre_values = Vec::new();

    for i in 0..100 {
        let key = SorobanString::from_str(&ctx.env, &format!("entry_{}", i));
        let val = Bytes::from_slice(&ctx.env, &[i as u8; 32]);
        ctx.save_with_count(&key, &val);
        keys.push(key);
        pre_values.push(val);
    }

    let pre_version = ctx.schema_version();

    let gas_forward = profile_migration(&ctx.env, "full_state_forward", || {
        let _ = ctx.bump_schema_version(2);
    });

    let integrity = verify_state_integrity(ctx, 2, &keys, &pre_values);

    let gas_rollback = profile_migration(&ctx.env, "full_state_rollback", || {
        ctx.set_schema_version(pre_version);
    });

    let rollback = verify_rollback(ctx, pre_version, &keys, &pre_values);
    let passed = rollback.lossless_round_trip && integrity.all_passed();

    ScenarioResult {
        name: "full_state",
        integrity,
        rollback,
        gas_profiles: vec![gas_forward, gas_rollback],
        passed,
    }
}
