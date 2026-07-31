use crate::events::FlashLoanEvent;
use crate::pause::{is_paused, PauseType};
use crate::reentrancy::{ReentrancyGuard, ReentrancyKey};
use soroban_sdk::{contracterror, contracttype, token, Address, Bytes, Env, IntoVal, Symbol};

/// RAII guard for flash loan reentrancy protection.
/// Automatically clears the guard when dropped, even on panic.
struct FlashLoanGuard {
    env: Env,
    guard_key: FlashLoanDataKey,
}

impl FlashLoanGuard {
    fn new(env: &Env, guard_key: FlashLoanDataKey) -> Result<Self, FlashLoanError> {
        if env.storage().instance().get(&guard_key).unwrap_or(false) {
            return Err(FlashLoanError::Reentrancy);
        }
        env.storage().instance().set(&guard_key, &true);
        Ok(FlashLoanGuard {
            env: env.clone(),
            guard_key,
        })
    }
}

impl Drop for FlashLoanGuard {
    fn drop(&mut self) {
        self.env.storage().instance().set(&self.guard_key, &false);
    }
}

/// Errors that can occur during flash loan operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FlashLoanError {
    InvalidAmount = 1,
    InsufficientRepayment = 2,
    Unauthorized = 3,
    InvalidFee = 4,
    CallbackFailed = 5,
    Reentrancy = 6,
    FlashLoanPaused = 7,
    /// Borrow exceeds the pool-relative liquidity cap.
    ExceedsLiquidityCap = 8,
    /// Price impact of the flash loan exceeds the allowed maximum.
    ExcessivePriceImpact = 9,
    /// A concurrent flash loan is already in flight (per-asset guard).
    ConcurrentLoan = 10,
    /// TWAP price deviates too far from the spot price — manipulation detected.
    PriceManipulationDetected = 11,
    /// Flash loan crossed into a later ledger sequence before completion.
    Expired = 12,
    Overflow = 13,
}

/// Storage keys for flash loan data.
#[contracttype]
#[derive(Clone)]
pub enum FlashLoanDataKey {
    FlashLoanFeeBps,
    ReentrancyGuard,
    /// Anti-manipulation config.
    ManipulationConfig,
    /// Per-asset TWAP accumulator (price_sum, sample_count, last_update).
    TwapAccumulator(Address),
    /// Per-asset concurrent-loan sentinel.
    AssetLoanActive(Address),
}

/// Configuration for flash loan attack prevention.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ManipulationConfig {
    /// Maximum fraction of pool liquidity that can be borrowed in one flash loan, in bps.
    /// E.g. 5000 = 50 % of the pool.
    pub max_borrow_liquidity_bps: i128,
    /// Maximum allowed price impact per flash loan, in bps. E.g. 100 = 1 %.
    pub max_price_impact_bps: i128,
    /// Maximum allowed TWAP-vs-spot deviation before the loan is blocked, in bps.
    pub max_twap_deviation_bps: i128,
    /// Minimum number of TWAP samples required before the check is enforced.
    pub min_twap_samples: u32,
    /// TWAP window length in ledger seconds (e.g. 300 = 5 min).
    pub twap_window_secs: u64,
}

/// Accumulated TWAP state for a single asset.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TwapAccumulator {
    pub price_sum: i128,
    pub sample_count: u32,
    pub last_update: u64,
    /// Running time-weighted average (re-computed on each sample).
    pub twap: i128,
}

const MAX_FEE_BPS: i128 = 1000;
const BPS_DENOM: i128 = 10_000;

fn default_manipulation_config() -> ManipulationConfig {
    ManipulationConfig {
        max_borrow_liquidity_bps: 5_000,
        max_price_impact_bps: 100,
        max_twap_deviation_bps: 200,
        min_twap_samples: 3,
        twap_window_secs: 300,
    }
}

fn get_manipulation_config(env: &Env) -> ManipulationConfig {
    env.storage()
        .persistent()
        .get(&FlashLoanDataKey::ManipulationConfig)
        .unwrap_or_else(default_manipulation_config)
}

/// Record a new oracle price sample into the TWAP accumulator for `asset`.
pub fn record_price_sample(env: &Env, asset: &Address, spot_price: i128) {
    if spot_price <= 0 {
        return;
    }
    let key = FlashLoanDataKey::TwapAccumulator(asset.clone());
    let mut acc: TwapAccumulator =
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(TwapAccumulator {
                price_sum: 0,
                sample_count: 0,
                last_update: 0,
                twap: spot_price,
            });

    let cfg = get_manipulation_config(env);
    let now = env.ledger().timestamp();

    // Age out the accumulator if the window has passed.
    if now.saturating_sub(acc.last_update) > cfg.twap_window_secs {
        acc.price_sum = spot_price;
        acc.sample_count = 1;
        acc.twap = spot_price;
    } else {
        acc.price_sum = acc.price_sum.saturating_add(spot_price);
        acc.sample_count = acc.sample_count.saturating_add(1);
        acc.twap = acc
            .price_sum
            .checked_div(acc.sample_count as i128)
            .unwrap_or(acc.twap);
    }
    acc.last_update = now;
    env.storage().persistent().set(&key, &acc);
}

/// Return the stored TWAP for `asset`, or None if not enough samples exist.
pub fn get_twap(env: &Env, asset: &Address) -> Option<TwapAccumulator> {
    let cfg = get_manipulation_config(env);
    let acc: TwapAccumulator = env
        .storage()
        .persistent()
        .get(&FlashLoanDataKey::TwapAccumulator(asset.clone()))?;
    if acc.sample_count < cfg.min_twap_samples {
        return None;
    }
    Some(acc)
}

/// Check whether `spot_price` deviates excessively from the stored TWAP.
fn check_twap_deviation(
    env: &Env,
    asset: &Address,
    spot_price: i128,
) -> Result<(), FlashLoanError> {
    let cfg = get_manipulation_config(env);
    let acc = match get_twap(env, asset) {
        Some(a) => a,
        None => return Ok(()), // not enough history — skip check
    };

    if acc.twap <= 0 || spot_price <= 0 {
        return Ok(());
    }

    let diff = (spot_price - acc.twap).abs();
    let deviation_bps = diff
        .checked_mul(BPS_DENOM)
        .ok_or(FlashLoanError::Overflow)?
        .checked_div(acc.twap)
        .ok_or(FlashLoanError::Overflow)?;

    if deviation_bps > cfg.max_twap_deviation_bps {
        return Err(FlashLoanError::PriceManipulationDetected);
    }
    Ok(())
}

/// Verify the borrow amount does not exceed the pool-relative liquidity cap.
fn check_liquidity_cap(env: &Env, pool_balance: i128, amount: i128) -> Result<(), FlashLoanError> {
    let cfg = get_manipulation_config(env);
    let max_borrow = pool_balance
        .checked_mul(cfg.max_borrow_liquidity_bps)
        .ok_or(FlashLoanError::Overflow)?
        .checked_div(BPS_DENOM)
        .ok_or(FlashLoanError::Overflow)?;
    if amount > max_borrow {
        return Err(FlashLoanError::ExceedsLiquidityCap);
    }
    Ok(())
}

/// Verify the price impact of borrowing `amount` from a pool of `pool_balance`.
///
/// Uses a constant-product approximation: impact ≈ amount / (pool_balance + amount).
fn check_price_impact(env: &Env, pool_balance: i128, amount: i128) -> Result<(), FlashLoanError> {
    if pool_balance <= 0 {
        return Err(FlashLoanError::ExcessivePriceImpact);
    }
    let cfg = get_manipulation_config(env);
    let denominator = pool_balance.saturating_add(amount);
    let impact_bps = amount
        .checked_mul(BPS_DENOM)
        .ok_or(FlashLoanError::Overflow)?
        .checked_div(denominator)
        .ok_or(FlashLoanError::Overflow)?;
    if impact_bps > cfg.max_price_impact_bps {
        return Err(FlashLoanError::ExcessivePriceImpact);
    }
    Ok(())
}

/// Per-asset concurrent loan guard — prevents two simultaneous flash loans on the
/// same asset (sandwich-attack vector).
struct AssetLoanGuard {
    env: Env,
    key: FlashLoanDataKey,
}

impl AssetLoanGuard {
    fn acquire(env: &Env, asset: &Address) -> Result<Self, FlashLoanError> {
        let key = FlashLoanDataKey::AssetLoanActive(asset.clone());
        if env
            .storage()
            .instance()
            .get::<_, bool>(&key)
            .unwrap_or(false)
        {
            return Err(FlashLoanError::ConcurrentLoan);
        }
        env.storage().instance().set(&key, &true);
        Ok(AssetLoanGuard {
            env: env.clone(),
            key,
        })
    }
}

impl Drop for AssetLoanGuard {
    fn drop(&mut self) {
        self.env.storage().instance().set(&self.key, &false);
    }
}

/// Initiate a flash loan with attack-prevention guards.
///
/// # Arguments
/// * `env`        - The contract environment
/// * `receiver`   - The address of the contract receiving the funds and implementing the callback
/// * `asset`      - The address of the token to borrow
/// * `amount`     - The amount to borrow
/// * `spot_price` - Current spot price of the asset (used for TWAP deviation check)
/// * `params`     - Arbitrary data to pass to the receiver's callback
pub fn flash_loan(
    env: &Env,
    receiver: Address,
    asset: Address,
    amount: i128,
    spot_price: i128,
    params: Bytes,
) -> Result<(), FlashLoanError> {
    // CHECKS-EFFECTS-INTERACTIONS PATTERN
    // 1. CHECKS: Reentrancy guard, pause state, validation, attack prevention
    let _guard = ReentrancyGuard::new_with_key(env, ReentrancyKey::FlashLoanLock, false)
        .map_err(|_| FlashLoanError::Reentrancy)?;

    if is_paused(env, PauseType::FlashLoan) {
        return Err(FlashLoanError::FlashLoanPaused);
    }

    if amount <= 0 {
        return Err(FlashLoanError::InvalidAmount);
    }

    // --- Attack prevention checks ---

    // 1. TWAP price-manipulation detection.
    check_twap_deviation(env, &asset, spot_price)?;

    let token_client = token::Client::new(env, &asset);
    let pool_balance = token_client.balance(&env.current_contract_address());

    // 2. Pool liquidity cap (e.g. max 50 % of pool per flash loan).
    check_liquidity_cap(env, pool_balance, amount)?;

    // 3. Price impact check (constant-product approximation).
    check_price_impact(env, pool_balance, amount)?;

    // 4. Per-asset concurrent loan guard (sandwich prevention).
    let _asset_guard = AssetLoanGuard::acquire(env, &asset)?;

    // 5. Global reentrancy guard (legacy, kept for compatibility).
    let _legacy_guard = FlashLoanGuard::new(env, FlashLoanDataKey::ReentrancyGuard)?;

    let fee = calculate_fee(env, amount);
    let initial_balance = pool_balance;
    let start_sequence = env.ledger().sequence_number();

    // Record this spot price into the TWAP before dispatching.
    record_price_sample(env, &asset, spot_price);

    // 2. EFFECTS: State updates (TWAP recording done above)

    // 3. INTERACTIONS: External calls (transfer, callback)
    // Transfer funds to the receiver.
    token_client.transfer(&env.current_contract_address(), &receiver, &amount);

    // Execute callback on receiver.
    let callback_result: bool = env.invoke_contract(
        &receiver,
        &Symbol::new(env, "on_flash_loan"),
        (
            env.current_contract_address(),
            asset.clone(),
            amount,
            fee,
            params,
        )
            .into_val(env),
    );

    if !callback_result {
        return Err(FlashLoanError::CallbackFailed);
    }

    if env.ledger().sequence_number() != start_sequence {
        return Err(FlashLoanError::Expired);
    }

    // Verify repayment.
    let final_balance = token_client.balance(&env.current_contract_address());
    if final_balance < initial_balance + fee {
        return Err(FlashLoanError::InsufficientRepayment);
    }

    FlashLoanEvent {
        receiver: receiver.clone(),
        asset: asset.clone(),
        amount,
        fee,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Calculate the fee for a flash loan.
fn calculate_fee(env: &Env, amount: i128) -> i128 {
    let fee_bps = get_flash_loan_fee_bps(env);
    amount.saturating_mul(fee_bps).saturating_div(10000)
}

/// Set the flash loan fee in basis points.
pub fn set_flash_loan_fee_bps(env: &Env, fee_bps: i128) -> Result<(), FlashLoanError> {
    if !(0..=MAX_FEE_BPS).contains(&fee_bps) {
        return Err(FlashLoanError::InvalidFee);
    }
    env.storage()
        .persistent()
        .set(&FlashLoanDataKey::FlashLoanFeeBps, &fee_bps);
    Ok(())
}

/// Get the current flash loan fee in basis points.
pub fn get_flash_loan_fee_bps(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&FlashLoanDataKey::FlashLoanFeeBps)
        .unwrap_or(9)
}

/// Update the attack-prevention configuration (admin only).
pub fn set_manipulation_config(
    env: &Env,
    config: ManipulationConfig,
) -> Result<(), FlashLoanError> {
    if config.max_borrow_liquidity_bps <= 0
        || config.max_borrow_liquidity_bps > BPS_DENOM
        || config.max_price_impact_bps <= 0
        || config.max_twap_deviation_bps <= 0
        || config.twap_window_secs == 0
    {
        return Err(FlashLoanError::InvalidFee);
    }
    env.storage()
        .persistent()
        .set(&FlashLoanDataKey::ManipulationConfig, &config);
    Ok(())
}
