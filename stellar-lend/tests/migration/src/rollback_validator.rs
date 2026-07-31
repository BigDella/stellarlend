use crate::storage::StorageContext;
use soroban_sdk::{Bytes, String as SorobanString};

pub fn verify_rollback(
    ctx: &StorageContext,
    pre_migration_version: u32,
    pre_migration_keys: &[SorobanString],
    pre_migration_values: &[Bytes],
) -> RollbackReport {
    let mut errors = Vec::new();

    let post_rollback_version = ctx.schema_version();
    let version_restored = post_rollback_version == pre_migration_version;
    if !version_restored {
        errors.push(format!(
            "Schema version not restored: expected {}, got {}",
            pre_migration_version, post_rollback_version
        ));
    }

    let mut all_values_match = true;
    for (i, key) in pre_migration_keys.iter().enumerate() {
        let loaded = ctx.data_load(key);
        let expected = &pre_migration_values[i];
        if loaded != *expected {
            errors.push(format!("Value mismatch for key at index {}", i));
            all_values_match = false;
        }
    }

    RollbackReport {
        version_restored,
        state_checksum_match: all_values_match,
        lossless_round_trip: version_restored && all_values_match,
        errors,
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct RollbackReport {
    pub version_restored: bool,
    pub state_checksum_match: bool,
    pub lossless_round_trip: bool,
    pub errors: Vec<String>,
}
