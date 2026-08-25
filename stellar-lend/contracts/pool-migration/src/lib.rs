#![no_std]

use soroban_sdk::{contract, contractimpl, log, Address, Env, String, Vec};

mod analytics;
mod safety;
mod types;

#[cfg(test)]
mod test;

use crate::types::{
    DataKey, MigrationError, MigrationPreview, MigrationStatus, PoolMigrationConfig,
    PoolMigrationRecord,
};

#[contract]
pub struct PoolMigration;

#[contractimpl]
impl PoolMigration {
    /// Initialize the pool migration contract.
    pub fn initialize(
        env: Env,
        admin: Address,
        lending_contract: Address,
        min_migration_amount: i128,
        max_migration_amount: i128,
        cooldown_secs: u64,
        max_slippage_bps: u32,
        deadline: u64,
        max_batch_size: u32,
    ) -> Result<(), MigrationError> {
        if env.storage().instance().has(&DataKey::Config) {
            return Err(MigrationError::AlreadyInitialized);
        }

        admin.require_auth();

        let config = PoolMigrationConfig {
            admin: admin.clone(),
            lending_contract,
            min_migration_amount,
            max_migration_amount,
            cooldown_secs,
            max_slippage_bps,
            deadline,
            max_batch_size,
        };

        env.storage().instance().set(&DataKey::Config, &config);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::NextMigrationId, &0u64);
        env.storage().instance().set(&DataKey::Paused, &false);

        Ok(())
    }

    /// Migrate assets from one pool to another.
    ///
    /// Gas-optimized: single storage read for config, minimal writes,
    /// reuses computed values across safety checks and execution.
    pub fn migrate(
        env: Env,
        user: Address,
        source_pool: Address,
        destination_pool: Address,
        asset: Address,
        amount: i128,
    ) -> Result<u64, MigrationError> {
        user.require_auth();

        let config: PoolMigrationConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(MigrationError::NotInitialized)?;

        // Run safety checks (single pass, all validations bundled)
        let safety = safety::run_safety_checks(
            &env,
            &user,
            &source_pool,
            &destination_pool,
            amount,
            &config,
        )?;

        if !safety.passed {
            return Err(match safety.failure_code {
                12 => MigrationError::MigrationTooSmall,
                13 => MigrationError::MigrationTooLarge,
                14 => MigrationError::CooldownNotElapsed,
                21 => MigrationError::SafetyCheckFailed,
                _ => MigrationError::SafetyCheckFailed,
            });
        }

        let id = Self::get_next_id(&env);
        let timestamp = env.ledger().timestamp();

        // Execute the migration: withdraw from source, deposit to destination.
        // In production these would be cross-contract calls to the lending contract.
        // Here we model the token transfers directly for gas efficiency.
        let token = soroban_sdk::token::Client::new(&env, &asset);

        // Pull tokens from user (user already authorized)
        token.transfer(&user, &env.current_contract_address(), &amount);

        // Push tokens to destination pool (in production: call lending.deposit)
        token.transfer(&env.current_contract_address(), &destination_pool, &amount);

        let record = PoolMigrationRecord {
            id,
            user: user.clone(),
            source_pool: source_pool.clone(),
            destination_pool: destination_pool.clone(),
            asset,
            amount,
            shares_burned: amount, // Simplified: 1:1 share ratio
            shares_minted: amount,
            status: MigrationStatus::Completed,
            timestamp,
            gas_used: 0, // Would be measured in production
            slippage_bps: 0,
            interest_accrued: 0,
        };

        // Persist record and update cooldown (two persistent writes — minimal)
        env.storage()
            .persistent()
            .set(&DataKey::Migration(id), &record);
        env.storage()
            .persistent()
            .set(&DataKey::UserLastMigration(user), &timestamp);

        // Update analytics (instance storage — cheap)
        analytics::record_migration(&env, &record);

        log!(
            &env,
            "Pool migration {} completed: {} from {} to {}",
            id,
            amount,
            source_pool,
            destination_pool
        );

        Ok(id)
    }

    /// Migrate a percentage of a user's position from source to destination pool.
    ///
    /// Percentage is in basis points (0-10000, where 10000 = 100%).
    pub fn migrate_partial(
        env: Env,
        user: Address,
        source_pool: Address,
        destination_pool: Address,
        asset: Address,
        percentage_bps: u32,
        source_balance: i128,
    ) -> Result<u64, MigrationError> {
        if percentage_bps == 0 || percentage_bps > 10000 {
            return Err(MigrationError::InvalidPercentage);
        }

        let amount = (source_balance * percentage_bps as i128) / 10_000;

        Self::migrate(env, user, source_pool, destination_pool, asset, amount)
    }

    /// Preview a migration before executing it.
    ///
    /// Returns estimated output, slippage, and safety check results
    /// without modifying any state.
    pub fn preview(
        env: Env,
        user: Address,
        source_pool: Address,
        destination_pool: Address,
        _asset: Address,
        amount: i128,
    ) -> Result<MigrationPreview, MigrationError> {
        let config: PoolMigrationConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(MigrationError::NotInitialized)?;

        let safety = safety::run_safety_checks(
            &env,
            &user,
            &source_pool,
            &destination_pool,
            amount,
            &config,
        )?;

        let estimated_slippage_bps = safety::estimate_slippage_bps(amount, amount);
        let interest_impact: i128 = 0; // Would compute from pool rates

        let mut warnings = Vec::new(&env);
        if estimated_slippage_bps > config.max_slippage_bps {
            warnings.push_back(String::from_str(&env, "slippage_exceeds_tolerance"));
        }
        if !safety.cooldown_elapsed {
            warnings.push_back(String::from_str(&env, "cooldown_not_elapsed"));
        }

        Ok(MigrationPreview {
            source_shares_to_burn: amount,
            destination_shares_to_mint: amount,
            estimated_output: amount,
            estimated_slippage_bps,
            interest_impact,
            destination_liquidity: 0, // Would query destination pool
            safety_passed: safety.passed,
            warnings,
        })
    }

    /// Batch migrate multiple users in a single transaction.
    ///
    /// Gas-optimized: reads config once, batches storage writes.
    /// Limited to `max_batch_size` migrations per call.
    pub fn batch_migrate(
        env: Env,
        admin: Address,
        users: Vec<Address>,
        source_pool: Address,
        destination_pool: Address,
        asset: Address,
        amounts: Vec<i128>,
    ) -> Result<Vec<u64>, MigrationError> {
        admin.require_auth();

        let config: PoolMigrationConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(MigrationError::NotInitialized)?;

        if admin != config.admin {
            return Err(MigrationError::Unauthorized);
        }

        if users.len() != amounts.len() {
            return Err(MigrationError::SafetyCheckFailed);
        }

        if users.len() > config.max_batch_size {
            return Err(MigrationError::BatchSizeExceeded);
        }

        let mut results = Vec::new(&env);
        let timestamp = env.ledger().timestamp();

        for i in 0..users.len() {
            let user = users.get(i).unwrap();
            let amount = amounts.get(i).unwrap();

            let safety = safety::run_safety_checks(
                &env,
                &user,
                &source_pool,
                &destination_pool,
                amount,
                &config,
            );

            let id = Self::get_next_id(&env);

            match safety {
                Ok(result) if result.passed => {
                    let record = PoolMigrationRecord {
                        id,
                        user: user.clone(),
                        source_pool: source_pool.clone(),
                        destination_pool: destination_pool.clone(),
                        asset: asset.clone(),
                        amount,
                        shares_burned: amount,
                        shares_minted: amount,
                        status: MigrationStatus::Completed,
                        timestamp,
                        gas_used: 0,
                        slippage_bps: 0,
                        interest_accrued: 0,
                    };

                    env.storage()
                        .persistent()
                        .set(&DataKey::Migration(id), &record);
                    env.storage()
                        .persistent()
                        .set(&DataKey::UserLastMigration(user.clone()), &timestamp);

                    analytics::record_migration(&env, &record);
                    results.push_back(id);
                }
                _ => {
                    // Record failed migration for analytics
                    let record = PoolMigrationRecord {
                        id,
                        user: user.clone(),
                        source_pool: source_pool.clone(),
                        destination_pool: destination_pool.clone(),
                        asset: asset.clone(),
                        amount,
                        shares_burned: 0,
                        shares_minted: 0,
                        status: MigrationStatus::Failed,
                        timestamp,
                        gas_used: 0,
                        slippage_bps: 0,
                        interest_accrued: 0,
                    };

                    env.storage()
                        .persistent()
                        .set(&DataKey::Migration(id), &record);
                    analytics::record_migration(&env, &record);
                    results.push_back(id);
                }
            }
        }

        Ok(results)
    }

    /// Emergency pause/unpause migration operations.
    pub fn set_paused(env: Env, admin: Address, paused: bool) -> Result<(), MigrationError> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(MigrationError::NotInitialized)?;

        if admin != stored_admin {
            return Err(MigrationError::Unauthorized);
        }
        admin.require_auth();

        env.storage().instance().set(&DataKey::Paused, &paused);
        Ok(())
    }

    /// Update migration configuration. Admin only.
    pub fn update_config(
        env: Env,
        admin: Address,
        min_migration_amount: Option<i128>,
        max_migration_amount: Option<i128>,
        cooldown_secs: Option<u64>,
        max_slippage_bps: Option<u32>,
        deadline: Option<u64>,
        max_batch_size: Option<u32>,
    ) -> Result<(), MigrationError> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(MigrationError::NotInitialized)?;

        if admin != stored_admin {
            return Err(MigrationError::Unauthorized);
        }
        admin.require_auth();

        let mut config: PoolMigrationConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(MigrationError::NotInitialized)?;

        if let Some(v) = min_migration_amount {
            config.min_migration_amount = v;
        }
        if let Some(v) = max_migration_amount {
            config.max_migration_amount = v;
        }
        if let Some(v) = cooldown_secs {
            config.cooldown_secs = v;
        }
        if let Some(v) = max_slippage_bps {
            config.max_slippage_bps = v;
        }
        if let Some(v) = deadline {
            config.deadline = v;
        }
        if let Some(v) = max_batch_size {
            config.max_batch_size = v;
        }

        env.storage().instance().set(&DataKey::Config, &config);
        Ok(())
    }

    // --- View functions ---

    pub fn get_migration(env: Env, id: u64) -> Option<PoolMigrationRecord> {
        env.storage().persistent().get(&DataKey::Migration(id))
    }

    pub fn get_analytics(env: Env) -> crate::types::MigrationAnalytics {
        analytics::get_analytics(&env)
    }

    pub fn get_pool_stats(env: Env, pool: Address) -> crate::types::PoolMigrationStats {
        analytics::get_pool_stats(&env, &pool)
    }

    pub fn get_pool_net_flow(env: Env, pool: Address) -> i128 {
        analytics::get_pool_net_flow(&env, &pool)
    }

    pub fn get_config(env: Env) -> Option<PoolMigrationConfig> {
        env.storage().instance().get(&DataKey::Config)
    }

    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    // --- Internal helpers ---

    fn get_next_id(env: &Env) -> u64 {
        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextMigrationId)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::NextMigrationId, &(id + 1));
        id
    }
}
