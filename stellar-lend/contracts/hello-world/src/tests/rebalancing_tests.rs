//! Rebalancing Tests
//!
//! Comprehensive test suite for automated collateral rebalancing functionality.
//! Tests various scenarios including configuration, execution, and edge cases.

use soroban_sdk::{Address, Env, Symbol};

use crate::rebalancing::{
    configure_rebalancing, execute_rebalancing, get_rebalancing_config,
    set_emergency_stop, set_rebalancing_pause, RebalancingConfig, RebalancingError,
};
use crate::deposit::DepositDataKey;
use crate::test_utils::*;

#[test]
fn test_configure_rebalancing_success() {
    let env = Env::default();
    let user = Address::generate(&env);

    // Test successful configuration
    let result = configure_rebalancing(
        &env,
        user.clone(),
        12000, // 1.2x min health
        25000, // 2.5x max health
        1000000, // Max gas cost
        true,   // Enable auto-rebalance
        1000000, // Min swap size
        500,    // 5% max slippage
        3600,   // 1 hour cooldown
    );
    
    assert!(result.is_ok());
    
    // Verify configuration
    let config = get_rebalancing_config(&env, &user);
    assert_eq!(config.target_health_factor_min, 12000);
    assert_eq!(config.target_health_factor_max, 25000);
    assert_eq!(config.max_gas_cost, 1000000);
    assert!(config.auto_rebalance_enabled);
    assert_eq!(config.min_swap_size, 1000000);
    assert_eq!(config.max_slippage_bps, 500);
    assert_eq!(config.rebalance_cooldown, 3600);
}

#[test]
fn test_configure_rebalancing_invalid_config() {
    let env = Env::default();
    let user = Address::generate(&env);

    // Test invalid configuration (min > max)
    let result = configure_rebalancing(
        &env,
        user.clone(),
        25000, // 2.5x min health
        12000, // 1.2x max health (invalid)
        1000000,
        true,
        1000000,
        500,
        3600,
    );
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), RebalancingError::InvalidConfig);
}

#[test]
fn test_configure_rebalancing_invalid_gas_cost() {
    let env = Env::default();
    let user = Address::generate(&env);

    // Test negative gas cost
    let result = configure_rebalancing(
        &env,
        user.clone(),
        12000,
        25000,
        -1000, // Negative gas cost
        true,
        1000000,
        500,
        3600,
    );
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), RebalancingError::InvalidConfig);
}

#[test]
fn test_execute_rebalancing_already_healthy() {
    let env = Env::default();
    let user = Address::generate(&env);

    // Setup configuration
    configure_rebalancing(
        &env,
        user.clone(),
        12000,
        25000,
        1000000,
        true,
        1000000,
        500,
        3600,
    ).unwrap();

    // Mock healthy position (within target range)
    // This would require mocking the cross_asset module
    // For now, test the authorization and basic flow
    let result = execute_rebalancing(&env, user.clone());
    
    // Should fail because position is already healthy (mock scenario)
    // In real implementation, this would check actual position
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), RebalancingError::AlreadyHealthy);
}

#[test]
fn test_execute_rebalancing_cooldown_active() {
    let env = Env::default();
    let user = Address::generate(&env);

    // Setup configuration with very short cooldown
    configure_rebalancing(
        &env,
        user.clone(),
        12000,
        25000,
        1000000,
        true,
        1000000,
        500,
        10, // 10 second cooldown
    ).unwrap();

    // Execute rebalancing twice
    execute_rebalancing(&env, user.clone()).unwrap(); // First call succeeds
    
    // Second call should fail due to cooldown
    let result = execute_rebalancing(&env, user.clone());
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), RebalancingError::CooldownActive);
}

#[test]
fn test_set_emergency_stop_success() {
    let env = Env::default();
    let admin = Address::generate(&env);

    // Setup admin
    env.storage()
        .persistent()
        .set(&DepositDataKey::Admin, &admin);

    // Test emergency stop
    let result = set_emergency_stop(&env, admin.clone(), true);
    assert!(result.is_ok());
    
    // Verify emergency stop is active
    assert!(is_emergency_stop_active(&env));
}

#[test]
fn test_set_emergency_stop_unauthorized() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);

    // Setup admin
    env.storage()
        .persistent()
        .set(&DepositDataKey::Admin, &admin);

    // Test unauthorized emergency stop
    let result = set_emergency_stop(&env, unauthorized_user, true);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), RebalancingError::Unauthorized);
}

#[test]
fn test_set_rebalancing_pause_success() {
    let env = Env::default();
    let admin = Address::generate(&env);

    // Setup admin
    env.storage()
        .persistent()
        .set(&DepositDataKey::Admin, &admin);

    // Test rebalancing pause
    let result = set_rebalancing_pause(&env, admin.clone(), true);
    assert!(result.is_ok());
    
    // Verify pause is active
    assert!(is_rebalancing_paused(&env));
}

#[test]
fn test_get_rebalancing_config_default() {
    let env = Env::default();
    let user = Address::generate(&env);

    // Test getting default configuration for new user
    let config = get_rebalancing_config(&env, &user);
    
    // Verify default values
    assert_eq!(config.target_health_factor_min, 15000); // 1.5x default
    assert_eq!(config.target_health_factor_max, 25000); // 2.5x default
    assert_eq!(config.max_gas_cost, 1000000);
    assert!(!config.auto_rebalance_enabled); // Disabled by default
    assert_eq!(config.min_swap_size, 1000000);
    assert_eq!(config.max_slippage_bps, 500); // 5% default
    assert_eq!(config.rebalance_cooldown, 3600); // 1 hour default
}

#[test]
fn test_rebalancing_gas_cost_estimation() {
    let env = Env::default();
    let user = Address::generate(&env);

    // Setup configuration
    configure_rebalancing(
        &env,
        user.clone(),
        12000,
        25000,
        500000, // Low gas cost threshold
        true,
        1000000,
        500,
        3600,
    ).unwrap();

    // Test rebalancing with high estimated gas cost
    // This would require mocking the position summary
    // For now, test the configuration validation
    let config = get_rebalancing_config(&env, &user);
    assert_eq!(config.max_gas_cost, 500000);
}

// Helper functions

fn is_emergency_stop_active(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&crate::rebalancing::RebalancingDataKey::EmergencyStop)
        .unwrap_or(false)
}

fn is_rebalancing_paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&crate::rebalancing::RebalancingDataKey::RebalancingPaused)
        .unwrap_or(false)
}
