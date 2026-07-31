use soroban_sdk::{contracttype, Address, Env, Map, Vec};

use crate::cross_asset::{
    AssetConfig, AssetKey, AssetPosition, CrossAssetError, UserAssetKey, UserPositionSummary,
    ASSET_CONFIGS, ASSET_LIST, USER_POSITIONS,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchedPositionData {
    pub asset: AssetKey,
    pub config: AssetConfig,
    pub position: AssetPosition,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthBatch {
    pub positions: Vec<BatchedPositionData>,
    pub storage_reads: u32,
    pub price_reads: u32,
}

pub fn batch_read_health_data(env: &Env, user: &Address) -> HealthBatch {
    let asset_list: Vec<AssetKey> = env
        .storage()
        .persistent()
        .get(&ASSET_LIST)
        .unwrap_or(Vec::new(env));
    let configs: Map<AssetKey, AssetConfig> = env
        .storage()
        .persistent()
        .get(&ASSET_CONFIGS)
        .unwrap_or(Map::new(env));
    let user_positions: Map<UserAssetKey, AssetPosition> = env
        .storage()
        .persistent()
        .get(&USER_POSITIONS)
        .unwrap_or(Map::new(env));

    let mut positions = Vec::new(env);
    let mut storage_reads = 3u32;
    let mut price_reads = 0u32;

    for i in 0..asset_list.len() {
        let asset = asset_list.get(i).unwrap();
        if let Some(config) = configs.get(asset.clone()) {
            let position = user_positions
                .get(UserAssetKey::new(user.clone(), asset.clone().to_option()))
                .unwrap_or(AssetPosition {
                    collateral: 0,
                    debt_principal: 0,
                    accrued_interest: 0,
                    last_updated: env.ledger().timestamp(),
                });

            if position.collateral != 0 || position.debt_principal != 0 {
                price_reads += 1;
                positions.push_back(BatchedPositionData {
                    asset,
                    config,
                    position,
                });
            }
        }
    }

    HealthBatch {
        positions,
        storage_reads,
        price_reads,
    }
}

pub fn calculate_health_from_batch(
    env: &Env,
    batch: &HealthBatch,
) -> Result<UserPositionSummary, CrossAssetError> {
    let mut total_collateral_value: i128 = 0;
    let mut weighted_collateral_value: i128 = 0;
    let mut total_debt_value: i128 = 0;
    let mut weighted_debt_value: i128 = 0;

    for i in 0..batch.positions.len() {
        let item = batch.positions.get(i).unwrap();
        let current_time = env.ledger().timestamp();
        if current_time > item.config.price_updated_at
            && current_time - item.config.price_updated_at > 3600
        {
            return Err(CrossAssetError::PriceStale);
        }

        let collateral_value = (item.position.collateral * item.config.price) / 10_000_000;
        total_collateral_value += collateral_value;

        if item.config.can_collateralize {
            weighted_collateral_value +=
                (collateral_value * item.config.liquidation_threshold) / 10_000;
        }

        let total_debt = item.position.debt_principal + item.position.accrued_interest;
        let debt_value = (total_debt * item.config.price) / 10_000_000;
        total_debt_value += debt_value;
        weighted_debt_value += debt_value;
    }

    let health_factor = if weighted_debt_value > 0 {
        (weighted_collateral_value * 10_000) / weighted_debt_value
    } else {
        i128::MAX
    };
    let is_liquidatable = health_factor < 10_000 && weighted_debt_value > 0;
    let borrow_capacity = if weighted_collateral_value > weighted_debt_value {
        weighted_collateral_value - weighted_debt_value
    } else {
        0
    };

    Ok(UserPositionSummary {
        total_collateral_value,
        weighted_collateral_value,
        total_debt_value,
        weighted_debt_value,
        health_factor,
        is_liquidatable,
        borrow_capacity,
    })
}

pub fn get_batched_user_position_summary(
    env: &Env,
    user: &Address,
) -> Result<UserPositionSummary, CrossAssetError> {
    let batch = batch_read_health_data(env, user);
    calculate_health_from_batch(env, &batch)
}
