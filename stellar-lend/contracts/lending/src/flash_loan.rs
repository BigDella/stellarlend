use crate::events::FlashLoanEvent;
use crate::pause::{is_paused, PauseType};
use soroban_sdk::{contracterror, contracttype, token, Address, Bytes, Env, IntoVal, Symbol, Vec};

/// RAII guard for flash loan reentrancy protection
/// Automatically clears the guard when dropped, even on panic
struct FlashLoanGuard {
    env: Env,
    guard_key: FlashLoanDataKey,
    active_key: FlashLoanDataKey,
}

impl FlashLoanGuard {
    fn new(env: &Env, asset: &Address) -> Result<Self, FlashLoanError> {
        let active_key = FlashLoanDataKey::ActiveFlashLoan(asset.clone());
        if env.storage().instance().get(&active_key).unwrap_or(false) {
            return Err(FlashLoanError::ConcurrentFlashLoan);
        }

        let guard_key = FlashLoanDataKey::ReentrancyGuard;
        if env.storage().instance().get(&guard_key).unwrap_or(false) {
            return Err(FlashLoanError::Reentrancy);
        }
        env.storage().instance().set(&guard_key, &true);
        env.storage().instance().set(&active_key, &true);
        Ok(FlashLoanGuard {
            env: env.clone(),
            guard_key,
            active_key,
        })
    }
}

impl Drop for FlashLoanGuard {
    fn drop(&mut self) {
        self.env.storage().instance().set(&self.guard_key, &false);
        self.env.storage().instance().set(&self.active_key, &false);
    }
}

/// Errors that can occur during flash loan operations
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
    /// Flash loan operations are currently paused
    FlashLoanPaused = 7,
    /// Loan exceeds configured liquidity-relative limits
    FlashLoanLimitExceeded = 8,
    /// Loan would create excessive price impact
    PriceImpactTooHigh = 9,
    /// Liquidity moved too far from its rolling TWAP reference
    TwapDeviationExceeded = 10,
    /// A loan for this asset is already in progress
    ConcurrentFlashLoan = 11,
    /// Arithmetic overflow during security checks
    Overflow = 12,
}

/// Storage keys for flash loan data
#[contracttype]
#[derive(Clone)]
pub enum FlashLoanDataKey {
    FlashLoanFeeBps,
    FlashLoanSecurityConfig,
    LiquidityObservations(Address),
    SecuritySnapshot(Address),
    ActiveFlashLoan(Address),
    ReentrancyGuard,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlashLoanSecurityConfig {
    pub max_loan_to_liquidity_bps: i128,
    pub max_price_impact_bps: i128,
    pub max_twap_deviation_bps: i128,
    pub twap_window_seconds: u64,
    pub max_observations: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiquidityObservation {
    pub liquidity: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlashLoanSecuritySnapshot {
    pub asset: Address,
    pub amount: i128,
    pub liquidity_before: i128,
    pub liquidity_twap: i128,
    pub loan_to_liquidity_bps: i128,
    pub price_impact_bps: i128,
    pub timestamp: u64,
}

const MAX_FEE_BPS: i128 = 1000; // 10% maximum fee
const BPS_SCALE: i128 = 10_000;
const DEFAULT_MAX_LOAN_TO_LIQUIDITY_BPS: i128 = 5_000;
const DEFAULT_MAX_PRICE_IMPACT_BPS: i128 = 2_000;
const DEFAULT_MAX_TWAP_DEVIATION_BPS: i128 = 1_000;
const DEFAULT_TWAP_WINDOW_SECONDS: u64 = 600;
const DEFAULT_MAX_OBSERVATIONS: u32 = 32;

/// Initiate a flash loan
///
/// # Arguments
/// * `env` - The contract environment
/// * `receiver` - The address of the contract receiving the funds and implementing the callback
/// * `asset` - The address of the token to borrow
/// * `amount` - The amount to borrow
/// * `params` - Arbitrary data to pass to the receiver's callback
pub fn flash_loan(
    env: &Env,
    receiver: Address,
    asset: Address,
    amount: i128,
    params: Bytes,
) -> Result<(), FlashLoanError> {
    if is_paused(env, PauseType::FlashLoan) {
        return Err(FlashLoanError::FlashLoanPaused);
    }

    if amount <= 0 {
        return Err(FlashLoanError::InvalidAmount);
    }

    // RAII guard automatically clears on scope exit (even on panic)
    let _guard = FlashLoanGuard::new(env, &asset)?;

    let fee = calculate_fee(env, amount);

    // 0. Record initial balance
    let token_client = token::Client::new(env, &asset);
    let initial_balance = token_client.balance(&env.current_contract_address());
    if initial_balance < amount {
        return Err(FlashLoanError::InvalidAmount);
    }

    let security_snapshot = validate_flash_loan_security(env, &asset, amount, initial_balance)?;
    store_security_snapshot(env, &asset, &security_snapshot);

    // 1. Transfer funds to the receiver
    token_client.transfer(&env.current_contract_address(), &receiver, &amount);

    // 2. Execute callback on receiver
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

    // 3. Verify repayment
    let final_balance = token_client.balance(&env.current_contract_address());

    let required_balance = initial_balance
        .checked_add(fee)
        .ok_or(FlashLoanError::Overflow)?;
    if final_balance < required_balance {
        return Err(FlashLoanError::InsufficientRepayment);
    }
    append_liquidity_observation(env, &asset, final_balance)?;

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

/// Calculate the fee for a flash loan
fn calculate_fee(env: &Env, amount: i128) -> i128 {
    let fee_bps = get_flash_loan_fee_bps(env);
    amount.saturating_mul(fee_bps).saturating_div(10000)
}

/// Set the flash loan fee in basis points
pub fn set_flash_loan_fee_bps(env: &Env, fee_bps: i128) -> Result<(), FlashLoanError> {
    if !(0..=MAX_FEE_BPS).contains(&fee_bps) {
        return Err(FlashLoanError::InvalidFee);
    }
    env.storage()
        .persistent()
        .set(&FlashLoanDataKey::FlashLoanFeeBps, &fee_bps);
    Ok(())
}

/// Get the current flash loan fee in basis points
pub fn get_flash_loan_fee_bps(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&FlashLoanDataKey::FlashLoanFeeBps)
        .unwrap_or(9) // Default 9 bps (0.09%) - matches major protocols like Aave
}

pub fn set_flash_loan_security_config(
    env: &Env,
    config: FlashLoanSecurityConfig,
) -> Result<(), FlashLoanError> {
    validate_security_config(&config)?;
    env.storage()
        .persistent()
        .set(&FlashLoanDataKey::FlashLoanSecurityConfig, &config);
    Ok(())
}

pub fn get_flash_loan_security_config(env: &Env) -> FlashLoanSecurityConfig {
    env.storage()
        .persistent()
        .get(&FlashLoanDataKey::FlashLoanSecurityConfig)
        .unwrap_or_else(default_security_config)
}

pub fn get_flash_loan_security_snapshot(
    env: &Env,
    asset: &Address,
) -> Option<FlashLoanSecuritySnapshot> {
    env.storage()
        .persistent()
        .get(&FlashLoanDataKey::SecuritySnapshot(asset.clone()))
}

pub fn is_asset_flash_loan_active(env: &Env, asset: &Address) -> bool {
    env.storage()
        .instance()
        .get(&FlashLoanDataKey::ActiveFlashLoan(asset.clone()))
        .unwrap_or(false)
}

fn default_security_config() -> FlashLoanSecurityConfig {
    FlashLoanSecurityConfig {
        max_loan_to_liquidity_bps: DEFAULT_MAX_LOAN_TO_LIQUIDITY_BPS,
        max_price_impact_bps: DEFAULT_MAX_PRICE_IMPACT_BPS,
        max_twap_deviation_bps: DEFAULT_MAX_TWAP_DEVIATION_BPS,
        twap_window_seconds: DEFAULT_TWAP_WINDOW_SECONDS,
        max_observations: DEFAULT_MAX_OBSERVATIONS,
    }
}

fn validate_security_config(config: &FlashLoanSecurityConfig) -> Result<(), FlashLoanError> {
    if config.max_loan_to_liquidity_bps <= 0
        || config.max_loan_to_liquidity_bps > BPS_SCALE
        || config.max_price_impact_bps <= 0
        || config.max_price_impact_bps > BPS_SCALE
        || config.max_twap_deviation_bps < 0
        || config.max_twap_deviation_bps > BPS_SCALE
        || config.twap_window_seconds == 0
        || config.max_observations == 0
        || config.max_observations > 256
    {
        return Err(FlashLoanError::InvalidAmount);
    }
    Ok(())
}

fn validate_flash_loan_security(
    env: &Env,
    asset: &Address,
    amount: i128,
    liquidity_before: i128,
) -> Result<FlashLoanSecuritySnapshot, FlashLoanError> {
    if liquidity_before <= 0 {
        return Err(FlashLoanError::InvalidAmount);
    }

    let config = get_flash_loan_security_config(env);
    let loan_to_liquidity_bps = ratio_bps(amount, liquidity_before)?;
    if loan_to_liquidity_bps > config.max_loan_to_liquidity_bps {
        return Err(FlashLoanError::FlashLoanLimitExceeded);
    }

    let liquidity_twap = liquidity_twap(env, asset, liquidity_before, &config)?;
    if liquidity_twap > 0 {
        let twap_deviation_bps = deviation_bps(liquidity_twap, liquidity_before)?;
        if twap_deviation_bps > config.max_twap_deviation_bps {
            return Err(FlashLoanError::TwapDeviationExceeded);
        }
    }

    let price_impact_bps = ratio_bps(amount, liquidity_twap.max(1))?;
    if price_impact_bps > config.max_price_impact_bps {
        return Err(FlashLoanError::PriceImpactTooHigh);
    }

    Ok(FlashLoanSecuritySnapshot {
        asset: asset.clone(),
        amount,
        liquidity_before,
        liquidity_twap,
        loan_to_liquidity_bps,
        price_impact_bps,
        timestamp: env.ledger().timestamp(),
    })
}

fn store_security_snapshot(env: &Env, asset: &Address, snapshot: &FlashLoanSecuritySnapshot) {
    env.storage()
        .persistent()
        .set(&FlashLoanDataKey::SecuritySnapshot(asset.clone()), snapshot);
}

fn load_liquidity_observations(env: &Env, asset: &Address) -> Vec<LiquidityObservation> {
    env.storage()
        .persistent()
        .get(&FlashLoanDataKey::LiquidityObservations(asset.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

fn append_liquidity_observation(
    env: &Env,
    asset: &Address,
    liquidity: i128,
) -> Result<(), FlashLoanError> {
    let config = get_flash_loan_security_config(env);
    let mut observations = load_liquidity_observations(env, asset);
    observations.push_back(LiquidityObservation {
        liquidity,
        timestamp: env.ledger().timestamp(),
    });
    while observations.len() > config.max_observations {
        observations.pop_front();
    }
    env.storage().persistent().set(
        &FlashLoanDataKey::LiquidityObservations(asset.clone()),
        &observations,
    );
    Ok(())
}

fn liquidity_twap(
    env: &Env,
    asset: &Address,
    fallback_liquidity: i128,
    config: &FlashLoanSecurityConfig,
) -> Result<i128, FlashLoanError> {
    let observations = load_liquidity_observations(env, asset);
    if observations.is_empty() {
        return Ok(fallback_liquidity);
    }

    let now = env.ledger().timestamp();
    let window_start = now.saturating_sub(config.twap_window_seconds);
    let mut weighted_sum: i128 = 0;
    let mut total_time: u64 = 0;
    let mut previous_liquidity = fallback_liquidity;
    let mut previous_time = window_start;
    let mut saw_observation = false;

    for observation in observations.iter() {
        if observation.timestamp < window_start {
            previous_liquidity = observation.liquidity;
            saw_observation = true;
            continue;
        }

        let observed_at = observation.timestamp.min(now);
        if observed_at > previous_time {
            let dt = observed_at - previous_time;
            weighted_sum = weighted_sum
                .checked_add(
                    previous_liquidity
                        .checked_mul(dt as i128)
                        .ok_or(FlashLoanError::Overflow)?,
                )
                .ok_or(FlashLoanError::Overflow)?;
            total_time = total_time.saturating_add(dt);
        }

        previous_liquidity = observation.liquidity;
        previous_time = observed_at;
        saw_observation = true;
    }

    if !saw_observation {
        return Ok(fallback_liquidity);
    }

    if now > previous_time {
        let dt = now - previous_time;
        weighted_sum = weighted_sum
            .checked_add(
                previous_liquidity
                    .checked_mul(dt as i128)
                    .ok_or(FlashLoanError::Overflow)?,
            )
            .ok_or(FlashLoanError::Overflow)?;
        total_time = total_time.saturating_add(dt);
    }

    if total_time == 0 {
        return Ok(previous_liquidity);
    }

    weighted_sum
        .checked_div(total_time as i128)
        .ok_or(FlashLoanError::Overflow)
}

fn ratio_bps(numerator: i128, denominator: i128) -> Result<i128, FlashLoanError> {
    if numerator < 0 || denominator <= 0 {
        return Err(FlashLoanError::InvalidAmount);
    }

    numerator
        .checked_mul(BPS_SCALE)
        .ok_or(FlashLoanError::Overflow)?
        .checked_div(denominator)
        .ok_or(FlashLoanError::Overflow)
}

fn deviation_bps(reference: i128, observed: i128) -> Result<i128, FlashLoanError> {
    if reference <= 0 || observed <= 0 {
        return Err(FlashLoanError::InvalidAmount);
    }

    let diff = if observed > reference {
        observed
            .checked_sub(reference)
            .ok_or(FlashLoanError::Overflow)?
    } else {
        reference
            .checked_sub(observed)
            .ok_or(FlashLoanError::Overflow)?
    };
    ratio_bps(diff, reference)
}
