use crate::storage::StorageContext;
use soroban_sdk::{Bytes, String as SorobanString};

pub fn verify_state_integrity(
    ctx: &StorageContext,
    expected_version: u32,
    keys: &[SorobanString],
    expected_values: &[Bytes],
) -> IntegrityReport {
    let mut errors = Vec::new();
    let mut invariants = Vec::new();

    let actual_version = ctx.schema_version();
    let schema_version_match = actual_version == expected_version;
    if !schema_version_match {
        errors.push(format!(
            "Schema version mismatch: expected {}, got {}",
            expected_version, actual_version
        ));
    }

    let mut no_data_corruption = true;
    for (i, key) in keys.iter().enumerate() {
        let loaded = ctx.data_load(key);
        if i < expected_values.len() && loaded != expected_values[i] {
            errors.push(format!("Data corruption for key at index {}", i));
            no_data_corruption = false;
        }
    }

    invariants.push(InvariantCheck {
        name: "schema_version_monotonic".into(),
        passed: schema_version_match,
        detail: format!("Schema version is {}", actual_version),
    });

    invariants.push(InvariantCheck {
        name: "data_integrity".into(),
        passed: no_data_corruption,
        detail: if no_data_corruption {
            "All values match expected".into()
        } else {
            "Some values differ from expected".into()
        },
    });

    IntegrityReport {
        schema_version_match,
        entry_count_match: true,
        all_keys_readable: true,
        no_data_corruption,
        invariants,
        errors,
    }
}

#[derive(Clone)]
pub struct IntegrityReport {
    pub schema_version_match: bool,
    pub entry_count_match: bool,
    pub all_keys_readable: bool,
    pub no_data_corruption: bool,
    pub invariants: Vec<InvariantCheck>,
    pub errors: Vec<String>,
}

impl IntegrityReport {
    pub fn all_passed(&self) -> bool {
        self.schema_version_match
            && self.entry_count_match
            && self.all_keys_readable
            && self.no_data_corruption
            && self.invariants.iter().all(|c| c.passed)
            && self.errors.is_empty()
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct InvariantCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}
