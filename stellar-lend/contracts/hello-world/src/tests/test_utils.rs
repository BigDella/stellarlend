//! Test Utilities
//!
//! Common utilities and helper functions for testing the StellarLend protocol.
//! Provides setup functions, mock data generators, and test helpers.

use soroban_sdk::{Address, Env};

use crate::deposit::DepositDataKey;

/// Setup admin for tests
pub fn setup_admin(env: &Env, admin: &Address) {
    env.storage()
        .persistent()
        .set(&DepositDataKey::Admin, admin);
}

/// Generate test asset configuration
pub fn create_test_asset_config() -> crate::cross_asset::AssetConfig {
    crate::cross_asset::AssetConfig {
        asset: Some(Address::generate(&Env::default())),
        collateral_factor: 7500,  // 75% LTV
        liquidation_threshold: 8000,  // 80% liquidation threshold
        reserve_factor: 1000,  // 10% reserve
        max_supply: 10000000,
        max_borrow: 5000000,
        can_collateralize: true,
        can_borrow: true,
        price: 1000000,  // 1:1 with base asset
        price_updated_at: 0,
        is_isolated: false,
        is_frozen: false,
    }
}

/// Generate test user with some initial balance
pub fn create_test_user_with_balance(env: &Env, initial_balance: i128) -> Address {
    let user = Address::generate(env);
    
    // Mock token balance setup (in real tests, this would involve token contracts)
    // For now, we just return the user address
    user
}

/// Mock time advancement for testing time-dependent functionality
pub fn advance_time(env: &Env, seconds: u64) {
    // In real Soroban testing, this would involve ledger manipulation
    // For now, this is a placeholder
    let current_time = env.ledger().timestamp();
    // Note: Actual time advancement would need test framework support
}

/// Create test environment with basic setup
pub fn setup_test_env() -> (Env, Address) {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    // Setup admin
    setup_admin(&env, &admin);
    
    (env, admin)
}
