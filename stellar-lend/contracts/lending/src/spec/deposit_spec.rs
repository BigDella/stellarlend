//! # Formal Specification — `deposit`
//!
//! ## Properties
//!
//! | ID   | Property                                                     |
//! |------|--------------------------------------------------------------|
//! | D-01 | Zero amount is rejected                                      |
//! | D-02 | Amount below minimum is rejected                             |
//! | D-03 | Deposit cap is enforced                                      |
//! | D-04 | User balance increases by exactly the deposited amount       |
//! | D-05 | Total deposits increases by exactly the deposited amount     |
//! | D-06 | Sequential deposits accumulate correctly                     |
//! | D-07 | No overflow for deposits within safe domain                  |

#[derive(Clone, Debug)]
pub struct DepositState {
    pub user_balance: i128,
    pub total_deposits: i128,
    pub deposit_cap: i128,
    pub min_deposit: i128,
}

#[derive(Debug, PartialEq)]
pub enum DepositError {
    InvalidAmount,
    ExceedsDepositCap,
    Overflow,
}

impl DepositState {
    pub fn new(cap: i128, min: i128) -> Self {
        Self {
            user_balance: 0,
            total_deposits: 0,
            deposit_cap: cap,
            min_deposit: min,
        }
    }

    pub fn apply_deposit(&mut self, amount: i128) -> Result<(), DepositError> {
        if amount <= 0 || amount < self.min_deposit {
            return Err(DepositError::InvalidAmount);
        }
        let new_total = self
            .total_deposits
            .checked_add(amount)
            .ok_or(DepositError::Overflow)?;
        if new_total > self.deposit_cap {
            return Err(DepositError::ExceedsDepositCap);
        }
        let new_user = self
            .user_balance
            .checked_add(amount)
            .ok_or(DepositError::Overflow)?;
        self.total_deposits = new_total;
        self.user_balance = new_user;
        Ok(())
    }
}

/// **D-01**: Zero amount is rejected.
#[test]
fn lemma_d01_zero_amount_rejected() {
    let mut s = DepositState::new(i128::MAX, 0);
    assert_eq!(s.apply_deposit(0), Err(DepositError::InvalidAmount), "D-01");
}

/// **D-02**: Below minimum is rejected.
#[test]
fn lemma_d02_below_minimum_rejected() {
    let mut s = DepositState::new(i128::MAX, 1_000);
    assert_eq!(
        s.apply_deposit(999),
        Err(DepositError::InvalidAmount),
        "D-02"
    );
}

/// **D-03**: Deposit cap is enforced.
#[test]
fn lemma_d03_deposit_cap_enforced() {
    let cap = 1_000_000i128;
    let mut s = DepositState::new(cap, 0);
    s.apply_deposit(cap).unwrap();
    assert_eq!(
        s.apply_deposit(1),
        Err(DepositError::ExceedsDepositCap),
        "D-03"
    );
}

/// **D-04**: User balance increases by exactly the deposit amount.
#[test]
fn lemma_d04_user_balance_incremented_correctly() {
    let mut s = DepositState::new(i128::MAX, 0);
    let before = s.user_balance;
    s.apply_deposit(50_000).unwrap();
    assert_eq!(s.user_balance, before + 50_000, "D-04");
}

/// **D-05**: Total deposits increases by exactly the deposit amount.
#[test]
fn lemma_d05_total_deposits_incremented_correctly() {
    let mut s = DepositState::new(i128::MAX, 0);
    let before = s.total_deposits;
    s.apply_deposit(75_000).unwrap();
    assert_eq!(s.total_deposits, before + 75_000, "D-05");
}

/// **D-06**: Sequential deposits accumulate correctly.
#[test]
fn lemma_d06_sequential_deposits_accumulate() {
    let mut s = DepositState::new(i128::MAX, 0);
    s.apply_deposit(10_000).unwrap();
    s.apply_deposit(20_000).unwrap();
    s.apply_deposit(30_000).unwrap();
    assert_eq!(s.user_balance, 60_000, "D-06");
    assert_eq!(s.total_deposits, 60_000, "D-06 total");
}

/// **D-07**: No overflow for MAX_SAFE_PRINCIPAL deposit.
#[test]
fn lemma_d07_no_overflow_safe_domain() {
    let safe = i128::MAX / 4;
    let mut s = DepositState::new(i128::MAX, 0);
    assert!(
        s.apply_deposit(safe).is_ok(),
        "D-07: overflow on safe deposit"
    );
}
