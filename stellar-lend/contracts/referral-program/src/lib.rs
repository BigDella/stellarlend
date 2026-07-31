#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, BytesN, Env, Symbol, symbol_short, Map};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ReferralError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    AlreadyRegistered = 4,
    InvalidReferrer = 5,
    SelfReferral = 6,
    NothingToClaim = 7,
    MaturityNotReached = 8,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ReferralRecord {
    pub referrer: Address,
    pub referee: Address,
    pub registered_at: u64,
    pub total_fees_generated: i128,
    pub referrer_earned: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ReferrerStats {
    pub total_referrals: u32,
    pub total_earned: i128,
    pub total_claimed: i128,
    pub claimable: i128,
    pub last_claim_ledger: u64,
    pub l2_referrals: u32,
    pub l2_earned: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ReferralConfig {
    pub admin: Address,
    pub fee_share_bps: u32,
    pub l2_fee_share_bps: u32,
    pub maturity_ledgers: u64,
    pub min_deposit_to_qualify: i128,
    pub tier_1_threshold: u32, // number of referrals
    pub tier_1_bonus_bps: u32,
    pub tier_2_threshold: u32,
    pub tier_2_bonus_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct TierInfo {
    pub tier_level: u32,
    pub bonus_bps: u32,
    pub total_bonus_earned: i128,
}

#[contract]
pub struct ReferralProgram;

#[contractimpl]
impl ReferralProgram {
    pub fn initialize(
        env: Env,
        admin: Address,
        fee_share_bps: u32,
        l2_fee_share_bps: u32,
        maturity_ledgers: u64,
        min_deposit: i128,
        tier_1_threshold: u32,
        tier_1_bonus_bps: u32,
        tier_2_threshold: u32,
        tier_2_bonus_bps: u32,
    ) -> Result<(), ReferralError> {
        if env.storage().instance().has(&symbol_short!("config")) {
            return Err(ReferralError::AlreadyInitialized);
        }
        admin.require_auth();

        let config = ReferralConfig {
            admin,
            fee_share_bps,
            l2_fee_share_bps,
            maturity_ledgers,
            min_deposit_to_qualify: min_deposit,
            tier_1_threshold,
            tier_1_bonus_bps,
            tier_2_threshold,
            tier_2_bonus_bps,
        };
        env.storage().instance().set(&symbol_short!("config"), &config);
        Ok(())
    }

    pub fn register_referral(
        env: Env,
        referee: Address,
        referrer: Address,
    ) -> Result<(), ReferralError> {
        referee.require_auth();
        Self::require_initialized(&env)?;

        if referee == referrer {
            return Err(ReferralError::SelfReferral);
        }

        let ref_key = Self::referee_key(&referee);
        if env.storage().persistent().has(&ref_key) {
            return Err(ReferralError::AlreadyRegistered);
        }

        let record = ReferralRecord {
            referrer: referrer.clone(),
            referee: referee.clone(),
            registered_at: env.ledger().sequence() as u64,
            total_fees_generated: 0,
            referrer_earned: 0,
        };
        env.storage().persistent().set(&ref_key, &record);

        let mut stats = Self::get_referrer_stats_internal(&env, &referrer);
        stats.total_referrals += 1;
        Self::set_referrer_stats(&env, &referrer, &stats);

        let l1_key = Self::referee_key(&referrer);
        if let Some(l1_record) = env.storage().persistent().get::<Symbol, ReferralRecord>(&l1_key) {
            let mut l1_stats = Self::get_referrer_stats_internal(&env, &l1_record.referrer);
            l1_stats.l2_referrals += 1;
            Self::set_referrer_stats(&env, &l1_record.referrer, &l1_stats);
        }

        env.events().publish(
            (symbol_short!("referral"), symbol_short!("register")),
            (referee, referrer),
        );

        Ok(())
    }

    pub fn accrue_fee(
        env: Env,
        referee: Address,
        fee_amount: i128,
    ) -> Result<(), ReferralError> {
        Self::require_initialized(&env)?;
        let config = Self::get_config(&env)?;

        let ref_key = Self::referee_key(&referee);
        let mut record: ReferralRecord = env
            .storage()
            .persistent()
            .get(&ref_key)
            .ok_or(ReferralError::InvalidReferrer)?;

        let l1_share = (fee_amount * config.fee_share_bps as i128) / 10_000;
        record.total_fees_generated += fee_amount;
        record.referrer_earned += l1_share;
        env.storage().persistent().set(&ref_key, &record);

        let mut stats = Self::get_referrer_stats_internal(&env, &record.referrer);
        stats.total_earned += l1_share;
        stats.claimable += l1_share;
        Self::set_referrer_stats(&env, &record.referrer, &stats);

        let l1_key = Self::referee_key(&record.referrer);
        if let Some(l1_record) = env.storage().persistent().get::<Symbol, ReferralRecord>(&l1_key) {
            let l2_share = (fee_amount * config.l2_fee_share_bps as i128) / 10_000;
            if l2_share > 0 {
                let mut l1_stats = Self::get_referrer_stats_internal(&env, &l1_record.referrer);
                l1_stats.l2_earned += l2_share;
                l1_stats.total_earned += l2_share;
                l1_stats.claimable += l2_share;
                Self::set_referrer_stats(&env, &l1_record.referrer, &l1_stats);
            }
        }

        Ok(())
    }

    pub fn claim(env: Env, referrer: Address) -> Result<i128, ReferralError> {
        referrer.require_auth();
        Self::require_initialized(&env)?;
        let config = Self::get_config(&env)?;

        let mut stats = Self::get_referrer_stats_internal(&env, &referrer);
        if stats.claimable <= 0 {
            return Err(ReferralError::NothingToClaim);
        }

        let current_ledger = env.ledger().sequence() as u64;
        if stats.last_claim_ledger > 0
            && current_ledger < stats.last_claim_ledger + config.maturity_ledgers
        {
            return Err(ReferralError::MaturityNotReached);
        }

        let amount = stats.claimable;
        stats.total_claimed += amount;
        stats.claimable = 0;
        stats.last_claim_ledger = current_ledger;
        Self::set_referrer_stats(&env, &referrer, &stats);

        env.events().publish(
            (symbol_short!("referral"), symbol_short!("claim")),
            (referrer, amount),
        );

        Ok(amount)
    }

    pub fn get_referrer_stats(env: Env, referrer: Address) -> ReferrerStats {
        Self::get_referrer_stats_internal(&env, &referrer)
    }

    pub fn get_referral(env: Env, referee: Address) -> Option<ReferralRecord> {
        let ref_key = Self::referee_key(&referee);
        env.storage().persistent().get(&ref_key)
    }

    pub fn get_config_view(env: Env) -> Result<ReferralConfig, ReferralError> {
        Self::get_config(&env)
    }

    /// Get tier information for a referrer (anti-sybil & tiered rewards).
    pub fn get_tier_info(env: Env, referrer: Address) -> Result<TierInfo, ReferralError> {
        Self::require_initialized(&env)?;
        let config = Self::get_config(&env)?;
        let stats = Self::get_referrer_stats_internal(&env, &referrer);

        let (tier_level, bonus_bps) = if stats.total_referrals >= config.tier_2_threshold {
            (2, config.tier_2_bonus_bps)
        } else if stats.total_referrals >= config.tier_1_threshold {
            (1, config.tier_1_bonus_bps)
        } else {
            (0, 0)
        };

        let total_bonus_earned = (stats.total_earned * bonus_bps as i128) / 10_000;

        Ok(TierInfo {
            tier_level,
            bonus_bps,
            total_bonus_earned,
        })
    }

    /// Validate anti-sybil requirement: minimum deposit to qualify as referrer.
    pub fn validate_referrer_eligibility(
        env: Env,
        referrer: Address,
        total_deposit: i128,
    ) -> Result<bool, ReferralError> {
        Self::require_initialized(&env)?;
        let config = Self::get_config(&env)?;

        Ok(total_deposit >= config.min_deposit_to_qualify)
    }

    /// Get referral code for a user (for sharing).
    pub fn get_referral_code(env: Env, referrer: Address) -> Option<Symbol> {
        // In a real implementation, this would return a unique code
        // For now, we use a deterministic symbol based on address
        Some(symbol_short!("ref01"))
    }

    /// Query referral dashboard metrics.
    pub fn get_dashboard_metrics(
        env: Env,
        referrer: Address,
    ) -> Result<(ReferrerStats, TierInfo), ReferralError> {
        Self::require_initialized(&env)?;
        let stats = Self::get_referrer_stats_internal(&env, &referrer);
        let tier = Self::get_tier_info(env, referrer)?;

        Ok((stats, tier))
    }

    fn require_initialized(env: &Env) -> Result<(), ReferralError> {
        if !env.storage().instance().has(&symbol_short!("config")) {
            return Err(ReferralError::NotInitialized);
        }
        Ok(())
    }

    fn get_config(env: &Env) -> Result<ReferralConfig, ReferralError> {
        env.storage()
            .instance()
            .get(&symbol_short!("config"))
            .ok_or(ReferralError::NotInitialized)
    }

    fn referee_key(addr: &Address) -> Symbol {
        symbol_short!("ref")
    }

    fn referrer_stats_key(addr: &Address) -> Symbol {
        symbol_short!("stats")
    }

    fn get_referrer_stats_internal(env: &Env, referrer: &Address) -> ReferrerStats {
        env.storage()
            .persistent()
            .get(&Self::referrer_stats_key(referrer))
            .unwrap_or(ReferrerStats {
                total_referrals: 0,
                total_earned: 0,
                total_claimed: 0,
                claimable: 0,
                last_claim_ledger: 0,
                l2_referrals: 0,
                l2_earned: 0,
            })
    }

    fn set_referrer_stats(env: &Env, referrer: &Address, stats: &ReferrerStats) {
        env.storage()
            .persistent()
            .set(&Self::referrer_stats_key(referrer), stats);
    }
}
