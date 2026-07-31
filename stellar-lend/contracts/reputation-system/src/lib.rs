#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ReputationError {
    /// Caller is not the admin.
    Unauthorized = 1,
    /// The contract has not been initialized yet.
    NotInitialized = 2,
    /// The contract is already initialized.
    AlreadyInitialized = 3,
    /// No reputation record exists for the given address.
    NotFound = 4,
    /// Invalid configuration parameters.
    InvalidConfig = 5,
}

// ---------------------------------------------------------------------------
// Storage types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReputationScore {
    pub address: Address,
    pub total_repayments: u32,
    pub on_time_repayments: u32,
    pub defaults: u32,
    pub total_borrowed: i128,
    /// Score in range 0..=1000.
    pub score: u32,
    pub tier: ReputationTier,
    pub last_activity_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ReputationTier {
    Bronze = 0,
    Silver = 1,
    Gold = 2,
    Platinum = 3,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TierBenefits {
    /// Basis-point discount on interest rate (e.g. 50 = 0.50%).
    pub interest_rate_discount_bps: u32,
    /// Multiplier for borrowing limit in basis points (10_000 = 1x).
    pub borrowing_limit_multiplier_bps: u32,
    /// Basis-point reduction in required collateral.
    pub collateral_reduction_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReputationConfig {
    pub admin: Address,
    /// Score threshold for Silver tier (default 250).
    pub silver_threshold: u32,
    /// Score threshold for Gold tier (default 500).
    pub gold_threshold: u32,
    /// Score threshold for Platinum tier (default 750).
    pub platinum_threshold: u32,
    /// Benefits per tier – stored in order Bronze, Silver, Gold, Platinum.
    pub bronze_benefits: TierBenefits,
    pub silver_benefits: TierBenefits,
    pub gold_benefits: TierBenefits,
    pub platinum_benefits: TierBenefits,
    /// Points subtracted per decay interval of inactivity.
    pub decay_rate: u32,
    /// Seconds of inactivity before each decay step applies.
    pub decay_interval: u64,
}

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Config,
    Score(Address),
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_admin(env: &Env, caller: &Address) -> Result<ReputationConfig, ReputationError> {
    let config: ReputationConfig = env
        .storage()
        .instance()
        .get(&DataKey::Config)
        .ok_or(ReputationError::NotInitialized)?;
    caller.require_auth();
    if *caller != config.admin {
        return Err(ReputationError::Unauthorized);
    }
    Ok(config)
}

fn tier_from_score(config: &ReputationConfig, score: u32) -> ReputationTier {
    if score >= config.platinum_threshold {
        ReputationTier::Platinum
    } else if score >= config.gold_threshold {
        ReputationTier::Gold
    } else if score >= config.silver_threshold {
        ReputationTier::Silver
    } else {
        ReputationTier::Bronze
    }
}

/// Compute a new score in 0..=1000 from the borrower's stats.
///
/// Formula (weights sum to 100):
///   - on_time_rate  : 40%  (on_time_repayments / total_repayments * 1000)
///   - count_factor  : 30%  (min(total_repayments, 100) / 100 * 1000)
///   - no_default    : 30%  (defaults == 0 ? 1000 : max(0, 1000 - defaults * 200))
///
/// All arithmetic avoids floats.
fn compute_score(total: u32, on_time: u32, defaults: u32) -> u32 {
    if total == 0 {
        return 0;
    }

    // on-time rate component: on_time / total * 1000, weight 40
    let on_time_component: u64 = (on_time as u64) * 1000 / (total as u64); // 0..1000

    // count factor: min(total, 100) / 100 * 1000, weight 30
    let capped_total: u64 = if total > 100 { 100 } else { total as u64 };
    let count_component: u64 = capped_total * 1000 / 100; // 0..1000

    // no-default component, weight 30
    let default_penalty: u64 = (defaults as u64) * 200;
    let no_default_component: u64 = if default_penalty >= 1000 {
        0
    } else {
        1000 - default_penalty
    };

    let weighted: u64 = on_time_component * 40 + count_component * 30 + no_default_component * 30;
    let score = weighted / 100; // back to 0..1000

    if score > 1000 {
        1000
    } else {
        score as u32
    }
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct ReputationContract;

#[contractimpl]
impl ReputationContract {
    // ── Initialization ────────────────────────────────────────────────────

    /// Initialize the reputation system. Can only be called once.
    pub fn initialize(
        env: Env,
        admin: Address,
        config: ReputationConfig,
    ) -> Result<(), ReputationError> {
        if env.storage().instance().has(&DataKey::Config) {
            return Err(ReputationError::AlreadyInitialized);
        }
        admin.require_auth();

        // Validate thresholds are ordered.
        if config.silver_threshold >= config.gold_threshold
            || config.gold_threshold >= config.platinum_threshold
            || config.platinum_threshold > 1000
        {
            return Err(ReputationError::InvalidConfig);
        }

        env.storage().instance().set(&DataKey::Config, &config);
        Ok(())
    }

    // ── Admin mutations ──────────────────────────────────────────────────

    /// Record a repayment for a borrower.
    pub fn record_repayment(
        env: Env,
        admin: Address,
        borrower: Address,
        amount: i128,
        on_time: bool,
    ) -> Result<ReputationScore, ReputationError> {
        let config = require_admin(&env, &admin)?;

        let mut rep = Self::get_or_default(&env, &borrower);
        rep.total_repayments += 1;
        if on_time {
            rep.on_time_repayments += 1;
        }
        rep.total_borrowed += amount;
        rep.score = compute_score(rep.total_repayments, rep.on_time_repayments, rep.defaults);
        rep.tier = tier_from_score(&config, rep.score);
        rep.last_activity_timestamp = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Score(borrower), &rep);
        Ok(rep)
    }

    /// Record a default for a borrower. Heavily penalises the score.
    pub fn record_default(
        env: Env,
        admin: Address,
        borrower: Address,
    ) -> Result<ReputationScore, ReputationError> {
        let config = require_admin(&env, &admin)?;

        let mut rep = Self::get_or_default(&env, &borrower);
        rep.defaults += 1;
        rep.score = compute_score(rep.total_repayments, rep.on_time_repayments, rep.defaults);
        rep.tier = tier_from_score(&config, rep.score);
        rep.last_activity_timestamp = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Score(borrower), &rep);
        Ok(rep)
    }

    // ── Queries ──────────────────────────────────────────────────────────

    /// Get the full reputation record for an address.
    pub fn get_reputation(env: Env, address: Address) -> Result<ReputationScore, ReputationError> {
        env.storage()
            .persistent()
            .get(&DataKey::Score(address))
            .ok_or(ReputationError::NotFound)
    }

    /// Get the current tier for an address.
    pub fn get_tier(env: Env, address: Address) -> Result<ReputationTier, ReputationError> {
        let rep = Self::get_reputation(env, address)?;
        Ok(rep.tier)
    }

    /// Get benefits associated with a given tier.
    pub fn get_tier_benefits(
        env: Env,
        tier: ReputationTier,
    ) -> Result<TierBenefits, ReputationError> {
        let config: ReputationConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(ReputationError::NotInitialized)?;

        let benefits = match tier {
            ReputationTier::Bronze => config.bronze_benefits,
            ReputationTier::Silver => config.silver_benefits,
            ReputationTier::Gold => config.gold_benefits,
            ReputationTier::Platinum => config.platinum_benefits,
        };
        Ok(benefits)
    }

    // ── Decay ────────────────────────────────────────────────────────────

    /// Apply inactivity decay to an address's score. Anyone may call this.
    pub fn apply_decay(env: Env, address: Address) -> Result<ReputationScore, ReputationError> {
        let config: ReputationConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(ReputationError::NotInitialized)?;

        let mut rep: ReputationScore = env
            .storage()
            .persistent()
            .get(&DataKey::Score(address.clone()))
            .ok_or(ReputationError::NotFound)?;

        let now = env.ledger().timestamp();
        if config.decay_interval == 0 || config.decay_rate == 0 {
            return Ok(rep);
        }

        let elapsed = now.saturating_sub(rep.last_activity_timestamp);
        let intervals = elapsed / config.decay_interval;
        if intervals == 0 {
            return Ok(rep);
        }

        let total_decay = (intervals as u32).saturating_mul(config.decay_rate);
        rep.score = rep.score.saturating_sub(total_decay);
        rep.tier = tier_from_score(&config, rep.score);
        // Do NOT update last_activity_timestamp — decay is not "activity".

        env.storage()
            .persistent()
            .set(&DataKey::Score(address), &rep);
        Ok(rep)
    }

    // ── Config management ────────────────────────────────────────────────

    /// Update the configuration. Admin only.
    pub fn update_config(
        env: Env,
        admin: Address,
        config: ReputationConfig,
    ) -> Result<(), ReputationError> {
        require_admin(&env, &admin)?;

        if config.silver_threshold >= config.gold_threshold
            || config.gold_threshold >= config.platinum_threshold
            || config.platinum_threshold > 1000
        {
            return Err(ReputationError::InvalidConfig);
        }

        env.storage().instance().set(&DataKey::Config, &config);
        Ok(())
    }

    /// Get the current configuration.
    pub fn get_config(env: Env) -> Result<ReputationConfig, ReputationError> {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(ReputationError::NotInitialized)
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    fn get_or_default(env: &Env, address: &Address) -> ReputationScore {
        env.storage()
            .persistent()
            .get(&DataKey::Score(address.clone()))
            .unwrap_or(ReputationScore {
                address: address.clone(),
                total_repayments: 0,
                on_time_repayments: 0,
                defaults: 0,
                total_borrowed: 0,
                score: 0,
                tier: ReputationTier::Bronze,
                last_activity_timestamp: 0,
            })
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
        Env,
    };

    fn default_benefits() -> TierBenefits {
        TierBenefits {
            interest_rate_discount_bps: 0,
            borrowing_limit_multiplier_bps: 10_000,
            collateral_reduction_bps: 0,
        }
    }

    fn default_config(admin: &Address) -> ReputationConfig {
        ReputationConfig {
            admin: admin.clone(),
            silver_threshold: 250,
            gold_threshold: 500,
            platinum_threshold: 750,
            bronze_benefits: default_benefits(),
            silver_benefits: TierBenefits {
                interest_rate_discount_bps: 25,
                borrowing_limit_multiplier_bps: 11_000,
                collateral_reduction_bps: 100,
            },
            gold_benefits: TierBenefits {
                interest_rate_discount_bps: 50,
                borrowing_limit_multiplier_bps: 12_500,
                collateral_reduction_bps: 200,
            },
            platinum_benefits: TierBenefits {
                interest_rate_discount_bps: 100,
                borrowing_limit_multiplier_bps: 15_000,
                collateral_reduction_bps: 300,
            },
            decay_rate: 10,
            decay_interval: 86_400, // 1 day
        }
    }

    struct TestSetup {
        env: Env,
        admin: Address,
        client: ReputationContractClient<'static>,
    }

    fn setup() -> TestSetup {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register(ReputationContract, ());
        let client = ReputationContractClient::new(&env, &contract_id);
        TestSetup { env, admin, client }
    }

    fn advance_time(env: &Env, seconds: u64) {
        let current = env.ledger().timestamp();
        env.ledger().with_mut(|li| li.timestamp = current + seconds);
    }

    // ── Initialization ──────────────────────────────────────────────────

    #[test]
    fn test_initialize() {
        let s = setup();
        let config = default_config(&s.admin);
        let result = s.client.initialize(&s.admin, &config);
        assert_eq!(result, ());
    }

    #[test]
    fn test_initialize_twice_fails() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);
        let result = s.client.try_initialize(&s.admin, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_initialize_invalid_thresholds() {
        let s = setup();
        let mut config = default_config(&s.admin);
        config.silver_threshold = 600;
        config.gold_threshold = 500; // silver >= gold, invalid
        let result = s.client.try_initialize(&s.admin, &config);
        assert!(result.is_err());
    }

    // ── Record repayment ────────────────────────────────────────────────

    #[test]
    fn test_record_repayment_on_time() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let borrower = Address::generate(&s.env);
        let rep = s.client.record_repayment(&s.admin, &borrower, &1000, &true);

        assert_eq!(rep.total_repayments, 1);
        assert_eq!(rep.on_time_repayments, 1);
        assert_eq!(rep.defaults, 0);
        assert_eq!(rep.total_borrowed, 1000);
        assert!(rep.score > 0);
    }

    #[test]
    fn test_record_repayment_late() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let borrower = Address::generate(&s.env);
        let rep = s.client.record_repayment(&s.admin, &borrower, &500, &false);

        assert_eq!(rep.total_repayments, 1);
        assert_eq!(rep.on_time_repayments, 0);
        assert_eq!(rep.defaults, 0);
        // Late repayment: on_time_component = 0, count component small, no-default full
        // Score should be relatively low but nonzero
        assert!(rep.score < 500);
    }

    #[test]
    fn test_multiple_repayments_increase_score() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let borrower = Address::generate(&s.env);
        let mut prev_score = 0u32;
        for _ in 0..10 {
            let rep = s.client.record_repayment(&s.admin, &borrower, &100, &true);
            assert!(rep.score >= prev_score);
            prev_score = rep.score;
        }
        assert!(prev_score > 0);
    }

    // ── Record default ──────────────────────────────────────────────────

    #[test]
    fn test_record_default() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let borrower = Address::generate(&s.env);

        // Build up a score first
        for _ in 0..5 {
            s.client.record_repayment(&s.admin, &borrower, &100, &true);
        }
        let before = s.client.get_reputation(&borrower);
        let before_score = before.score;

        // Record a default
        let rep = s.client.record_default(&s.admin, &borrower);
        assert_eq!(rep.defaults, 1);
        assert!(rep.score < before_score);
    }

    #[test]
    fn test_multiple_defaults_crush_score() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let borrower = Address::generate(&s.env);

        // Build up
        for _ in 0..10 {
            s.client.record_repayment(&s.admin, &borrower, &100, &true);
        }

        // Multiple defaults
        for _ in 0..5 {
            s.client.record_default(&s.admin, &borrower);
        }

        let rep = s.client.get_reputation(&borrower);
        assert_eq!(rep.defaults, 5);
        // With 5 defaults the no-default component is 0
        assert!(rep.score < 500);
    }

    // ── Queries ─────────────────────────────────────────────────────────

    #[test]
    fn test_get_reputation_not_found() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let unknown = Address::generate(&s.env);
        let result = s.client.try_get_reputation(&unknown);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_tier() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let borrower = Address::generate(&s.env);
        // One on-time repayment: on_time=1000*40, count=10*30, no_default=1000*30
        // => (40000 + 300 + 30000) / 100 = 703 => Gold tier
        s.client.record_repayment(&s.admin, &borrower, &100, &true);
        let tier = s.client.get_tier(&borrower);
        assert_eq!(tier, ReputationTier::Gold);
    }

    #[test]
    fn test_tier_benefits() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let benefits = s.client.get_tier_benefits(&ReputationTier::Platinum);
        assert_eq!(benefits.interest_rate_discount_bps, 100);
        assert_eq!(benefits.borrowing_limit_multiplier_bps, 15_000);
        assert_eq!(benefits.collateral_reduction_bps, 300);
    }

    // ── Tier progression ────────────────────────────────────────────────

    #[test]
    fn test_tier_progression_with_many_repayments() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let borrower = Address::generate(&s.env);

        // Many on-time repayments should push the score up through tiers
        for _ in 0..100 {
            s.client.record_repayment(&s.admin, &borrower, &100, &true);
        }
        let rep = s.client.get_reputation(&borrower);
        // 100 on-time repayments, 0 defaults:
        // on_time = 1000 * 40 = 40000
        // count = 1000 * 30 = 30000
        // no_default = 1000 * 30 = 30000
        // total = 100000 / 100 = 1000
        assert_eq!(rep.score, 1000);
        assert_eq!(rep.tier, ReputationTier::Platinum);
    }

    // ── Decay ───────────────────────────────────────────────────────────

    #[test]
    fn test_apply_decay() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let borrower = Address::generate(&s.env);

        // Build up a score
        for _ in 0..50 {
            s.client.record_repayment(&s.admin, &borrower, &100, &true);
        }
        let before = s.client.get_reputation(&borrower);

        // Advance time by 5 days (5 decay intervals)
        advance_time(&s.env, 86_400 * 5);

        let after = s.client.apply_decay(&borrower);
        // Decay = 5 intervals * 10 points = 50
        assert_eq!(after.score, before.score - 50);
    }

    #[test]
    fn test_decay_does_not_go_below_zero() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let borrower = Address::generate(&s.env);
        // One repayment gives a small score
        s.client.record_repayment(&s.admin, &borrower, &100, &true);

        // Advance a very long time
        advance_time(&s.env, 86_400 * 10_000);

        let rep = s.client.apply_decay(&borrower);
        assert_eq!(rep.score, 0);
        assert_eq!(rep.tier, ReputationTier::Bronze);
    }

    #[test]
    fn test_no_decay_within_interval() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let borrower = Address::generate(&s.env);
        for _ in 0..10 {
            s.client.record_repayment(&s.admin, &borrower, &100, &true);
        }
        let before = s.client.get_reputation(&borrower);

        // Advance less than one interval
        advance_time(&s.env, 86_400 - 1);

        let after = s.client.apply_decay(&borrower);
        assert_eq!(after.score, before.score);
    }

    // ── Config management ───────────────────────────────────────────────

    #[test]
    fn test_update_config() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let mut new_config = config.clone();
        new_config.decay_rate = 20;
        s.client.update_config(&s.admin, &new_config);

        let fetched = s.client.get_config();
        assert_eq!(fetched.decay_rate, 20);
    }

    #[test]
    fn test_update_config_unauthorized() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let imposter = Address::generate(&s.env);
        let mut new_config = config.clone();
        new_config.admin = imposter.clone();
        let result = s.client.try_update_config(&imposter, &new_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_config() {
        let s = setup();
        let config = default_config(&s.admin);
        s.client.initialize(&s.admin, &config);

        let fetched = s.client.get_config();
        assert_eq!(fetched.admin, s.admin);
        assert_eq!(fetched.silver_threshold, 250);
        assert_eq!(fetched.decay_rate, 10);
    }

    // ── Score computation unit tests ────────────────────────────────────

    #[test]
    fn test_compute_score_zero_repayments() {
        assert_eq!(compute_score(0, 0, 0), 0);
    }

    #[test]
    fn test_compute_score_perfect() {
        // 100 total, 100 on-time, 0 defaults -> 1000
        assert_eq!(compute_score(100, 100, 0), 1000);
    }

    #[test]
    fn test_compute_score_all_late() {
        // 10 total, 0 on-time, 0 defaults
        // on_time = 0, count = 100*30=3000, no_default = 1000*30 = 30000
        // total = 33000 / 100 = 330
        let score = compute_score(10, 0, 0);
        assert_eq!(score, 330);
    }

    #[test]
    fn test_compute_score_with_defaults() {
        // 10 total, 10 on-time, 3 defaults
        // on_time = 1000*40 = 40000, count = 100*30 = 3000, no_default = (1000-600)*30 = 12000
        // total = 55000 / 100 = 550
        let score = compute_score(10, 10, 3);
        assert_eq!(score, 550);
    }
}
