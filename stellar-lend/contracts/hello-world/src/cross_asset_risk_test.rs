//! Correlation-aware health factor and dynamic collateral factor tests (Issue #663).

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::cross_asset::{
    detect_cross_asset_arbitrage, get_asset_correlation, get_dynamic_collateral_factor,
    get_pair_liquidation_threshold, get_unified_health_factor, initialize, initialize_asset,
    set_asset_correlation, set_asset_volatility, AssetConfig,
};

fn sample_config(asset: Option<Address>, price: i128) -> AssetConfig {
    AssetConfig {
        asset,
        collateral_factor: 7500,
        liquidation_threshold: 8000,
        reserve_factor: 1000,
        max_supply: 0,
        max_borrow: 0,
        can_collateralize: true,
        can_borrow: true,
        price,
        price_updated_at: 0,
        is_isolated: false,
        is_frozen: false,
    }
}

fn setup() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let usdc = Address::generate(&env);
    let eth = Address::generate(&env);
    initialize(&env, admin.clone()).unwrap();
    initialize_asset(&env, Some(usdc.clone()), sample_config(Some(usdc.clone()), 10_000_000)).unwrap();
    initialize_asset(&env, Some(eth.clone()), sample_config(Some(eth.clone()), 20_000_000)).unwrap();
    (env, admin, usdc, eth)
}

#[test]
fn test_correlation_round_trip() {
    let (env, _admin, usdc, eth) = setup();
    set_asset_correlation(&env, Some(usdc.clone()), Some(eth.clone()), 8_000).unwrap();
    assert_eq!(get_asset_correlation(&env, Some(usdc.clone()), Some(eth.clone())), 8_000);
    assert_eq!(get_asset_correlation(&env, Some(eth), Some(usdc)), 8_000);
}

#[test]
fn test_invalid_correlation_rejected() {
    let (env, _admin, usdc, eth) = setup();
    let err = set_asset_correlation(&env, Some(usdc), Some(eth), 12_000);
    assert!(err.is_err());
}

#[test]
fn test_dynamic_collateral_factor_haircuts_high_vol() {
    let (env, _admin, usdc, _eth) = setup();
    set_asset_volatility(&env, Some(usdc.clone()), 4_000).unwrap();
    let cf = get_dynamic_collateral_factor(&env, Some(usdc)).unwrap();
    // 7500 * (10000 - 1000) / 10000 = 6750
    assert_eq!(cf, 6750);
}

#[test]
fn test_pair_liquidation_threshold_tightens_with_correlation() {
    let (env, _admin, usdc, eth) = setup();
    set_asset_correlation(&env, Some(usdc.clone()), Some(eth.clone()), 10_000).unwrap();
    let lt = get_pair_liquidation_threshold(&env, Some(usdc), Some(eth)).unwrap();
    assert!(lt > 8000);
    assert!(lt <= 9500);
}

#[test]
fn test_unified_health_factor_empty_position() {
    let (env, _admin, _usdc, _eth) = setup();
    let user = Address::generate(&env);
    let summary = get_unified_health_factor(&env, &user).unwrap();
    assert!(!summary.is_liquidatable);
    assert_eq!(summary.total_debt_value, 0);
}

#[test]
fn test_arbitrage_detection_empty_when_utils_match() {
    let (env, _admin, _usdc, _eth) = setup();
    let opps = detect_cross_asset_arbitrage(&env);
    assert_eq!(opps.len(), 0);
}
