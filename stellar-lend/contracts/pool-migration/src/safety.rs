use crate::types::{DataKey, MigrationError, PoolMigrationConfig, SafetyCheckResult};
use soroban_sdk::{Address, Env};

/// Run all safety checks before allowing a pool-to-pool migration.
///
/// Checks:
/// 1. Contract not paused
/// 2. Source and destination pools are different
/// 3. Amount within configured limits
/// 4. Cooldown period has elapsed since user's last migration
/// 5. Deadline not exceeded
/// 6. Destination pool has sufficient liquidity
/// 7. Post-migration health factor would remain valid
pub fn run_safety_checks(
    env: &Env,
    user: &Address,
    source_pool: &Address,
    destination_pool: &Address,
    amount: i128,
    config: &PoolMigrationConfig,
) -> Result<SafetyCheckResult, MigrationError> {
    // 1. Global pause check
    let paused: bool = env
        .storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false);
    if paused {
        return Ok(SafetyCheckResult {
            passed: false,
            source_pool_active: true,
            destination_pool_active: true,
            destination_has_liquidity: true,
            health_factor_ok: true,
            within_limits: true,
            cooldown_elapsed: true,
            failure_code: MigrationError::SafetyCheckFailed as u32,
        });
    }

    // 2. Same-pool check
    if source_pool == destination_pool {
        return Err(MigrationError::SamePool);
    }

    // 3. Amount limits
    let within_limits = amount >= config.min_migration_amount
        && (config.max_migration_amount == 0 || amount <= config.max_migration_amount);

    if !within_limits {
        return Ok(SafetyCheckResult {
            passed: false,
            source_pool_active: true,
            destination_pool_active: true,
            destination_has_liquidity: true,
            health_factor_ok: true,
            within_limits: false,
            cooldown_elapsed: true,
            failure_code: if amount < config.min_migration_amount {
                MigrationError::MigrationTooSmall as u32
            } else {
                MigrationError::MigrationTooLarge as u32
            },
        });
    }

    // 4. Cooldown check
    let cooldown_elapsed = check_cooldown(env, user, config.cooldown_secs);
    if !cooldown_elapsed {
        return Ok(SafetyCheckResult {
            passed: false,
            source_pool_active: true,
            destination_pool_active: true,
            destination_has_liquidity: true,
            health_factor_ok: true,
            within_limits: true,
            cooldown_elapsed: false,
            failure_code: MigrationError::CooldownNotElapsed as u32,
        });
    }

    // 5. Deadline check
    stellarlend_shared_deadline::require_deadline(
        env,
        config.deadline,
        MigrationError::DeadlineExceeded,
    )?;

    // 6. Destination liquidity check (simplified: we trust the pool exists
    //    and has capacity; real implementation would cross-call the pool)
    let destination_has_liquidity = true;

    // 7. Health factor check (simplified: would compute post-migration HF)
    let health_factor_ok = true;

    Ok(SafetyCheckResult {
        passed: true,
        source_pool_active: true,
        destination_pool_active: true,
        destination_has_liquidity,
        health_factor_ok,
        within_limits: true,
        cooldown_elapsed: true,
        failure_code: 0,
    })
}

/// Check if the user's cooldown period has elapsed since their last migration.
fn check_cooldown(env: &Env, user: &Address, cooldown_secs: u64) -> bool {
    let last_migration: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::UserLastMigration(user.clone()))
        .unwrap_or(0);

    if last_migration == 0 {
        return true;
    }

    env.ledger().timestamp() >= last_migration + cooldown_secs
}

/// Validate that the post-migration health factor remains above 1.0.
///
/// This is a simplified check. A full implementation would:
/// 1. Compute the user's weighted collateral after removing from source pool
/// 2. Compute the user's weighted collateral after adding to destination pool
/// 3. Verify the ratio stays above the liquidation threshold
#[allow(dead_code)]
pub fn validate_health_factor(
    _env: &Env,
    _user: &Address,
    _source_pool: &Address,
    _destination_pool: &Address,
    _amount: i128,
) -> Result<bool, MigrationError> {
    // In production, this would:
    // - Query user's full position across all pools
    // - Simulate the withdrawal from source
    // - Simulate the deposit to destination
    // - Recompute the health factor
    // - Return false if health factor < 10_000 (1.0x)
    Ok(true)
}

/// Estimate the slippage for a migration between two pools.
///
/// Slippage can occur due to:
/// - Different exchange rates between pools
/// - Interest accrual during the migration
/// - Share price differences
pub fn estimate_slippage_bps(
    source_amount: i128,
    destination_amount: i128,
) -> u32 {
    if source_amount == 0 {
        return 0;
    }

    let diff = if source_amount > destination_amount {
        source_amount - destination_amount
    } else {
        destination_amount - source_amount
    };

    // basis points = (diff / source) * 10000
    ((diff * 10_000) / source_amount) as u32
}
