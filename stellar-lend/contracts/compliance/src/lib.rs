#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env, Map, Symbol, Vec, BytesN};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ComplianceError {
    Unauthorized = 1,
    AddressSanctioned = 2,
    TransactionLimitExceeded = 3,
    GeographicRestricted = 4,
    KYCRequired = 5,
    KYCExpired = 6,
    TransactionTooLarge = 7,
    DailyLimitExceeded = 8,
    WeeklyLimitExceeded = 9,
    AlreadySanctioned = 10,
    AddressNotSanctioned = 11,
    InvalidJurisdiction = 12,
    SARAlreadyFiled = 13,
    CompliancePaused = 14,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SanctionsEntry {
    pub address: Address,
    pub source: Symbol,
    pub reason: Symbol,
    pub sanctioned_at: u64,
    pub expires_at: Option<u64>,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TransactionLimits {
    pub daily_limit: i128,
    pub weekly_limit: i128,
    pub max_single_tx: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ComplianceConfig {
    pub admin: Address,
    pub paused: bool,
    pub default_limits: TransactionLimits,
    pub restricted_jurisdictions: Vec<Symbol>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct KycVerification {
    pub address: Address,
    pub verified: bool,
    pub tier: u32,
    pub verified_at: u64,
    pub expires_at: u64,
    pub jurisdiction: Symbol,
    pub kyc_provider: Symbol,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ComplianceEvent {
    pub event_type: Symbol,
    pub address: Address,
    pub amount: Option<i128>,
    pub asset: Option<Address>,
    pub timestamp: u64,
    pub details: Symbol,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SuspiciousActivityReport {
    pub sar_id: u64,
    pub address: Address,
    pub reason: Symbol,
    pub amount: i128,
    pub asset: Address,
    pub filed_at: u64,
    pub filed_by: Address,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TxVolume {
    pub address: Address,
    pub daily_volume: i128,
    pub weekly_volume: i128,
    pub last_tx_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Config,
    SanctionedAddress(Address),
    SanctionedList,
    KycData(Address),
    TxVolume(Address),
    TxLimits(Address),
    SarList,
    SarById(u64),
    ComplianceEvents,
    TransactionLog(u64),
    NextSarId,
    NextEventId,
    NextTxLogId,
}

#[contract]
pub struct ComplianceContract;

#[contractimpl]
impl ComplianceContract {
    pub fn initialize(env: Env, admin: Address) -> Result<(), ComplianceError> {
        if env.storage().persistent().has(&DataKey::Config) {
            return Err(ComplianceError::Unauthorized);
        }
        admin.require_auth();
        let config = ComplianceConfig {
            admin: admin.clone(),
            paused: false,
            default_limits: TransactionLimits {
                daily_limit: 1_000_000_000_000,
                weekly_limit: 5_000_000_000_000,
                max_single_tx: 500_000_000_000,
            },
            restricted_jurisdictions: Vec::new(&env),
        };
        env.storage().persistent().set(&DataKey::Config, &config);
        env.storage().persistent().set(&DataKey::NextSarId, &0u64);
        env.storage().persistent().set(&DataKey::NextEventId, &0u64);
        env.storage().persistent().set(&DataKey::NextTxLogId, &0u64);
        Ok(())
    }

    pub fn add_sanction(
        env: Env,
        admin: Address,
        address: Address,
        source: Symbol,
        reason: Symbol,
        expires_at: Option<u64>,
    ) -> Result<(), ComplianceError> {
        Self::require_admin(&env, &admin)?;
        if env.storage().persistent().has(&DataKey::SanctionedAddress(address.clone())) {
            return Err(ComplianceError::AlreadySanctioned);
        }
        let entry = SanctionsEntry {
            address: address.clone(),
            source,
            reason,
            sanctioned_at: env.ledger().timestamp(),
            expires_at,
            active: true,
        };
        env.storage().persistent().set(&DataKey::SanctionedAddress(address.clone()), &entry);
        Self::record_event(&env, &Symbol::new(&env, "SANCTION_ADDED"), &address, None::<i128>, None::<Address>, &reason);
        Ok(())
    }

    pub fn remove_sanction(env: Env, admin: Address, address: Address) -> Result<(), ComplianceError> {
        Self::require_admin(&env, &admin)?;
        let key = DataKey::SanctionedAddress(address.clone());
        let mut entry: SanctionsEntry = env.storage().persistent().get(&key).ok_or(ComplianceError::AddressNotSanctioned)?;
        entry.active = false;
        env.storage().persistent().set(&key, &entry);
        Self::record_event(&env, &Symbol::new(&env, "SANCTION_REMOVED"), &address, None::<i128>, None::<Address>, &Symbol::new(&env, "admin_remove"));
        Ok(())
    }

    pub fn check_sanctioned(env: Env, address: Address) -> bool {
        match env.storage().persistent().get::<DataKey, SanctionsEntry>(&DataKey::SanctionedAddress(address)) {
            Some(entry) => entry.active && entry.expires_at.map_or(true, |exp| exp > env.ledger().timestamp()),
            None => false,
        }
    }

    pub fn set_kyc_verification(
        env: Env,
        admin: Address,
        address: Address,
        tier: u32,
        jurisdiction: Symbol,
        kyc_provider: Symbol,
        validity_secs: u64,
    ) -> Result<(), ComplianceError> {
        Self::require_admin(&env, &admin)?;
        let now = env.ledger().timestamp();
        let kyc = KycVerification {
            address: address.clone(),
            verified: true,
            tier,
            verified_at: now,
            expires_at: now + validity_secs,
            jurisdiction,
            kyc_provider,
        };
        env.storage().persistent().set(&DataKey::KycData(address.clone()), &kyc);
        Self::record_event(&env, &Symbol::new(&env, "KYC_VERIFIED"), &address, None::<i128>, None::<Address>, &Symbol::new(&env, "kyc_set"));
        Ok(())
    }

    pub fn revoke_kyc(env: Env, admin: Address, address: Address) -> Result<(), ComplianceError> {
        Self::require_admin(&env, &admin)?;
        let mut kyc: KycVerification = env.storage().persistent().get(&DataKey::KycData(address.clone())).ok_or(ComplianceError::KYCRequired)?;
        kyc.verified = false;
        env.storage().persistent().set(&DataKey::KycData(address), &kyc);
        Ok(())
    }

    pub fn check_kyc(env: Env, address: Address) -> bool {
        match env.storage().persistent().get::<DataKey, KycVerification>(&DataKey::KycData(address)) {
            Some(kyc) => kyc.verified && kyc.expires_at > env.ledger().timestamp(),
            None => false,
        }
    }

    pub fn get_kyc(env: Env, address: Address) -> Option<KycVerification> {
        env.storage().persistent().get(&DataKey::KycData(address))
    }

    pub fn set_tx_limits(env: Env, admin: Address, address: Address, limits: TransactionLimits) -> Result<(), ComplianceError> {
        Self::require_admin(&env, &admin)?;
        env.storage().persistent().set(&DataKey::TxLimits(address), &limits);
        Ok(())
    }

    pub fn get_tx_limits(env: Env, address: Address) -> TransactionLimits {
        env.storage().persistent().get(&DataKey::TxLimits(address.clone()))
            .unwrap_or_else(|| Self::get_config(&env).default_limits)
    }

    pub fn check_transaction(
        env: Env,
        from: Address,
        to: Address,
        amount: i128,
        asset: Address,
    ) -> Result<(), ComplianceError> {
        let config = Self::get_config(&env);
        if config.paused {
            return Err(ComplianceError::CompliancePaused);
        }
        if Self::check_sanctioned(env.clone(), from.clone()) || Self::check_sanctioned(env.clone(), to.clone()) {
            return Err(ComplianceError::AddressSanctioned);
        }
        let limits = Self::get_tx_limits(env.clone(), from.clone());
        if amount > limits.max_single_tx {
            return Err(ComplianceError::TransactionTooLarge);
        }
        if let Some(kyc) = env.storage().persistent().get::<DataKey, KycVerification>(&DataKey::KycData(from.clone())) {
            for jurisdiction in config.restricted_jurisdictions.iter() {
                if kyc.jurisdiction == jurisdiction {
                    return Err(ComplianceError::GeographicRestricted);
                }
            }
        }
        let mut volume = Self::get_tx_volume(env.clone(), &from);
        volume.daily_volume += amount;
        volume.weekly_volume += amount;
        let now = env.ledger().timestamp();
        if now.saturating_sub(volume.last_tx_timestamp) > 86400 {
            volume.daily_volume = amount;
        }
        if now.saturating_sub(volume.last_tx_timestamp) > 604800 {
            volume.weekly_volume = amount;
        }
        volume.last_tx_timestamp = now;
        env.storage().persistent().set(&DataKey::TxVolume(from.clone()), &volume);
        if volume.daily_volume > limits.daily_limit {
            return Err(ComplianceError::DailyLimitExceeded);
        }
        if volume.weekly_volume > limits.weekly_limit {
            return Err(ComplianceError::WeeklyLimitExceeded);
        }
        Self::record_event(&env, &Symbol::new(&env, "TX_CHECKED"), &from, Some(amount), Some(asset), &Symbol::new(&env, "passed"));
        Ok(())
    }

    pub fn get_tx_volume(env: Env, address: &Address) -> TxVolume {
        env.storage().persistent().get(&DataKey::TxVolume(address.clone())).unwrap_or(TxVolume {
            address: address.clone(),
            daily_volume: 0,
            weekly_volume: 0,
            last_tx_timestamp: 0,
        })
    }

    pub fn file_sar(
        env: Env,
        admin: Address,
        address: Address,
        reason: Symbol,
        amount: i128,
        asset: Address,
    ) -> Result<u64, ComplianceError> {
        Self::require_admin(&env, &admin)?;
        let sar_id: u64 = env.storage().persistent().get(&DataKey::NextSarId).unwrap_or(0);
        let sar = SuspiciousActivityReport {
            sar_id,
            address: address.clone(),
            reason,
            amount,
            asset,
            filed_at: env.ledger().timestamp(),
            filed_by: admin,
        };
        env.storage().persistent().set(&DataKey::SarById(sar_id), &sar);
        env.storage().persistent().set(&DataKey::NextSarId, &(sar_id + 1));
        Self::record_event(&env, &Symbol::new(&env, "SAR_FILED"), &address, Some(amount), None::<Address>, &Symbol::new(&env, "sar_filed"));
        Ok(sar_id)
    }

    pub fn get_sar(env: Env, sar_id: u64) -> Option<SuspiciousActivityReport> {
        env.storage().persistent().get(&DataKey::SarById(sar_id))
    }

    pub fn add_restricted_jurisdiction(env: Env, admin: Address, jurisdiction: Symbol) -> Result<(), ComplianceError> {
        Self::require_admin(&env, &admin)?;
        let mut config = Self::get_config(&env);
        if !config.restricted_jurisdictions.contains(&jurisdiction) {
            config.restricted_jurisdictions.push_back(jurisdiction);
        }
        env.storage().persistent().set(&DataKey::Config, &config);
        Ok(())
    }

    pub fn remove_restricted_jurisdiction(env: Env, admin: Address, jurisdiction: Symbol) -> Result<(), ComplianceError> {
        Self::require_admin(&env, &admin)?;
        let mut config = Self::get_config(&env);
        let mut new_list: Vec<Symbol> = Vec::new(&env);
        for j in config.restricted_jurisdictions.iter() {
            if j != jurisdiction {
                new_list.push_back(j);
            }
        }
        config.restricted_jurisdictions = new_list;
        env.storage().persistent().set(&DataKey::Config, &config);
        Ok(())
    }

    pub fn pause(env: Env, admin: Address) -> Result<(), ComplianceError> {
        Self::require_admin(&env, &admin)?;
        let mut config = Self::get_config(&env);
        config.paused = true;
        env.storage().persistent().set(&DataKey::Config, &config);
        Ok(())
    }

    pub fn unpause(env: Env, admin: Address) -> Result<(), ComplianceError> {
        Self::require_admin(&env, &admin)?;
        let mut config = Self::get_config(&env);
        config.paused = false;
        env.storage().persistent().set(&DataKey::Config, &config);
        Ok(())
    }

    pub fn get_config(env: Env) -> ComplianceConfig {
        env.storage().persistent().get(&DataKey::Config).unwrap()
    }

    fn require_admin(env: &Env, admin: &Address) -> Result<(), ComplianceError> {
        let config: ComplianceConfig = env.storage().persistent().get(&DataKey::Config).ok_or(ComplianceError::Unauthorized)?;
        if admin != &config.admin {
            return Err(ComplianceError::Unauthorized);
        }
        admin.require_auth();
        Ok(())
    }

    fn record_event(
        env: &Env,
        event_type: &Symbol,
        address: &Address,
        amount: Option<i128>,
        asset: Option<Address>,
        details: &Symbol,
    ) {
        let event_id: u64 = env.storage().persistent().get(&DataKey::NextEventId).unwrap_or(0);
        let event = ComplianceEvent {
            event_type: event_type.clone(),
            address: address.clone(),
            amount,
            asset,
            timestamp: env.ledger().timestamp(),
            details: details.clone(),
        };
        env.storage().persistent().set(&DataKey::TransactionLog(event_id), &event);
        env.storage().persistent().set(&DataKey::NextEventId, &(event_id + 1));
    }
}
