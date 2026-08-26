use crate::types::{DataKey, MigrationAnalytics, PoolMigrationRecord, PoolMigrationStats};
use soroban_sdk::{Address, Env};

/// Record a completed migration in the analytics.
pub fn record_migration(env: &Env, record: &PoolMigrationRecord) {
    let mut analytics: MigrationAnalytics = env
        .storage()
        .instance()
        .get(&DataKey::Analytics)
        .unwrap_or(default_analytics(env));

    analytics.total_migrations += 1;

    match record.status {
        crate::types::MigrationStatus::Completed => {
            analytics.successful_migrations += 1;
            analytics.total_volume_migrated += record.amount;
            analytics.total_gas_consumed += record.gas_used;

            if record.amount > analytics.largest_migration {
                analytics.largest_migration = record.amount;
            }

            // Update rolling average slippage
            if analytics.successful_migrations == 1 {
                analytics.average_slippage_bps = record.slippage_bps;
            } else {
                let prev = analytics.average_slippage_bps as u64;
                let new = record.slippage_bps as u64;
                let count = analytics.successful_migrations;
                analytics.average_slippage_bps =
                    ((prev * (count - 1) + new) / count) as u32;
            }

            analytics.most_active_source_pool = Some(record.source_pool.clone());
            analytics.most_active_destination_pool = Some(record.destination_pool.clone());
        }
        _ => {
            analytics.failed_migrations += 1;
        }
    }

    env.storage().instance().set(&DataKey::Analytics, &analytics);

    // Update per-pool stats
    update_pool_stats(env, &record.source_pool, record, true);
    update_pool_stats(env, &record.destination_pool, record, false);

    // Track unique users
    let count_key = DataKey::UserMigrationCount(record.user.clone());
    let user_count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);
    if user_count == 0 {
        analytics.unique_users += 1;
        env.storage().instance().set(&DataKey::Analytics, &analytics);
    }
    env.storage()
        .persistent()
        .set(&count_key, &(user_count + 1));
}

/// Update per-pool migration statistics.
fn update_pool_stats(
    env: &Env,
    pool: &Address,
    record: &PoolMigrationRecord,
    is_source: bool,
) {
    let stats_key = DataKey::PoolStats(pool.clone());
    let mut stats: PoolMigrationStats = env
        .storage()
        .persistent()
        .get(&stats_key)
        .unwrap_or(PoolMigrationStats {
            pool: pool.clone(),
            total_outflow: 0,
            total_inflow: 0,
            migration_count: 0,
            unique_migrators: 0,
        });

    if is_source {
        stats.total_outflow += record.amount;
    } else {
        stats.total_inflow += record.amount;
    }
    stats.migration_count += 1;

    env.storage().persistent().set(&stats_key, &stats);
}

/// Get global migration analytics.
pub fn get_analytics(env: &Env) -> MigrationAnalytics {
    env.storage()
        .instance()
        .get(&DataKey::Analytics)
        .unwrap_or(default_analytics(env))
}

/// Get migration statistics for a specific pool.
pub fn get_pool_stats(env: &Env, pool: &Address) -> PoolMigrationStats {
    env.storage()
        .persistent()
        .get(&DataKey::PoolStats(pool.clone()))
        .unwrap_or(PoolMigrationStats {
            pool: pool.clone(),
            total_outflow: 0,
            total_inflow: 0,
            migration_count: 0,
            unique_migrators: 0,
        })
}

/// Compute the net flow for a pool (positive = net inflow, negative = net outflow).
pub fn get_pool_net_flow(env: &Env, pool: &Address) -> i128 {
    let stats = get_pool_stats(env, pool);
    stats.total_inflow - stats.total_outflow
}

fn default_analytics(_env: &Env) -> MigrationAnalytics {
    MigrationAnalytics {
        total_migrations: 0,
        successful_migrations: 0,
        failed_migrations: 0,
        total_volume_migrated: 0,
        total_gas_consumed: 0,
        average_slippage_bps: 0,
        unique_users: 0,
        largest_migration: 0,
        most_active_source_pool: None,
        most_active_destination_pool: None,
    }
}
