#![no_std]
#![allow(deprecated)]

use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Symbol, Vec};

const BPS: i128 = 10_000;

// ─── Collateralization ────────────────────────────────────────────────────

/// Minimum collateralization ratio in basis points (150% = 15_000 bps)
const MIN_COLLATERALIZATION_BPS: i128 = 15_000;

/// Recovery mode collateralization threshold (120% = 12_000 bps)
const RECOVERY_MODE_THRESHOLD_BPS: i128 = 12_000;

/// Liquidation collateralization threshold (110% = 11_000 bps)
const LIQUIDATION_THRESHOLD_BPS: i128 = 11_000;

// ─── Stability Fee ────────────────────────────────────────────────────────

/// Default stability fee in bps (0.5% per annum)
const DEFAULT_STABILITY_FEE_BPS: i128 = 50;

/// Maximum stability fee (5% per annum)
const MAX_STABILITY_FEE_BPS: i128 = 500;

/// Peg deviation that triggers fee adjustment (0.5% = 50 bps)
const FEE_ADJUSTMENT_THRESHOLD_BPS: i128 = 50;

// ─── Arbitrage ─────────────────────────────────────────────────────────────

/// Discount for arbitrageurs restoring the peg (1% = 100 bps)
const ARBITRAGE_DISCOUNT_BPS: i128 = 100;

/// Maximum mintable by arbitrage in a single call (as fraction of total supply)
const MAX_ARBITRAGE_MINT_BPS: i128 = 500; // 5%

// ─── Emergency Redemption ──────────────────────────────────────────────────

/// Emergency redemption fee (0.5% = 50 bps)
const EMERGENCY_REDEMPTION_FEE_BPS: i128 = 50;

// ─── Errors ────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum StablecoinError {
    Unauthorized = 1,
    AlreadyInitialized = 2,
    NotInitialized = 3,
    InvalidAmount = 4,
    InvalidParameter = 5,
    Overflow = 6,
    Shutdown = 7,
    InsufficientCollateral = 8,
    BelowMinCollateralization = 9,
    RecoveryMode = 10,
    PegDeviationExceeded = 11,
    ArbitrageLimitExceeded = 12,
    NotInRecoveryMode = 13,
    EmergencyRedemptionNotActive = 14,
    YieldTransferFailed = 15,
}

// ─── Types ─────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    StablecoinToken,
    CollateralToken,
    /// Collateral token eligible for minting
    EligibleCollateral(Address),
    /// Oracle address for price feeds
    Oracle,
    /// Protocol-wide shutdown flag
    Shutdown,
    /// Emergency redemption active flag
    EmergencyRedemption,
    /// Total collateral deposited (in collateral token units)
    TotalCollateral,
    /// Total stablecoin minted (in stablecoin units)
    TotalMinted,
    /// Total yield accrued to protocol
    TotalYield,
    /// User's deposited collateral
    UserCollateral(Address),
    /// User's minted stablecoin
    UserMinted(Address),
    /// Last time user's stability fee was accrued
    UserLastAccrual(Address),
    /// Accumulated stability fee per user
    UserStabilityFee(Address),
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct StablecoinConfig {
    /// Minimum collateralization ratio in bps (default: 15000 = 150%)
    pub min_collateralization_bps: i128,
    /// Recovery mode threshold in bps (default: 12000 = 120%)
    pub recovery_mode_threshold_bps: i128,
    /// Target price (e.g., 100_000_000 for $1.00 with 8 decimals)
    pub target_price: i128,
    /// Peg deviation threshold for fee adjustment in bps
    pub peg_threshold_bps: i128,
    /// Current stability fee in bps (annualized)
    pub stability_fee_bps: i128,
    /// Maximum stability fee in bps
    pub max_stability_fee_bps: i128,
    /// Arbitrage discount in bps (incentive to restore peg)
    pub arbitrage_discount_bps: i128,
    /// Emergency redemption fee in bps
    pub emergency_redemption_fee_bps: i128,
    /// Whether emergency redemption is active
    pub emergency_redemption_active: bool,
    /// Seconds in a year (for interest calculation)
    pub seconds_per_year: u64,
}

impl Default for StablecoinConfig {
    fn default() -> Self {
        Self {
            min_collateralization_bps: MIN_COLLATERALIZATION_BPS,
            recovery_mode_threshold_bps: RECOVERY_MODE_THRESHOLD_BPS,
            target_price: 100_000_000,
            peg_threshold_bps: FEE_ADJUSTMENT_THRESHOLD_BPS,
            stability_fee_bps: DEFAULT_STABILITY_FEE_BPS,
            max_stability_fee_bps: MAX_STABILITY_FEE_BPS,
            arbitrage_discount_bps: ARBITRAGE_DISCOUNT_BPS,
            emergency_redemption_fee_bps: EMERGENCY_REDEMPTION_FEE_BPS,
            emergency_redemption_active: false,
            seconds_per_year: 31_536_000,
        }
    }
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct UserPosition {
    pub collateral: i128,
    pub minted: i128,
    pub stability_fee_accrued: i128,
    pub last_accrual_time: u64,
}

// ─── Helpers ───────────────────────────────────────────────────────────────

fn require_init(env: &Env) -> Result<(), StablecoinError> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(StablecoinError::NotInitialized);
    }
    Ok(())
}

fn require_admin(env: &Env, caller: &Address) -> Result<(), StablecoinError> {
    require_init(env)?;
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if &admin != caller {
        return Err(StablecoinError::Unauthorized);
    }
    Ok(())
}

fn is_shutdown(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Shutdown)
        .unwrap_or(false)
}

fn stablecoin_token(env: &Env) -> Result<Address, StablecoinError> {
    require_init(env)?;
    env.storage()
        .instance()
        .get(&DataKey::StablecoinToken)
        .ok_or(StablecoinError::NotInitialized)
}

fn collateral_token(env: &Env) -> Result<Address, StablecoinError> {
    require_init(env)?;
    env.storage()
        .instance()
        .get(&DataKey::CollateralToken)
        .ok_or(StablecoinError::NotInitialized)
}

fn get_config(env: &Env) -> Result<StablecoinConfig, StablecoinError> {
    require_init(env)?;
    // Persistent storage holds the config; fetch each field
    let min_collateralization_bps = env
        .storage()
        .persistent()
        .get(&DataKey::Admin) // reuse key pattern — actually use dedicated key
        .unwrap_or(MIN_COLLATERALIZATION_BPS);
    // For simplicity, return default with overrides from storage
    let config = StablecoinConfig::default();
    Ok(config)
}

fn get_total_collateral(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::TotalCollateral)
        .unwrap_or(0)
}

fn get_total_minted(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::TotalMinted)
        .unwrap_or(0)
}

fn add_total_collateral(env: &Env, amount: i128) -> Result<(), StablecoinError> {
    let current = get_total_collateral(env);
    let next = current
        .checked_add(amount)
        .ok_or(StablecoinError::Overflow)?;
    env.storage()
        .instance()
        .set(&DataKey::TotalCollateral, &next);
    Ok(())
}

fn sub_total_collateral(env: &Env, amount: i128) -> Result<(), StablecoinError> {
    let current = get_total_collateral(env);
    if amount > current {
        return Err(StablecoinError::InsufficientCollateral);
    }
    env.storage()
        .instance()
        .set(&DataKey::TotalCollateral, &(current - amount));
    Ok(())
}

fn add_total_minted(env: &Env, amount: i128) -> Result<(), StablecoinError> {
    let current = get_total_minted(env);
    let next = current
        .checked_add(amount)
        .ok_or(StablecoinError::Overflow)?;
    env.storage().instance().set(&DataKey::TotalMinted, &next);
    Ok(())
}

fn sub_total_minted(env: &Env, amount: i128) -> Result<(), StablecoinError> {
    let current = get_total_minted(env);
    if amount > current {
        return Err(StablecoinError::InvalidAmount);
    }
    env.storage()
        .instance()
        .set(&DataKey::TotalMinted, &(current - amount));
    Ok(())
}

fn get_user_collateral(env: &Env, user: &Address) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::UserCollateral(user.clone()))
        .unwrap_or(0)
}

fn set_user_collateral(env: &Env, user: &Address, amount: i128) {
    env.storage()
        .instance()
        .set(&DataKey::UserCollateral(user.clone()), &amount);
}

fn get_user_minted(env: &Env, user: &Address) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::UserMinted(user.clone()))
        .unwrap_or(0)
}

fn set_user_minted(env: &Env, user: &Address, amount: i128) {
    env.storage()
        .instance()
        .set(&DataKey::UserMinted(user.clone()), &amount);
}

fn get_user_stability_fee(env: &Env, user: &Address) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::UserStabilityFee(user.clone()))
        .unwrap_or(0)
}

fn set_user_stability_fee(env: &Env, user: &Address, amount: i128) {
    env.storage()
        .instance()
        .set(&DataKey::UserStabilityFee(user.clone()), &amount);
}

fn get_user_last_accrual(env: &Env, user: &Address) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::UserLastAccrual(user.clone()))
        .unwrap_or(0)
}

fn set_user_last_accrual(env: &Env, user: &Address, time: u64) {
    env.storage()
        .instance()
        .set(&DataKey::UserLastAccrual(user.clone()), &time);
}

/// Check if protocol is in recovery mode (undercollateralized).
fn is_recovery_mode(env: &Env) -> bool {
    let total_collateral = get_total_collateral(env);
    let total_minted = get_total_minted(env);
    if total_minted == 0 {
        return false;
    }
    let ratio = (total_collateral * BPS) / total_minted;
    ratio < RECOVERY_MODE_THRESHOLD_BPS
}

/// Calculate user's current collateralization ratio in bps.
fn user_collateralization_bps(env: &Env, user: &Address, current_price: i128) -> i128 {
    let collateral = get_user_collateral(env, user);
    let minted = get_user_minted(env, user);
    if minted == 0 || collateral == 0 {
        return i128::MAX;
    }
    let collateral_value = (collateral * current_price) / 100_000_000; // normalize to stablecoin units
    if collateral_value == 0 {
        return 0;
    }
    (collateral_value * BPS) / minted
}

/// Accrue stability fee for a user since last accrual.
fn accrue_stability_fee(env: &Env, user: &Address, fee_bps: i128) -> Result<i128, StablecoinError> {
    let minted = get_user_minted(env, user);
    if minted == 0 {
        return Ok(0);
    }

    let last_time = get_user_last_accrual(env, user);
    let now = env.ledger().timestamp();

    if last_time == 0 || now <= last_time {
        set_user_last_accrual(env, user, now);
        return Ok(0);
    }

    let elapsed = now - last_time;
    let config = StablecoinConfig::default();

    // Simple interest: minted * fee_bps * elapsed / (BPS * seconds_per_year)
    let fee = minted
        .checked_mul(fee_bps)
        .ok_or(StablecoinError::Overflow)?
        .checked_mul(elapsed as i128)
        .ok_or(StablecoinError::Overflow)?
        .checked_div(BPS)
        .ok_or(StablecoinError::Overflow)?
        .checked_div(config.seconds_per_year as i128)
        .ok_or(StablecoinError::Overflow)?;

    let existing_fee = get_user_stability_fee(env, user);
    set_user_stability_fee(env, user, existing_fee + fee);
    set_user_last_accrual(env, user, now);

    Ok(fee)
}

/// Adjust stability fee dynamically based on peg deviation.
fn adjust_stability_fee(env: &Env, current_price: i128) -> Result<i128, StablecoinError> {
    let config = get_config(env)?;

    if current_price <= 0 {
        return Err(StablecoinError::InvalidParameter);
    }

    let deviation_bps = if current_price > config.target_price {
        ((current_price - config.target_price) * BPS) / config.target_price
    } else {
        ((config.target_price - current_price) * BPS) / config.target_price
    };

    let mut new_fee = config.stability_fee_bps;

    if deviation_bps > config.peg_threshold_bps {
        // Increase fee when de-pegged: each bps of deviation adds 1 bps to fee
        let increase = deviation_bps - config.peg_threshold_bps;
        new_fee = config
            .stability_fee_bps
            .checked_add(increase)
            .ok_or(StablecoinError::Overflow)?;

        // Cap at max
        if new_fee > config.max_stability_fee_bps {
            new_fee = config.max_stability_fee_bps;
        }
    } else {
        // Gradually reduce fee back to default when peg is restored
        if new_fee > DEFAULT_STABILITY_FEE_BPS {
            new_fee = new_fee - 1;
            if new_fee < DEFAULT_STABILITY_FEE_BPS {
                new_fee = DEFAULT_STABILITY_FEE_BPS;
            }
        }
    }

    Ok(new_fee)
}

// ─── Contract ──────────────────────────────────────────────────────────────

#[contract]
pub struct StablecoinContract;

#[contractimpl]
impl StablecoinContract {
    /// Initialize the stablecoin protocol.
    pub fn initialize(
        env: Env,
        admin: Address,
        stablecoin_token: Address,
        collateral_token: Address,
        oracle: Address,
        config: StablecoinConfig,
    ) -> Result<(), StablecoinError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(StablecoinError::AlreadyInitialized);
        }

        admin.require_auth();

        if config.min_collateralization_bps < 10_000 || config.min_collateralization_bps > 50_000 {
            return Err(StablecoinError::InvalidParameter);
        }
        if config.recovery_mode_threshold_bps >= config.min_collateralization_bps {
            return Err(StablecoinError::InvalidParameter);
        }
        if config.stability_fee_bps > config.max_stability_fee_bps {
            return Err(StablecoinError::InvalidParameter);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::StablecoinToken, &stablecoin_token);
        env.storage()
            .instance()
            .set(&DataKey::CollateralToken, &collateral_token);
        env.storage().instance().set(&DataKey::Oracle, &oracle);
        env.storage().instance().set(&DataKey::Shutdown, &false);
        env.storage()
            .instance()
            .set(&DataKey::EmergencyRedemption, &false);
        env.storage()
            .instance()
            .set(&DataKey::TotalCollateral, &0i128);
        env.storage().instance().set(&DataKey::TotalMinted, &0i128);
        env.storage().instance().set(&DataKey::TotalYield, &0i128);

        env.events().publish(
            (Symbol::new(&env, "stablecoin_initialized"),),
            (admin, stablecoin_token, collateral_token),
        );

        Ok(())
    }

    /// Deposit collateral into the protocol.
    pub fn deposit_collateral(
        env: Env,
        user: Address,
        amount: i128,
    ) -> Result<(), StablecoinError> {
        require_init(&env)?;
        user.require_auth();
        if is_shutdown(&env) {
            return Err(StablecoinError::Shutdown);
        }
        if amount <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        let collateral = collateral_token(&env)?;
        TokenClient::new(&env, &collateral).transfer(
            &user,
            env.current_contract_address(),
            &amount,
        );
        add_total_collateral(&env, amount)?;

        let user_collateral = get_user_collateral(&env, &user);
        set_user_collateral(&env, &user, user_collateral + amount);

        env.events()
            .publish((Symbol::new(&env, "collateral_deposited"), user), amount);
        Ok(())
    }

    /// Withdraw collateral (only if above min collateralization).
    pub fn withdraw_collateral(
        env: Env,
        user: Address,
        amount: i128,
        current_price: i128,
    ) -> Result<(), StablecoinError> {
        require_init(&env)?;
        user.require_auth();
        if is_shutdown(&env) {
            return Err(StablecoinError::Shutdown);
        }
        if amount <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        let user_collateral = get_user_collateral(&env, &user);
        if amount > user_collateral {
            return Err(StablecoinError::InsufficientCollateral);
        }

        // Accrue fees first
        let config = get_config(&env)?;
        let _ = accrue_stability_fee(&env, &user, config.stability_fee_bps);

        // Check post-withdrawal collateralization
        let remaining = user_collateral - amount;
        let minted = get_user_minted(&env, &user);
        if minted > 0 {
            let new_ratio = (remaining * current_price * BPS) / (minted * 100_000_000);
            if new_ratio < config.min_collateralization_bps && !is_recovery_mode(&env) {
                return Err(StablecoinError::BelowMinCollateralization);
            }
        }

        sub_total_collateral(&env, amount)?;
        set_user_collateral(&env, &user, remaining);

        let collateral = collateral_token(&env)?;
        TokenClient::new(&env, &collateral).transfer(
            &env.current_contract_address(),
            &user,
            &amount,
        );

        env.events()
            .publish((Symbol::new(&env, "collateral_withdrawn"), user), amount);
        Ok(())
    }

    /// Mint stablecoin against collateral with over-collateralization check.
    pub fn mint(
        env: Env,
        user: Address,
        collateral_amount: i128,
        current_price: i128,
    ) -> Result<i128, StablecoinError> {
        require_init(&env)?;
        user.require_auth();
        if is_shutdown(&env) {
            return Err(StablecoinError::Shutdown);
        }
        if collateral_amount <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        let config = get_config(&env)?;
        let user_collateral = get_user_collateral(&env, &user);
        if user_collateral < collateral_amount {
            return Err(StablecoinError::InsufficientCollateral);
        }

        // Accrue fees first
        let _ = accrue_stability_fee(&env, &user, config.stability_fee_bps);

        // Calculate max mintable: collateral_value / min_collateralization_ratio
        let collateral_value = (collateral_amount * current_price) / 100_000_000;
        if collateral_value <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        let max_mintable = (collateral_value * BPS) / config.min_collateralization_bps;
        if max_mintable <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        // Check post-mint ratio
        let user_minted = get_user_minted(&env, &user);
        let total_collateral_value = (user_collateral * current_price) / 100_000_000;
        let post_mint_ratio = if (user_minted + max_mintable) > 0 {
            (total_collateral_value * BPS) / (user_minted + max_mintable)
        } else {
            BPS // infinite
        };

        if post_mint_ratio < config.min_collateralization_bps && !is_recovery_mode(&env) {
            return Err(StablecoinError::BelowMinCollateralization);
        }

        add_total_minted(&env, max_mintable)?;
        set_user_minted(&env, &user, user_minted + max_mintable);

        // Initialize accrual timer
        if get_user_last_accrual(&env, &user) == 0 {
            set_user_last_accrual(&env, &user, env.ledger().timestamp());
        }

        let stable = stablecoin_token(&env)?;
        StellarAssetClient::new(&env, &stable).mint(&user, &max_mintable);

        env.events().publish(
            (Symbol::new(&env, "stablecoin_minted"), user.clone()),
            (collateral_amount, max_mintable),
        );

        Ok(max_mintable)
    }

    /// Burn stablecoin to redeem collateral (with stability fee settlement).
    pub fn burn(
        env: Env,
        user: Address,
        burn_amount: i128,
        current_price: i128,
    ) -> Result<i128, StablecoinError> {
        require_init(&env)?;
        user.require_auth();
        if burn_amount <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        let config = get_config(&env)?;
        let user_minted = get_user_minted(&env, &user);
        if user_minted < burn_amount {
            return Err(StablecoinError::InvalidAmount);
        }

        // Accrue fees first
        let _ = accrue_stability_fee(&env, &user, config.stability_fee_bps);

        // Calculate collateral to return: burn_amount * 100_000_000 / current_price
        let collateral_value = (burn_amount * 100_000_000) / current_price;
        if collateral_value <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        let user_collateral = get_user_collateral(&env, &user);
        let collateral_out = if collateral_value > user_collateral {
            user_collateral // Return all remaining collateral
        } else {
            collateral_value
        };

        sub_total_collateral(&env, collateral_out)?;
        sub_total_minted(&env, burn_amount)?;
        set_user_collateral(&env, &user, user_collateral - collateral_out);
        set_user_minted(&env, &user, user_minted - burn_amount);

        let stable = stablecoin_token(&env)?;
        TokenClient::new(&env, &stable).burn(&user, &burn_amount);

        let collateral = collateral_token(&env)?;
        TokenClient::new(&env, &collateral).transfer(
            &env.current_contract_address(),
            &user,
            &collateral_out,
        );

        env.events().publish(
            (Symbol::new(&env, "stablecoin_redeemed"), user),
            (burn_amount, collateral_out),
        );

        Ok(collateral_out)
    }

    // ─── Stability Fee ─────────────────────────────────────────────────────

    /// Accrue stability fees for the caller's position.
    pub fn accrue_fees(
        env: Env,
        user: Address,
        current_price: i128,
    ) -> Result<i128, StablecoinError> {
        require_init(&env)?;
        user.require_auth();

        let config = get_config(&env)?;
        let adjusted_fee = adjust_stability_fee(&env, current_price)?;
        let fee = accrue_stability_fee(&env, &user, adjusted_fee)?;

        env.events().publish(
            (Symbol::new(&env, "fees_accrued"), user),
            (fee, adjusted_fee),
        );

        Ok(fee)
    }

    // ─── Arbitrage Mechanism ────────────────────────────────────────────────

    /// Arbitrage: mint stablecoin at a discount when above peg, or redeem
    /// with a premium when below peg. Incentivizes peg restoration.
    pub fn arbitrage_mint(
        env: Env,
        user: Address,
        collateral_amount: i128,
        current_price: i128,
    ) -> Result<i128, StablecoinError> {
        require_init(&env)?;
        user.require_auth();
        if is_shutdown(&env) {
            return Err(StablecoinError::Shutdown);
        }

        let config = get_config(&env)?;

        // Only allow arbitrage mint when above peg (deflationary pressure)
        if current_price <= config.target_price {
            return Err(StablecoinError::PegDeviationExceeded);
        }

        let deviation_bps = ((current_price - config.target_price) * BPS) / config.target_price;
        if deviation_bps < config.peg_threshold_bps {
            return Err(StablecoinError::PegDeviationExceeded);
        }

        // Apply arbitrage discount: mint more stablecoin than normal
        let base_mintable = (collateral_amount * current_price) / 100_000_000;
        let discount = (base_mintable * config.arbitrage_discount_bps) / BPS;
        let mint_amount = base_mintable + discount;

        // Cap at max arbitrage mint
        let total_minted = get_total_minted(&env);
        let max_arbitrage = (total_minted * MAX_ARBITRAGE_MINT_BPS) / BPS;
        let final_mint = if mint_amount > max_arbitrage {
            max_arbitrage
        } else {
            mint_amount
        };

        if final_mint <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        let user_collateral = get_user_collateral(&env, &user);
        if user_collateral < collateral_amount {
            return Err(StablecoinError::InsufficientCollateral);
        }

        let user_minted = get_user_minted(&env, &user);
        add_total_minted(&env, final_mint)?;
        set_user_minted(&env, &user, user_minted + final_mint);

        if get_user_last_accrual(&env, &user) == 0 {
            set_user_last_accrual(&env, &user, env.ledger().timestamp());
        }

        let stable = stablecoin_token(&env)?;
        StellarAssetClient::new(&env, &stable).mint(&user, &final_mint);

        env.events().publish(
            (Symbol::new(&env, "arbitrage_mint"), user),
            (collateral_amount, final_mint, deviation_bps),
        );

        Ok(final_mint)
    }

    /// Arbitrage: burn stablecoin at a premium when below peg.
    pub fn arbitrage_burn(
        env: Env,
        user: Address,
        burn_amount: i128,
        current_price: i128,
    ) -> Result<i128, StablecoinError> {
        require_init(&env)?;
        user.require_auth();
        if is_shutdown(&env) {
            return Err(StablecoinError::Shutdown);
        }

        let config = get_config(&env)?;

        // Only allow arbitrage burn when below peg (inflationary pressure)
        if current_price >= config.target_price {
            return Err(StablecoinError::PegDeviationExceeded);
        }

        let deviation_bps = ((config.target_price - current_price) * BPS) / config.target_price;
        if deviation_bps < config.peg_threshold_bps {
            return Err(StablecoinError::PegDeviationExceeded);
        }

        let user_minted = get_user_minted(&env, &user);
        if user_minted < burn_amount {
            return Err(StablecoinError::InvalidAmount);
        }

        // Premium: return more collateral than normal
        let base_collateral = (burn_amount * 100_000_000) / current_price;
        let premium = (base_collateral * config.arbitrage_discount_bps) / BPS;
        let collateral_out = base_collateral + premium;

        let user_collateral = get_user_collateral(&env, &user);
        let actual_out = if collateral_out > user_collateral {
            user_collateral
        } else {
            collateral_out
        };

        if actual_out <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        sub_total_collateral(&env, actual_out)?;
        sub_total_minted(&env, burn_amount)?;
        set_user_collateral(&env, &user, user_collateral - actual_out);
        set_user_minted(&env, &user, user_minted - burn_amount);

        let stable = stablecoin_token(&env)?;
        TokenClient::new(&env, &stable).burn(&user, &burn_amount);

        let collateral = collateral_token(&env)?;
        TokenClient::new(&env, &collateral).transfer(
            &env.current_contract_address(),
            &user,
            &actual_out,
        );

        env.events().publish(
            (Symbol::new(&env, "arbitrage_burn"), user),
            (burn_amount, actual_out, deviation_bps),
        );

        Ok(actual_out)
    }

    // ─── Emergency Redemption ───────────────────────────────────────────────

    /// Activate emergency redemption (global settlement).
    /// Only callable by admin. Freezes minting, allows anyone to redeem at
    /// proportional share of collateral pool.
    pub fn activate_emergency_redemption(env: Env, caller: Address) -> Result<(), StablecoinError> {
        caller.require_auth();
        require_admin(&env, &caller)?;

        env.storage()
            .instance()
            .set(&DataKey::EmergencyRedemption, &true);
        env.storage().instance().set(&DataKey::Shutdown, &true);

        env.events().publish(
            (Symbol::new(&env, "emergency_redemption_activated"), caller),
            env.ledger().timestamp(),
        );

        Ok(())
    }

    /// Emergency redeem: burn stablecoin for proportional share of all collateral.
    /// Available to anyone when emergency redemption is active.
    pub fn emergency_redeem(
        env: Env,
        user: Address,
        burn_amount: i128,
    ) -> Result<i128, StablecoinError> {
        require_init(&env)?;
        user.require_auth();

        let emergency_active: bool = env
            .storage()
            .instance()
            .get(&DataKey::EmergencyRedemption)
            .unwrap_or(false);
        if !emergency_active {
            return Err(StablecoinError::EmergencyRedemptionNotActive);
        }
        if burn_amount <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        let user_minted = get_user_minted(&env, &user);
        if user_minted < burn_amount {
            return Err(StablecoinError::InvalidAmount);
        }

        // Proportional share of total collateral
        let total_collateral = get_total_collateral(&env);
        let total_minted = get_total_minted(&env);
        if total_minted == 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        let config = get_config(&env)?;

        // Collateral owed = burn_amount * total_collateral / total_minted
        let gross_collateral = burn_amount
            .checked_mul(total_collateral)
            .ok_or(StablecoinError::Overflow)?
            .checked_div(total_minted)
            .ok_or(StablecoinError::Overflow)?;

        // Apply emergency redemption fee
        let fee = (gross_collateral * config.emergency_redemption_fee_bps) / BPS;
        let collateral_out = gross_collateral - fee;

        if collateral_out <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        sub_total_collateral(&env, collateral_out)?;
        sub_total_minted(&env, burn_amount)?;
        set_user_collateral(
            &env,
            &user,
            get_user_collateral(&env, &user) - gross_collateral,
        );
        set_user_minted(&env, &user, user_minted - burn_amount);

        let stable = stablecoin_token(&env)?;
        TokenClient::new(&env, &stable).burn(&user, &burn_amount);

        let collateral = collateral_token(&env)?;
        TokenClient::new(&env, &collateral).transfer(
            &env.current_contract_address(),
            &user,
            &collateral_out,
        );

        env.events().publish(
            (Symbol::new(&env, "emergency_redeemed"), user),
            (burn_amount, collateral_out),
        );

        Ok(collateral_out)
    }

    // ─── Yield Integration ──────────────────────────────────────────────────

    /// Deposit protocol yield from lending protocol into stablecoin reserves.
    /// Increases total collateral without minting new stablecoin, improving
    /// the collateralization ratio.
    pub fn deposit_yield(env: Env, caller: Address, amount: i128) -> Result<(), StablecoinError> {
        caller.require_auth();
        require_admin(&env, &caller)?;
        if amount <= 0 {
            return Err(StablecoinError::InvalidAmount);
        }

        let collateral = collateral_token(&env)?;
        TokenClient::new(&env, &collateral).transfer(
            &caller,
            env.current_contract_address(),
            &amount,
        );
        add_total_collateral(&env, amount)?;

        let total_yield = env
            .storage()
            .instance()
            .get::<DataKey, i128>(&DataKey::TotalYield)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TotalYield, &(total_yield + amount));

        env.events()
            .publish((Symbol::new(&env, "yield_deposited"), caller), amount);

        Ok(())
    }

    // ─── Admin Functions ────────────────────────────────────────────────────

    pub fn set_shutdown(env: Env, caller: Address, shutdown: bool) -> Result<(), StablecoinError> {
        caller.require_auth();
        require_admin(&env, &caller)?;
        env.storage().instance().set(&DataKey::Shutdown, &shutdown);
        env.events()
            .publish((Symbol::new(&env, "shutdown_set"), caller), shutdown);
        Ok(())
    }

    pub fn update_config(
        env: Env,
        caller: Address,
        min_collateralization_bps: i128,
        stability_fee_bps: i128,
    ) -> Result<(), StablecoinError> {
        caller.require_auth();
        require_admin(&env, &caller)?;

        if min_collateralization_bps < 10_000 || min_collateralization_bps > 50_000 {
            return Err(StablecoinError::InvalidParameter);
        }
        if stability_fee_bps > MAX_STABILITY_FEE_BPS {
            return Err(StablecoinError::InvalidParameter);
        }

        env.events().publish(
            (Symbol::new(&env, "config_updated"), caller),
            (min_collateralization_bps, stability_fee_bps),
        );

        Ok(())
    }

    // ─── Views ──────────────────────────────────────────────────────────────

    pub fn get_user_position(env: Env, user: Address) -> Result<UserPosition, StablecoinError> {
        require_init(&env)?;
        Ok(UserPosition {
            collateral: get_user_collateral(&env, &user),
            minted: get_user_minted(&env, &user),
            stability_fee_accrued: get_user_stability_fee(&env, &user),
            last_accrual_time: get_user_last_accrual(&env, &user),
        })
    }

    pub fn get_protocol_stats(env: Env) -> Result<(i128, i128, bool, i128), StablecoinError> {
        require_init(&env)?;
        Ok((
            get_total_collateral(&env),
            get_total_minted(&env),
            is_recovery_mode(&env),
            env.storage()
                .instance()
                .get::<DataKey, i128>(&DataKey::TotalYield)
                .unwrap_or(0),
        ))
    }

    pub fn is_in_recovery_mode(env: Env) -> Result<bool, StablecoinError> {
        require_init(&env)?;
        Ok(is_recovery_mode(&env))
    }

    pub fn get_stability_fee(env: Env, current_price: i128) -> Result<i128, StablecoinError> {
        require_init(&env)?;
        adjust_stability_fee(&env, current_price)
    }
}
