//! Off-chain intent verification (Soroban auth-based).
//!
//! Soroban does not use EIP-712; instead, signatures are attached as transaction
//! authorization entries. We can still model "intents" by requiring the user to
//! authorize a typed payload (nonce + expiration + action parameters) and then
//! allowing a relayer to submit the transaction.

#![allow(unused)]

use soroban_sdk::{contracterror, contracttype, Address, Env, IntoVal, Symbol, Val, Vec};
use stellarlend_shared_deadline::require_deadline;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum IntentError {
    /// Nonce does not match expected value.
    BadNonce = 1,
    /// Intent has expired.
    Expired = 2,
}

#[contracttype]
#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum IntentDataKey {
    /// Next expected nonce for (user, operation).
    NextNonce(Address, Symbol),
}

pub fn next_nonce(env: &Env, user: &Address, op: &Symbol) -> u64 {
    let key = IntentDataKey::NextNonce(user.clone(), op.clone());
    env.storage()
        .persistent()
        .get::<IntentDataKey, u64>(&key)
        .unwrap_or(0)
}

fn set_next_nonce(env: &Env, user: &Address, op: &Symbol, nonce: u64) {
    let key = IntentDataKey::NextNonce(user.clone(), op.clone());
    env.storage().persistent().set(&key, &nonce);
}

/// Verify nonce + expiry and require the user to authorize the typed payload.
///
/// This should be called **before any state changes** in meta-tx style entrypoints.
pub fn require_intent_auth(
    env: &Env,
    user: &Address,
    op: &Symbol,
    nonce: u64,
    expires_at: u64,
    args: Vec<Val>,
) -> Result<(), IntentError> {
    require_deadline(env, expires_at, IntentError::Expired)?;

    let expected = next_nonce(env, user, op);
    if nonce != expected {
        return Err(IntentError::BadNonce);
    }

    // Typed payload: (contract, op, nonce, expires_at, args...)
    // This ensures replay protection across operations and binds the authorization
    // to this specific contract call + parameters.
    let mut auth_args = Vec::new(env);
    auth_args.push_back(env.current_contract_address().into_val(env));
    auth_args.push_back(op.clone().into_val(env));
    auth_args.push_back(nonce.into_val(env));
    auth_args.push_back(expires_at.into_val(env));
    for v in args.iter() {
        auth_args.push_back(v);
    }

    user.require_auth_for_args(auth_args);

    // Consume nonce (increment expected nonce).
    set_next_nonce(env, user, op, expected.saturating_add(1));
    Ok(())
}

/// Read-only: returns the next expected nonce for (user, operation).
pub fn get_next_nonce(env: &Env, user: Address, op: Symbol) -> u64 {
    next_nonce(env, &user, &op)
}
