#![no_std]

use soroban_sdk::Env;

/// Return `true` when the current ledger timestamp has passed `deadline`.
///
/// A deadline of `0` is treated as already expired so time-sensitive callers
/// cannot silently opt out of enforcement.
pub fn is_expired(env: &Env, deadline: u64) -> bool {
    env.ledger().timestamp() > deadline
}

/// Return `true` when the current ledger timestamp has reached or passed `deadline`.
pub fn is_expired_strict(env: &Env, deadline: u64) -> bool {
    env.ledger().timestamp() >= deadline
}

/// Require the current ledger timestamp to be at or before `deadline`.
pub fn require_deadline<E: Copy>(env: &Env, deadline: u64, error: E) -> Result<(), E> {
    if is_expired(env, deadline) {
        return Err(error);
    }
    Ok(())
}

/// Require the current ledger timestamp to be strictly before `deadline`.
pub fn require_strict_deadline<E: Copy>(env: &Env, deadline: u64, error: E) -> Result<(), E> {
    if is_expired_strict(env, deadline) {
        return Err(error);
    }
    Ok(())
}

/// Require the current ledger timestamp to fall inside a start/end window.
///
/// The window is inclusive at `start_at` and inclusive at `end_at`.
/// Pass `end_at = 0` to leave the upper bound open for callers that need it.
pub fn require_window<E: Copy>(
    env: &Env,
    start_at: u64,
    end_at: u64,
    not_started: E,
    expired: E,
) -> Result<(), E> {
    let now = env.ledger().timestamp();
    if now < start_at {
        return Err(not_started);
    }
    if end_at != 0 && now > end_at {
        return Err(expired);
    }
    Ok(())
}
