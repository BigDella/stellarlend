//! Generic helpers for the "get a value out of storage, falling back to a
//! default" pattern that's currently hand-rolled in nearly every contract
//! crate (`env.storage().instance().get(&Key).unwrap_or(default)`,
//! repeated with minor variations across `hello-world`, `liquidation-strategy`,
//! `parameter-store`, and others). Issue #816 asks to consolidate duplicate
//! storage patterns across contract crates — this is that consolidation for
//! the instance/persistent get-or-default and get-or-init-with pattern,
//! kept crate-agnostic (generic over key/value types) so any contract crate
//! can depend on it without adopting a whole new storage architecture.
//!
//! This intentionally does not replace [`shared-storage`](../../shared-storage),
//! which defines a shared *schema* (`StorageKey`, `UserPosition`, ...) for
//! contracts that opt into a common data model. This module instead
//! consolidates the storage *access pattern* itself, independent of what
//! keys or value types a given contract uses.

use core::fmt::Debug;
use soroban_sdk::{Env, IntoVal, TryFromVal, Val};

/// Reads `key` from instance storage, returning `default` if absent.
/// Equivalent to `env.storage().instance().get(key).unwrap_or(default)`,
/// spelled once instead of at every call site.
pub fn get_instance_or<K, V>(env: &Env, key: &K, default: V) -> V
where
    K: IntoVal<Env, Val>,
    V: TryFromVal<Env, Val>,
    V::Error: Debug,
{
    env.storage().instance().get(key).unwrap_or(default)
}

/// Reads `key` from persistent storage, returning `default` if absent.
pub fn get_persistent_or<K, V>(env: &Env, key: &K, default: V) -> V
where
    K: IntoVal<Env, Val>,
    V: TryFromVal<Env, Val>,
    V::Error: Debug,
{
    env.storage().persistent().get(key).unwrap_or(default)
}

/// Reads `key` from instance storage, lazily computing (and *not* persisting)
/// a default via `make_default` if absent. Useful when the default itself
/// needs the `Env` to construct (e.g. `Vec::new(&env)`, `Map::new(&env)`).
pub fn get_instance_or_else<K, V>(env: &Env, key: &K, make_default: impl FnOnce() -> V) -> V
where
    K: IntoVal<Env, Val>,
    V: TryFromVal<Env, Val>,
    V::Error: Debug,
{
    env.storage().instance().get(key).unwrap_or_else(make_default)
}

/// Reads `key` from persistent storage, lazily computing a default if absent.
pub fn get_persistent_or_else<K, V>(env: &Env, key: &K, make_default: impl FnOnce() -> V) -> V
where
    K: IntoVal<Env, Val>,
    V: TryFromVal<Env, Val>,
    V::Error: Debug,
{
    env.storage().persistent().get(key).unwrap_or_else(make_default)
}

/// Writes `value` to instance storage under `key`. Thin wrapper kept for
/// symmetry with the getters above, so call sites can use one module for
/// the whole read/write pattern.
pub fn set_instance<K, V>(env: &Env, key: &K, value: &V)
where
    K: IntoVal<Env, Val>,
    V: IntoVal<Env, Val>,
{
    env.storage().instance().set(key, value);
}

/// Writes `value` to persistent storage under `key`.
pub fn set_persistent<K, V>(env: &Env, key: &K, value: &V)
where
    K: IntoVal<Env, Val>,
    V: IntoVal<Env, Val>,
{
    env.storage().persistent().set(key, value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

    #[contracttype]
    #[derive(Clone)]
    enum TestKey {
        Counter,
    }

    #[contract]
    struct StorageTestContract;

    #[contractimpl]
    impl StorageTestContract {
        pub fn ping() {}
    }

    fn setup() -> (Env, Address) {
        let env = Env::default();
        let contract = env.register(StorageTestContract, ());
        (env, contract)
    }

    #[test]
    fn get_instance_or_returns_default_when_absent() {
        let (env, contract) = setup();
        env.as_contract(&contract, || {
            let value: i128 = get_instance_or(&env, &TestKey::Counter, 42);
            assert_eq!(value, 42);
        });
    }

    #[test]
    fn set_then_get_instance_round_trips() {
        let (env, contract) = setup();
        env.as_contract(&contract, || {
            set_instance(&env, &TestKey::Counter, &7i128);
            let value: i128 = get_instance_or(&env, &TestKey::Counter, 0);
            assert_eq!(value, 7);
        });
    }

    #[test]
    fn get_persistent_or_else_only_invokes_default_when_absent() {
        let (env, contract) = setup();
        env.as_contract(&contract, || {
            set_persistent(&env, &TestKey::Counter, &99i128);
            let mut called = false;
            let value: i128 = get_persistent_or_else(&env, &TestKey::Counter, || {
                called = true;
                0
            });
            assert_eq!(value, 99);
            assert!(!called);
        });
    }
}
