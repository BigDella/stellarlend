#![cfg(test)]

use soroban_sdk::{Address, Env};
use crate::cross_asset::{AssetConfig, initialize_asset};

pub fn create_test_asset_config() -> AssetConfig {
    AssetConfig {
        asset: Some(Address::generate(&Env::default())),
        collateral_factor: 7500,
        liquidation_threshold: 8000,
        reserve_factor: 1000,
        max_supply: 10000000,
        max_borrow: 5000000,
        can_collateralize: true,
        can_borrow: true,
        price: 1000000,
        price_updated_at: 0,
        is_isolated: false,
        is_frozen: false,
    }
}

pub fn setup_admin(env: &Env) {
    use crate::deposit::DepositDataKey;
    let admin = Address::generate(env);
    env.storage()
        .persistent()
        .set(&DepositDataKey::Admin, &admin);
}

pub fn setup_test_environment(env: &Env, admin: &Address, asset: Option<Address>, config: AssetConfig) {
    setup_admin(env);
    let _ = initialize_asset(env, asset, config);
}
