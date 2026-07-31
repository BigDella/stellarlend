use core::cmp::max;

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, log, symbol_short, Address,
    BytesN, Env, String, Symbol, Vec, I256,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    AlreadyInitialised = 1,
    NotInitialised = 2,
    Unauthorised = 3,
    BridgeAlreadyExists = 4,
    BridgeNotFound = 5,
    BridgeInactive = 6,
    FeeTooHigh = 7,
    InvalidBridgeIdLen = 8,
    InvalidBridgeIdChar = 9,
    NegativeMinAmount = 10,
    AmountNotPositive = 11,
    AmountBelowMinimum = 12,
    Overflow = 13,
    BridgeAcceptancePaused = 14,
    ValidatorAlreadyExists = 15,
    ValidatorNotFound = 16,
    ValidatorInactive = 17,
    InvalidValidatorStake = 18,
    MessageAlreadyExists = 19,
    MessageNotFound = 20,
    DuplicateAttestation = 21,
    QuorumNotReached = 22,
    MessageNotFinal = 23,
    MessageAlreadyExecuted = 24,
    MessageInvalidated = 25,
    ReplayDetected = 26,
    ChannelClosed = 27,
    InvalidMessageVersion = 28,
    InvalidSecurityConfig = 29,
    ConflictingMessages = 30,
    NothingToSlash = 31,
    MessageRejected = 32,
    InvalidMessageOrdering = 33,
}

// ─── Standardized bridge events ──────────────────────────────────────────────
// All events carry: relevant identifiers, state fields, and a `timestamp` field
// (ledger timestamp in seconds). Topic prefixes follow the StellarLend
// short snake_case convention used by indexers.

#[contractevent(topics = ["br_reg"])]
#[derive(Clone, Debug)]
pub struct BridgeRegisteredEvent {
    pub bridge_id: String,
    pub fee_bps: u64,
    pub min_amount: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["br_fee"])]
#[derive(Clone, Debug)]
pub struct BridgeFeeUpdatedEvent {
    pub bridge_id: String,
    pub fee_bps: u64,
    pub timestamp: u64,
}

#[contractevent(topics = ["br_active"])]
#[derive(Clone, Debug)]
pub struct BridgeActiveUpdatedEvent {
    pub bridge_id: String,
    pub active: bool,
    pub timestamp: u64,
}

#[contractevent(topics = ["br_dep"])]
#[derive(Clone, Debug)]
pub struct BridgeDepositEvent {
    pub bridge_id: String,
    pub amount: i128,
    pub fee: i128,
    pub net: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["br_wdraw"])]
#[derive(Clone, Debug)]
pub struct BridgeWithdrawalEvent {
    pub bridge_id: String,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["br_pause"])]
#[derive(Clone, Debug)]
pub struct BridgeAcceptancePauseEvent {
    pub paused: bool,
    pub admin: Address,
    pub timestamp: u64,
}

#[contractevent(topics = ["br_val_upd"])]
#[derive(Clone, Debug)]
pub struct ValidatorUpdatedEvent {
    pub validator: Address,
    pub stake: i128,
    pub active: bool,
    pub timestamp: u64,
}

#[contractevent(topics = ["br_sec_cfg"])]
#[derive(Clone, Debug)]
pub struct SecurityConfigUpdatedEvent {
    pub min_validator_signatures: u32,
    pub min_finality_ledgers: u32,
    pub optimistic_delay_ledgers: u32,
    pub slash_bps: u64,
    pub supported_message_version: u32,
    pub timestamp: u64,
}

#[contractevent(topics = ["br_slash"])]
#[derive(Clone, Debug)]
pub struct ValidatorSlashedEvent {
    pub validator: Address,
    pub amount: i128,
    pub remaining_stake: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["br_ch_emrg"])]
#[derive(Clone, Debug)]
pub struct ChannelEmergencyCloseEvent {
    pub channel_id: String,
    pub closed: bool,
    pub reason: String,
    pub timestamp: u64,
}

#[contractevent(topics = ["br_anomaly"])]
#[derive(Clone, Debug)]
pub struct BridgeAnomalyEvent {
    pub channel_id: String,
    pub anomaly_count: u32,
    pub reason: String,
    pub timestamp: u64,
}

const MAX_FEE_BPS: u64 = 1_000;
const MAX_ID_LEN: u32 = 64;
const DEFAULT_MIN_VALIDATOR_SIGNATURES: u32 = 2;
const DEFAULT_MIN_FINALITY_LEDGERS: u32 = 3;
const DEFAULT_OPTIMISTIC_DELAY_LEDGERS: u32 = 2;
const DEFAULT_SLASH_BPS: u64 = 2_500;
const DEFAULT_SUPPORTED_MESSAGE_VERSION: u32 = 1;
const DEFAULT_ANOMALY_CLOSE_THRESHOLD: u32 = 3;

#[allow(dead_code)]
const ADMIN_KEY: Symbol = symbol_short!("ADMIN");

#[contracttype]
#[derive(Clone, Debug)]
pub struct BridgeConfig {
    pub bridge_id: String,
    pub fee_bps: u64,
    pub min_amount: i128,
    pub active: bool,
    pub total_deposited: i128,
    pub total_withdrawn: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SecurityConfig {
    pub min_validator_signatures: u32,
    pub min_finality_ledgers: u32,
    pub optimistic_delay_ledgers: u32,
    pub slash_bps: u64,
    pub supported_message_version: u32,
    pub anomaly_close_threshold: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ValidatorRecord {
    pub stake: i128,
    pub active: bool,
    pub slashed_total: i128,
    pub approvals: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ChannelState {
    pub emergency_closed: bool,
    pub anomaly_count: u32,
    pub last_observed_nonce: u64,
    pub last_observed_height: u64,
    pub reason: String,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BridgeSecurityStats {
    pub total_messages: u64,
    pub executed_messages: u64,
    pub rejected_messages: u64,
    pub replay_rejections: u64,
    pub anomaly_events: u64,
    pub slashes: u64,
    pub emergency_closures: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BridgeAnalytics {
    pub bridge_id: String,
    pub total_volume_deposited: i128,
    pub total_volume_withdrawn: i128,
    pub net_volume: i128,
    pub fee_bps: u64,
    pub total_fees_collected: i128,
    pub active_validators: u32,
    pub total_validators: u32,
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SourceMessageKey {
    pub channel_id: String,
    pub source_chain: String,
    pub source_tx_id: String,
    pub nonce: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct CrossChainMessage {
    pub message_id: u64,
    pub bridge_id: String,
    pub channel_id: String,
    pub source_chain: String,
    pub source_tx_id: String,
    pub source_height: u64,
    pub nonce: u64,
    pub recipient: Address,
    pub amount: i128,
    pub payload_version: u32,
    pub submitted_at_ledger: u32,
    pub finality_ledger: u32,
    pub approvals: u32,
    pub rejections: u32,
    pub executed: bool,
    pub invalidated: bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct CrossChainMessageInput {
    pub bridge_id: String,
    pub channel_id: String,
    pub source_chain: String,
    pub source_tx_id: String,
    pub source_height: u64,
    pub nonce: u64,
    pub recipient: Address,
    pub amount: i128,
    pub payload_version: u32,
}

#[contracttype]
pub enum DataKey {
    Bridge(String),
    BridgeList,
    BridgeAcceptancePaused,
    SecurityConfig,
    Validator(Address),
    ValidatorList,
    Channel(String),
    Message(u64),
    MessageSource(SourceMessageKey),
    MessageAttestation((u64, Address)),
    NextMessageId,
    Stats,
}

#[contract]
pub struct BridgeContract;

#[contractimpl]
impl BridgeContract {
    pub fn init(env: Env, admin: Address) -> Result<(), ContractError> {
        if env.storage().instance().has(&ADMIN_KEY) {
            return Err(ContractError::AlreadyInitialised);
        }
        env.storage().instance().set(&ADMIN_KEY, &admin);
        env.storage()
            .instance()
            .set(&DataKey::SecurityConfig, &Self::default_security_config());
        env.storage()
            .instance()
            .set(&DataKey::Stats, &Self::default_stats());
        Ok(())
    }

    fn default_security_config() -> SecurityConfig {
        SecurityConfig {
            min_validator_signatures: DEFAULT_MIN_VALIDATOR_SIGNATURES,
            min_finality_ledgers: DEFAULT_MIN_FINALITY_LEDGERS,
            optimistic_delay_ledgers: DEFAULT_OPTIMISTIC_DELAY_LEDGERS,
            slash_bps: DEFAULT_SLASH_BPS,
            supported_message_version: DEFAULT_SUPPORTED_MESSAGE_VERSION,
            anomaly_close_threshold: DEFAULT_ANOMALY_CLOSE_THRESHOLD,
        }
    }

    fn default_stats() -> BridgeSecurityStats {
        BridgeSecurityStats {
            total_messages: 0,
            executed_messages: 0,
            rejected_messages: 0,
            replay_rejections: 0,
            anomaly_events: 0,
            slashes: 0,
            emergency_closures: 0,
        }
    }

    fn empty_string(env: &Env) -> String {
        String::from_str(env, "")
    }

    fn default_channel_state(env: &Env) -> ChannelState {
        ChannelState {
            emergency_closed: false,
            anomaly_count: 0,
            last_observed_nonce: 0,
            last_observed_height: 0,
            reason: Self::empty_string(env),
        }
    }

    fn load_admin(env: &Env) -> Result<Address, ContractError> {
        env.storage()
            .instance()
            .get(&ADMIN_KEY)
            .ok_or(ContractError::NotInitialised)
    }

    fn require_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
        caller.require_auth();
        if *caller != Self::load_admin(env)? {
            return Err(ContractError::Unauthorised);
        }
        Ok(())
    }

    fn load_security_config(env: &Env) -> SecurityConfig {
        env.storage()
            .instance()
            .get(&DataKey::SecurityConfig)
            .unwrap_or_else(Self::default_security_config)
    }

    fn save_security_config(env: &Env, cfg: &SecurityConfig) {
        env.storage().instance().set(&DataKey::SecurityConfig, cfg);
    }

    fn load_stats(env: &Env) -> BridgeSecurityStats {
        env.storage()
            .instance()
            .get(&DataKey::Stats)
            .unwrap_or_else(Self::default_stats)
    }

    fn save_stats(env: &Env, stats: &BridgeSecurityStats) {
        env.storage().instance().set(&DataKey::Stats, stats);
    }

    fn with_stats<F>(env: &Env, mutate: F)
    where
        F: FnOnce(&mut BridgeSecurityStats),
    {
        let mut stats = Self::load_stats(env);
        mutate(&mut stats);
        Self::save_stats(env, &stats);
    }

    fn validate_security_config(cfg: &SecurityConfig) -> Result<(), ContractError> {
        if cfg.min_validator_signatures == 0
            || cfg.min_finality_ledgers == 0
            || cfg.optimistic_delay_ledgers == 0
            || cfg.slash_bps == 0
            || cfg.slash_bps > 10_000
            || cfg.supported_message_version == 0
            || cfg.anomaly_close_threshold == 0
        {
            return Err(ContractError::InvalidSecurityConfig);
        }
        Ok(())
    }

    fn validate_id(id: &String) -> Result<(), ContractError> {
        let len = id.len();
        if len == 0 || len > MAX_ID_LEN {
            return Err(ContractError::InvalidBridgeIdLen);
        }
        Ok(())
    }

    fn load_bridge(env: &Env, bridge_id: &String) -> Result<BridgeConfig, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::Bridge(bridge_id.clone()))
            .ok_or(ContractError::BridgeNotFound)
    }

    fn save_bridge(env: &Env, bridge_id: &String, cfg: &BridgeConfig) {
        env.storage()
            .persistent()
            .set(&DataKey::Bridge(bridge_id.clone()), cfg);
    }

    fn bridge_list(env: &Env) -> Vec<String> {
        env.storage()
            .instance()
            .get(&DataKey::BridgeList)
            .unwrap_or_else(|| Vec::new(env))
    }

    fn validator_list(env: &Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::ValidatorList)
            .unwrap_or_else(|| Vec::new(env))
    }

    fn load_validator(env: &Env, validator: &Address) -> Result<ValidatorRecord, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::Validator(validator.clone()))
            .ok_or(ContractError::ValidatorNotFound)
    }

    fn save_validator(env: &Env, validator: &Address, record: &ValidatorRecord) {
        env.storage()
            .persistent()
            .set(&DataKey::Validator(validator.clone()), record);
    }

    fn load_channel_state(env: &Env, channel_id: &String) -> ChannelState {
        env.storage()
            .persistent()
            .get(&DataKey::Channel(channel_id.clone()))
            .unwrap_or_else(|| Self::default_channel_state(env))
    }

    fn save_channel_state(env: &Env, channel_id: &String, state: &ChannelState) {
        env.storage()
            .persistent()
            .set(&DataKey::Channel(channel_id.clone()), state);
    }

    fn record_channel_anomaly(env: &Env, channel_id: &String, reason: String) {
        let mut channel = Self::load_channel_state(env, channel_id);
        channel.anomaly_count = channel.anomaly_count.saturating_add(1);
        channel.reason = reason.clone();

        let cfg = Self::load_security_config(env);
        if channel.anomaly_count >= cfg.anomaly_close_threshold {
            channel.emergency_closed = true;
            Self::with_stats(env, |stats| {
                stats.emergency_closures = stats.emergency_closures.saturating_add(1);
            });
            ChannelEmergencyCloseEvent {
                channel_id: channel_id.clone(),
                closed: true,
                reason: reason.clone(),
                timestamp: env.ledger().timestamp(),
            }
            .publish(env);
        }

        Self::save_channel_state(env, channel_id, &channel);
        Self::with_stats(env, |stats| {
            stats.anomaly_events = stats.anomaly_events.saturating_add(1);
        });
        BridgeAnomalyEvent {
            channel_id: channel_id.clone(),
            anomaly_count: channel.anomaly_count,
            reason,
            timestamp: env.ledger().timestamp(),
        }
        .publish(env);
    }

    fn next_message_id(env: &Env) -> u64 {
        let current = env
            .storage()
            .instance()
            .get(&DataKey::NextMessageId)
            .unwrap_or(0u64);
        let next = current.saturating_add(1);
        env.storage().instance().set(&DataKey::NextMessageId, &next);
        next
    }

    fn load_message(env: &Env, message_id: u64) -> Result<CrossChainMessage, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::Message(message_id))
            .ok_or(ContractError::MessageNotFound)
    }

    fn save_message(env: &Env, message: &CrossChainMessage) {
        env.storage()
            .persistent()
            .set(&DataKey::Message(message.message_id), message);
    }

    fn message_payload_matches(a: &CrossChainMessage, b: &CrossChainMessage) -> bool {
        a.bridge_id == b.bridge_id
            && a.channel_id == b.channel_id
            && a.source_chain == b.source_chain
            && a.source_tx_id == b.source_tx_id
            && a.source_height == b.source_height
            && a.nonce == b.nonce
            && a.recipient == b.recipient
            && a.amount == b.amount
            && a.payload_version == b.payload_version
    }

    fn slash_amount(stake: i128, slash_bps: u64) -> i128 {
        let slash = stake.saturating_mul(slash_bps as i128) / 10_000i128;
        if slash == 0 && stake > 0 {
            1
        } else {
            slash
        }
    }

    pub fn register_bridge(
        env: Env,
        caller: Address,
        bridge_id: String,
        fee_bps: u64,
        min_amount: i128,
    ) -> Result<(), ContractError> {
        Self::require_admin(&env, &caller)?;
        Self::validate_id(&bridge_id)?;

        if fee_bps > MAX_FEE_BPS {
            return Err(ContractError::FeeTooHigh);
        }
        if min_amount < 0 {
            return Err(ContractError::NegativeMinAmount);
        }
        if env
            .storage()
            .persistent()
            .has(&DataKey::Bridge(bridge_id.clone()))
        {
            return Err(ContractError::BridgeAlreadyExists);
        }

        let cfg = BridgeConfig {
            bridge_id: bridge_id.clone(),
            fee_bps,
            min_amount,
            active: true,
            total_deposited: 0,
            total_withdrawn: 0,
        };
        Self::save_bridge(&env, &bridge_id, &cfg);

        let mut list = Self::bridge_list(&env);
        list.push_back(bridge_id.clone());
        env.storage().instance().set(&DataKey::BridgeList, &list);

        BridgeRegisteredEvent {
            bridge_id: bridge_id.clone(),
            fee_bps,
            min_amount,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        log!(&env, "register_bridge {}", bridge_id);
        Ok(())
    }

    pub fn set_bridge_fee(
        env: Env,
        caller: Address,
        bridge_id: String,
        fee_bps: u64,
    ) -> Result<(), ContractError> {
        Self::require_admin(&env, &caller)?;
        if fee_bps > MAX_FEE_BPS {
            return Err(ContractError::FeeTooHigh);
        }

        let mut cfg = Self::load_bridge(&env, &bridge_id)?;
        cfg.fee_bps = fee_bps;
        Self::save_bridge(&env, &bridge_id, &cfg);

        BridgeFeeUpdatedEvent {
            bridge_id,
            fee_bps,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn set_bridge_active(
        env: Env,
        caller: Address,
        bridge_id: String,
        active: bool,
    ) -> Result<(), ContractError> {
        Self::require_admin(&env, &caller)?;
        let mut cfg = Self::load_bridge(&env, &bridge_id)?;
        cfg.active = active;
        Self::save_bridge(&env, &bridge_id, &cfg);

        BridgeActiveUpdatedEvent {
            bridge_id,
            active,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn bridge_deposit(
        env: Env,
        sender: Address,
        bridge_id: String,
        amount: i128,
    ) -> Result<i128, ContractError> {
        sender.require_auth();

        if env
            .storage()
            .persistent()
            .get::<DataKey, bool>(&DataKey::BridgeAcceptancePaused)
            .unwrap_or(false)
        {
            return Err(ContractError::BridgeAcceptancePaused);
        }

        if amount <= 0 {
            return Err(ContractError::AmountNotPositive);
        }

        let mut cfg = Self::load_bridge(&env, &bridge_id)?;
        if !cfg.active {
            return Err(ContractError::BridgeInactive);
        }
        if amount < cfg.min_amount {
            return Err(ContractError::AmountBelowMinimum);
        }

        let fee = Self::compute_fee(env.clone(), amount, cfg.fee_bps);
        let net = amount.checked_sub(fee).ok_or(ContractError::Overflow)?;
        cfg.total_deposited = cfg
            .total_deposited
            .checked_add(amount)
            .ok_or(ContractError::Overflow)?;
        Self::save_bridge(&env, &bridge_id, &cfg);

        BridgeDepositEvent {
            bridge_id: bridge_id.clone(),
            amount,
            fee,
            net,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        log!(
            &env,
            "bridge_deposit {} amount={} fee={} net={}",
            bridge_id,
            amount,
            fee,
            net
        );

        Ok(net)
    }

    pub fn bridge_withdraw(
        env: Env,
        caller: Address,
        bridge_id: String,
        recipient: Address,
        amount: i128,
    ) -> Result<(), ContractError> {
        Self::require_admin(&env, &caller)?;

        if amount <= 0 {
            return Err(ContractError::AmountNotPositive);
        }

        let mut cfg = Self::load_bridge(&env, &bridge_id)?;
        if amount < cfg.min_amount {
            return Err(ContractError::AmountBelowMinimum);
        }

        cfg.total_withdrawn = cfg
            .total_withdrawn
            .checked_add(amount)
            .ok_or(ContractError::Overflow)?;
        Self::save_bridge(&env, &bridge_id, &cfg);

        BridgeWithdrawalEvent {
            bridge_id: bridge_id.clone(),
            amount,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        log!(
            &env,
            "bridge_withdraw {} -> {} amount={}",
            bridge_id,
            recipient,
            amount
        );
        Ok(())
    }

    pub fn set_bridge_acceptance_paused(
        env: Env,
        caller: Address,
        paused: bool,
    ) -> Result<(), ContractError> {
        Self::require_admin(&env, &caller)?;
        env.storage()
            .persistent()
            .set(&DataKey::BridgeAcceptancePaused, &paused);

        BridgeAcceptancePauseEvent {
            paused,
            admin: caller,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn is_bridge_acceptance_paused(env: Env) -> bool {
        env.storage()
            .persistent()
            .get::<DataKey, bool>(&DataKey::BridgeAcceptancePaused)
            .unwrap_or(false)
    }

    pub fn set_bridge_security_config(
        env: Env,
        caller: Address,
        cfg: SecurityConfig,
    ) -> Result<(), ContractError> {
        Self::require_admin(&env, &caller)?;
        Self::validate_security_config(&cfg)?;
        Self::save_security_config(&env, &cfg);

        SecurityConfigUpdatedEvent {
            min_validator_signatures: cfg.min_validator_signatures,
            min_finality_ledgers: cfg.min_finality_ledgers,
            optimistic_delay_ledgers: cfg.optimistic_delay_ledgers,
            slash_bps: cfg.slash_bps,
            supported_message_version: cfg.supported_message_version,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn register_validator(
        env: Env,
        caller: Address,
        validator: Address,
        stake: i128,
    ) -> Result<(), ContractError> {
        Self::require_admin(&env, &caller)?;
        if stake <= 0 {
            return Err(ContractError::InvalidValidatorStake);
        }
        if env
            .storage()
            .persistent()
            .has(&DataKey::Validator(validator.clone()))
        {
            return Err(ContractError::ValidatorAlreadyExists);
        }

        let record = ValidatorRecord {
            stake,
            active: true,
            slashed_total: 0,
            approvals: 0,
        };
        Self::save_validator(&env, &validator, &record);

        let mut validators = Self::validator_list(&env);
        validators.push_back(validator.clone());
        env.storage()
            .instance()
            .set(&DataKey::ValidatorList, &validators);

        ValidatorUpdatedEvent {
            validator,
            stake,
            active: true,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn set_validator_active(
        env: Env,
        caller: Address,
        validator: Address,
        active: bool,
    ) -> Result<(), ContractError> {
        Self::require_admin(&env, &caller)?;
        let mut record = Self::load_validator(&env, &validator)?;
        record.active = active;
        let stake = record.stake;
        Self::save_validator(&env, &validator, &record);

        ValidatorUpdatedEvent {
            validator,
            stake,
            active,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn submit_cross_chain_message(
        env: Env,
        caller: Address,
        input: CrossChainMessageInput,
    ) -> Result<u64, ContractError> {
        caller.require_auth();
        Self::validate_id(&input.bridge_id)?;
        Self::validate_id(&input.channel_id)?;
        Self::validate_id(&input.source_chain)?;

        if input.amount <= 0 {
            return Err(ContractError::AmountNotPositive);
        }

        let cfg = Self::load_security_config(&env);
        if input.payload_version != cfg.supported_message_version {
            Self::record_channel_anomaly(
                &env,
                &input.channel_id,
                String::from_str(&env, "unsupported_message_version"),
            );
            return Err(ContractError::InvalidMessageVersion);
        }

        let bridge = Self::load_bridge(&env, &input.bridge_id)?;
        if input.amount < bridge.min_amount {
            return Err(ContractError::AmountBelowMinimum);
        }

        let mut channel = Self::load_channel_state(&env, &input.channel_id);
        if channel.emergency_closed {
            return Err(ContractError::ChannelClosed);
        }

        let source_key = SourceMessageKey {
            channel_id: input.channel_id.clone(),
            source_chain: input.source_chain.clone(),
            source_tx_id: input.source_tx_id.clone(),
            nonce: input.nonce,
        };
        if env
            .storage()
            .persistent()
            .has(&DataKey::MessageSource(source_key.clone()))
        {
            Self::with_stats(&env, |stats| {
                stats.replay_rejections = stats.replay_rejections.saturating_add(1);
            });
            Self::record_channel_anomaly(
                &env,
                &input.channel_id,
                String::from_str(&env, "replay_detected"),
            );
            return Err(ContractError::ReplayDetected);
        }
        if channel.last_observed_nonce != 0 && input.nonce <= channel.last_observed_nonce {
            Self::record_channel_anomaly(
                &env,
                &input.channel_id,
                String::from_str(&env, "out_of_order_nonce"),
            );
            return Err(ContractError::InvalidMessageOrdering);
        }
        if channel.last_observed_height != 0 && input.source_height < channel.last_observed_height {
            Self::record_channel_anomaly(
                &env,
                &input.channel_id,
                String::from_str(&env, "source_height_regression"),
            );
            return Err(ContractError::InvalidMessageOrdering);
        }

        let message_id = Self::next_message_id(&env);
        let wait_ledgers = max(cfg.min_finality_ledgers, cfg.optimistic_delay_ledgers);
        let message = CrossChainMessage {
            message_id,
            bridge_id: input.bridge_id.clone(),
            channel_id: input.channel_id.clone(),
            source_chain: input.source_chain.clone(),
            source_tx_id: input.source_tx_id,
            source_height: input.source_height,
            nonce: input.nonce,
            recipient: input.recipient,
            amount: input.amount,
            payload_version: input.payload_version,
            submitted_at_ledger: env.ledger().sequence(),
            finality_ledger: env.ledger().sequence().saturating_add(wait_ledgers),
            approvals: 0,
            rejections: 0,
            executed: false,
            invalidated: false,
        };

        channel.last_observed_nonce = input.nonce;
        channel.last_observed_height = input.source_height;
        Self::save_channel_state(&env, &input.channel_id, &channel);
        env.storage()
            .persistent()
            .set(&DataKey::MessageSource(source_key), &message_id);
        Self::save_message(&env, &message);
        Self::with_stats(&env, |stats| {
            stats.total_messages = stats.total_messages.saturating_add(1);
        });

        log!(
            &env,
            "submit_cross_chain_message id={} bridge={} channel={} source={} nonce={} amount={}",
            message_id,
            input.bridge_id,
            input.channel_id,
            input.source_chain,
            input.nonce,
            input.amount
        );

        Ok(message_id)
    }

    pub fn attest_cross_chain_message(
        env: Env,
        validator: Address,
        message_id: u64,
        approve: bool,
    ) -> Result<(), ContractError> {
        validator.require_auth();

        let mut validator_record = Self::load_validator(&env, &validator)?;
        if !validator_record.active {
            return Err(ContractError::ValidatorInactive);
        }

        let mut message = Self::load_message(&env, message_id)?;
        if message.executed {
            return Err(ContractError::MessageAlreadyExecuted);
        }
        if message.invalidated {
            return Err(ContractError::MessageInvalidated);
        }

        let attestation_key = DataKey::MessageAttestation((message_id, validator.clone()));
        if env.storage().persistent().has(&attestation_key) {
            return Err(ContractError::DuplicateAttestation);
        }

        env.storage().persistent().set(&attestation_key, &approve);
        if approve {
            message.approvals = message.approvals.saturating_add(1);
            validator_record.approvals = validator_record.approvals.saturating_add(1);
            Self::save_validator(&env, &validator, &validator_record);
        } else {
            message.rejections = message.rejections.saturating_add(1);
            Self::record_channel_anomaly(
                &env,
                &message.channel_id,
                String::from_str(&env, "validator_rejection"),
            );
        }

        Self::save_message(&env, &message);
        log!(
            &env,
            "attest_cross_chain_message id={} validator={} approve={} approvals={} rejections={}",
            message_id,
            validator,
            approve,
            message.approvals,
            message.rejections
        );
        Ok(())
    }

    pub fn execute_verified_withdrawal(
        env: Env,
        caller: Address,
        message_id: u64,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let cfg = Self::load_security_config(&env);
        let mut message = Self::load_message(&env, message_id)?;
        if message.executed {
            return Err(ContractError::MessageAlreadyExecuted);
        }
        if message.invalidated {
            return Err(ContractError::MessageInvalidated);
        }
        if message.approvals < cfg.min_validator_signatures {
            return Err(ContractError::QuorumNotReached);
        }
        if env.ledger().sequence() < message.finality_ledger {
            return Err(ContractError::MessageNotFinal);
        }
        if message.rejections > 0 {
            Self::with_stats(&env, |stats| {
                stats.rejected_messages = stats.rejected_messages.saturating_add(1);
            });
            return Err(ContractError::MessageRejected);
        }

        let channel = Self::load_channel_state(&env, &message.channel_id);
        if channel.emergency_closed {
            return Err(ContractError::ChannelClosed);
        }

        let mut bridge = Self::load_bridge(&env, &message.bridge_id)?;
        if message.amount < bridge.min_amount {
            return Err(ContractError::AmountBelowMinimum);
        }

        bridge.total_withdrawn = bridge
            .total_withdrawn
            .checked_add(message.amount)
            .ok_or(ContractError::Overflow)?;
        Self::save_bridge(&env, &message.bridge_id, &bridge);

        message.executed = true;
        Self::save_message(&env, &message);
        Self::with_stats(&env, |stats| {
            stats.executed_messages = stats.executed_messages.saturating_add(1);
        });

        BridgeWithdrawalEvent {
            bridge_id: message.bridge_id.clone(),
            amount: message.amount,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        log!(
            &env,
            "execute_verified_withdrawal id={} bridge={} recipient={} amount={}",
            message_id,
            message.bridge_id,
            message.recipient,
            message.amount
        );
        Ok(())
    }

    pub fn slash_conflicting_messages(
        env: Env,
        caller: Address,
        first_message_id: u64,
        second_message_id: u64,
    ) -> Result<Vec<Address>, ContractError> {
        caller.require_auth();

        let mut first = Self::load_message(&env, first_message_id)?;
        let mut second = Self::load_message(&env, second_message_id)?;
        let same_source = first.channel_id == second.channel_id
            && first.source_chain == second.source_chain
            && (first.nonce == second.nonce || first.source_tx_id == second.source_tx_id);

        if !same_source || Self::message_payload_matches(&first, &second) {
            return Err(ContractError::ConflictingMessages);
        }

        let validators = Self::validator_list(&env);
        let cfg = Self::load_security_config(&env);
        let mut slashed = Vec::new(&env);

        for validator in validators.iter() {
            let first_vote =
                env.storage()
                    .persistent()
                    .get::<DataKey, bool>(&DataKey::MessageAttestation((
                        first_message_id,
                        validator.clone(),
                    )));
            let second_vote =
                env.storage()
                    .persistent()
                    .get::<DataKey, bool>(&DataKey::MessageAttestation((
                        second_message_id,
                        validator.clone(),
                    )));

            if first_vote == Some(true) && second_vote == Some(true) {
                let mut record = Self::load_validator(&env, &validator)?;
                let slash_amount = Self::slash_amount(record.stake, cfg.slash_bps);
                record.stake = record.stake.saturating_sub(slash_amount);
                record.slashed_total = record.slashed_total.saturating_add(slash_amount);
                if record.stake <= 0 {
                    record.active = false;
                }
                Self::save_validator(&env, &validator, &record);
                Self::with_stats(&env, |stats| {
                    stats.slashes = stats.slashes.saturating_add(1);
                });
                ValidatorSlashedEvent {
                    validator: validator.clone(),
                    amount: slash_amount,
                    remaining_stake: record.stake,
                    timestamp: env.ledger().timestamp(),
                }
                .publish(&env);
                slashed.push_back(validator);
            }
        }

        if slashed.is_empty() {
            return Err(ContractError::NothingToSlash);
        }

        first.invalidated = true;
        second.invalidated = true;
        Self::save_message(&env, &first);
        Self::save_message(&env, &second);
        Self::record_channel_anomaly(
            &env,
            &first.channel_id,
            String::from_str(&env, "conflicting_message_attestations"),
        );
        Ok(slashed)
    }

    pub fn close_channel_emergency(
        env: Env,
        caller: Address,
        channel_id: String,
        reason: String,
    ) -> Result<(), ContractError> {
        Self::require_admin(&env, &caller)?;
        let mut state = Self::load_channel_state(&env, &channel_id);
        state.emergency_closed = true;
        state.reason = reason.clone();
        Self::save_channel_state(&env, &channel_id, &state);
        Self::with_stats(&env, |stats| {
            stats.emergency_closures = stats.emergency_closures.saturating_add(1);
        });

        ChannelEmergencyCloseEvent {
            channel_id,
            closed: true,
            reason,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn reopen_channel(
        env: Env,
        caller: Address,
        channel_id: String,
    ) -> Result<(), ContractError> {
        Self::require_admin(&env, &caller)?;
        let mut state = Self::load_channel_state(&env, &channel_id);
        state.emergency_closed = false;
        state.reason = Self::empty_string(&env);
        Self::save_channel_state(&env, &channel_id, &state);

        ChannelEmergencyCloseEvent {
            channel_id,
            closed: false,
            reason: Self::empty_string(&env),
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        Ok(())
    }

    pub fn transfer_admin(
        env: Env,
        caller: Address,
        new_admin: Address,
    ) -> Result<(), ContractError> {
        Self::require_admin(&env, &caller)?;
        env.storage().instance().set(&ADMIN_KEY, &new_admin);
        log!(&env, "transfer_admin new={}", new_admin);
        Ok(())
    }

    pub fn get_bridge_config(env: Env, bridge_id: String) -> Result<BridgeConfig, ContractError> {
        Self::load_bridge(&env, &bridge_id)
    }

    pub fn list_bridges(env: Env) -> Vec<String> {
        Self::bridge_list(&env)
    }

    pub fn get_admin(env: Env) -> Result<Address, ContractError> {
        Self::load_admin(&env)
    }

    pub fn compute_fee(env: Env, amount: i128, fee_bps: u64) -> i128 {
        let amount_256 = I256::from_i128(&env, amount);
        let bps_256 = I256::from_i128(&env, fee_bps as i128);

        amount_256
            .mul(&bps_256)
            .div(&I256::from_i128(&env, 10_000))
            .to_i128()
            .unwrap_or(0)
    }

    pub fn get_bridge_security_config(env: Env) -> SecurityConfig {
        Self::load_security_config(&env)
    }

    pub fn get_validator(env: Env, validator: Address) -> Result<ValidatorRecord, ContractError> {
        Self::load_validator(&env, &validator)
    }

    pub fn list_validators(env: Env) -> Vec<Address> {
        Self::validator_list(&env)
    }

    pub fn get_channel_state(env: Env, channel_id: String) -> ChannelState {
        Self::load_channel_state(&env, &channel_id)
    }

    pub fn get_cross_chain_message(
        env: Env,
        message_id: u64,
    ) -> Result<CrossChainMessage, ContractError> {
        Self::load_message(&env, message_id)
    }

    pub fn get_bridge_security_stats(env: Env) -> BridgeSecurityStats {
        Self::load_stats(&env)
    }

    pub fn get_bridge_analytics(
        env: Env,
        bridge_id: String,
    ) -> Result<BridgeAnalytics, ContractError> {
        let bridge = Self::load_bridge(&env, &bridge_id)?;
        let validators = Self::validator_list(&env);
        let active_validators = validators
            .iter()
            .filter_map(|v| Self::load_validator(&env, &v).ok())
            .filter(|r| r.active)
            .count() as u32;

        Ok(BridgeAnalytics {
            bridge_id: bridge_id.clone(),
            total_volume_deposited: bridge.total_deposited,
            total_volume_withdrawn: bridge.total_withdrawn,
            net_volume: bridge
                .total_deposited
                .saturating_sub(bridge.total_withdrawn),
            fee_bps: bridge.fee_bps,
            total_fees_collected: Self::compute_fee(
                env.clone(),
                bridge.total_deposited,
                bridge.fee_bps,
            ),
            active_validators,
            total_validators: validators.len() as u32,
            is_active: bridge.active,
        })
    }

    pub fn get_all_bridge_analytics(env: Env) -> Vec<BridgeAnalytics> {
        let bridges = Self::bridge_list(&env);
        let mut analytics = Vec::new(&env);

        for bridge_id in bridges.iter() {
            if let Ok(stats) = Self::get_bridge_analytics(env.clone(), bridge_id.clone()) {
                analytics.push_back(stats);
            }
        }

        analytics
    }

    pub fn upgrade_init(
        env: Env,
        admin: Address,
        current_wasm_hash: BytesN<32>,
        required_approvals: u32,
    ) {
        stellarlend_common::upgrade::UpgradeManager::init(
            env,
            admin,
            current_wasm_hash,
            required_approvals,
        );
    }

    pub fn upgrade_add_approver(env: Env, caller: Address, approver: Address) {
        stellarlend_common::upgrade::UpgradeManager::add_approver(env, caller, approver);
    }

    pub fn upgrade_remove_approver(env: Env, caller: Address, approver: Address) {
        stellarlend_common::upgrade::UpgradeManager::remove_approver(env, caller, approver);
    }

    pub fn upgrade_propose(
        env: Env,
        caller: Address,
        new_wasm_hash: BytesN<32>,
        new_version: u32,
    ) -> u64 {
        stellarlend_common::upgrade::UpgradeManager::upgrade_propose(
            env,
            caller,
            new_wasm_hash,
            new_version,
        )
    }

    pub fn upgrade_approve(env: Env, caller: Address, proposal_id: u64) -> u32 {
        stellarlend_common::upgrade::UpgradeManager::upgrade_approve(env, caller, proposal_id)
    }

    pub fn upgrade_execute(env: Env, caller: Address, proposal_id: u64) {
        stellarlend_common::upgrade::UpgradeManager::upgrade_execute(env, caller, proposal_id);
    }

    pub fn upgrade_rollback(env: Env, caller: Address, proposal_id: u64) {
        stellarlend_common::upgrade::UpgradeManager::upgrade_rollback(env, caller, proposal_id);
    }

    pub fn upgrade_status(
        env: Env,
        proposal_id: u64,
    ) -> stellarlend_common::upgrade::UpgradeStatus {
        stellarlend_common::upgrade::UpgradeManager::upgrade_status(env, proposal_id)
    }

    pub fn current_wasm_hash(env: Env) -> BytesN<32> {
        stellarlend_common::upgrade::UpgradeManager::current_wasm_hash(env)
    }

    pub fn current_version(env: Env) -> u32 {
        stellarlend_common::upgrade::UpgradeManager::current_version(env)
    }
}
