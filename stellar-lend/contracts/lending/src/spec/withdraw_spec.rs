//! # Formal Specification — `withdraw`
//!
//! ## Properties
//!
//! | ID   | Property                                                                 |
//! |------|--------------------------------------------------------------------------|
//! | W-01 | Zero amount is rejected                                                  |
//! | W-02 | Withdraw more than balance is rejected                                   |
//! | W-03 | Remaining balance ≥ min_collateral_ratio * outstanding_debt              |
//! | W-04 | User balance decreases by exactly the withdrawn amount                   |
//! | W-05 | Total deposits decreases by exactly the withdrawn amount                 |
//! | W-06 | Full withdraw with no debt succeeds                                      |
//! | W-07 | Withdraw leaving insufficient collateral for active debt is rejected     |

const MIN_COLLATERAL_RATIO_BPS: i128 = 15_000;
const BPS_DIVISOR: i128 = 10_000;

#[derive(Clone, Debug)]
pub struct WithdrawState {
    pub user_collateral: i128,
    pub total_deposits: i128,
    pub outstanding_debt: i128,
    pub min_withdraw: i128,
}

#[derive(Debug, PartialEq)]
pub enum WithdrawError {
    InvalidAmount,
    InsufficientCollateral,
    InsufficientCollateralRatio,
    Overflow,
}

impl WithdrawState {
    pub fn new(collateral: i128, total: i128, debt: i128, min: i128) -> Self {
        Self {
            user_collateral: collateral,
            total_deposits: total,
            outstanding_debt: debt,
            min_withdraw: min,
        }
    }

    pub fn apply_withdraw(&mut self, amount: i128) -> Result<(), WithdrawError> {
        if amount <= 0 || amount < self.min_withdraw {
            return Err(WithdrawError::InvalidAmount);
        }
        if amount > self.user_collateral {
            return Err(WithdrawError::InsufficientCollateral);
        }
        let remaining = self
            .user_collateral
            .checked_sub(amount)
            .ok_or(WithdrawError::Overflow)?;
        // Enforce collateral ratio if user has outstanding debt
        if self.outstanding_debt > 0 {
            let min_collateral = self
                .outstanding_debt
                .checked_mul(MIN_COLLATERAL_RATIO_BPS)
                .ok_or(WithdrawError::Overflow)?
                .checked_div(BPS_DIVISOR)
                .ok_or(WithdrawError::Overflow)?;
            if remaining < min_collateral {
                return Err(WithdrawError::InsufficientCollateralRatio);
            }
        }
        self.user_collateral = remaining;
        self.total_deposits = self.total_deposits.checked_sub(amount).unwrap_or(0);
        Ok(())
    }
}

/// **W-01**: Zero amount is rejected.
#[test]
fn lemma_w01_zero_amount_rejected() {
    let mut s = WithdrawState::new(100_000, 100_000, 0, 0);
    assert_eq!(
        s.apply_withdraw(0),
        Err(WithdrawError::InvalidAmount),
        "W-01"
    );
}

/// **W-02**: Withdraw exceeding balance is rejected.
#[test]
fn lemma_w02_over_balance_rejected() {
    let mut s = WithdrawState::new(100_000, 100_000, 0, 0);
    assert_eq!(
        s.apply_withdraw(100_001),
        Err(WithdrawError::InsufficientCollateral),
        "W-02"
    );
}

/// **W-03**: Remaining balance satisfies the collateral ratio for outstanding debt.
#[test]
fn lemma_w03_collateral_ratio_maintained() {
    // User: 150_000 collateral, 100_000 debt → ratio = 150% exactly at threshold
    let mut s = WithdrawState::new(150_000, 150_000, 100_000, 0);
    // Attempt to withdraw 1 unit, leaving 149_999 < 150_000 min → should fail
    assert_eq!(
        s.apply_withdraw(1),
        Err(WithdrawError::InsufficientCollateralRatio),
        "W-03: ratio below threshold should be rejected"
    );
    // Withdrawing 0 of existing debt leaves ratio intact (no-op via W-01)
    // Confirm balance is unchanged
    assert_eq!(
        s.user_collateral, 150_000,
        "W-03: balance mutated on rejection"
    );
}

/// **W-04**: User balance decreases by exactly the withdrawn amount.
#[test]
fn lemma_w04_user_balance_decremented_correctly() {
    let mut s = WithdrawState::new(500_000, 500_000, 0, 0);
    s.apply_withdraw(200_000).unwrap();
    assert_eq!(s.user_collateral, 300_000, "W-04");
}

/// **W-05**: Total deposits decreases by exactly the withdrawn amount.
#[test]
fn lemma_w05_total_decremented_correctly() {
    let mut s = WithdrawState::new(500_000, 500_000, 0, 0);
    s.apply_withdraw(200_000).unwrap();
    assert_eq!(s.total_deposits, 300_000, "W-05");
}

/// **W-06**: Full withdraw with no outstanding debt succeeds.
#[test]
fn lemma_w06_full_withdraw_no_debt() {
    let mut s = WithdrawState::new(500_000, 500_000, 0, 0);
    assert!(s.apply_withdraw(500_000).is_ok(), "W-06");
    assert_eq!(s.user_collateral, 0, "W-06: balance not zeroed");
}

/// **W-07**: Withdraw leaving insufficient collateral for active debt is rejected.
#[test]
fn lemma_w07_insufficient_collateral_for_debt_rejected() {
    // 200_000 collateral, 100_000 debt → min_collateral = 150_000
    // Withdrawing 100_000 leaves 100_000 < 150_000 → rejected
    let mut s = WithdrawState::new(200_000, 200_000, 100_000, 0);
    assert_eq!(
        s.apply_withdraw(100_000),
        Err(WithdrawError::InsufficientCollateralRatio),
        "W-07"
    );
    assert_eq!(
        s.user_collateral, 200_000,
        "W-07: state mutated on rejection"
    );
}
