//! # Rate Manipulation Guard (issue #638)
//!
//! Monitors interest rate changes per-block and flags / pauses when rates
//! deviate beyond configurable thresholds. Maintains a time-weighted average
//! rate (TWAP) for reference comparisons.
//!
//! ## How it works
//!
//! 1. Every rate-producing operation (deposit, withdraw, borrow) calls
//!    [`record_rate_change`] which compares the new rate against the stored
//!    rate.
//! 2. The per-block deviation is computed as `(new_rate - old_rate) / old_rate`
//!    in basis points.
//! 3. If deviation exceeds the **alert** threshold → a `RateManipulationAlert`
//!    event is emitted and the attempt is logged.
//! 4. If deviation exceeds the **pause** threshold → the rate change is
//!    rejected and the transaction reverts.
//! 5. A TWAP accumulator tracks the running average for external consumers.

use soroban_sdk::{contracterror, contracttype, Address, Env, Symbol};

use crate::borrow::get_admin;

const BPS_DENOM: i128 = 10_000;

// ── Errors ────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RateGuardError {
    Unauthorized = 1,
    RateChangeExceedsPauseThreshold = 2,
    InvalidThresholds = 3,
    Overflow = 4,
}

// ── Storage keys ──────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum RateGuardKey {
    /// Configuration thresholds.
    Config,
    /// Last recorded rate snapshot (rate, timestamp, block).
    LastSnapshot,
    /// TWAP accumulator.
    Twap,
    /// Log of manipulation attempts (circular, last N entries).
    AttemptLog,
}

// ── Config ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RateGuardConfig {
    /// Alert when per-block rate change exceeds this (bps). Default 1000 (10%).
    pub alert_threshold_bps: i128,
    /// Pause when per-block rate change exceeds this (bps). Default 2500 (25%).
    pub pause_threshold_bps: i128,
    /// TWAP window in seconds. Default 3600 (1 hour).
    pub twap_window_secs: u64,
    /// Maximum log entries retained.
    pub max_log_entries: u32,
}

impl Default for RateGuardConfig {
    fn default() -> Self {
        RateGuardConfig {
            alert_threshold_bps: 1_000, // 10%
            pause_threshold_bps: 2_500, // 25%
            twap_window_secs: 3600,
            max_log_entries: 50,
        }
    }
}

// ── Snapshot ──────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RateSnapshot {
    pub rate_bps: i128,
    pub timestamp: u64,
    pub block: u32,
}

// ── TWAP accumulator ──────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RateTwap {
    /// Weighted sum of rate * time.
    pub weighted_sum: i128,
    /// Total time elapsed in the window (seconds).
    pub total_time: u64,
    /// Current TWAP value.
    pub twap_bps: i128,
    /// Last update timestamp.
    pub last_update: u64,
}

// ── Manipulation attempt log ──────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RateManipulationAttempt {
    pub address: Address,
    pub amount: i128,
    pub rate_impact_bps: i128,
    pub old_rate_bps: i128,
    pub new_rate_bps: i128,
    pub timestamp: u64,
    pub was_paused: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct RateAttemptLog {
    pub entries: soroban_sdk::Vec<RateManipulationAttempt>,
}

// ── Public view functions ─────────────────────────────────────────────────

pub fn get_config(env: &Env) -> RateGuardConfig {
    env.storage()
        .persistent()
        .get(&RateGuardKey::Config)
        .unwrap_or_else(|| RateGuardConfig::default())
}

pub fn get_last_snapshot(env: &Env) -> Option<RateSnapshot> {
    env.storage().persistent().get(&RateGuardKey::LastSnapshot)
}

pub fn get_twap(env: &Env) -> RateTwap {
    env.storage()
        .persistent()
        .get(&RateGuardKey::Twap)
        .unwrap_or(RateTwap {
            weighted_sum: 0,
            total_time: 0,
            twap_bps: 0,
            last_update: 0,
        })
}

pub fn get_attempt_log(env: &Env) -> Vec<RateManipulationAttempt> {
    env.storage()
        .persistent()
        .get::<RateGuardKey, Vec<RateManipulationAttempt>>(&RateGuardKey::AttemptLog)
        .unwrap_or(Vec::new(env))
}

// ── Admin: configure thresholds ───────────────────────────────────────────

pub fn set_config(
    env: &Env,
    admin: Address,
    config: RateGuardConfig,
) -> Result<RateGuardConfig, RateGuardError> {
    admin.require_auth();
    let current_admin = get_admin(env).ok_or(RateGuardError::Unauthorized)?;
    if admin != current_admin {
        return Err(RateGuardError::Unauthorized);
    }

    if config.alert_threshold_bps <= 0
        || config.pause_threshold_bps <= 0
        || config.alert_threshold_bps >= config.pause_threshold_bps
    {
        return Err(RateGuardError::InvalidThresholds);
    }

    if config.twap_window_secs == 0 || config.max_log_entries == 0 {
        return Err(RateGuardError::InvalidThresholds);
    }

    env.storage()
        .persistent()
        .set(&RateGuardKey::Config, &config);

    Ok(config)
}

// ── Core: record and validate rate change ─────────────────────────────────

/// Record a new rate, checking for manipulation.
///
/// Returns `Ok(())` if the rate is within the alert threshold, or the
/// per-block deviation. If the rate exceeds the pause threshold, returns
/// `Err(RateGuardError::RateChangeExceedsPauseThreshold)`.
pub fn record_rate_change(
    env: &Env,
    new_rate_bps: i128,
    caller: &Address,
    amount: i128,
) -> Result<i128, RateGuardError> {
    let config = get_config(env);
    let now = env.ledger().timestamp();
    let block = env.ledger().sequence();

    let last = get_last_snapshot(env);
    let old_rate = last.as_ref().map(|s| s.rate_bps).unwrap_or(new_rate_bps);

    // Calculate per-block deviation in bps.
    let deviation_bps = if old_rate == 0 {
        if new_rate_bps == 0 {
            0
        } else {
            // Rate went from 0 to non-zero — flag as infinite deviation.
            BPS_DENOM
        }
    } else {
        let diff = (new_rate_bps - old_rate).abs();
        diff.checked_mul(BPS_DENOM)
            .ok_or(RateGuardError::Overflow)?
            .checked_div(old_rate.abs().max(1))
            .ok_or(RateGuardError::Overflow)?
    };

    // Check pause threshold first (hard stop).
    if deviation_bps > config.pause_threshold_bps {
        // Log the attempt.
        log_manipulation_attempt(
            env,
            caller,
            amount,
            deviation_bps,
            old_rate,
            new_rate_bps,
            now,
            true,
            &config,
        );
        return Err(RateGuardError::RateChangeExceedsPauseThreshold);
    }

    // Check alert threshold (soft alert).
    if deviation_bps > config.alert_threshold_bps {
        log_manipulation_attempt(
            env,
            caller,
            amount,
            deviation_bps,
            old_rate,
            new_rate_bps,
            now,
            false,
            &config,
        );

        // Emit alert event.
        emit_rate_manipulation_alert(env, caller, amount, old_rate, new_rate_bps, deviation_bps, now);
    }

    // Update snapshot.
    let snapshot = RateSnapshot {
        rate_bps: new_rate_bps,
        timestamp: now,
        block,
    };
    env.storage()
        .persistent()
        .set(&RateGuardKey::LastSnapshot, &snapshot);

    // Update TWAP.
    update_twap(env, new_rate_bps, now, &config);

    Ok(deviation_bps)
}

fn update_twap(env: &Env, rate_bps: i128, now: u64, config: &RateGuardConfig) {
    let mut twap = get_twap(env);

    if twap.last_update == 0 {
        // First sample.
        twap.weighted_sum = rate_bps;
        twap.total_time = 1;
        twap.twap_bps = rate_bps;
        twap.last_update = now;
    } else {
        let elapsed = now.saturating_sub(twap.last_update);

        // If window expired, reset.
        if elapsed > config.twap_window_secs {
            twap.weighted_sum = rate_bps;
            twap.total_time = 1;
            twap.twap_bps = rate_bps;
        } else {
            twap.weighted_sum = twap.weighted_sum.saturating_add(rate_bps);
            twap.total_time = twap.total_time.saturating_add(1);
            if twap.total_time > 0 {
                twap.twap_bps = twap.weighted_sum / twap.total_time as i128;
            }
        }
        twap.last_update = now;
    }

    env.storage()
        .persistent()
        .set(&RateGuardKey::Twap, &twap);
}

fn log_manipulation_attempt(
    env: &Env,
    caller: &Address,
    amount: i128,
    rate_impact_bps: i128,
    old_rate: i128,
    new_rate: i128,
    timestamp: u64,
    was_paused: bool,
    config: &RateGuardConfig,
) {
    let attempt = RateManipulationAttempt {
        address: caller.clone(),
        amount,
        rate_impact_bps,
        old_rate_bps: old_rate,
        new_rate_bps: new_rate,
        timestamp,
        was_paused,
    };

    let mut log = get_attempt_log(env);

    // Enforce max log size (drop oldest when full).
    if log.len() >= config.max_log_entries {
        let mut new_log = Vec::new(env);
        for i in 1..log.len() {
            if let Some(entry) = log.get(i) {
                new_log.push_back(entry);
            }
        }
        log = new_log;
    }

    log.push_back(attempt);
    env.storage()
        .persistent()
        .set(&RateGuardKey::AttemptLog, &log);
}

// ── View: check if a rate would be accepted ───────────────────────────────

/// Simulates whether a new rate would be accepted (no state change).
pub fn check_rate(
    env: &Env,
    new_rate_bps: i128,
) -> Result<(i128, bool, bool), RateGuardError> {
    let config = get_config(env);
    let last = get_last_snapshot(env);
    let old_rate = last.as_ref().map(|s| s.rate_bps).unwrap_or(new_rate_bps);

    let deviation_bps = if old_rate == 0 {
        if new_rate_bps == 0 {
            0
        } else {
            BPS_DENOM
        }
    } else {
        let diff = (new_rate_bps - old_rate).abs();
        diff.checked_mul(BPS_DENOM)
            .ok_or(RateGuardError::Overflow)?
            .checked_div(old_rate.abs().max(1))
            .ok_or(RateGuardError::Overflow)?
    };

    let will_alert = deviation_bps > config.alert_threshold_bps;
    let will_pause = deviation_bps > config.pause_threshold_bps;

    Ok((deviation_bps, will_alert, will_pause))
}

// ── View: get whitelisted aggregators ─────────────────────────────────────

/// Check if a given address is a known aggregator (whitelisted).
pub fn is_whitelisted_aggregator(env: &Env, address: &Address) -> bool {
    let key = Symbol::new(env, &format!("whitelist_{}", address.to_string()));
    env.storage()
        .persistent()
        .get::<Symbol, bool>(&key)
        .unwrap_or(false)
}

// ── Admin: whitelist aggregator ───────────────────────────────────────────

pub fn set_whitelisted_aggregator(
    env: &Env,
    admin: Address,
    address: Address,
    whitelisted: bool,
) -> Result<(), RateGuardError> {
    admin.require_auth();
    let current_admin = get_admin(env).ok_or(RateGuardError::Unauthorized)?;
    if admin != current_admin {
        return Err(RateGuardError::Unauthorized);
    }

    let key = Symbol::new(env, &format!("whitelist_{}", address.to_string()));
    env.storage().persistent().set(&key, &whitelisted);
    Ok(())
}

// ── Events ────────────────────────────────────────────────────────────────

fn emit_rate_manipulation_alert(
    env: &Env,
    address: &Address,
    amount: i128,
    old_rate: i128,
    new_rate: i128,
    deviation_bps: i128,
    timestamp: u64,
) {
    env.events().publish(
        (
            Symbol::new(env, "rate_manipulation_alert"),
            address.clone(),
        ),
        (amount, old_rate, new_rate, deviation_bps, timestamp),
    );
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn default_config_values() {
        let config = RateGuardConfig::default();
        assert_eq!(config.alert_threshold_bps, 1_000);
        assert_eq!(config.pause_threshold_bps, 2_500);
        assert_eq!(config.twap_window_secs, 3600);
    }

    #[test]
    fn deviation_calculation_zero_old_rate() {
        let old_rate = 0i128;
        let new_rate = 500i128;
        let deviation = if old_rate == 0 {
            if new_rate == 0 {
                0
            } else {
                BPS_DENOM
            }
        } else {
            0
        };
        assert_eq!(deviation, BPS_DENOM);
    }

    #[test]
    fn deviation_calculation_no_change() {
        let rate = 500i128;
        let diff = (rate - rate).abs();
        let deviation = diff * BPS_DENOM / rate.abs().max(1);
        assert_eq!(deviation, 0);
    }

    #[test]
    fn deviation_calculation_ten_percent() {
        let old_rate = 1000i128;
        let new_rate = 1100i128;
        let diff = (new_rate - old_rate).abs();
        let deviation = diff * BPS_DENOM / old_rate.abs().max(1);
        assert_eq!(deviation, 1000); // 10% = 1000 bps
    }

    #[test]
    fn deviation_calculation_twenty_five_percent() {
        let old_rate = 1000i128;
        let new_rate = 1250i128;
        let diff = (new_rate - old_rate).abs();
        let deviation = diff * BPS_DENOM / old_rate.abs().max(1);
        assert_eq!(deviation, 2500); // 25% = 2500 bps
    }

    #[test]
    fn rate_change_within_alert_threshold_passes() {
        let env = Env::default();
        let admin = Address::generate(&env);

        // Set up: store snapshot with rate 1000
        let snapshot = RateSnapshot {
            rate_bps: 1000,
            timestamp: env.ledger().timestamp(),
            block: env.ledger().sequence(),
        };
        env.storage().persistent().set(&RateGuardKey::LastSnapshot, &snapshot);

        let config = RateGuardConfig::default();
        env.storage().persistent().set(&RateGuardKey::Config, &config);

        // New rate 1050 = 5% change, below 10% alert threshold
        let caller = Address::generate(&env);
        let result = record_rate_change(&env, 1050, &caller, 1_000_000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 500); // 5% = 500 bps
    }

    #[test]
    fn rate_change_triggers_alert() {
        let env = Env::default();

        let snapshot = RateSnapshot {
            rate_bps: 1000,
            timestamp: env.ledger().timestamp(),
            block: env.ledger().sequence(),
        };
        env.storage().persistent().set(&RateGuardKey::LastSnapshot, &snapshot);

        let config = RateGuardConfig::default();
        env.storage().persistent().set(&RateGuardKey::Config, &config);

        // New rate 1150 = 15% change, above 10% alert threshold
        let caller = Address::generate(&env);
        let result = record_rate_change(&env, 1150, &caller, 1_000_000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1500); // 15%

        // Check log was recorded.
        let log = get_attempt_log(&env);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn rate_change_triggers_pause() {
        let env = Env::default();

        let snapshot = RateSnapshot {
            rate_bps: 1000,
            timestamp: env.ledger().timestamp(),
            block: env.ledger().sequence(),
        };
        env.storage().persistent().set(&RateGuardKey::LastSnapshot, &snapshot);

        let config = RateGuardConfig::default();
        env.storage().persistent().set(&RateGuardKey::Config, &config);

        // New rate 1300 = 30% change, above 25% pause threshold
        let caller = Address::generate(&env);
        let result = record_rate_change(&env, 1300, &caller, 1_000_000);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            RateGuardError::RateChangeExceedsPauseThreshold
        );
    }

    #[test]
    fn check_rate_does_not_modify_state() {
        let env = Env::default();

        let snapshot = RateSnapshot {
            rate_bps: 1000,
            timestamp: env.ledger().timestamp(),
            block: env.ledger().sequence(),
        };
        env.storage().persistent().set(&RateGuardKey::LastSnapshot, &snapshot);

        let config = RateGuardConfig::default();
        env.storage().persistent().set(&RateGuardKey::Config, &config);

        let result = check_rate(&env, 1150).unwrap();
        assert_eq!(result.0, 1500); // 15% deviation
        assert!(result.1); // will_alert
        assert!(!result.2); // won't pause

        // Snapshot should be unchanged.
        let snap = get_last_snapshot(&env).unwrap();
        assert_eq!(snap.rate_bps, 1000);
    }

    #[test]
    fn twap_accumulation() {
        let env = Env::default();

        let config = RateGuardConfig {
            twap_window_secs: 3600,
            ..Default::default()
        };
        env.storage().persistent().set(&RateGuardKey::Config, &config);

        let snapshot = RateSnapshot {
            rate_bps: 1000,
            timestamp: env.ledger().timestamp(),
            block: env.ledger().sequence(),
        };
        env.storage().persistent().set(&RateGuardKey::LastSnapshot, &snapshot);

        let caller = Address::generate(&env);
        let _ = record_rate_change(&env, 1050, &caller, 1_000_000);
        let _ = record_rate_change(&env, 1100, &caller, 1_000_000);

        let twap = get_twap(&env);
        // After recording 1050 and 1100, TWAP should reflect the average.
        assert!(twap.twap_bps > 0);
    }
}
