//! # Lazy Pool-State Initialisation (issue #634, re-filed as #600)
//!
//! Creating a pool previously initialised *every* state field up front, paying
//! storage rent for slots that aren't touched until much later (or ever). This
//! module defers those fields: a slot is only written the first time an
//! operation actually needs it, and reads fall back to a well-defined default
//! until then.
//!
//! ## Field classification
//!
//! | field                 | strategy | first needed by      |
//! |-----------------------|----------|----------------------|
//! | admin / debt ceiling  | eager    | pool creation        |
//! | `ReserveBalance`      | lazy     | first reserve accrual|
//! | `AccumulatedFees`     | lazy     | first fee charge     |
//! | `LiquidationCounter`  | lazy     | first liquidation    |
//! | `TotalReserves`       | lazy     | first reserve accrual|
//! | `BorrowIndexSnapshot` | lazy     | first borrow         |
//!
//! Eager fields stay in their existing modules; only the deferrable fields below
//! are routed through the [`LazyField`] check-exists pattern.

use soroban_sdk::{contracterror, contracttype, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum LazyError {
    /// `set` received a negative value for a non-negative field.
    InvalidValue = 1,
}

/// Deferrable pool-state fields. Each is initialised on first use, not at
/// pool-creation time.
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum LazyField {
    /// Protocol reserve balance.
    ReserveBalance = 0,
    /// Accumulated protocol fees.
    AccumulatedFees = 1,
    /// Number of liquidations executed.
    LiquidationCounter = 2,
    /// Aggregate reserves across assets.
    TotalReserves = 3,
    /// Snapshot of the borrow index at pool's first borrow.
    BorrowIndexSnapshot = 4,
}

/// Storage key wrapping a [`LazyField`] so deferred slots are namespaced.
#[contracttype]
#[derive(Clone)]
pub enum LazyKey {
    Field(LazyField),
}

/// The default value a field reads as before it is first initialised.
///
/// Pure (no `Env`) so callers and tests share one source of truth.
pub fn default_for(field: LazyField) -> i128 {
    match field {
        // The borrow index is a ratio anchored at 1.0 (scaled 1e9); everything
        // else is a counter/balance that starts empty.
        LazyField::BorrowIndexSnapshot => 1_000_000_000,
        _ => 0,
    }
}

/// `true` once the field's slot has been written at least once.
pub fn is_initialized(env: &Env, field: LazyField) -> bool {
    env.storage().persistent().has(&LazyKey::Field(field))
}

/// Read a lazy field. If the slot has never been written this returns
/// [`default_for`] **without** allocating storage — pure read, no rent.
pub fn get(env: &Env, field: LazyField) -> i128 {
    env.storage()
        .persistent()
        .get(&LazyKey::Field(field))
        .unwrap_or_else(|| default_for(field))
}

/// Initialise a field to its default on first use, returning `true` if this call
/// performed the initialisation (i.e. the slot was previously empty).
///
/// Idempotent and safe under concurrent first-use: a second caller in the same
/// transaction sees the slot already present and does nothing.
pub fn ensure_initialized(env: &Env, field: LazyField) -> bool {
    if is_initialized(env, field) {
        return false;
    }
    env.storage()
        .persistent()
        .set(&LazyKey::Field(field), &default_for(field));
    true
}

/// Persist a value for a lazy field (initialising the slot if needed).
pub fn set(env: &Env, field: LazyField, value: i128) -> Result<(), LazyError> {
    if value < 0 {
        return Err(LazyError::InvalidValue);
    }
    env.storage()
        .persistent()
        .set(&LazyKey::Field(field), &value);
    Ok(())
}

/// Read-modify-write helper that initialises on first use, then adds `delta`.
/// This is the common "first time a reserve/fee accrues" path — it front-loads
/// the storage cost to the first real accrual instead of pool creation.
pub fn add(env: &Env, field: LazyField, delta: i128) -> Result<i128, LazyError> {
    let current = get(env, field);
    let next = current.checked_add(delta).ok_or(LazyError::InvalidValue)?;
    set(env, field, next)?;
    Ok(next)
}

/// Migration path for pre-existing pools: eagerly materialise every lazy field
/// to its default so historical pools behave identically to lazily-initialised
/// ones. Returns the number of fields that were newly written.
pub fn migrate_initialize_all(env: &Env) -> u32 {
    let fields = [
        LazyField::ReserveBalance,
        LazyField::AccumulatedFees,
        LazyField::LiquidationCounter,
        LazyField::TotalReserves,
        LazyField::BorrowIndexSnapshot,
    ];
    let mut written = 0u32;
    for f in fields.iter() {
        if ensure_initialized(env, *f) {
            written += 1;
        }
    }
    written
}

#[cfg(test)]
mod unit {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn defaults_are_pure() {
        assert_eq!(default_for(LazyField::ReserveBalance), 0);
        assert_eq!(default_for(LazyField::AccumulatedFees), 0);
        assert_eq!(default_for(LazyField::BorrowIndexSnapshot), 1_000_000_000);
    }

    #[test]
    fn read_before_init_returns_default_without_writing() {
        let env = Env::default();
        let id = env.register(crate::LendingContract, ());
        env.as_contract(&id, || {
            assert!(!is_initialized(&env, LazyField::ReserveBalance));
            assert_eq!(get(&env, LazyField::ReserveBalance), 0);
            // Pure read must not have allocated the slot.
            assert!(!is_initialized(&env, LazyField::ReserveBalance));
        });
    }

    #[test]
    fn ensure_initialized_is_idempotent() {
        let env = Env::default();
        let id = env.register(crate::LendingContract, ());
        env.as_contract(&id, || {
            assert!(ensure_initialized(&env, LazyField::AccumulatedFees));
            assert!(is_initialized(&env, LazyField::AccumulatedFees));
            // Second call is a no-op.
            assert!(!ensure_initialized(&env, LazyField::AccumulatedFees));
        });
    }

    #[test]
    fn add_initialises_on_first_use() {
        let env = Env::default();
        let id = env.register(crate::LendingContract, ());
        env.as_contract(&id, || {
            assert!(!is_initialized(&env, LazyField::TotalReserves));
            let total = add(&env, LazyField::TotalReserves, 500).unwrap();
            assert_eq!(total, 500);
            assert!(is_initialized(&env, LazyField::TotalReserves));
            assert_eq!(add(&env, LazyField::TotalReserves, 250).unwrap(), 750);
        });
    }

    #[test]
    fn set_rejects_negative() {
        let env = Env::default();
        let id = env.register(crate::LendingContract, ());
        env.as_contract(&id, || {
            assert_eq!(
                set(&env, LazyField::ReserveBalance, -1),
                Err(LazyError::InvalidValue)
            );
        });
    }

    #[test]
    fn migration_writes_all_then_is_noop() {
        let env = Env::default();
        let id = env.register(crate::LendingContract, ());
        env.as_contract(&id, || {
            assert_eq!(migrate_initialize_all(&env), 5);
            assert_eq!(migrate_initialize_all(&env), 0);
            assert_eq!(get(&env, LazyField::BorrowIndexSnapshot), 1_000_000_000);
        });
    }
}
