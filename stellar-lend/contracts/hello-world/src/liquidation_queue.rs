use soroban_sdk::{Address, Env, Vec, contracttype, Map};

use crate::errors::LendingError;
use crate::storage;

/// Maximum queue size to prevent overflow
pub const MAX_QUEUE_SIZE: u32 = 1000;

/// Queue entry expiration time (24 hours)
pub const QUEUE_ENTRY_EXPIRATION: u64 = 86400;

/// Health factor threshold for entering queue (in basis points, 10000 = 1.0)
pub const LIQUIDATION_THRESHOLD_BPS: i128 = 10000;

/// Priority multiplier for severely unhealthy positions
pub const SEVERE_HEALTH_MULTIPLIER: i128 = 2;
pub const CRITICAL_HEALTH_MULTIPLIER: i128 = 3;

/// Health factor thresholds for priority
pub const SEVERE_HEALTH_THRESHOLD_BPS: i128 = 8000; // 0.8
pub const CRITICAL_HEALTH_THRESHOLD_BPS: i128 = 5000; // 0.5

/// Maximum batch size for processing liquidations from queue
pub const MAX_BATCH_SIZE: u32 = 10;

/// Minimum bonus for liquidators clearing from queue (5%)
pub const QUEUE_LIQUIDATOR_BONUS_BPS: i128 = 500;

/// Enhanced bonus for liquidators clearing critical positions (10%)
pub const CRITICAL_LIQUIDATOR_BONUS_BPS: i128 = 1000;

/// Price volatility threshold for reordering (20% = 2000 bps)
pub const VOLATILITY_REORDER_THRESHOLD_BPS: i128 = 2000;

/// Price snapshot window for volatility comparison (1 hour)
pub const PRICE_SNAPSHOT_WINDOW_SECONDS: u64 = 3600;

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum QueueEntryStatus {
    Pending,
    Processing,
    Completed,
    Expired,
    Cancelled,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct LiquidationQueueEntry {
    pub id: u64,
    pub borrower: Address,
    pub liquidator: Address,
    pub health_factor: i128,
    pub priority_score: i128,
    pub queued_at: u64,
    pub expires_at: u64,
    pub status: QueueEntryStatus,
    pub debt_value: i128,
    pub collateral_value: i128,
    /// Price of collateral at queue time (for volatility reordering)
    pub price_at_queue: i128,
    /// Whether this entry was reordered due to volatility
    pub volatility_reordered: bool,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct LiquidatorRegistration {
    pub liquidator: Address,
    pub registered_at: u64,
    pub active: bool,
    /// Total value liquidated by this liquidator
    pub total_liquidated_value: i128,
    /// Total bonus earned by this liquidator
    pub total_bonus_earned: i128,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct QueueConfig {
    pub max_queue_size: u32,
    pub entry_expiration: u64,
    pub fifo_enabled: bool,
    pub priority_enabled: bool,
    /// Whether to reorder queue during price volatility
    pub volatility_reorder_enabled: bool,
    /// Maximum batch size for batch liquidation from queue
    pub max_batch_size: u32,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_queue_size: MAX_QUEUE_SIZE,
            entry_expiration: QUEUE_ENTRY_EXPIRATION,
            fifo_enabled: true,
            priority_enabled: true,
            volatility_reorder_enabled: true,
            max_batch_size: MAX_BATCH_SIZE,
        }
    }
}

/// Price snapshot for volatility comparison
#[derive(Clone, Debug)]
#[contracttype]
pub struct PriceSnapshot {
    pub asset: Address,
    pub price: i128,
    pub timestamp: u64,
}

/// Alert severity for queue monitoring
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Queue monitoring alert
#[derive(Clone, Debug)]
#[contracttype]
pub struct QueueAlert {
    pub alert_id: u64,
    pub severity: AlertSeverity,
    pub message: Symbol,
    pub timestamp: u64,
    pub queue_size: u32,
    pub critical_positions: u32,
}

/// Initialize liquidation queue
pub fn initialize_queue(env: &Env, config: QueueConfig) -> Result<(), LendingError> {
    let config_key = storage::DataKey::LiquidationQueueConfig;
    env.storage().instance().set(&config_key, &config);

    let next_id_key = storage::DataKey::NextLiquidationQueueId;
    env.storage().instance().set(&next_id_key, &0u64);

    let alert_id_key = storage::DataKey::NextQueueAlertId;
    env.storage().instance().set(&alert_id_key, &0u64);

    Ok(())
}

/// Get queue configuration
pub fn get_queue_config(env: &Env) -> QueueConfig {
    let config_key = storage::DataKey::LiquidationQueueConfig;
    env.storage()
        .instance()
        .get(&config_key)
        .unwrap_or_default()
}

/// Register liquidator interest in unhealthy position
pub fn register_liquidation_interest(
    env: &Env,
    liquidator: Address,
    borrower: Address,
    current_collateral_price: i128,
) -> Result<u64, LendingError> {
    liquidator.require_auth();

    // Check if borrower position is unhealthy
    let health_factor = crate::analytics::calculate_health_factor(env, &borrower)
        .map_err(|_| LendingError::InvalidState)?;

    if health_factor >= LIQUIDATION_THRESHOLD_BPS {
        return Err(LendingError::InvalidState); // Position is healthy
    }

    let config = get_queue_config(env);

    // Check queue size
    let queue = get_pending_queue_entries(env);
    if queue.len() >= config.max_queue_size {
        return Err(LendingError::LimitExceeded);
    }

    // Get position values
    let position = crate::analytics::get_user_position_summary(env, &borrower)
        .map_err(|_| LendingError::DataNotFound)?;

    let debt_value = position.debt;
    let collateral_value = position.collateral;

    // Calculate priority score
    let priority_score = calculate_priority_score(health_factor, debt_value);

    let next_id_key = storage::DataKey::NextLiquidationQueueId;
    let entry_id: u64 = env.storage().instance().get(&next_id_key).unwrap_or(0);

    let now = env.ledger().timestamp();
    let expires_at = now + config.entry_expiration;

    let entry = LiquidationQueueEntry {
        id: entry_id,
        borrower: borrower.clone(),
        liquidator: liquidator.clone(),
        health_factor,
        priority_score,
        queued_at: now,
        expires_at,
        status: QueueEntryStatus::Pending,
        debt_value,
        collateral_value,
        price_at_queue: current_collateral_price,
        volatility_reordered: false,
    };

    let entry_key = storage::DataKey::LiquidationQueueEntry(entry_id);
    env.storage().persistent().set(&entry_key, &entry);

    env.storage()
        .instance()
        .set(&next_id_key, &(entry_id + 1));

    // Store price snapshot for volatility tracking
    let snapshot = PriceSnapshot {
        asset: borrower.clone(),
        price: current_collateral_price,
        timestamp: now,
    };
    let snapshot_key = storage::DataKey::LiquidationPriceSnapshot(entry_id);
    env.storage().persistent().set(&snapshot_key, &snapshot);

    // Emit event
    crate::events::LiquidationQueuedEvent {
        entry_id,
        borrower,
        liquidator,
        health_factor,
        priority_score,
        timestamp: now,
    }
    .publish(env);

    // Generate monitoring alert if queue is large
    generate_queue_size_alert(env, &config);

    Ok(entry_id)
}

/// Calculate priority score based on health factor and debt value
fn calculate_priority_score(health_factor: i128, debt_value: i128) -> i128 {
    let base_score = 10000 - health_factor; // Lower health = higher priority

    let multiplier = if health_factor <= CRITICAL_HEALTH_THRESHOLD_BPS {
        CRITICAL_HEALTH_MULTIPLIER
    } else if health_factor <= SEVERE_HEALTH_THRESHOLD_BPS {
        SEVERE_HEALTH_MULTIPLIER
    } else {
        1
    };

    let debt_bonus = (debt_value / 1_000_000).min(1000);

    base_score * multiplier + debt_bonus
}

/// Calculate liquidator bonus based on position severity.
/// More severe (lower health factor) positions yield higher bonuses.
pub fn calculate_liquidator_bonus(health_factor: i128) -> i128 {
    if health_factor <= CRITICAL_HEALTH_THRESHOLD_BPS {
        CRITICAL_LIQUIDATOR_BONUS_BPS
    } else if health_factor <= SEVERE_HEALTH_THRESHOLD_BPS {
        (QUEUE_LIQUIDATOR_BONUS_BPS + CRITICAL_LIQUIDATOR_BONUS_BPS) / 2
    } else {
        QUEUE_LIQUIDATOR_BONUS_BPS
    }
}

/// Reorder queue based on price volatility.
/// When prices move significantly since queue time, affected entries
/// are promoted to the top of the priority order.
pub fn reorder_by_volatility(
    env: &Env,
    current_prices: Map<Address, i128>,
) -> Result<u32, LendingError> {
    let config = get_queue_config(env);
    if !config.volatility_reorder_enabled {
        return Ok(0);
    }

    let mut reordered = 0u32;
    let now = env.ledger().timestamp();
    let next_id_key = storage::DataKey::NextLiquidationQueueId;
    let next_id: u64 = env.storage().instance().get(&next_id_key).unwrap_or(0);

    for id in 0..next_id {
        let entry_key = storage::DataKey::LiquidationQueueEntry(id);
        if let Some(mut entry) = env
            .storage()
            .persistent()
            .get::<storage::DataKey, LiquidationQueueEntry>(&entry_key)
        {
            if entry.status != QueueEntryStatus::Pending {
                continue;
            }
            if entry.volatility_reordered {
                continue; // Already reordered once
            }

            // Check if we have a current price for this borrower's asset
            if let Some(current_price) = current_prices.get(entry.borrower.clone()) {
                let snapshot_key = storage::DataKey::LiquidationPriceSnapshot(id);
                if let Some(snapshot) = env
                    .storage()
                    .persistent()
                    .get::<storage::DataKey, PriceSnapshot>(&snapshot_key)
                {
                    // Only reorder if snapshot is still relevant (within window)
                    if now - snapshot.timestamp <= PRICE_SNAPSHOT_WINDOW_SECONDS && snapshot.price > 0 {
                        let deviation_bps = if current_price > snapshot.price {
                            ((current_price - snapshot.price) * 10000) / snapshot.price
                        } else {
                            ((snapshot.price - current_price) * 10000) / snapshot.price
                        };

                        if deviation_bps > VOLATILITY_REORDER_THRESHOLD_BPS {
                            // Boost priority score significantly for volatile positions
                            let volatility_boost = (deviation_bps / 100) * CRITICAL_HEALTH_MULTIPLIER;
                            entry.priority_score = entry.priority_score
                                .checked_add(volatility_boost)
                                .unwrap_or(i128::MAX);
                            entry.volatility_reordered = true;
                            env.storage().persistent().set(&entry_key, &entry);
                            reordered += 1;

                            // Emit reorder event
                            crate::events::LiquidationReorderedEvent {
                                entry_id: id,
                                borrower: entry.borrower,
                                deviation_bps,
                                new_priority: entry.priority_score,
                                timestamp: now,
                            }
                            .publish(env);
                        }
                    }
                }
            }
        }
    }

    Ok(reordered)
}

/// Get next liquidation from queue (highest priority or FIFO)
pub fn get_next_liquidation(env: &Env) -> Option<LiquidationQueueEntry> {
    let config = get_queue_config(env);
    let mut queue = get_pending_queue_entries(env);

    if queue.is_empty() {
        return None;
    }

    // Remove expired entries
    cleanup_expired_entries(env);

    // Refresh queue after cleanup
    queue = get_pending_queue_entries(env);

    if queue.is_empty() {
        return None;
    }

    if config.priority_enabled {
        let mut best_entry: Option<LiquidationQueueEntry> = None;
        let mut best_score = i128::MIN;

        for entry in queue.iter() {
            if entry.priority_score > best_score {
                best_score = entry.priority_score;
                best_entry = Some(entry);
            }
        }

        best_entry
    } else {
        let mut oldest_entry: Option<LiquidationQueueEntry> = None;
        let mut oldest_time = u64::MAX;

        for entry in queue.iter() {
            if entry.queued_at < oldest_time {
                oldest_time = entry.queued_at;
                oldest_entry = Some(entry);
            }
        }

        oldest_entry
    }
}

/// Process a batch of liquidations from the queue.
/// Processes up to `max_batch_size` entries in one call for gas efficiency.
pub fn process_batch_from_queue(
    env: &Env,
    executor: Address,
    max_count: u32,
) -> Result<(u32, u32, i128), LendingError> {
    executor.require_auth();

    let config = get_queue_config(env);
    let batch_size = max_count.min(config.max_batch_size);

    let mut processed = 0u32;
    let mut failed = 0u32;
    let mut total_reward = 0i128;
    let now = env.ledger().timestamp();

    for _ in 0..batch_size {
        let next = get_next_liquidation(env);
        if next.is_none() {
            break;
        }

        let entry = next.unwrap();
        let entry_key = storage::DataKey::LiquidationQueueEntry(entry.id);
        let mut stored_entry: LiquidationQueueEntry = env
            .storage()
            .persistent()
            .get(&entry_key)
            .ok_or(LendingError::DataNotFound)?;

        if stored_entry.status != QueueEntryStatus::Pending {
            continue;
        }

        if now > stored_entry.expires_at {
            stored_entry.status = QueueEntryStatus::Expired;
            env.storage().persistent().set(&entry_key, &stored_entry);
            failed += 1;
            continue;
        }

        let current_health = crate::analytics::calculate_health_factor(env, &stored_entry.borrower)
            .map_err(|_| LendingError::InvalidState)?;

        if current_health >= LIQUIDATION_THRESHOLD_BPS {
            stored_entry.status = QueueEntryStatus::Cancelled;
            env.storage().persistent().set(&entry_key, &stored_entry);
            failed += 1;
            continue;
        }

        // Update health factor to current state
        stored_entry.health_factor = current_health;
        stored_entry.priority_score = calculate_priority_score(current_health, stored_entry.debt_value);
        stored_entry.status = QueueEntryStatus::Processing;
        env.storage().persistent().set(&entry_key, &stored_entry);

        // Calculate liquidator bonus
        let bonus = calculate_liquidator_bonus(current_health);
        total_reward = total_reward
            .checked_add(bonus)
            .unwrap_or(i128::MAX);

        // Execute liquidation via the liquidate module
        let result = crate::liquidate::liquidate(
            env,
            executor.clone(),
            stored_entry.borrower.clone(),
            None, // debt_asset: use default
            None, // collateral_asset: use default
            stored_entry.debt_value, // liquidate full debt
        );

        match result {
            Ok(_) => {
                stored_entry.status = QueueEntryStatus::Completed;
                env.storage().persistent().set(&entry_key, &stored_entry);
                processed += 1;

                // Update liquidator stats
                update_liquidator_stats(env, &executor, stored_entry.debt_value, bonus);

                crate::events::LiquidationProcessedEvent {
                    entry_id: stored_entry.id,
                    borrower: stored_entry.borrower,
                    liquidator: stored_entry.liquidator,
                    executor: executor.clone(),
                    timestamp: now,
                }
                .publish(env);
            }
            Err(_) => {
                // Reset to pending so another liquidator can try
                stored_entry.status = QueueEntryStatus::Pending;
                env.storage().persistent().set(&entry_key, &stored_entry);
                failed += 1;
            }
        }
    }

    Ok((processed, failed, total_reward))
}

/// Process single liquidation from queue
pub fn process_queue_liquidation(
    env: &Env,
    entry_id: u64,
    executor: Address,
) -> Result<(), LendingError> {
    executor.require_auth();

    let entry_key = storage::DataKey::LiquidationQueueEntry(entry_id);
    let mut entry: LiquidationQueueEntry = env
        .storage()
        .persistent()
        .get(&entry_key)
        .ok_or(LendingError::DataNotFound)?;

    if entry.status != QueueEntryStatus::Pending {
        return Err(LendingError::InvalidState);
    }

    let now = env.ledger().timestamp();
    if now > entry.expires_at {
        entry.status = QueueEntryStatus::Expired;
        env.storage().persistent().set(&entry_key, &entry);
        return Err(LendingError::InvalidState);
    }

    let current_health = crate::analytics::calculate_health_factor(env, &entry.borrower)
        .map_err(|_| LendingError::InvalidState)?;

    if current_health >= LIQUIDATION_THRESHOLD_BPS {
        entry.status = QueueEntryStatus::Cancelled;
        env.storage().persistent().set(&entry_key, &entry);
        return Err(LendingError::InvalidState);
    }

    entry.status = QueueEntryStatus::Processing;
    env.storage().persistent().set(&entry_key, &entry);

    // Execute liquidation
    let result = crate::liquidate::liquidate(
        env,
        executor.clone(),
        entry.borrower.clone(),
        None,
        None,
        entry.debt_value,
    );

    match result {
        Ok(_) => {
            entry.status = QueueEntryStatus::Completed;
            let bonus = calculate_liquidator_bonus(current_health);
            update_liquidator_stats(env, &executor, entry.debt_value, bonus);
        }
        Err(_) => {
            entry.status = QueueEntryStatus::Pending; // Reset for retry
        }
    }

    env.storage().persistent().set(&entry_key, &entry);

    crate::events::LiquidationProcessedEvent {
        entry_id,
        borrower: entry.borrower,
        liquidator: entry.liquidator,
        executor,
        timestamp: now,
    }
    .publish(env);

    Ok(())
}

/// Update liquidator statistics
fn update_liquidator_stats(env: &Env, liquidator: &Address, value: i128, bonus: i128) {
    let key = storage::DataKey::LiquidatorRegistration(liquidator.clone());
    let mut reg = env
        .storage()
        .persistent()
        .get::<storage::DataKey, LiquidatorRegistration>(&key)
        .unwrap_or(LiquidatorRegistration {
            liquidator: liquidator.clone(),
            registered_at: env.ledger().timestamp(),
            active: true,
            total_liquidated_value: 0,
            total_bonus_earned: 0,
        });

    reg.total_liquidated_value = reg.total_liquidated_value
        .checked_add(value)
        .unwrap_or(i128::MAX);
    reg.total_bonus_earned = reg.total_bonus_earned
        .checked_add(bonus)
        .unwrap_or(i128::MAX);
    reg.active = true;

    env.storage().persistent().set(&key, &reg);
}

/// Get liquidator statistics
pub fn get_liquidator_stats(env: &Env, liquidator: &Address) -> Option<LiquidatorRegistration> {
    let key = storage::DataKey::LiquidatorRegistration(liquidator.clone());
    env.storage().persistent().get(&key)
}

/// Cancel queue entry
pub fn cancel_queue_entry(
    env: &Env,
    entry_id: u64,
    caller: Address,
) -> Result<(), LendingError> {
    caller.require_auth();

    let entry_key = storage::DataKey::LiquidationQueueEntry(entry_id);
    let mut entry: LiquidationQueueEntry = env
        .storage()
        .persistent()
        .get(&entry_key)
        .ok_or(LendingError::DataNotFound)?;

    let admin = crate::admin::get_admin(env).ok_or(LendingError::Unauthorized)?;
    if caller != entry.liquidator && caller != admin {
        return Err(LendingError::Unauthorized);
    }

    if entry.status != QueueEntryStatus::Pending {
        return Err(LendingError::InvalidState);
    }

    entry.status = QueueEntryStatus::Cancelled;
    env.storage().persistent().set(&entry_key, &entry);

    crate::events::LiquidationCancelledEvent {
        entry_id,
        caller,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Get all pending queue entries
pub fn get_pending_queue_entries(env: &Env) -> Vec<LiquidationQueueEntry> {
    let next_id_key = storage::DataKey::NextLiquidationQueueId;
    let next_id: u64 = env.storage().instance().get(&next_id_key).unwrap_or(0);

    let mut pending = Vec::new(env);

    for id in 0..next_id {
        let entry_key = storage::DataKey::LiquidationQueueEntry(id);
        if let Some(entry) = env
            .storage()
            .persistent()
            .get::<storage::DataKey, LiquidationQueueEntry>(&entry_key)
        {
            if entry.status == QueueEntryStatus::Pending {
                pending.push_back(entry);
            }
        }
    }

    pending
}

/// Get queue entry by ID
pub fn get_queue_entry(env: &Env, entry_id: u64) -> Option<LiquidationQueueEntry> {
    let entry_key = storage::DataKey::LiquidationQueueEntry(entry_id);
    env.storage().persistent().get(&entry_key)
}

/// Cleanup expired entries
pub fn cleanup_expired_entries(env: &Env) -> u32 {
    let now = env.ledger().timestamp();
    let next_id_key = storage::DataKey::NextLiquidationQueueId;
    let next_id: u64 = env.storage().instance().get(&next_id_key).unwrap_or(0);

    let mut cleaned = 0u32;

    for id in 0..next_id {
        let entry_key = storage::DataKey::LiquidationQueueEntry(id);
        if let Some(mut entry) = env
            .storage()
            .persistent()
            .get::<storage::DataKey, LiquidationQueueEntry>(&entry_key)
        {
            if entry.status == QueueEntryStatus::Pending && now > entry.expires_at {
                entry.status = QueueEntryStatus::Expired;
                env.storage().persistent().set(&entry_key, &entry);
                cleaned += 1;
            }
        }
    }

    cleaned
}

/// Get queue statistics with monitoring data
pub fn get_queue_stats(env: &Env) -> QueueStats {
    let next_id_key = storage::DataKey::NextLiquidationQueueId;
    let next_id: u64 = env.storage().instance().get(&next_id_key).unwrap_or(0);

    let mut pending = 0u32;
    let mut processing = 0u32;
    let mut completed = 0u32;
    let mut expired = 0u32;
    let mut cancelled = 0u32;
    let mut critical_positions = 0u32;
    let mut total_debt_queued = 0i128;

    for id in 0..next_id {
        let entry_key = storage::DataKey::LiquidationQueueEntry(id);
        if let Some(entry) = env
            .storage()
            .persistent()
            .get::<storage::DataKey, LiquidationQueueEntry>(&entry_key)
        {
            match entry.status {
                QueueEntryStatus::Pending => {
                    pending += 1;
                    total_debt_queued = total_debt_queued
                        .checked_add(entry.debt_value)
                        .unwrap_or(i128::MAX);
                    if entry.health_factor <= CRITICAL_HEALTH_THRESHOLD_BPS {
                        critical_positions += 1;
                    }
                }
                QueueEntryStatus::Processing => processing += 1,
                QueueEntryStatus::Completed => completed += 1,
                QueueEntryStatus::Expired => expired += 1,
                QueueEntryStatus::Cancelled => cancelled += 1,
            }
        }
    }

    QueueStats {
        total_entries: next_id,
        pending,
        processing,
        completed,
        expired,
        cancelled,
        critical_positions,
        total_debt_queued,
        queue_health: if pending == 0 { 0 } else { 
            ((pending - critical_positions) as i128 * 10000i128) / pending as i128
        },
    }
}

/// Generate a monitoring alert based on queue conditions
pub fn generate_queue_size_alert(env: &Env, config: &QueueConfig) -> Option<QueueAlert> {
    let stats = get_queue_stats(env);

    let (severity, message) = if stats.critical_positions > 10 {
        (
            AlertSeverity::Critical,
            Symbol::new(env, "queue_critical_backlog"),
        )
    } else if stats.pending > (config.max_queue_size / 2) {
        (
            AlertSeverity::Warning,
            Symbol::new(env, "queue_high_utilization"),
        )
    } else {
        return None; // No alert needed
    };

    let alert_id_key = storage::DataKey::NextQueueAlertId;
    let alert_id: u64 = env.storage().instance().get(&alert_id_key).unwrap_or(0);

    let alert = QueueAlert {
        alert_id,
        severity,
        message,
        timestamp: env.ledger().timestamp(),
        queue_size: stats.pending,
        critical_positions: stats.critical_positions,
    };

    let alert_store_key = storage::DataKey::QueueAlert(alert_id);
    env.storage().persistent().set(&alert_store_key, &alert);
    env.storage().instance().set(&alert_id_key, &(alert_id + 1));

    crate::events::QueueAlertEvent {
        alert_id,
        severity: alert.severity.clone(),
        queue_size: stats.pending,
        critical_positions: stats.critical_positions,
        timestamp: alert.timestamp,
    }
    .publish(env);

    Some(alert)
}

/// Get recent queue alerts
pub fn get_queue_alerts(env: &Env, max_count: u32) -> Vec<QueueAlert> {
    let alert_id_key = storage::DataKey::NextQueueAlertId;
    let next_id: u64 = env.storage().instance().get(&alert_id_key).unwrap_or(0);

    let mut alerts = Vec::new(env);
    let start = if next_id > max_count as u64 { next_id - max_count as u64 } else { 0 };

    for id in start..next_id {
        let alert_key = storage::DataKey::QueueAlert(id);
        if let Some(alert) = env
            .storage()
            .persistent()
            .get::<storage::DataKey, QueueAlert>(&alert_key)
        {
            alerts.push_back(alert);
            if alerts.len() as u32 >= max_count {
                break;
            }
        }
    }

    alerts
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct QueueStats {
    pub total_entries: u64,
    pub pending: u32,
    pub processing: u32,
    pub completed: u32,
    pub expired: u32,
    pub cancelled: u32,
    pub critical_positions: u32,
    pub total_debt_queued: i128,
    pub queue_health: i128,
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_initialize_queue() {
        let env = Env::default();
        let config = QueueConfig::default();

        initialize_queue(&env, config.clone()).unwrap();
        let stored_config = get_queue_config(&env);

        assert_eq!(stored_config.max_queue_size, config.max_queue_size);
    }

    #[test]
    fn test_priority_score_calculation() {
        let score1 = calculate_priority_score(4000, 1_000_000_000);
        let score2 = calculate_priority_score(7000, 1_000_000_000);
        let score3 = calculate_priority_score(9000, 1_000_000_000);

        assert!(score1 > score2);
        assert!(score2 > score3);
    }

    #[test]
    fn test_liquidator_bonus_calculation() {
        let bonus1 = calculate_liquidator_bonus(4000); // Critical
        let bonus2 = calculate_liquidator_bonus(7000); // Severe
        let bonus3 = calculate_liquidator_bonus(9000); // Normal

        assert!(bonus1 > bonus2);
        assert!(bonus2 > bonus3);
    }

    #[test]
    fn test_queue_entry_expiration() {
        let env = Env::default();
        env.mock_all_auths();

        let config = QueueConfig::default();
        initialize_queue(&env, config).unwrap();
    }
}
