use soroban_sdk::{Address, Env, Vec, contracttype};

use crate::errors::LendingError;
use crate::storage;

/// Credit score range: 0-1000
pub const MIN_CREDIT_SCORE: i128 = 0;
pub const MAX_CREDIT_SCORE: i128 = 1000;
pub const DEFAULT_CREDIT_SCORE: i128 = 500;

/// Score impact factors (in basis points)
pub const ON_TIME_REPAYMENT_BONUS: i128 = 10; // +1% per on-time repayment
pub const LATE_REPAYMENT_PENALTY: i128 = 50; // -5% per late repayment
pub const LIQUIDATION_PENALTY: i128 = 100; // -10% per liquidation
pub const HEALTHY_POSITION_BONUS: i128 = 5; // +0.5% for maintaining healthy position

/// LTV adjustments based on credit score (in basis points)
pub const BASE_LTV_BPS: i128 = 7000; // 70% base LTV
pub const MAX_LTV_BONUS_BPS: i128 = 1500; // Up to +15% for excellent credit
pub const MAX_LTV_PENALTY_BPS: i128 = 2000; // Up to -20% for poor credit

/// Interest rate adjustments (in basis points)
pub const BASE_RATE_ADJUSTMENT_BPS: i128 = 500; // ±5% max rate adjustment

#[derive(Clone, Debug)]
#[contracttype]
pub struct CreditScore {
    pub user: Address,
    pub score: i128,
    pub on_time_repayments: u32,
    pub late_repayments: u32,
    pub liquidations: u32,
    pub total_borrowed: i128,
    pub total_repaid: i128,
    pub last_updated: u64,
    pub history: Vec<CreditScoreSnapshot>,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct CreditScoreSnapshot {
    pub score: i128,
    pub timestamp: u64,
    pub reason: CreditScoreChangeReason,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum CreditScoreChangeReason {
    OnTimeRepayment,
    LateRepayment,
    Liquidation,
    HealthyPosition,
    NewUser,
}

/// Initialize credit score for a new user
pub fn initialize_credit_score(env: &Env, user: &Address) -> Result<(), LendingError> {
    let key = storage::DataKey::CreditScore(user.clone());
    
    if env.storage().persistent().has(&key) {
        return Ok(()); // Already initialized
    }

    let mut history = Vec::new(env);
    history.push_back(CreditScoreSnapshot {
        score: DEFAULT_CREDIT_SCORE,
        timestamp: env.ledger().timestamp(),
        reason: CreditScoreChangeReason::NewUser,
    });

    let credit_score = CreditScore {
        user: user.clone(),
        score: DEFAULT_CREDIT_SCORE,
        on_time_repayments: 0,
        late_repayments: 0,
        liquidations: 0,
        total_borrowed: 0,
        total_repaid: 0,
        last_updated: env.ledger().timestamp(),
        history,
    };

    env.storage().persistent().set(&key, &credit_score);
    Ok(())
}

/// Get credit score for a user
pub fn get_credit_score(env: &Env, user: &Address) -> Result<CreditScore, LendingError> {
    let key = storage::DataKey::CreditScore(user.clone());
    env.storage()
        .persistent()
        .get(&key)
        .ok_or(LendingError::NotFound)
}

/// Update credit score after repayment
pub fn update_score_on_repayment(
    env: &Env,
    user: &Address,
    amount: i128,
    is_on_time: bool,
) -> Result<(), LendingError> {
    initialize_credit_score(env, user)?;
    
    let key = storage::DataKey::CreditScore(user.clone());
    let mut credit_score: CreditScore = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(LendingError::NotFound)?;

    credit_score.total_repaid += amount;

    let (score_change, reason) = if is_on_time {
        credit_score.on_time_repayments += 1;
        (ON_TIME_REPAYMENT_BONUS, CreditScoreChangeReason::OnTimeRepayment)
    } else {
        credit_score.late_repayments += 1;
        (-LATE_REPAYMENT_PENALTY, CreditScoreChangeReason::LateRepayment)
    };

    credit_score.score = (credit_score.score + score_change)
        .max(MIN_CREDIT_SCORE)
        .min(MAX_CREDIT_SCORE);
    credit_score.last_updated = env.ledger().timestamp();

    // Add to history
    credit_score.history.push_back(CreditScoreSnapshot {
        score: credit_score.score,
        timestamp: env.ledger().timestamp(),
        reason,
    });

    env.storage().persistent().set(&key, &credit_score);
    Ok(())
}

/// Update credit score after liquidation
pub fn update_score_on_liquidation(env: &Env, user: &Address) -> Result<(), LendingError> {
    initialize_credit_score(env, user)?;
    
    let key = storage::DataKey::CreditScore(user.clone());
    let mut credit_score: CreditScore = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(LendingError::NotFound)?;

    credit_score.liquidations += 1;
    credit_score.score = (credit_score.score - LIQUIDATION_PENALTY)
        .max(MIN_CREDIT_SCORE);
    credit_score.last_updated = env.ledger().timestamp();

    credit_score.history.push_back(CreditScoreSnapshot {
        score: credit_score.score,
        timestamp: env.ledger().timestamp(),
        reason: CreditScoreChangeReason::Liquidation,
    });

    env.storage().persistent().set(&key, &credit_score);
    Ok(())
}

/// Update credit score for maintaining healthy position
pub fn update_score_on_healthy_position(env: &Env, user: &Address) -> Result<(), LendingError> {
    initialize_credit_score(env, user)?;
    
    let key = storage::DataKey::CreditScore(user.clone());
    let mut credit_score: CreditScore = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(LendingError::NotFound)?;

    credit_score.score = (credit_score.score + HEALTHY_POSITION_BONUS)
        .min(MAX_CREDIT_SCORE);
    credit_score.last_updated = env.ledger().timestamp();

    credit_score.history.push_back(CreditScoreSnapshot {
        score: credit_score.score,
        timestamp: env.ledger().timestamp(),
        reason: CreditScoreChangeReason::HealthyPosition,
    });

    env.storage().persistent().set(&key, &credit_score);
    Ok(())
}

/// Calculate adjusted LTV based on credit score
pub fn calculate_adjusted_ltv(env: &Env, user: &Address) -> Result<i128, LendingError> {
    let credit_score = get_credit_score(env, user).unwrap_or(CreditScore {
        user: user.clone(),
        score: DEFAULT_CREDIT_SCORE,
        on_time_repayments: 0,
        late_repayments: 0,
        liquidations: 0,
        total_borrowed: 0,
        total_repaid: 0,
        last_updated: env.ledger().timestamp(),
        history: Vec::new(env),
    });

    // Score ranges: 0-300 (poor), 301-600 (fair), 601-800 (good), 801-1000 (excellent)
    let adjustment = if credit_score.score >= 800 {
        // Excellent: +15% LTV
        MAX_LTV_BONUS_BPS
    } else if credit_score.score >= 600 {
        // Good: +5% to +15% LTV (linear)
        ((credit_score.score - 600) * MAX_LTV_BONUS_BPS) / 200
    } else if credit_score.score >= 300 {
        // Fair: -5% to +5% LTV (linear)
        ((credit_score.score - 450) * MAX_LTV_BONUS_BPS) / 150
    } else {
        // Poor: -20% to -5% LTV
        -MAX_LTV_PENALTY_BPS + ((credit_score.score * (MAX_LTV_PENALTY_BPS - 500)) / 300)
    };

    Ok(BASE_LTV_BPS + adjustment)
}

/// Calculate adjusted interest rate based on credit score
pub fn calculate_adjusted_interest_rate(
    env: &Env,
    user: &Address,
    base_rate_bps: i128,
) -> Result<i128, LendingError> {
    let credit_score = get_credit_score(env, user).unwrap_or(CreditScore {
        user: user.clone(),
        score: DEFAULT_CREDIT_SCORE,
        on_time_repayments: 0,
        late_repayments: 0,
        liquidations: 0,
        total_borrowed: 0,
        total_repaid: 0,
        last_updated: env.ledger().timestamp(),
        history: Vec::new(env),
    });

    // Better credit score = lower interest rate
    // Score 1000 = -5% rate, Score 500 = 0% adjustment, Score 0 = +5% rate
    let adjustment = ((500 - credit_score.score) * BASE_RATE_ADJUSTMENT_BPS) / 500;

    Ok(base_rate_bps + adjustment)
}

/// Record borrow for credit tracking
pub fn record_borrow(env: &Env, user: &Address, amount: i128) -> Result<(), LendingError> {
    initialize_credit_score(env, user)?;
    
    let key = storage::DataKey::CreditScore(user.clone());
    let mut credit_score: CreditScore = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(LendingError::NotFound)?;

    credit_score.total_borrowed += amount;
    env.storage().persistent().set(&key, &credit_score);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_initialize_credit_score() {
        let env = Env::default();
        let user = Address::generate(&env);

        initialize_credit_score(&env, &user).unwrap();
        let score = get_credit_score(&env, &user).unwrap();

        assert_eq!(score.score, DEFAULT_CREDIT_SCORE);
        assert_eq!(score.on_time_repayments, 0);
        assert_eq!(score.late_repayments, 0);
        assert_eq!(score.liquidations, 0);
    }

    #[test]
    fn test_on_time_repayment_increases_score() {
        let env = Env::default();
        let user = Address::generate(&env);

        initialize_credit_score(&env, &user).unwrap();
        update_score_on_repayment(&env, &user, 1000, true).unwrap();

        let score = get_credit_score(&env, &user).unwrap();
        assert_eq!(score.score, DEFAULT_CREDIT_SCORE + ON_TIME_REPAYMENT_BONUS);
        assert_eq!(score.on_time_repayments, 1);
    }

    #[test]
    fn test_late_repayment_decreases_score() {
        let env = Env::default();
        let user = Address::generate(&env);

        initialize_credit_score(&env, &user).unwrap();
        update_score_on_repayment(&env, &user, 1000, false).unwrap();

        let score = get_credit_score(&env, &user).unwrap();
        assert_eq!(score.score, DEFAULT_CREDIT_SCORE - LATE_REPAYMENT_PENALTY);
        assert_eq!(score.late_repayments, 1);
    }

    #[test]
    fn test_liquidation_decreases_score() {
        let env = Env::default();
        let user = Address::generate(&env);

        initialize_credit_score(&env, &user).unwrap();
        update_score_on_liquidation(&env, &user).unwrap();

        let score = get_credit_score(&env, &user).unwrap();
        assert_eq!(score.score, DEFAULT_CREDIT_SCORE - LIQUIDATION_PENALTY);
        assert_eq!(score.liquidations, 1);
    }

    #[test]
    fn test_score_bounds() {
        let env = Env::default();
        let user = Address::generate(&env);

        initialize_credit_score(&env, &user).unwrap();

        // Test max score
        for _ in 0..100 {
            update_score_on_repayment(&env, &user, 1000, true).unwrap();
        }
        let score = get_credit_score(&env, &user).unwrap();
        assert_eq!(score.score, MAX_CREDIT_SCORE);

        // Test min score
        for _ in 0..100 {
            update_score_on_liquidation(&env, &user).unwrap();
        }
        let score = get_credit_score(&env, &user).unwrap();
        assert_eq!(score.score, MIN_CREDIT_SCORE);
    }

    #[test]
    fn test_adjusted_ltv() {
        let env = Env::default();
        let user = Address::generate(&env);

        initialize_credit_score(&env, &user).unwrap();

        // Default score should give base LTV
        let ltv = calculate_adjusted_ltv(&env, &user).unwrap();
        assert!(ltv >= BASE_LTV_BPS - 1000 && ltv <= BASE_LTV_BPS + 1000);

        // Improve score to excellent
        for _ in 0..50 {
            update_score_on_repayment(&env, &user, 1000, true).unwrap();
        }

        let ltv = calculate_adjusted_ltv(&env, &user).unwrap();
        assert!(ltv > BASE_LTV_BPS);
    }
}
