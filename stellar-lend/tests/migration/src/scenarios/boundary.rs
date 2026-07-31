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

    let zero_balance = Bytes::from_slice(&ctx.env, &0i128.to_be_bytes());
    let max_balance = Bytes::from_slice(&ctx.env, &i128::MAX.to_be_bytes());
    let min_balance = Bytes::from_slice(&ctx.env, &i128::MIN.to_be_bytes());
    let max_u32_val = Bytes::from_slice(&ctx.env, &u32::MAX.to_be_bytes());
    let empty_bytes = Bytes::new(&ctx.env);

    let boundary_data: Vec<(&str, Bytes)> = vec![
        ("balance_zero", zero_balance.clone()),
        ("balance_max", max_balance.clone()),
        ("balance_min", min_balance.clone()),
        ("max_u32_field", max_u32_val.clone()),
        ("empty_value", empty_bytes.clone()),
    ];

    for (key_str, val) in &boundary_data {
        let key = SorobanString::from_str(&ctx.env, key_str);
        ctx.save_with_count(&key, val);
        keys.push(key);
        pre_values.push(val.clone());
    }

    let pre_version = ctx.schema_version();

    let gas_forward = profile_migration(&ctx.env, "boundary_forward", || {
        let _ = ctx.bump_schema_version(2);
    });

    let integrity = verify_state_integrity(ctx, 2, &keys, &pre_values);

    let gas_rollback = profile_migration(&ctx.env, "boundary_rollback", || {
        ctx.set_schema_version(pre_version);
    });

    let rollback = verify_rollback(ctx, pre_version, &keys, &pre_values);
    let passed = rollback.lossless_round_trip && integrity.all_passed();

    ScenarioResult {
        name: "boundary",
        integrity,
        rollback,
        gas_profiles: vec![gas_forward, gas_rollback],
        passed,
    }
}
