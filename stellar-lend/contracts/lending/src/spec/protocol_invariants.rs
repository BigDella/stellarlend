//! Protocol-level formal specification for StellarLend lending invariants.
//!
//! The models in this file are intentionally pure Rust. They do not touch
//! Soroban storage, which lets CI run them as deterministic specification
//! checks and lets Kani symbolically explore bounded state transitions.
//!
//! Invariants covered:
//! - P-01: total_deposits = total_debt + reserves
//! - P-02: borrow and repay preserve double-entry accounting
//! - P-03: health factor agrees with liquidation boundary during operations
//! - P-04: interest index accrual is monotonic in non-decreasing time

pub const BPS_SCALE: i128 = 10_000;
pub const HEALTH_FACTOR_SCALE: i128 = 10_000;
pub const HEALTH_FACTOR_NO_DEBT: i128 = 100_000_000;
pub const DEFAULT_LIQ_THRESHOLD_BPS: i128 = 8_000;
pub const SECONDS_PER_YEAR: u64 = 31_536_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolSpecError {
    InvalidAmount,
    InvalidState,
    InsufficientLiquidity,
    RepayAmountTooHigh,
    Overflow,
    TimestampRewind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolAccountingState {
    pub total_deposits: i128,
    pub total_debt: i128,
    pub reserves: i128,
}

impl ProtocolAccountingState {
    pub fn new(
        total_deposits: i128,
        total_debt: i128,
        reserves: i128,
    ) -> Result<Self, ProtocolSpecError> {
        let state = Self {
            total_deposits,
            total_debt,
            reserves,
        };
        state.validate()?;
        Ok(state)
    }

    pub fn empty() -> Self {
        Self {
            total_deposits: 0,
            total_debt: 0,
            reserves: 0,
        }
    }

    pub fn validate(&self) -> Result<(), ProtocolSpecError> {
        if self.total_deposits < 0 || self.total_debt < 0 || self.reserves < 0 {
            return Err(ProtocolSpecError::InvalidState);
        }

        let liabilities = self
            .total_debt
            .checked_add(self.reserves)
            .ok_or(ProtocolSpecError::Overflow)?;

        if self.total_deposits != liabilities {
            return Err(ProtocolSpecError::InvalidState);
        }

        Ok(())
    }

    pub fn apply_deposit(&mut self, amount: i128) -> Result<(), ProtocolSpecError> {
        if amount <= 0 {
            return Err(ProtocolSpecError::InvalidAmount);
        }

        self.total_deposits = self
            .total_deposits
            .checked_add(amount)
            .ok_or(ProtocolSpecError::Overflow)?;
        self.reserves = self
            .reserves
            .checked_add(amount)
            .ok_or(ProtocolSpecError::Overflow)?;
        self.validate()
    }

    pub fn apply_borrow(&mut self, amount: i128) -> Result<(), ProtocolSpecError> {
        if amount <= 0 {
            return Err(ProtocolSpecError::InvalidAmount);
        }
        if amount > self.reserves {
            return Err(ProtocolSpecError::InsufficientLiquidity);
        }

        self.reserves = self
            .reserves
            .checked_sub(amount)
            .ok_or(ProtocolSpecError::Overflow)?;
        self.total_debt = self
            .total_debt
            .checked_add(amount)
            .ok_or(ProtocolSpecError::Overflow)?;
        self.validate()
    }

    pub fn apply_repay(&mut self, amount: i128) -> Result<(), ProtocolSpecError> {
        if amount <= 0 {
            return Err(ProtocolSpecError::InvalidAmount);
        }
        if amount > self.total_debt {
            return Err(ProtocolSpecError::RepayAmountTooHigh);
        }

        self.total_debt = self
            .total_debt
            .checked_sub(amount)
            .ok_or(ProtocolSpecError::Overflow)?;
        self.reserves = self
            .reserves
            .checked_add(amount)
            .ok_or(ProtocolSpecError::Overflow)?;
        self.validate()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HealthFactorState {
    pub collateral_value: i128,
    pub debt_value: i128,
    pub liquidation_threshold_bps: i128,
}

impl HealthFactorState {
    pub fn new(
        collateral_value: i128,
        debt_value: i128,
        liquidation_threshold_bps: i128,
    ) -> Result<Self, ProtocolSpecError> {
        let state = Self {
            collateral_value,
            debt_value,
            liquidation_threshold_bps,
        };
        state.validate_inputs()?;
        Ok(state)
    }

    fn validate_inputs(&self) -> Result<(), ProtocolSpecError> {
        if self.collateral_value < 0 || self.debt_value < 0 {
            return Err(ProtocolSpecError::InvalidState);
        }
        if self.liquidation_threshold_bps <= 0 || self.liquidation_threshold_bps > BPS_SCALE {
            return Err(ProtocolSpecError::InvalidAmount);
        }
        Ok(())
    }

    pub fn health_factor(&self) -> Result<i128, ProtocolSpecError> {
        self.validate_inputs()?;
        if self.debt_value == 0 {
            return Ok(HEALTH_FACTOR_NO_DEBT);
        }

        let weighted_collateral = self
            .collateral_value
            .checked_mul(self.liquidation_threshold_bps)
            .ok_or(ProtocolSpecError::Overflow)?
            .checked_div(BPS_SCALE)
            .ok_or(ProtocolSpecError::Overflow)?;

        weighted_collateral
            .checked_mul(HEALTH_FACTOR_SCALE)
            .ok_or(ProtocolSpecError::Overflow)?
            .checked_div(self.debt_value)
            .ok_or(ProtocolSpecError::Overflow)
    }

    pub fn is_liquidatable(&self) -> Result<bool, ProtocolSpecError> {
        Ok(self.debt_value > 0 && self.health_factor()? < HEALTH_FACTOR_SCALE)
    }

    pub fn apply_collateral_delta(&mut self, delta: i128) -> Result<(), ProtocolSpecError> {
        self.collateral_value = self
            .collateral_value
            .checked_add(delta)
            .ok_or(ProtocolSpecError::Overflow)?;
        self.validate_inputs()
    }

    pub fn apply_debt_delta(&mut self, delta: i128) -> Result<(), ProtocolSpecError> {
        self.debt_value = self
            .debt_value
            .checked_add(delta)
            .ok_or(ProtocolSpecError::Overflow)?;
        self.validate_inputs()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InterestIndexState {
    pub index: i128,
    pub rate_bps: i128,
    pub last_timestamp: u64,
}

impl InterestIndexState {
    pub fn new(
        index: i128,
        rate_bps: i128,
        last_timestamp: u64,
    ) -> Result<Self, ProtocolSpecError> {
        let state = Self {
            index,
            rate_bps,
            last_timestamp,
        };
        state.validate()?;
        Ok(state)
    }

    fn validate(&self) -> Result<(), ProtocolSpecError> {
        if self.index <= 0 || self.rate_bps < 0 {
            return Err(ProtocolSpecError::InvalidState);
        }
        Ok(())
    }

    pub fn accrue_to(&mut self, now: u64) -> Result<i128, ProtocolSpecError> {
        self.validate()?;
        if now < self.last_timestamp {
            return Err(ProtocolSpecError::TimestampRewind);
        }

        let elapsed = now - self.last_timestamp;
        if elapsed == 0 || self.rate_bps == 0 {
            self.last_timestamp = now;
            return Ok(self.index);
        }

        let increment = self
            .index
            .checked_mul(self.rate_bps)
            .ok_or(ProtocolSpecError::Overflow)?
            .checked_mul(elapsed as i128)
            .ok_or(ProtocolSpecError::Overflow)?
            .checked_div(BPS_SCALE)
            .ok_or(ProtocolSpecError::Overflow)?
            .checked_div(SECONDS_PER_YEAR as i128)
            .ok_or(ProtocolSpecError::Overflow)?;

        self.index = self
            .index
            .checked_add(increment)
            .ok_or(ProtocolSpecError::Overflow)?;
        self.last_timestamp = now;
        Ok(self.index)
    }
}

#[test]
fn lemma_p01_accounting_identity_holds_after_deposits() {
    let mut state = ProtocolAccountingState::empty();

    for amount in [1, 10_000, 500_000, 1_000_000_000] {
        state
            .apply_deposit(amount)
            .expect("deposit should preserve accounting");
        assert_eq!(state.total_deposits, state.total_debt + state.reserves);
    }
}

#[test]
fn lemma_p02_borrow_preserves_deposit_debt_reserve_identity() {
    let mut state = ProtocolAccountingState::empty();
    state.apply_deposit(1_000_000).unwrap();

    for amount in [1, 20_000, 250_000, 500_000] {
        state
            .apply_borrow(amount)
            .expect("borrow should preserve accounting");
        assert_eq!(state.total_deposits, state.total_debt + state.reserves);
    }
}

#[test]
fn lemma_p03_repay_preserves_deposit_debt_reserve_identity() {
    let mut state = ProtocolAccountingState::empty();
    state.apply_deposit(1_000_000).unwrap();
    state.apply_borrow(900_000).unwrap();

    for amount in [1, 20_000, 250_000, 500_000] {
        state
            .apply_repay(amount)
            .expect("repay should preserve accounting");
        assert_eq!(state.total_deposits, state.total_debt + state.reserves);
    }
}

#[test]
fn lemma_p04_over_borrow_is_rejected_without_mutation() {
    let mut state = ProtocolAccountingState::empty();
    state.apply_deposit(50_000).unwrap();
    let before = state;

    assert_eq!(
        state.apply_borrow(50_001),
        Err(ProtocolSpecError::InsufficientLiquidity)
    );
    assert_eq!(state, before);
}

#[test]
fn lemma_p05_over_repay_is_rejected_without_mutation() {
    let mut state = ProtocolAccountingState::empty();
    state.apply_deposit(100_000).unwrap();
    state.apply_borrow(75_000).unwrap();
    let before = state;

    assert_eq!(
        state.apply_repay(75_001),
        Err(ProtocolSpecError::RepayAmountTooHigh)
    );
    assert_eq!(state, before);
}

#[test]
fn lemma_h01_health_factor_matches_exact_liquidation_boundary() {
    let debt = 1_000_000;
    let collateral = debt * BPS_SCALE / DEFAULT_LIQ_THRESHOLD_BPS;
    let state = HealthFactorState::new(collateral, debt, DEFAULT_LIQ_THRESHOLD_BPS).unwrap();

    assert_eq!(state.health_factor().unwrap(), HEALTH_FACTOR_SCALE);
    assert!(!state.is_liquidatable().unwrap());
}

#[test]
fn lemma_h02_collateral_increase_never_decreases_health_factor() {
    let mut state =
        HealthFactorState::new(1_250_000, 1_000_000, DEFAULT_LIQ_THRESHOLD_BPS).unwrap();
    let before = state.health_factor().unwrap();
    state.apply_collateral_delta(500_000).unwrap();
    let after = state.health_factor().unwrap();

    assert!(
        after >= before,
        "health factor decreased after collateral increase"
    );
}

#[test]
fn lemma_h03_borrow_increase_never_increases_health_factor() {
    let mut state = HealthFactorState::new(2_000_000, 500_000, DEFAULT_LIQ_THRESHOLD_BPS).unwrap();
    let before = state.health_factor().unwrap();
    state.apply_debt_delta(500_000).unwrap();
    let after = state.health_factor().unwrap();

    assert!(
        after <= before,
        "health factor increased after debt increase"
    );
}

#[test]
fn lemma_h04_repay_never_decreases_health_factor() {
    let mut state =
        HealthFactorState::new(2_000_000, 1_000_000, DEFAULT_LIQ_THRESHOLD_BPS).unwrap();
    let before = state.health_factor().unwrap();
    state.apply_debt_delta(-500_000).unwrap();
    let after = state.health_factor().unwrap();

    assert!(
        after >= before,
        "health factor decreased after debt reduction"
    );
}

#[test]
fn lemma_h05_liquidation_flag_tracks_health_factor() {
    let healthy = HealthFactorState::new(2_000_000, 1_000_000, DEFAULT_LIQ_THRESHOLD_BPS).unwrap();
    let unhealthy =
        HealthFactorState::new(1_000_000, 1_000_000, DEFAULT_LIQ_THRESHOLD_BPS).unwrap();

    assert!(!healthy.is_liquidatable().unwrap());
    assert!(unhealthy.is_liquidatable().unwrap());
}

#[test]
fn lemma_i01_interest_index_is_monotonic() {
    let mut index = InterestIndexState::new(BPS_SCALE, 500, 1_700_000_000).unwrap();
    let before = index.index;
    let after = index.accrue_to(1_700_000_000 + SECONDS_PER_YEAR).unwrap();

    assert!(after >= before);
}

#[test]
fn lemma_i02_zero_elapsed_time_is_noop() {
    let mut index = InterestIndexState::new(BPS_SCALE, 500, 1_700_000_000).unwrap();
    let after = index.accrue_to(1_700_000_000).unwrap();

    assert_eq!(after, BPS_SCALE);
}

#[test]
fn lemma_i03_timestamp_rewind_is_rejected() {
    let mut index = InterestIndexState::new(BPS_SCALE, 500, 1_700_000_000).unwrap();

    assert_eq!(
        index.accrue_to(1_699_999_999),
        Err(ProtocolSpecError::TimestampRewind)
    );
}

#[cfg(kani)]
#[kani::proof]
pub fn kani_protocol_accounting_preserves_identity() {
    let deposit_amount: i128 = kani::any();
    let borrow_amount: i128 = kani::any();
    let repay_amount: i128 = kani::any();

    kani::assume(deposit_amount > 0 && deposit_amount <= 1_000_000_000);
    kani::assume(borrow_amount > 0 && borrow_amount <= deposit_amount);
    kani::assume(repay_amount > 0 && repay_amount <= borrow_amount);

    let mut state = ProtocolAccountingState::empty();
    state.apply_deposit(deposit_amount).unwrap();
    state.apply_borrow(borrow_amount).unwrap();
    state.apply_repay(repay_amount).unwrap();

    kani::assert(
        state.total_deposits == state.total_debt + state.reserves,
        "P-01 total deposits equal total debt plus reserves",
    );
}

#[cfg(kani)]
#[kani::proof]
pub fn kani_health_factor_consistency() {
    let collateral_value: i128 = kani::any();
    let debt_value: i128 = kani::any();

    kani::assume(collateral_value >= 0 && collateral_value <= 1_000_000_000_000);
    kani::assume(debt_value >= 0 && debt_value <= 1_000_000_000_000);

    let state =
        HealthFactorState::new(collateral_value, debt_value, DEFAULT_LIQ_THRESHOLD_BPS).unwrap();
    let hf = state.health_factor().unwrap();

    if debt_value == 0 {
        kani::assert(hf == HEALTH_FACTOR_NO_DEBT, "zero debt sentinel");
    } else {
        kani::assert(
            state.is_liquidatable().unwrap() == (hf < HEALTH_FACTOR_SCALE),
            "liquidation flag tracks health factor",
        );
    }
}

#[cfg(kani)]
#[kani::proof]
pub fn kani_interest_index_monotonic() {
    let start: u64 = kani::any();
    let elapsed: u64 = kani::any();
    let rate_bps: i128 = kani::any();

    kani::assume(start <= u64::MAX - 31_536_000);
    kani::assume(elapsed <= 31_536_000);
    kani::assume(rate_bps >= 0 && rate_bps <= BPS_SCALE);

    let mut index = InterestIndexState::new(BPS_SCALE, rate_bps, start).unwrap();
    let before = index.index;
    let after = index.accrue_to(start + elapsed).unwrap();

    kani::assert(after >= before, "interest index must not decrease");
}
