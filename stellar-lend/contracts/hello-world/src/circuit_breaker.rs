use soroban_sdk::{Address, Env, Vec, contracttype};

use crate::errors::LendingError;
use crate::storage;

/// Circuit breaker cooldown period (default 1 hour)
pub const DEFAULT_COOLDOWN_PERIOD: u64 = 3600;

/// Maximum cooldown period (24 hours)
pub const MAX_COOLDOWN_PERIOD: u64 = 86400;

/// Price deviation threshold for automatic trigger (20%)
pub const PRICE_DEVIATION_THRESHOLD_BPS: u64 = 2000;

/// Abnormal utilization threshold (95%)
pub const ABNORMAL_UTILIZATION_THRESHOLD_BPS: u64 = 9500;

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum CircuitBreakerTier {
    Tier1 = 1,
    Tier2 = 2,
    Tier3 = 3,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum CircuitBreakerStatus {
    Active,
    Tier1Paused,
    Tier2Paused,
    Tier3Halted,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct CircuitBreakerState {
    pub status: CircuitBreakerStatus,
    pub tier: CircuitBreakerTier,
    pub activated_at: u64,
    pub activated_by: Address,
    pub cooldown_period: u64,
    pub auto_deactivate_at: Option<u64>,
    pub reason: CircuitBreakerReason,
    pub affected_pool: Option<Address>,
    pub guardian_multisig: Option<Address>,
    pub governance_vote_required: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum CircuitBreakerReason {
    OracleFailure,
    FlashCrash,
    ExcessiveLiquidations,
    ManualActivation,
    SystemMaintenance,
    PriceDeviation,
    AbnormalUtilization,
    GuardianTrigger,
    GovernanceTrigger,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct CircuitBreakerConfig {
    pub cooldown_period: u64,
    pub auto_deactivate_enabled: bool,
    pub whitelist_enabled: bool,
    pub price_deviation_threshold_bps: u64,
    pub abnormal_utilization_threshold_bps: u64,
    pub guardian_multisig: Option<Address>,
    pub tier1_auto_trigger_enabled: bool,
    pub tier2_auto_trigger_enabled: bool,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            cooldown_period: DEFAULT_COOLDOWN_PERIOD,
            auto_deactivate_enabled: true,
            whitelist_enabled: true,
            price_deviation_threshold_bps: PRICE_DEVIATION_THRESHOLD_BPS,
            abnormal_utilization_threshold_bps: ABNORMAL_UTILIZATION_THRESHOLD_BPS,
            guardian_multisig: None,
            tier1_auto_trigger_enabled: true,
            tier2_auto_trigger_enabled: true,
        }
    }
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct TriggerCondition {
    pub pool_address: Option<Address>,
    pub price_deviation_bps: Option<u64>,
    pub utilization_bps: Option<u64>,
    pub consecutive_failures: Option<u32>,
    pub timestamp: u64,
}

/// Initialize circuit breaker
pub fn initialize_circuit_breaker(
    env: &Env,
    config: CircuitBreakerConfig,
) -> Result<(), LendingError> {
    if config.cooldown_period > MAX_COOLDOWN_PERIOD {
        return Err(LendingError::InvalidParameter);
    }

    let key = storage::DataKey::CircuitBreakerConfig;
    env.storage().instance().set(&key, &config);

    let state = CircuitBreakerState {
        status: CircuitBreakerStatus::Active,
        tier: CircuitBreakerTier::Tier1,
        activated_at: env.ledger().timestamp(),
        activated_by: env.current_contract_address(),
        cooldown_period: config.cooldown_period,
        auto_deactivate_at: None,
        reason: CircuitBreakerReason::SystemMaintenance,
        affected_pool: None,
        guardian_multisig: config.guardian_multisig.clone(),
        governance_vote_required: false,
    };

    let state_key = storage::DataKey::CircuitBreakerState;
    env.storage().persistent().set(&state_key, &state);

    let whitelist_key = storage::DataKey::CircuitBreakerWhitelist;
    let whitelist: Vec<Address> = Vec::new(env);
    env.storage().persistent().set(&whitelist_key, &whitelist);

    Ok(())
}

/// Get circuit breaker configuration
pub fn get_circuit_breaker_config(env: &Env) -> CircuitBreakerConfig {
    let key = storage::DataKey::CircuitBreakerConfig;
    env.storage()
        .instance()
        .get(&key)
        .unwrap_or_default()
}

/// Get circuit breaker state
pub fn get_circuit_breaker_state(env: &Env) -> Result<CircuitBreakerState, LendingError> {
    let key = storage::DataKey::CircuitBreakerState;
    env.storage()
        .persistent()
        .get(&key)
        .ok_or(LendingError::NotFound)
}

/// Activate circuit breaker (governance or admin only)
pub fn activate_circuit_breaker(
    env: &Env,
    caller: Address,
    reason: CircuitBreakerReason,
    emergency_mode: bool,
) -> Result<(), LendingError> {
    caller.require_auth();

    // Check authorization
    let admin = crate::admin::get_admin(env).ok_or(LendingError::Unauthorized)?;
    if caller != admin {
        return Err(LendingError::Unauthorized);
    }

    let config = get_circuit_breaker_config(env);
    let now = env.ledger().timestamp();

    let auto_deactivate_at = if config.auto_deactivate_enabled {
        Some(now + config.cooldown_period)
    } else {
        None
    };

    let status = if emergency_mode {
        CircuitBreakerStatus::Emergency
    } else {
        CircuitBreakerStatus::Paused
    };

    let state = CircuitBreakerState {
        status,
        activated_at: now,
        activated_by: caller.clone(),
        cooldown_period: config.cooldown_period,
        auto_deactivate_at,
        reason: reason.clone(),
    };

    let state_key = storage::DataKey::CircuitBreakerState;
    env.storage().persistent().set(&state_key, &state);

    // Emit event
    crate::events::CircuitBreakerActivatedEvent {
        activated_by: caller,
        emergency_mode,
        timestamp: now,
    }
    .publish(env);

    Ok(())
}

/// Deactivate circuit breaker (governance or admin only)
pub fn deactivate_circuit_breaker(env: &Env, caller: Address) -> Result<(), LendingError> {
    caller.require_auth();

    // Check authorization
    let admin = crate::admin::get_admin(env).ok_or(LendingError::Unauthorized)?;
    if caller != admin {
        return Err(LendingError::Unauthorized);
    }

    let state_key = storage::DataKey::CircuitBreakerState;
    let mut state: CircuitBreakerState = env
        .storage()
        .persistent()
        .get(&state_key)
        .ok_or(LendingError::NotFound)?;

    if state.status == CircuitBreakerStatus::Active {
        return Err(LendingError::InvalidState);
    }

    state.status = CircuitBreakerStatus::Active;
    state.auto_deactivate_at = None;
    env.storage().persistent().set(&state_key, &state);

    // Emit event
    crate::events::CircuitBreakerDeactivatedEvent {
        deactivated_by: caller,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Check if liquidations are allowed
pub fn is_liquidation_allowed(env: &Env, liquidator: &Address) -> Result<bool, LendingError> {
    let state = get_circuit_breaker_state(env)?;

    // Check auto-deactivation
    if let Some(auto_deactivate_at) = state.auto_deactivate_at {
        if env.ledger().timestamp() >= auto_deactivate_at {
            // Auto-deactivate
            let state_key = storage::DataKey::CircuitBreakerState;
            let mut new_state = state.clone();
            new_state.status = CircuitBreakerStatus::Active;
            new_state.auto_deactivate_at = None;
            env.storage().persistent().set(&state_key, &new_state);

            return Ok(true);
        }
    }

    match state.status {
        CircuitBreakerStatus::Active => Ok(true),
        CircuitBreakerStatus::Paused => Ok(false),
        CircuitBreakerStatus::Emergency => {
            // Check whitelist
            is_whitelisted(env, liquidator)
        }
    }
}

/// Add address to emergency liquidator whitelist
pub fn add_to_whitelist(env: &Env, admin: Address, liquidator: Address) -> Result<(), LendingError> {
    admin.require_auth();

    let stored_admin = crate::admin::get_admin(env).ok_or(LendingError::Unauthorized)?;
    if admin != stored_admin {
        return Err(LendingError::Unauthorized);
    }

    let whitelist_key = storage::DataKey::CircuitBreakerWhitelist;
    let mut whitelist: Vec<Address> = env
        .storage()
        .persistent()
        .get(&whitelist_key)
        .unwrap_or_else(|| Vec::new(env));

    if whitelist.contains(&liquidator) {
        return Err(LendingError::AlreadyExists);
    }

    whitelist.push_back(liquidator.clone());
    env.storage().persistent().set(&whitelist_key, &whitelist);

    // Emit event
    crate::events::WhitelistAddedEvent {
        liquidator,
        added_by: admin,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Remove address from emergency liquidator whitelist
pub fn remove_from_whitelist(
    env: &Env,
    admin: Address,
    liquidator: Address,
) -> Result<(), LendingError> {
    admin.require_auth();

    let stored_admin = crate::admin::get_admin(env).ok_or(LendingError::Unauthorized)?;
    if admin != stored_admin {
        return Err(LendingError::Unauthorized);
    }

    let whitelist_key = storage::DataKey::CircuitBreakerWhitelist;
    let whitelist: Vec<Address> = env
        .storage()
        .persistent()
        .get(&whitelist_key)
        .ok_or(LendingError::NotFound)?;

    let mut new_whitelist = Vec::new(env);
    let mut found = false;

    for addr in whitelist.iter() {
        if addr != liquidator {
            new_whitelist.push_back(addr);
        } else {
            found = true;
        }
    }

    if !found {
        return Err(LendingError::NotFound);
    }

    env.storage()
        .persistent()
        .set(&whitelist_key, &new_whitelist);

    // Emit event
    crate::events::WhitelistRemovedEvent {
        liquidator,
        removed_by: admin,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Check if address is whitelisted
pub fn is_whitelisted(env: &Env, liquidator: &Address) -> Result<bool, LendingError> {
    let whitelist_key = storage::DataKey::CircuitBreakerWhitelist;
    let whitelist: Vec<Address> = env
        .storage()
        .persistent()
        .get(&whitelist_key)
        .unwrap_or_else(|| Vec::new(env));

    Ok(whitelist.contains(liquidator))
}

/// Get whitelist
pub fn get_whitelist(env: &Env) -> Vec<Address> {
    let whitelist_key = storage::DataKey::CircuitBreakerWhitelist;
    env.storage()
        .persistent()
        .get(&whitelist_key)
        .unwrap_or_else(|| Vec::new(env))
}

/// Update circuit breaker configuration
pub fn update_circuit_breaker_config(
    env: &Env,
    admin: Address,
    config: CircuitBreakerConfig,
) -> Result<(), LendingError> {
    admin.require_auth();

    let stored_admin = crate::admin::get_admin(env).ok_or(LendingError::Unauthorized)?;
    if admin != stored_admin {
        return Err(LendingError::Unauthorized);
    }

    if config.cooldown_period > MAX_COOLDOWN_PERIOD {
        return Err(LendingError::InvalidParameter);
    }

    let key = storage::DataKey::CircuitBreakerConfig;
    env.storage().instance().set(&key, &config);

    Ok(())
}

/// Activate Tier 1 pause (single pool)
pub fn activate_tier1_pause(
    env: &Env,
    caller: Address,
    pool_address: Address,
    reason: CircuitBreakerReason,
) -> Result<(), LendingError> {
    caller.require_auth();

    let admin = crate::admin::get_admin(env).ok_or(LendingError::Unauthorized)?;
    let config = get_circuit_breaker_config(env);
    let is_guardian = config.guardian_multisig.as_ref() == Some(&caller);

    if caller != admin && !is_guardian {
        return Err(LendingError::Unauthorized);
    }

    let state_key = storage::DataKey::CircuitBreakerState;
    let current_state: CircuitBreakerState = env
        .storage()
        .persistent()
        .get(&state_key)
        .ok_or(LendingError::NotFound)?;

    if current_state.status == CircuitBreakerStatus::Tier3Halted {
        return Err(LendingError::InvalidState);
    }

    let now = env.ledger().timestamp();
    let auto_deactivate_at = if config.auto_deactivate_enabled {
        Some(now + config.cooldown_period)
    } else {
        None
    };

    let state = CircuitBreakerState {
        status: CircuitBreakerStatus::Tier1Paused,
        tier: CircuitBreakerTier::Tier1,
        activated_at: now,
        activated_by: caller.clone(),
        cooldown_period: config.cooldown_period,
        auto_deactivate_at,
        reason: reason.clone(),
        affected_pool: Some(pool_address.clone()),
        guardian_multisig: config.guardian_multisig,
        governance_vote_required: false,
    };

    env.storage().persistent().set(&state_key, &state);

    crate::events::CircuitBreakerActivatedEvent {
        activated_by: caller,
        emergency_mode: false,
        timestamp: now,
    }
    .publish(env);

    Ok(())
}

/// Activate Tier 2 pause (all lending)
pub fn activate_tier2_pause(
    env: &Env,
    caller: Address,
    reason: CircuitBreakerReason,
) -> Result<(), LendingError> {
    caller.require_auth();

    let admin = crate::admin::get_admin(env).ok_or(LendingError::Unauthorized)?;
    let config = get_circuit_breaker_config(env);
    let is_guardian = config.guardian_multisig.as_ref() == Some(&caller);

    if caller != admin && !is_guardian {
        return Err(LendingError::Unauthorized);
    }

    let state_key = storage::DataKey::CircuitBreakerState;
    let current_state: CircuitBreakerState = env
        .storage()
        .persistent()
        .get(&state_key)
        .ok_or(LendingError::NotFound)?;

    if current_state.status == CircuitBreakerStatus::Tier3Halted {
        return Err(LendingError::InvalidState);
    }

    let now = env.ledger().timestamp();
    let auto_deactivate_at = if config.auto_deactivate_enabled {
        Some(now + config.cooldown_period)
    } else {
        None
    };

    let state = CircuitBreakerState {
        status: CircuitBreakerStatus::Tier2Paused,
        tier: CircuitBreakerTier::Tier2,
        activated_at: now,
        activated_by: caller.clone(),
        cooldown_period: config.cooldown_period,
        auto_deactivate_at,
        reason: reason.clone(),
        affected_pool: None,
        guardian_multisig: config.guardian_multisig,
        governance_vote_required: false,
    };

    env.storage().persistent().set(&state_key, &state);

    crate::events::CircuitBreakerActivatedEvent {
        activated_by: caller,
        emergency_mode: true,
        timestamp: now,
    }
    .publish(env);

    Ok(())
}

/// Activate Tier 3 halt (full protocol)
pub fn activate_tier3_halt(
    env: &Env,
    caller: Address,
    reason: CircuitBreakerReason,
) -> Result<(), LendingError> {
    caller.require_auth();

    let admin = crate::admin::get_admin(env).ok_or(LendingError::Unauthorized)?;
    let config = get_circuit_breaker_config(env);
    let is_guardian = config.guardian_multisig.as_ref() == Some(&caller);

    if caller != admin && !is_guardian {
        return Err(LendingError::Unauthorized);
    }

    let now = env.ledger().timestamp();

    let state = CircuitBreakerState {
        status: CircuitBreakerStatus::Tier3Halted,
        tier: CircuitBreakerTier::Tier3,
        activated_at: now,
        activated_by: caller.clone(),
        cooldown_period: 0,
        auto_deactivate_at: None,
        reason: reason.clone(),
        affected_pool: None,
        guardian_multisig: config.guardian_multisig,
        governance_vote_required: true,
    };

    let state_key = storage::DataKey::CircuitBreakerState;
    env.storage().persistent().set(&state_key, &state);

    crate::events::CircuitBreakerActivatedEvent {
        activated_by: caller,
        emergency_mode: true,
        timestamp: now,
    }
    .publish(env);

    Ok(())
}

/// Check automatic trigger conditions
pub fn check_automatic_triggers(
    env: &Env,
    pool_address: &Address,
    price_deviation_bps: u64,
    utilization_bps: u64,
) -> Result<Option<CircuitBreakerTier>, LendingError> {
    let config = get_circuit_breaker_config(env);

    let state_key = storage::DataKey::CircuitBreakerState;
    let current_state: CircuitBreakerState = env
        .storage()
        .persistent()
        .get(&state_key)
        .ok_or(LendingError::NotFound)?;

    if current_state.status != CircuitBreakerStatus::Active {
        return Ok(None);
    }

    if config.tier1_auto_trigger_enabled && price_deviation_bps >= config.price_deviation_threshold_bps {
        return Ok(Some(CircuitBreakerTier::Tier1));
    }

    if config.tier2_auto_trigger_enabled && utilization_bps >= config.abnormal_utilization_threshold_bps {
        return Ok(Some(CircuitBreakerTier::Tier2));
    }

    Ok(None)
}

/// Deactivate circuit breaker with governance check for Tier 3
pub fn deactivate_tiered_circuit_breaker(
    env: &Env,
    caller: Address,
    governance_approved: bool,
) -> Result<(), LendingError> {
    caller.require_auth();

    let admin = crate::admin::get_admin(env).ok_or(LendingError::Unauthorized)?;
    let state_key = storage::DataKey::CircuitBreakerState;
    let mut state: CircuitBreakerState = env
        .storage()
        .persistent()
        .get(&state_key)
        .ok_or(LendingError::NotFound)?;

    let is_guardian = state.guardian_multisig.as_ref() == Some(&caller);

    if caller != admin && !is_guardian {
        return Err(LendingError::Unauthorized);
    }

    if state.status == CircuitBreakerStatus::Active {
        return Err(LendingError::InvalidState);
    }

    if state.tier == CircuitBreakerTier::Tier3 && !governance_approved {
        return Err(LendingError::Unauthorized);
    }

    state.status = CircuitBreakerStatus::Active;
    state.auto_deactivate_at = None;
    state.governance_vote_required = false;
    env.storage().persistent().set(&state_key, &state);

    crate::events::CircuitBreakerDeactivatedEvent {
        deactivated_by: caller,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Check if operation is allowed based on current tier
pub fn is_operation_allowed(
    env: &Env,
    operation_type: &str,
    pool_address: &Address,
    caller: &Address,
) -> Result<bool, LendingError> {
    let state = get_circuit_breaker_state(env)?;

    match state.status {
        CircuitBreakerStatus::Active => Ok(true),
        CircuitBreakerStatus::Tier1Paused => {
            if state.affected_pool.as_ref() == Some(pool_address) {
                Ok(false)
            } else {
                Ok(true)
            }
        }
        CircuitBreakerStatus::Tier2Paused => {
            Ok(false)
        }
        CircuitBreakerStatus::Tier3Halted => {
            is_whitelisted(env, caller)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    #[test]
    fn test_initialize_circuit_breaker() {
        let env = Env::default();
        let config = CircuitBreakerConfig::default();

        initialize_circuit_breaker(&env, config).unwrap();

        let state = get_circuit_breaker_state(&env).unwrap();
        assert_eq!(state.status, CircuitBreakerStatus::Active);
    }

    #[test]
    fn test_activate_circuit_breaker() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        crate::admin::set_admin(&env, admin.clone(), None).unwrap();

        let config = CircuitBreakerConfig::default();
        initialize_circuit_breaker(&env, config).unwrap();

        activate_circuit_breaker(
            &env,
            admin,
            CircuitBreakerReason::FlashCrash,
            false,
        )
        .unwrap();

        let state = get_circuit_breaker_state(&env).unwrap();
        assert_eq!(state.status, CircuitBreakerStatus::Paused);
    }

    #[test]
    fn test_liquidation_blocked_when_paused() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let liquidator = Address::generate(&env);
        crate::admin::set_admin(&env, admin.clone(), None).unwrap();

        let config = CircuitBreakerConfig::default();
        initialize_circuit_breaker(&env, config).unwrap();

        activate_circuit_breaker(
            &env,
            admin,
            CircuitBreakerReason::FlashCrash,
            false,
        )
        .unwrap();

        let allowed = is_liquidation_allowed(&env, &liquidator).unwrap();
        assert!(!allowed);
    }

    #[test]
    fn test_whitelist_in_emergency_mode() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let liquidator = Address::generate(&env);
        crate::admin::set_admin(&env, admin.clone(), None).unwrap();

        let config = CircuitBreakerConfig::default();
        initialize_circuit_breaker(&env, config).unwrap();

        // Add to whitelist
        add_to_whitelist(&env, admin.clone(), liquidator.clone()).unwrap();

        // Activate emergency mode
        activate_circuit_breaker(
            &env,
            admin,
            CircuitBreakerReason::OracleFailure,
            true,
        )
        .unwrap();

        // Whitelisted liquidator should be allowed
        let allowed = is_liquidation_allowed(&env, &liquidator).unwrap();
        assert!(allowed);

        // Non-whitelisted should not be allowed
        let other_liquidator = Address::generate(&env);
        let allowed = is_liquidation_allowed(&env, &other_liquidator).unwrap();
        assert!(!allowed);
    }

    #[test]
    fn test_auto_deactivation() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let liquidator = Address::generate(&env);
        crate::admin::set_admin(&env, admin.clone(), None).unwrap();

        let config = CircuitBreakerConfig::default();
        initialize_circuit_breaker(&env, config.clone()).unwrap();

        activate_circuit_breaker(
            &env,
            admin,
            CircuitBreakerReason::FlashCrash,
            false,
        )
        .unwrap();

        // Should be blocked initially
        let allowed = is_liquidation_allowed(&env, &liquidator).unwrap();
        assert!(!allowed);

        // Advance time past cooldown
        env.ledger().with_mut(|li| {
            li.timestamp += config.cooldown_period + 1;
        });

        // Should auto-deactivate and allow liquidations
        let allowed = is_liquidation_allowed(&env, &liquidator).unwrap();
        assert!(allowed);

        // State should be active
        let state = get_circuit_breaker_state(&env).unwrap();
        assert_eq!(state.status, CircuitBreakerStatus::Active);
    }

    #[test]
    fn test_deactivate_circuit_breaker() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        crate::admin::set_admin(&env, admin.clone(), None).unwrap();

        let config = CircuitBreakerConfig::default();
        initialize_circuit_breaker(&env, config).unwrap();

        activate_circuit_breaker(
            &env,
            admin.clone(),
            CircuitBreakerReason::FlashCrash,
            false,
        )
        .unwrap();

        deactivate_circuit_breaker(&env, admin).unwrap();

        let state = get_circuit_breaker_state(&env).unwrap();
        assert_eq!(state.status, CircuitBreakerStatus::Active);
    }
}
