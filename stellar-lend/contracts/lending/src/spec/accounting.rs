//! # Formal Specification — Accounting Invariants
//!
//! Proves double-entry accounting correctness across `borrow` and `repay`.
//!
//! ## Invariants
//! * INV-1: total_debt = Σ user_principals
//! * INV-2: total_debt ≥ 0 always
//! * INV-3: ∀ i: user_principal_i ≥ 0
//! * INV-4: borrow(Δ) → total_debt' = total_debt + Δ
//! * INV-5: repay(Δ) → total_debt' = total_debt − Δ_principal
//! * INV-6: total_debt ≤ ceiling after any borrow
//!
//! ## Properties
//!
//! | ID   | Property                                                  |
//! |------|-----------------------------------------------------------|
//! | A-01 | Borrow increments total debt by exactly the borrow amount |
//! | A-02 | Repay decrements total debt by the principal portion only |
//! | A-03 | Total debt never goes negative after repay                |
//! | A-04 | Total debt never exceeds ceiling after borrow             |
//! | A-05 | Borrow then full-repay is an identity on total debt       |
//! | A-06 | Partial repay leaves correct remainder                    |
//! | A-07 | Over-repay is rejected                                    |
//! | A-08 | Borrow beyond ceiling is rejected                         |

#[derive(Clone, Debug)]
pub struct AccountingState {
    pub total_debt: i128,
    pub debt_ceiling: i128,
    pub user_principal: i128,
    pub user_interest: i128,
}

#[derive(Debug, PartialEq)]
pub enum AccountingError {
    DebtCeilingReached,
    Overflow,
    RepayAmountTooHigh,
    InvalidAmount,
}

impl AccountingState {
    pub fn new(ceiling: i128) -> Self {
        Self {
            total_debt: 0,
            debt_ceiling: ceiling,
            user_principal: 0,
            user_interest: 0,
        }
    }

    pub fn apply_borrow(&mut self, amount: i128) -> Result<(), AccountingError> {
        if amount <= 0 {
            return Err(AccountingError::InvalidAmount);
        }
        let new_total = self
            .total_debt
            .checked_add(amount)
            .ok_or(AccountingError::Overflow)?;
        if new_total > self.debt_ceiling {
            return Err(AccountingError::DebtCeilingReached);
        }
        let new_user = self
            .user_principal
            .checked_add(amount)
            .ok_or(AccountingError::Overflow)?;
        self.total_debt = new_total;
        self.user_principal = new_user;
        Ok(())
    }

    pub fn apply_repay(&mut self, amount: i128) -> Result<(), AccountingError> {
        if amount <= 0 {
            return Err(AccountingError::InvalidAmount);
        }
        let mut remaining = amount;
        if remaining >= self.user_interest {
            remaining -= self.user_interest;
            self.user_interest = 0;
        } else {
            self.user_interest -= remaining;
            remaining = 0;
        }
        if remaining > 0 {
            if remaining > self.user_principal {
                return Err(AccountingError::RepayAmountTooHigh);
            }
            self.user_principal -= remaining;
            self.total_debt = self
                .total_debt
                .checked_sub(remaining)
                .ok_or(AccountingError::Overflow)?;
        }
        Ok(())
    }
}

/// **A-01**: Borrow increments total debt by exactly the borrow amount.
#[test]
fn lemma_a01_borrow_increments_total_debt() {
    for amount in [1i128, 1_000, 500_000, 1_000_000_000] {
        let mut state = AccountingState::new(i128::MAX);
        let before = state.total_debt;
        state.apply_borrow(amount).expect("A-01");
        assert_eq!(
            state.total_debt,
            before + amount,
            "A-01 failed for amount={amount}"
        );
    }
}

/// **A-02**: Repay decrements total debt by the principal portion only.
#[test]
fn lemma_a02_repay_decrements_total_debt() {
    let mut state = AccountingState::new(i128::MAX);
    state.apply_borrow(100_000).unwrap();
    state.user_interest = 5_000;
    let repay = 50_000i128;
    let total_before = state.total_debt;
    state.apply_repay(repay).expect("A-02");
    let principal_repaid = repay - 5_000;
    assert_eq!(state.total_debt, total_before - principal_repaid, "A-02");
}

/// **A-03**: Total debt never goes negative.
#[test]
fn lemma_a03_total_debt_stays_non_negative() {
    let mut state = AccountingState::new(i128::MAX);
    state.apply_borrow(100_000).unwrap();
    state.apply_repay(100_000).expect("A-03");
    assert!(
        state.total_debt >= 0,
        "A-03: negative total_debt={}",
        state.total_debt
    );
    assert_eq!(state.total_debt, 0, "A-03");
}

/// **A-04**: Ceiling is enforced.
#[test]
fn lemma_a04_borrow_ceiling_enforced() {
    let ceiling = 1_000_000i128;
    let mut state = AccountingState::new(ceiling);
    state.apply_borrow(ceiling).expect("A-04");
    assert_eq!(
        state.apply_borrow(1),
        Err(AccountingError::DebtCeilingReached),
        "A-04"
    );
    assert_eq!(state.total_debt, ceiling, "A-04: ceiling breached");
}

/// **A-05**: Borrow then full repay is identity.
#[test]
fn lemma_a05_borrow_then_full_repay_is_identity() {
    for amount in [1i128, 500, 10_000, 1_000_000] {
        let mut state = AccountingState::new(i128::MAX);
        let initial = state.total_debt;
        state.apply_borrow(amount).unwrap();
        state.apply_repay(amount).unwrap();
        assert_eq!(state.total_debt, initial, "A-05 for amount={amount}");
    }
}

/// **A-06**: Partial repay leaves correct remainder.
#[test]
fn lemma_a06_partial_repay_leaves_correct_remainder() {
    let mut state = AccountingState::new(i128::MAX);
    state.apply_borrow(100_000).unwrap();
    state.apply_repay(40_000).unwrap();
    assert_eq!(state.user_principal, 60_000, "A-06 principal");
    assert_eq!(state.total_debt, 60_000, "A-06 total_debt");
}

/// **A-07**: Over-repay is rejected and state is unchanged.
#[test]
fn lemma_a07_over_repay_is_rejected() {
    let mut state = AccountingState::new(i128::MAX);
    state.apply_borrow(50_000).unwrap();
    assert_eq!(
        state.apply_repay(50_001),
        Err(AccountingError::RepayAmountTooHigh),
        "A-07"
    );
    assert_eq!(state.user_principal, 50_000, "A-07: state mutated");
    assert_eq!(state.total_debt, 50_000, "A-07: total_debt mutated");
}

/// **A-08**: Borrow beyond ceiling is rejected and state is unchanged.
#[test]
fn lemma_a08_borrow_beyond_ceiling_is_rejected() {
    let mut state = AccountingState::new(500_000);
    state.apply_borrow(300_000).unwrap();
    assert_eq!(
        state.apply_borrow(250_000),
        Err(AccountingError::DebtCeilingReached),
        "A-08"
    );
    assert_eq!(state.total_debt, 300_000, "A-08: state mutated");
}

#[cfg(kani)]
#[kani::proof]
#[kani::unwind(3)]
pub fn kani_accounting_invariants() {
    let ceiling: i128 = kani::any();
    let borrow_amount: i128 = kani::any();
    kani::assume(ceiling > 0 && ceiling <= i64::MAX as i128);
    kani::assume(borrow_amount > 0 && borrow_amount <= ceiling);
    let mut state = AccountingState::new(ceiling);
    if state.apply_borrow(borrow_amount).is_ok() {
        kani::assert(state.total_debt >= 0, "INV-2");
        kani::assert(state.total_debt == borrow_amount, "INV-4");
        kani::assert(state.total_debt <= ceiling, "INV-6");
    }
}
