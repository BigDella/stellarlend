#![cfg(test)]

use super::*;
use crate::types::{MigrationAnalytics, MigrationStatus, PoolMigrationStats};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, Env,
};

fn setup() -> (Env, Address, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let lending = Address::generate(&env);
    let source_pool = Address::generate(&env);
    let destination_pool = Address::generate(&env);

    (env, admin, user, lending, source_pool, destination_pool)
}

fn init_contract(
    env: &Env,
    admin: &Address,
    lending: &Address,
) -> PoolMigrationClient<'_> {
    let contract_id = env.register_contract(None, PoolMigration);
    let client = PoolMigrationClient::new(env, &contract_id);

    client.initialize(
        admin,
        lending,
        &100i128,    // min_migration_amount
        &1_000_000i128, // max_migration_amount
        &60u64,      // cooldown_secs
        &50u32,      // max_slippage_bps (0.5%)
        &2_000_000u64, // deadline
        &100u32,     // max_batch_size
    );

    client
}

#[test]
fn test_initialize() {
    let (env, admin, _, lending, _, _) = setup();
    let client = init_contract(&env, &admin, &lending);

    let config = client.get_config().unwrap();
    assert_eq!(config.admin, admin);
    assert_eq!(config.min_migration_amount, 100);
    assert_eq!(config.max_migration_amount, 1_000_000);
    assert_eq!(config.cooldown_secs, 60);
    assert_eq!(config.max_slippage_bps, 50);
    assert_eq!(config.max_batch_size, 100);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn test_double_initialize() {
    let (env, admin, _, lending, _, _) = setup();
    let client = init_contract(&env, &admin, &lending);

    // Second init should fail
    client.initialize(
        &admin,
        &lending,
        &100,
        &1_000_000,
        &60,
        &50,
        &2_000_000,
        &100,
    );
}

#[test]
fn test_migrate_basic() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());
    let token = soroban_sdk::token::Client::new(&env, &asset);
    token.mint(&user, &10_000);

    let migration_id = client.migrate(&user, &source, &dest, &asset, &5000);

    let record = client.get_migration(&migration_id).unwrap();
    assert_eq!(record.status, MigrationStatus::Completed);
    assert_eq!(record.amount, 5000);
    assert_eq!(record.user, user);
    assert_eq!(record.source_pool, source);
    assert_eq!(record.destination_pool, dest);

    let analytics = client.get_analytics();
    assert_eq!(analytics.total_migrations, 1);
    assert_eq!(analytics.successful_migrations, 1);
    assert_eq!(analytics.total_volume_migrated, 5000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #16)")]
fn test_migrate_same_pool() {
    let (env, admin, user, lending, pool, _) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());
    let token = soroban_sdk::token::Client::new(&env, &asset);
    token.mint(&user, &10_000);

    // Same source and destination should fail
    client.migrate(&user, &pool, &pool, &asset, &5000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #12)")]
fn test_migrate_below_minimum() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());
    let token = soroban_sdk::token::Client::new(&env, &asset);
    token.mint(&user, &10_000);

    // Amount below minimum (100) should fail
    client.migrate(&user, &source, &dest, &asset, &50);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #13)")]
fn test_migrate_above_maximum() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());
    let token = soroban_sdk::token::Client::new(&env, &asset);
    token.mint(&user, &10_000_000);

    // Amount above maximum (1_000_000) should fail
    client.migrate(&user, &source, &dest, &asset, &2_000_000);
}

#[test]
fn test_migrate_partial() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());
    let token = soroban_sdk::token::Client::new(&env, &asset);
    token.mint(&user, &10_000);

    // Migrate 50% of 10000 = 5000
    let migration_id = client.migrate_partial(&user, &source, &dest, &asset, &5000u32, &10000i128);

    let record = client.get_migration(&migration_id).unwrap();
    assert_eq!(record.status, MigrationStatus::Completed);
    assert_eq!(record.amount, 5000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #17)")]
fn test_migrate_partial_invalid_percentage() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());

    // 0% should fail
    client.migrate_partial(&user, &source, &dest, &asset, &0u32, &10000i128);
}

#[test]
fn test_preview() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());

    let preview = client.preview(&user, &source, &dest, &asset, &5000);

    assert!(preview.safety_passed);
    assert_eq!(preview.estimated_output, 5000);
    assert_eq!(preview.source_shares_to_burn, 5000);
    assert_eq!(preview.destination_shares_to_mint, 5000);
}

#[test]
fn test_batch_migrate() {
    let (env, admin, _, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());
    let token = soroban_sdk::token::Client::new(&env, &asset);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    token.mint(&user1, &10_000);
    token.mint(&user2, &10_000);
    token.mint(&user3, &10_000);

    let users = vec![&env, user1.clone(), user2.clone(), user3.clone()];
    let amounts = vec![&env, 1000i128, 2000i128, 3000i128];

    let results = client.batch_migrate(&admin, &users, &source, &dest, &asset, &amounts);

    assert_eq!(results.len(), 3);

    let analytics = client.get_analytics();
    assert_eq!(analytics.total_migrations, 3);
    assert_eq!(analytics.successful_migrations, 3);
    assert_eq!(analytics.total_volume_migrated, 6000);
}

#[test]
fn test_pool_stats() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());
    let token = soroban_sdk::token::Client::new(&env, &asset);
    token.mint(&user, &10_000);

    client.migrate(&user, &source, &dest, &asset, &5000);

    let source_stats = client.get_pool_stats(&source);
    assert_eq!(source_stats.total_outflow, 5000);
    assert_eq!(source_stats.migration_count, 1);

    let dest_stats = client.get_pool_stats(&dest);
    assert_eq!(dest_stats.total_inflow, 5000);
    assert_eq!(dest_stats.migration_count, 1);

    // Net flow: source is negative, dest is positive
    assert_eq!(client.get_pool_net_flow(&source), -5000);
    assert_eq!(client.get_pool_net_flow(&dest), 5000);
}

#[test]
fn test_pause_and_unpause() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    assert!(!client.is_paused());

    client.set_paused(&admin, &true);
    assert!(client.is_paused());

    client.set_paused(&admin, &false);
    assert!(!client.is_paused());
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #21)")]
fn test_migrate_while_paused() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());
    let token = soroban_sdk::token::Client::new(&env, &asset);
    token.mint(&user, &10_000);

    client.set_paused(&admin, &true);

    client.migrate(&user, &source, &dest, &asset, &5000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn test_unauthorized_pause() {
    let (env, admin, user, lending, _, _) = setup();
    let client = init_contract(&env, &admin, &lending);

    // Non-admin cannot pause
    client.set_paused(&user, &true);
}

#[test]
fn test_update_config() {
    let (env, admin, _, lending, _, _) = setup();
    let client = init_contract(&env, &admin, &lending);

    client.update_config(
        &admin,
        &Some(200i128),     // new min
        &Some(2_000_000i128), // new max
        &None,              // keep cooldown
        &Some(100u32),      // new slippage
        &None,              // keep deadline
        &None,              // keep batch size
    );

    let config = client.get_config().unwrap();
    assert_eq!(config.min_migration_amount, 200);
    assert_eq!(config.max_migration_amount, 2_000_000);
    assert_eq!(config.cooldown_secs, 60); // unchanged
    assert_eq!(config.max_slippage_bps, 100);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #15)")]
fn test_deadline_exceeded() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());
    let token = soroban_sdk::token::Client::new(&env, &asset);
    token.mint(&user, &10_000);

    // Set time past deadline (2_000_000)
    env.ledger().set_timestamp(2_000_001);

    client.migrate(&user, &source, &dest, &asset, &5000);
}

#[test]
fn test_cooldown_enforcement() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());
    let token = soroban_sdk::token::Client::new(&env, &asset);
    token.mint(&user, &10_000);

    // First migration succeeds
    client.migrate(&user, &source, &dest, &asset, &1000);

    // Second migration within cooldown should fail via preview
    let preview = client.preview(&user, &source, &dest, &asset, &1000);
    assert!(!preview.safety_passed);
}

#[test]
fn test_multiple_migrations_analytics() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());
    let token = soroban_sdk::token::Client::new(&env, &asset);
    token.mint(&user, &100_000);

    // First migration
    env.ledger().set_timestamp(100);
    client.migrate(&user, &source, &dest, &asset, &1000);

    // Wait for cooldown (60 secs)
    env.ledger().set_timestamp(200);
    client.migrate(&user, &source, &dest, &asset, &2000);

    let analytics = client.get_analytics();
    assert_eq!(analytics.total_migrations, 2);
    assert_eq!(analytics.successful_migrations, 2);
    assert_eq!(analytics.total_volume_migrated, 3000);
    assert_eq!(analytics.largest_migration, 2000);
}

#[test]
fn test_migration_record_fields() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());
    let token = soroban_sdk::token::Client::new(&env, &asset);
    token.mint(&user, &10_000);

    env.ledger().set_timestamp(12345);
    let id = client.migrate(&user, &source, &dest, &asset, &7500);

    let record = client.get_migration(&id).unwrap();
    assert_eq!(record.id, 0);
    assert_eq!(record.user, user);
    assert_eq!(record.source_pool, source);
    assert_eq!(record.destination_pool, dest);
    assert_eq!(record.amount, 7500);
    assert_eq!(record.shares_burned, 7500);
    assert_eq!(record.shares_minted, 7500);
    assert_eq!(record.timestamp, 12345);
    assert_eq!(record.status, MigrationStatus::Completed);
}

#[test]
fn test_sequential_migration_ids() {
    let (env, admin, user, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());
    let token = soroban_sdk::token::Client::new(&env, &asset);
    token.mint(&user, &100_000);

    let id0 = client.migrate(&user, &source, &dest, &asset, &1000);
    assert_eq!(id0, 0);

    env.ledger().set_timestamp(200);
    let id1 = client.migrate(&user, &source, &dest, &asset, &1000);
    assert_eq!(id1, 1);

    env.ledger().set_timestamp(400);
    let id2 = client.migrate(&user, &source, &dest, &asset, &1000);
    assert_eq!(id2, 2);
}

#[test]
fn test_batch_size_limit() {
    let (env, admin, _, lending, source, dest) = setup();
    let client = init_contract(&env, &admin, &lending);

    let asset = env.register_stellar_asset_contract(admin.clone());

    // Create 101 users (exceeds max_batch_size of 100)
    let mut users = Vec::new(&env);
    let mut amounts = Vec::new(&env);
    for _ in 0..101 {
        users.push_back(Address::generate(&env));
        amounts.push_back(1000i128);
    }

    // Should fail because batch size exceeds limit
    // Note: this will panic with BatchSizeExceeded
    let result = client.try_batch_migrate(&admin, &users, &source, &dest, &asset, &amounts);
    assert!(result.is_err());
}
