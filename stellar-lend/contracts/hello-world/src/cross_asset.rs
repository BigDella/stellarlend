//! # Cross-Asset Lending Module
//!
//! Extends the lending protocol with multi-asset support, allowing users to
//! deposit collateral and borrow across different asset types simultaneously.
//!
//! ## Features
//! - Per-asset configuration: collateral factor, borrow factor, reserve factor, caps
//! - Oracle-based price feeds for cross-asset value calculation
//! - Unified position summary with health factor across all assets
//! - Supply and borrow cap enforcement per asset
//!
//! ## Health Factor
//! Computed as `weighted_collateral_value / weighted_debt_value * 10000`.
//! A health factor below 10,000 (1.0x) makes the position liquidatable.
//!
//! ## Invariants
//! - Withdrawals and borrows are rejected if they would lower health factor below 1.0.
//! - Prices must not be stale (> 1 hour old) for position calculations.

#![allow(dead_code)]
use soroban_sdk::{
    contracterror, contractevent, contracttype, symbol_short, Address, Env, Map, Symbol, Vec,
};

// -------------------------------------------------------------------------
// Events for cap changes and pool state changes
// -------------------------------------------------------------------------

#[contractevent]
#[derive(Clone, Debug)]
pub struct SupplyCapChangedEvent {
    pub asset: Option<Address>,
    pub old_cap: i128,
    pub new_cap: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct BorrowCapChangedEvent {
    pub asset: Option<Address>,
    pub old_cap: i128,
    pub new_cap: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct PoolFrozenEvent {
    pub asset: Option<Address>,
    pub frozen: bool,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetConfig {
    /// Asset contract address (None for native XLM)
    pub asset: Option<Address>,
    /// Collateral factor (LTV) in basis points (e.g., 7500 = 75%)
    /// Maximum percentage of collateral value that can be borrowed
    pub collateral_factor: i128,
    /// Liquidation threshold in basis points (e.g., 8000 = 80%)
    /// Health factor below this triggers liquidation
    pub liquidation_threshold: i128,
    /// Reserve factor in basis points (e.g., 1000 = 10%)
    pub reserve_factor: i128,
    /// Maximum supply cap (0 = unlimited)
    pub max_supply: i128,
    /// Maximum borrow cap / debt ceiling (0 = unlimited)
    pub max_borrow: i128,
    /// Whether asset is enabled for collateral
    pub can_collateralize: bool,
    /// Whether asset is enabled for borrowing
    pub can_borrow: bool,
    /// Asset price in base units (normalized to 7 decimals)
    pub price: i128,
    /// Last price update timestamp
    pub price_updated_at: u64,
    /// Isolated pool: collateral in this pool can only back debt in this pool.
    /// Prevents cross-pool contagion from correlated asset failures.
    pub is_isolated: bool,
    /// Emergency freeze: when true, no new deposits or borrows are accepted.
    pub is_frozen: bool,
}

/// User position across a single asset
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetPosition {
    /// Collateral balance in asset's native units
    pub collateral: i128,
    /// Debt principal in asset's native units
    pub debt_principal: i128,
    /// Accrued interest in asset's native units
    pub accrued_interest: i128,
    /// Last update timestamp
    pub last_updated: u64,
}

/// Unified user position summary across all assets
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserPositionSummary {
    /// Total collateral value in USD (7 decimals)
    pub total_collateral_value: i128,
    /// Total weighted collateral (considering collateral factors)
    pub weighted_collateral_value: i128,
    /// Total debt value in USD (7 decimals)
    pub total_debt_value: i128,
    /// Total weighted debt (considering borrow factors)
    pub weighted_debt_value: i128,
    /// Current health factor (scaled by 10000, e.g., 15000 = 1.5)
    pub health_factor: i128,
    /// Whether position can be liquidated
    pub is_liquidatable: bool,
    /// Maximum additional borrow capacity in USD
    pub borrow_capacity: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssetKey {
    Native,
    Token(Address),
}

/// Errors that can occur during cross-asset lending operations.
#[contracterror]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CrossAssetError {
    /// The specified asset has no configuration registered
    AssetNotConfigured = 1,
    /// The asset is configured but disabled for the requested operation
    AssetDisabled = 2,
    /// Insufficient collateral for the requested withdrawal or borrow
    InsufficientCollateral = 3,
    /// Borrow would exceed the user's remaining borrow capacity
    ExceedsBorrowCapacity = 4,
    /// Operation would result in a health factor below 1.0
    UnhealthyPosition = 5,
    /// Deposit would exceed the asset's supply cap
    SupplyCapExceeded = 6,
    /// Borrow would exceed the asset's borrow cap
    BorrowCapExceeded = 7,
    /// Price is zero or negative
    InvalidPrice = 8,
    /// Asset price is older than the staleness threshold (1 hour)
    PriceStale = 9,
    /// Caller is not authorized (not admin)
    NotAuthorized = 10,
}

/// Admin address authorized for protocol management
const ADMIN: Symbol = symbol_short!("admin");

/// Storage key for the map of asset configurations: Map<AssetKey, AssetConfig>
pub(crate) const ASSET_CONFIGS: Symbol = symbol_short!("configs");

/// Storage key for the map of user positions: Map<UserAssetKey, AssetPosition>
pub(crate) const USER_POSITIONS: Symbol = symbol_short!("positions");

/// Storage key for the map of total supplies per asset: Map<AssetKey, i128>
const TOTAL_SUPPLIES: Symbol = symbol_short!("supplies");

/// Storage key for the map of total borrows per asset: Map<AssetKey, i128>
const TOTAL_BORROWS: Symbol = symbol_short!("borrows");

/// Storage key for the global list of registered assets: Vec<AssetKey>
pub(crate) const ASSET_LIST: Symbol = symbol_short!("assets");

/// Initialize the cross-asset lending module.
///
/// Sets the admin address. Can only be called once; subsequent calls return
/// `NotAuthorized`.
///
/// # Arguments
/// * `admin` - The admin address (must authorize the transaction)
pub fn initialize(env: &Env, admin: Address) -> Result<(), CrossAssetError> {
    if env.storage().persistent().has(&ADMIN) {
        return Err(CrossAssetError::NotAuthorized);
    }

    admin.require_auth();

    env.storage().persistent().set(&ADMIN, &admin);

    Ok(())
}

fn require_admin(env: &Env) -> Result<(), CrossAssetError> {
    let admin: Address = env
        .storage()
        .persistent()
        .get(&ADMIN)
        .ok_or(CrossAssetError::NotAuthorized)?;

    admin.require_auth();

    Ok(())
}

/// Register a new asset with the cross-asset lending module.
///
/// Validates the configuration (factors in basis-point range, positive price)
/// and appends the asset to the global asset list if not already present.
///
/// # Arguments
/// * `env` - The contract environment
/// * `asset` - Asset to configure (`None` for native XLM)
/// * `config` - Full asset configuration (factors, caps, price)
///
/// # Errors
/// * `NotAuthorized` - Caller is not the admin
/// * `AssetNotConfigured` - A basis-point field is out of [0, 10000]
/// * `InvalidPrice` - Price is zero or negative
pub fn initialize_asset(
    env: &Env,
    asset: Option<Address>,
    config: AssetConfig,
) -> Result<(), CrossAssetError> {
    require_admin(env)?;

    require_valid_config(&config)?;

    let asset_key = AssetKey::from_option(asset.clone());
    let mut configs: Map<AssetKey, AssetConfig> = env
        .storage()
        .persistent()
        .get(&ASSET_CONFIGS)
        .unwrap_or(Map::new(env));

    configs.set(asset_key.clone(), config);
    env.storage().persistent().set(&ASSET_CONFIGS, &configs);

    let mut asset_list: Vec<AssetKey> = env
        .storage()
        .persistent()
        .get(&ASSET_LIST)
        .unwrap_or(Vec::new(env));

    if !asset_list.contains(&asset_key) {
        asset_list.push_back(asset_key);
        env.storage().persistent().set(&ASSET_LIST, &asset_list);
    }

    Ok(())
}

/// Selectively update an existing asset's configuration.
///
/// Only the provided `Some` fields are updated; `None` fields keep their
/// current values. Factor fields are validated to be in [0, 10000] bps.
///
/// # Arguments
/// * `env` - The contract environment
/// * `asset` - Asset to update (`None` for XLM)
/// * `collateral_factor` - Optional new collateral factor/LTV (basis points)
/// * `liquidation_threshold` - Optional new liquidation threshold (basis points)
/// * `max_supply` - Optional new supply cap
/// * `max_borrow` - Optional new borrow cap/debt ceiling
/// * `can_collateralize` - Optional flag to enable/disable as collateral
/// * `can_borrow` - Optional flag to enable/disable borrowing
///
/// # Errors
/// * `NotAuthorized` - Caller is not the admin
/// * `AssetNotConfigured` - Asset has not been initialized or factor out of range
#[allow(clippy::too_many_arguments)]
pub fn update_asset_config(
    env: &Env,
    asset: Option<Address>,
    collateral_factor: Option<i128>,
    liquidation_threshold: Option<i128>,
    max_supply: Option<i128>,
    max_borrow: Option<i128>,
    can_collateralize: Option<bool>,
    can_borrow: Option<bool>,
) -> Result<(), CrossAssetError> {
    require_admin(env)?;

    let asset_key = AssetKey::from_option(asset.clone());
    let mut config = get_asset_config(env, &asset_key)?;

    // Snapshot old caps for event emission.
    let old_supply_cap = config.max_supply;
    let old_borrow_cap = config.max_borrow;

    if let Some(cf) = collateral_factor {
        require_valid_basis_points(cf)?;
        config.collateral_factor = cf;
    }

    if let Some(lt) = liquidation_threshold {
        require_valid_basis_points(lt)?;
        config.liquidation_threshold = lt;
    }

    if let Some(ms) = max_supply {
        config.max_supply = ms;
    }

    if let Some(mb) = max_borrow {
        config.max_borrow = mb;
    }

    if let Some(cc) = can_collateralize {
        config.can_collateralize = cc;
    }

    if let Some(cb) = can_borrow {
        config.can_borrow = cb;
    }

    // Update storage
    let mut configs: Map<AssetKey, AssetConfig> = env
        .storage()
        .persistent()
        .get(&ASSET_CONFIGS)
        .unwrap_or(Map::new(env));

    configs.set(asset_key, config.clone());
    env.storage().persistent().set(&ASSET_CONFIGS, &configs);

    let ts = env.ledger().timestamp();

    // Emit supply-cap-changed event when the cap changed.
    if config.max_supply != old_supply_cap {
        SupplyCapChangedEvent {
            asset: asset.clone(),
            old_cap: old_supply_cap,
            new_cap: config.max_supply,
            timestamp: ts,
        }
        .publish(env);
    }

    // Emit borrow-cap-changed event when the cap changed.
    if config.max_borrow != old_borrow_cap {
        BorrowCapChangedEvent {
            asset: asset.clone(),
            old_cap: old_borrow_cap,
            new_cap: config.max_borrow,
            timestamp: ts,
        }
        .publish(env);
    }

    Ok(())
}

/// Update the oracle price for an asset.
///
/// Records the new price and the current ledger timestamp for staleness checks.
///
/// # Arguments
/// * `env` - The contract environment
/// * `asset` - Asset to update price for (`None` for XLM)
/// * `price` - New price in base units (7 decimals, must be > 0)
///
/// # Errors
/// * `NotAuthorized` - Caller is not the admin
/// * `InvalidPrice` - Price is zero or negative
/// * `AssetNotConfigured` - Asset has not been initialized
pub fn update_asset_price(
    env: &Env,
    asset: Option<Address>,
    price: i128,
) -> Result<(), CrossAssetError> {
    require_admin(env)?;

    if price <= 0 {
        return Err(CrossAssetError::InvalidPrice);
    }

    let asset_key = AssetKey::from_option(asset);
    let mut config = get_asset_config(env, &asset_key)?;
    config.price = price;
    config.price_updated_at = env.ledger().timestamp();

    let mut configs: Map<AssetKey, AssetConfig> = env
        .storage()
        .persistent()
        .get(&ASSET_CONFIGS)
        .unwrap_or(Map::new(env));

    configs.set(asset_key, config);
    env.storage().persistent().set(&ASSET_CONFIGS, &configs);

    Ok(())
}

/// Get user's position for a specific asset
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - User address
/// * `asset` - Asset address (None for XLM)
///
/// # Returns
/// Asset position or default empty position
pub fn get_user_asset_position(env: &Env, user: &Address, asset: Option<Address>) -> AssetPosition {
    let key = UserAssetKey::new(user.clone(), asset);
    let positions: Map<UserAssetKey, AssetPosition> = env
        .storage()
        .persistent()
        .get(&USER_POSITIONS)
        .unwrap_or(Map::new(env));

    positions.get(key).unwrap_or(AssetPosition {
        collateral: 0,
        debt_principal: 0,
        accrued_interest: 0,
        last_updated: env.ledger().timestamp(),
    })
}

/// Update user's position for a specific asset
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - User address
/// * `asset` - Asset address (None for XLM)
/// * `position` - Updated position data
fn set_user_asset_position(
    env: &Env,
    user: &Address,
    asset: Option<Address>,
    position: AssetPosition,
) {
    let key = UserAssetKey::new(user.clone(), asset);
    let mut positions: Map<UserAssetKey, AssetPosition> = env
        .storage()
        .persistent()
        .get(&USER_POSITIONS)
        .unwrap_or(Map::new(env));

    positions.set(key, position);
    env.storage().persistent().set(&USER_POSITIONS, &positions);
}

/// Calculate a unified position summary across all registered assets.
///
/// Iterates over all configured assets, aggregates collateral and debt values
/// weighted by their respective factors, and computes the health factor.
/// Prices older than 1 hour are rejected.
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - User address
///
/// # Returns
/// [`UserPositionSummary`] with health factor, liquidation status, and borrow capacity.
///
/// # Errors
/// * `PriceStale` - Any asset with a non-zero position has a price older than 1 hour
pub fn get_user_position_summary(
    env: &Env,
    user: &Address,
) -> Result<UserPositionSummary, CrossAssetError> {
    crate::health::get_batched_user_position_summary(env, user)
}

/// Deposit collateral for a specific asset.
///
/// Requires user authorization. Validates the asset is enabled for collateral
/// and that the deposit does not exceed the supply cap.
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - User depositing collateral (must authorize)
/// * `asset` - Asset to deposit (`None` for XLM)
/// * `amount` - Amount to deposit
///
/// # Returns
/// Updated [`AssetPosition`] after the deposit.
///
/// # Errors
/// * `AssetNotConfigured` - Asset is not registered
/// * `AssetDisabled` - Asset is not enabled for collateral
/// * `SupplyCapExceeded` - Deposit would exceed the asset's supply cap
pub fn cross_asset_deposit(
    env: &Env,
    user: Address,
    asset: Option<Address>,
    amount: i128,
) -> Result<AssetPosition, CrossAssetError> {
    user.require_auth();

    let asset_key = AssetKey::from_option(asset.clone());
    let config = get_asset_config(env, &asset_key)?;

    // Reject deposits into a frozen pool.
    if config.is_frozen {
        return Err(CrossAssetError::AssetDisabled);
    }

    if !config.can_collateralize {
        return Err(CrossAssetError::AssetDisabled);
    }

    // Supply cap enforcement (considers only raw supply; accrued interest checked separately).
    if config.max_supply > 0 {
        let total_supply = get_total_supply(env, &asset_key);
        if total_supply + amount > config.max_supply {
            return Err(CrossAssetError::SupplyCapExceeded);
        }
    }

    // Per-user supply limit check
    check_per_user_supply_limit(env, &user, &asset_key, amount)?;

    let mut position = get_user_asset_position(env, &user, asset.clone());

    position.collateral += amount;
    position.last_updated = env.ledger().timestamp();

    set_user_asset_position(env, &user, asset, position.clone());
    update_total_supply(env, &asset_key, amount);
    update_per_user_supply(env, &user, &asset_key, amount);

    Ok(position)
}

/// Borrow a specific asset against cross-asset collateral.
///
/// Requires user authorization. Validates the asset is enabled for borrowing,
/// checks the borrow cap, and verifies the post-borrow health factor stays
/// above 1.0. If the health check fails, the borrow is rolled back.
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - User borrowing (must authorize)
/// * `asset` - Asset to borrow (`None` for XLM)
/// * `amount` - Amount to borrow
///
/// # Returns
/// Updated [`AssetPosition`] after the borrow.
///
/// # Errors
/// * `AssetNotConfigured` - Asset is not registered
/// * `AssetDisabled` - Asset is not enabled for borrowing
/// * `BorrowCapExceeded` - Borrow would exceed the asset's borrow cap
/// * `ExceedsBorrowCapacity` - Health factor would drop below 1.0
/// * `PriceStale` - Stale price prevents health factor calculation
pub fn cross_asset_borrow(
    env: &Env,
    user: Address,
    asset: Option<Address>,
    amount: i128,
) -> Result<AssetPosition, CrossAssetError> {
    user.require_auth();

    let asset_key = AssetKey::from_option(asset.clone());
    let config = get_asset_config(env, &asset_key)?;

    // Reject borrows from a frozen pool.
    if config.is_frozen {
        return Err(CrossAssetError::AssetDisabled);
    }

    if !config.can_borrow {
        return Err(CrossAssetError::AssetDisabled);
    }

    // Borrow-cap enforcement with dynamic liquidity-based adjustment.
    let effective_borrow_cap = calculate_dynamic_borrow_cap(env, asset.clone())?;
    if effective_borrow_cap > 0 {
        let total_borrow = get_total_borrow(env, &asset_key);
        if total_borrow + amount > effective_borrow_cap {
            return Err(CrossAssetError::BorrowCapExceeded);
        }
    }

    let mut position = get_user_asset_position(env, &user, asset.clone());

    position.debt_principal += amount;
    position.last_updated = env.ledger().timestamp();

    set_user_asset_position(env, &user, asset.clone(), position.clone());

    if config.is_isolated {
        // Isolated pool: only collateral deposited in THIS pool may back its debt.
        let pool_collateral = position.collateral;
        let pool_debt = position.debt_principal + position.accrued_interest;
        let max_pool_debt = pool_collateral
            .checked_mul(config.collateral_factor)
            .unwrap_or(0)
            .checked_div(10_000)
            .unwrap_or(0);

        if pool_debt > max_pool_debt {
            position.debt_principal -= amount;
            set_user_asset_position(env, &user, asset, position);
            return Err(CrossAssetError::ExceedsBorrowCapacity);
        }
    } else {
        // Non-isolated: use cross-pool health factor as before.
        let summary = get_user_position_summary(env, &user)?;
        if summary.health_factor < 10_000 {
            position.debt_principal -= amount;
            set_user_asset_position(env, &user, asset, position);
            return Err(CrossAssetError::ExceedsBorrowCapacity);
        }
    }

    update_total_borrow(env, &asset_key, amount);

    Ok(position)
}

/// Withdraw collateral for a specific asset.
///
/// Requires user authorization. Checks that the user has sufficient collateral
/// and that the withdrawal does not bring the health factor below 1.0. If the
/// health check fails, the withdrawal is rolled back.
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - User withdrawing collateral (must authorize)
/// * `asset` - Asset to withdraw (`None` for XLM)
/// * `amount` - Amount to withdraw
///
/// # Returns
/// Updated [`AssetPosition`] after the withdrawal.
///
/// # Errors
/// * `InsufficientCollateral` - User's collateral balance is below `amount`
/// * `UnhealthyPosition` - Withdrawal would drop health factor below 1.0
/// * `PriceStale` - Stale price prevents health factor calculation
pub fn cross_asset_withdraw(
    env: &Env,
    user: Address,
    asset: Option<Address>,
    amount: i128,
) -> Result<AssetPosition, CrossAssetError> {
    user.require_auth();

    let asset_key = AssetKey::from_option(asset.clone());

    let mut position = get_user_asset_position(env, &user, asset.clone());

    if position.collateral < amount {
        return Err(CrossAssetError::InsufficientCollateral);
    }

    position.collateral -= amount;
    position.last_updated = env.ledger().timestamp();

    set_user_asset_position(env, &user, asset.clone(), position.clone());

    let summary = get_user_position_summary(env, &user)?;

    if summary.total_debt_value > 0 && summary.health_factor < 10_000 {
        position.collateral += amount;
        set_user_asset_position(env, &user, asset, position);
        return Err(CrossAssetError::UnhealthyPosition);
    }

    update_total_supply(env, &asset_key, -amount);

    Ok(position)
}

/// Liquidate an unhealthy cross-asset position.
///
/// Liquidators can repay debt in exchange for collateral at a discount.
/// This function handles multi-asset liquidation where the liquidator can choose
/// which collateral asset to receive.
///
/// # Arguments
/// * `env` - The contract environment
/// * `liquidator` - Address performing the liquidation
/// * `user` - Address of the user being liquidated
/// * `debt_asset` - Asset to repay (None for XLM)
/// * `collateral_asset` - Asset to receive as collateral (None for XLM)
/// * `debt_to_repay` - Amount of debt to repay
/// * `collateral_to_receive` - Expected amount of collateral to receive
///
/// # Returns
/// Amount of collateral actually transferred to liquidator.
///
/// # Errors
/// * `AssetNotConfigured` - Either asset is not registered
/// * `AssetDisabled` - Assets are disabled for liquidation
/// * `InsufficientCollateral` - User position is not liquidatable
/// * `InvalidPrice` - Price data is invalid
/// * `PriceStale` - Price data is stale
/// * `NotAuthorized` - Liquidator is not authorized
pub fn cross_asset_liquidate(
    env: &Env,
    liquidator: Address,
    user: Address,
    debt_asset: Option<Address>,
    collateral_asset: Option<Address>,
    debt_to_repay: i128,
    collateral_to_receive: i128,
) -> Result<i128, CrossAssetError> {
    liquidator.require_auth();

    // Get asset configurations
    let debt_asset_key = AssetKey::from_option(debt_asset.clone());
    let collateral_asset_key = AssetKey::from_option(collateral_asset.clone());
    
    let debt_config = get_asset_config(env, &debt_asset_key)?;
    let collateral_config = get_asset_config(env, &collateral_asset_key)?;

    // Check if position is liquidatable
    let position_summary = get_user_position_summary(env, &user)?;
    if !position_summary.is_liquidatable {
        return Err(CrossAssetError::InsufficientCollateral);
    }

    // Get user positions for both assets
    let mut debt_position = get_user_asset_position(env, &user, debt_asset.clone());
    let mut collateral_position = get_user_asset_position(env, &user, collateral_asset.clone());

    // Validate liquidation amounts
    if debt_to_repay <= 0 || collateral_to_receive <= 0 {
        return Err(CrossAssetError::InsufficientCollateral);
    }

    // Calculate actual collateral to receive with liquidation incentive
    let liquidation_incentive = collateral_config.liquidation_threshold - collateral_config.collateral_factor;
    let actual_collateral = (collateral_to_receive * (10_000 - liquidation_incentive)) / 10_000;

    // Ensure user has enough collateral
    if collateral_position.collateral < actual_collateral {
        return Err(CrossAssetError::InsufficientCollateral);
    }

    // Ensure user has enough debt to repay
    let total_debt = debt_position.debt_principal + debt_position.accrued_interest;
    if debt_to_repay > total_debt {
        return Err(CrossAssetError::InsufficientCollateral);
    }

    // Update positions
    debt_position.debt_principal -= debt_to_repay;
    debt_position.last_updated = env.ledger().timestamp();
    
    collateral_position.collateral -= actual_collateral;
    collateral_position.last_updated = env.ledger().timestamp();

    // Store updated positions
    set_user_asset_position(env, &user, debt_asset, debt_position);
    set_user_asset_position(env, &user, collateral_asset, collateral_position);

    // Update total supplies
    update_total_borrow(env, &debt_asset_key, -debt_to_repay);
    update_total_supply(env, &collateral_asset_key, -actual_collateral);

    Ok(actual_collateral)
}

/// Repay debt for a specific asset.
///
/// Requires user authorization. Repayment is capped at the total outstanding
/// debt (principal + accrued interest). Interest is paid first, then principal.
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - User repaying debt (must authorize)
/// * `asset` - Asset to repay (`None` for XLM)
/// * `amount` - Amount to repay (capped at total debt)
///
/// # Returns
/// Updated [`AssetPosition`] after the repayment.
pub fn cross_asset_repay(
    env: &Env,
    user: Address,
    asset: Option<Address>,
    amount: i128,
) -> Result<AssetPosition, CrossAssetError> {
    user.require_auth();

    let asset_key = AssetKey::from_option(asset.clone());

    // Get current position
    let mut position = get_user_asset_position(env, &user, asset.clone());

    let total_debt = position.debt_principal + position.accrued_interest;
    let repay_amount = amount.min(total_debt);

    // Pay interest first, then principal
    if repay_amount <= position.accrued_interest {
        position.accrued_interest -= repay_amount;
    } else {
        let remaining = repay_amount - position.accrued_interest;
        position.accrued_interest = 0;
        position.debt_principal -= remaining;
    }

    position.last_updated = env.ledger().timestamp();

    // Update storage
    set_user_asset_position(env, &user, asset, position.clone());
    update_total_borrow(env, &asset_key, -repay_amount);

    Ok(position)
}

/// Return the list of all registered asset keys.
///
/// Returns an empty vector if no assets have been configured.
pub fn get_asset_list(env: &Env) -> Vec<AssetKey> {
    env.storage()
        .persistent()
        .get(&ASSET_LIST)
        .unwrap_or(Vec::new(env))
}

/// Look up the configuration for a specific asset by address.
///
/// # Arguments
/// * `env` - The contract environment
/// * `asset` - Asset address (`None` for native XLM)
///
/// # Returns
/// The [`AssetConfig`] for the requested asset.
///
/// # Errors
/// * `AssetNotConfigured` - No configuration exists for this asset
pub fn get_asset_config_by_address(
    env: &Env,
    asset: Option<Address>,
) -> Result<AssetConfig, CrossAssetError> {
    let asset_key = AssetKey::from_option(asset);
    get_asset_config(env, &asset_key)
}

// -------------------------------------------------------------------------
// Analytics endpoints
// -------------------------------------------------------------------------

/// Return the available supply headroom for an asset.
///
/// Returns `(available, cap, current_supply)`:
/// - `available`: how much more can be deposited (0 if at/over cap, or cap=0 for unlimited).
/// - `cap`: the configured supply cap (0 means unlimited).
/// - `current_supply`: total supply currently deposited.
pub fn get_supply_headroom(
    env: &Env,
    asset: Option<Address>,
) -> Result<(i128, i128, i128), CrossAssetError> {
    let asset_key = AssetKey::from_option(asset);
    let config = get_asset_config(env, &asset_key)?;
    let current_supply = get_total_supply(env, &asset_key);

    if config.max_supply == 0 {
        return Ok((i128::MAX, 0, current_supply));
    }

    let available = (config.max_supply - current_supply).max(0);
    Ok((available, config.max_supply, current_supply))
}

/// Return borrow utilization for an asset.
///
/// Returns `(current_borrows, cap)`:
/// - `current_borrows`: total amount currently borrowed.
/// - `cap`: the configured borrow cap (0 means unlimited).
pub fn get_borrow_utilization(
    env: &Env,
    asset: Option<Address>,
) -> Result<(i128, i128), CrossAssetError> {
    let asset_key = AssetKey::from_option(asset);
    let config = get_asset_config(env, &asset_key)?;
    let current_borrows = get_total_borrow(env, &asset_key);
    Ok((current_borrows, config.max_borrow))
}

/// Per-user supply cap storage key suffix
const PER_USER_SUPPLY_KEY: Symbol = symbol_short!("per_user");

/// Dynamic cap adjustment based on utilization.
///
/// Calculates a suggested supply cap based on the current utilization rate.
/// When utilization is above the target threshold, caps are tightened;
/// when below, caps are relaxed.
pub fn calculate_dynamic_supply_cap(
    env: &Env,
    asset: Option<Address>,
) -> Result<i128, CrossAssetError> {
    let asset_key = AssetKey::from_option(asset);
    let config = get_asset_config(env, &asset_key)?;
    let current_supply = get_total_supply(env, &asset_key);
    let current_borrow = get_total_borrow(env, &asset_key);

    // Target utilization: 75% (7500 bps)
    let target_util_bps: i128 = 7500;

    // If no supply or borrow, use configured cap directly
    if current_supply == 0 || current_borrow == 0 {
        return Ok(config.max_supply);
    }

    // Current utilization in basis points
    let current_util_bps = (current_borrow * 10_000) / current_supply;

    // If utilization exceeds target, suggest a tighter cap
    if current_util_bps > target_util_bps {
        let overage_bps = current_util_bps - target_util_bps;
        // Reduce effective cap proportionally to overage (max 50% reduction)
        let reduction_bps = (overage_bps * 5000) / 10000;
        let reduction = (config.max_supply * reduction_bps) / 10_000;
        let dynamic_cap = (config.max_supply - reduction).max(current_supply);
        Ok(dynamic_cap)
    } else {
        // Below target: allow full cap
        Ok(config.max_supply)
    }
}

/// Calculate dynamic borrow cap based on pool liquidity.
///
/// The borrow cap is a function of available liquidity and utilization.
pub fn calculate_dynamic_borrow_cap(
    env: &Env,
    asset: Option<Address>,
) -> Result<i128, CrossAssetError> {
    let asset_key = AssetKey::from_option(asset);
    let config = get_asset_config(env, &asset_key)?;
    let current_supply = get_total_supply(env, &asset_key);
    let current_borrow = get_total_borrow(env, &asset_key);

    if config.max_borrow == 0 {
        return Ok(0); // unlimited
    }

    // Available liquidity = total_supply - total_borrow
    let available_liquidity = current_supply - current_borrow;
    if available_liquidity <= 0 {
        // Pool is fully utilized; restrict new borrows
        return Ok(current_borrow);
    }

    // Dynamic cap = base_cap * (available_liquidity / total_supply) adjustment
    // Prevents borrowing more than a fraction of available liquidity
    let liquidity_ratio_bps = (available_liquidity * 10_000) / current_supply.max(1);
    let adjusted_cap = current_borrow + (available_liquidity * liquidity_ratio_bps) / 10_000;
    Ok(adjusted_cap.min(config.max_borrow))
}

/// Check and enforce per-user supply limit.
///
/// When a per-user max is configured, this validates that the user's deposit
/// does not exceed their individual cap.
pub fn check_per_user_supply_limit(
    env: &Env,
    user: &Address,
    asset: &AssetKey,
    amount: i128,
) -> Result<(), CrossAssetError> {
    let user_supply_key = (PER_USER_SUPPLY_KEY, user.clone(), asset.clone());
    let user_supply: i128 = env.storage().persistent().get(&user_supply_key).unwrap_or(0);

    // Per-user cap is 20% of global supply cap by default
    let config = get_asset_config(env, asset)?;
    let per_user_cap = if config.max_supply > 0 {
        (config.max_supply * 2000) / 10_000 // 20% of global cap
    } else {
        i128::MAX // No global cap => no per-user cap
    };

    if user_supply + amount > per_user_cap {
        return Err(CrossAssetError::SupplyCapExceeded);
    }
    Ok(())
}

/// Update per-user supply tracking.
pub fn update_per_user_supply(
    env: &Env,
    user: &Address,
    asset: &AssetKey,
    delta: i128,
) {
    let user_supply_key = (PER_USER_SUPPLY_KEY, user.clone(), asset.clone());
    let current: i128 = env.storage().persistent().get(&user_supply_key).unwrap_or(0);
    env.storage().persistent().set(&user_supply_key, &(current + delta));
}

// -------------------------------------------------------------------------
// Pool Registry
// -------------------------------------------------------------------------

/// Pool metadata for the registry
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolInfo {
    pub asset: Option<Address>,
    pub is_isolated: bool,
    pub is_frozen: bool,
    pub total_supply: i128,
    pub total_borrow: i128,
    pub supply_cap: i128,
    pub borrow_cap: i128,
    pub collateral_factor: i128,
    pub liquidation_threshold: i128,
    pub utilization_bps: i128,
}

/// Get a summary of all registered pools with their current status.
pub fn get_pool_registry(env: &Env) -> Vec<PoolInfo> {
    let asset_list = get_asset_list(env);
    let mut registry = Vec::new(env);

    for asset_key in asset_list.iter() {
        if let Ok(config) = get_asset_config(env, &asset_key) {
            let asset_option = asset_key.to_option();
            let total_supply = get_total_supply(env, &asset_key);
            let total_borrow = get_total_borrow(env, &asset_key);
            let utilization_bps = if total_supply > 0 {
                (total_borrow * 10_000) / total_supply
            } else {
                0
            };

            registry.push_back(PoolInfo {
                asset: asset_option,
                is_isolated: config.is_isolated,
                is_frozen: config.is_frozen,
                total_supply,
                total_borrow,
                supply_cap: config.max_supply,
                borrow_cap: config.max_borrow,
                collateral_factor: config.collateral_factor,
                liquidation_threshold: config.liquidation_threshold,
                utilization_bps,
            });
        }
    }

    registry
}

/// Get detailed pool info for a single asset.
pub fn get_pool_info(env: &Env, asset: Option<Address>) -> Result<PoolInfo, CrossAssetError> {
    let asset_key = AssetKey::from_option(asset.clone());
    let config = get_asset_config(env, &asset_key)?;
    let total_supply = get_total_supply(env, &asset_key);
    let total_borrow = get_total_borrow(env, &asset_key);
    let utilization_bps = if total_supply > 0 {
        (total_borrow * 10_000) / total_supply
    } else {
        0
    };

    Ok(PoolInfo {
        asset,
        is_isolated: config.is_isolated,
        is_frozen: config.is_frozen,
        total_supply,
        total_borrow,
        supply_cap: config.max_supply,
        borrow_cap: config.max_borrow,
        collateral_factor: config.collateral_factor,
        liquidation_threshold: config.liquidation_threshold,
        utilization_bps,
    })
}

/// Create a new isolated pool with default risk parameters.
///
/// This is a convenience factory that sets up a pool with isolated=true
/// and sensible defaults for the risk parameters.
pub fn create_isolated_pool(
    env: &Env,
    admin: Address,
    asset: Option<Address>,
    collateral_factor: i128,
    liquidation_threshold: i128,
    supply_cap: i128,
    borrow_cap: i128,
) -> Result<(), CrossAssetError> {
    require_admin(env)?;

    let config = AssetConfig {
        asset: asset.clone(),
        collateral_factor,
        liquidation_threshold,
        reserve_factor: 1000, // 10% default
        max_supply: supply_cap,
        max_borrow: borrow_cap,
        can_collateralize: true,
        can_borrow: true,
        price: 1_0000000,
        price_updated_at: env.ledger().timestamp(),
        is_isolated: true,
        is_frozen: false,
    };

    require_valid_config(&config)?;
    initialize_asset(env, asset, config)
}

// -------------------------------------------------------------------------
// Emergency pool management
// -------------------------------------------------------------------------

/// Freeze or unfreeze a pool, preventing new deposits and borrows.
///
/// # Arguments
/// * `env` - The contract environment
/// * `admin` - Admin address (must authorize)
/// * `asset` - Asset to freeze/unfreeze (`None` for XLM)
/// * `freeze` - `true` to freeze, `false` to unfreeze
///
/// # Errors
/// * `NotAuthorized` - Caller is not the admin
/// * `AssetNotConfigured` - Asset has not been initialized
pub fn freeze_pool(
    env: &Env,
    admin: Address,
    asset: Option<Address>,
    freeze: bool,
) -> Result<(), CrossAssetError> {
    // Verify caller is the registered CA admin.
    let stored_admin: Address = env
        .storage()
        .persistent()
        .get(&ADMIN)
        .ok_or(CrossAssetError::NotAuthorized)?;
    if admin != stored_admin {
        return Err(CrossAssetError::NotAuthorized);
    }
    admin.require_auth();

    let asset_key = AssetKey::from_option(asset.clone());
    let mut config = get_asset_config(env, &asset_key)?;
    config.is_frozen = freeze;

    let mut configs: Map<AssetKey, AssetConfig> = env
        .storage()
        .persistent()
        .get(&ASSET_CONFIGS)
        .unwrap_or(Map::new(env));

    configs.set(asset_key, config);
    env.storage().persistent().set(&ASSET_CONFIGS, &configs);

    PoolFrozenEvent {
        asset,
        frozen: freeze,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

// Helper functions

fn get_asset_config(env: &Env, asset_key: &AssetKey) -> Result<AssetConfig, CrossAssetError> {
    let configs: Map<AssetKey, AssetConfig> = env
        .storage()
        .persistent()
        .get(&ASSET_CONFIGS)
        .unwrap_or(Map::new(env));

    configs
        .get(asset_key.clone())
        .ok_or(CrossAssetError::AssetNotConfigured)
}

fn require_valid_config(config: &AssetConfig) -> Result<(), CrossAssetError> {
    require_valid_basis_points(config.collateral_factor)?;
    require_valid_basis_points(config.liquidation_threshold)?;
    require_valid_basis_points(config.reserve_factor)?;

    if config.price <= 0 {
        return Err(CrossAssetError::InvalidPrice);
    }

    // Liquidation threshold must be >= collateral factor (LTV)
    if config.liquidation_threshold < config.collateral_factor {
        return Err(CrossAssetError::AssetNotConfigured);
    }

    Ok(())
}

fn require_valid_basis_points(value: i128) -> Result<(), CrossAssetError> {
    if !(0..=10_000).contains(&value) {
        return Err(CrossAssetError::AssetNotConfigured);
    }
    Ok(())
}

fn get_total_supply(env: &Env, asset_key: &AssetKey) -> i128 {
    let supplies: Map<AssetKey, i128> = env
        .storage()
        .persistent()
        .get(&TOTAL_SUPPLIES)
        .unwrap_or(Map::new(env));

    supplies.get(asset_key.clone()).unwrap_or(0)
}

fn update_total_supply(env: &Env, asset_key: &AssetKey, delta: i128) {
    let mut supplies: Map<AssetKey, i128> = env
        .storage()
        .persistent()
        .get(&TOTAL_SUPPLIES)
        .unwrap_or(Map::new(env));

    let current = supplies.get(asset_key.clone()).unwrap_or(0);
    supplies.set(asset_key.clone(), current + delta);
    env.storage().persistent().set(&TOTAL_SUPPLIES, &supplies);
}

fn get_total_borrow(env: &Env, asset_key: &AssetKey) -> i128 {
    let borrows: Map<AssetKey, i128> = env
        .storage()
        .persistent()
        .get(&TOTAL_BORROWS)
        .unwrap_or(Map::new(env));

    borrows.get(asset_key.clone()).unwrap_or(0)
}

fn update_total_borrow(env: &Env, asset_key: &AssetKey, delta: i128) {
    let mut borrows: Map<AssetKey, i128> = env
        .storage()
        .persistent()
        .get(&TOTAL_BORROWS)
        .unwrap_or(Map::new(env));

    let current = borrows.get(asset_key.clone()).unwrap_or(0);
    borrows.set(asset_key.clone(), current + delta);
    env.storage().persistent().set(&TOTAL_BORROWS, &borrows);
}

/// Combined key for user-asset position lookups
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserAssetKey {
    pub user: Address,
    pub asset: AssetKey,
}

impl UserAssetKey {
    pub fn new(user: Address, asset: Option<Address>) -> Self {
        Self {
            user,
            asset: AssetKey::from_option(asset),
        }
    }
}

impl AssetKey {
    /// Convert an `Option<Address>` into an `AssetKey` (`None` → `Native`).
    pub fn from_option(asset: Option<Address>) -> Self {
        match asset {
            Some(addr) => AssetKey::Token(addr),
            None => AssetKey::Native,
        }
    }

    /// Convert back to `Option<Address>` (`Native` → `None`).
    pub fn to_option(&self) -> Option<Address> {
        match self {
            AssetKey::Native => None,
            AssetKey::Token(addr) => Some(addr.clone()),
        }
    }
}
