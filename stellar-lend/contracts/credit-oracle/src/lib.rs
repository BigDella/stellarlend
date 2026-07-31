#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, Vec};

const SCORE_MAX_AGE_SECONDS: u64 = 30 * 24 * 60 * 60; // 30 days

#[derive(Clone)]
#[contracttype]
pub struct CreditScore {
    pub user: Address,
    pub score: u32,
    pub timestamp: u64,
    pub borrow_limit: i128,
    pub status: u32, // 0 = active, 1 = disputed
}

#[derive(Clone)]
#[contracttype]
pub struct CreditTier {
    pub min_score: u32,
    pub max_score: u32,
    pub borrow_limit_factor: u32, // multiplied by 10000
}

#[contract]
pub struct CreditOracle;

#[contractimpl]
impl CreditOracle {
    pub fn initialize(env: Env, admin: Address, oracle_source: Address) {
        admin.require_auth();

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "admin"), &admin);

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "oracle_source"), &oracle_source);

        let mut tiers: Vec<CreditTier> = Vec::new(&env);

        tiers.push_back(CreditTier {
            min_score: 0,
            max_score: 400,
            borrow_limit_factor: 0,
        });

        tiers.push_back(CreditTier {
            min_score: 401,
            max_score: 600,
            borrow_limit_factor: 2500, // 25%
        });

        tiers.push_back(CreditTier {
            min_score: 601,
            max_score: 750,
            borrow_limit_factor: 5000, // 50%
        });

        tiers.push_back(CreditTier {
            min_score: 751,
            max_score: 850,
            borrow_limit_factor: 7500, // 75%
        });

        tiers.push_back(CreditTier {
            min_score: 851,
            max_score: 1000,
            borrow_limit_factor: 10000, // 100%
        });

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "tiers"), &tiers);
    }

    pub fn update_credit_score(
        env: Env,
        user: Address,
        score: u32,
        collateral_value: i128,
    ) {
        let oracle_source: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "oracle_source"))
            .expect("Oracle source not set");

        oracle_source.require_auth();

        if score > 1000 {
            panic!("Credit score cannot exceed 1000");
        }

        let tiers: Vec<CreditTier> = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "tiers"))
            .unwrap_or_else(|| Vec::new(&env));

        let tier = tiers
            .iter()
            .find(|t| score >= t.min_score && score <= t.max_score);

        if tier.is_none() {
            panic!("No tier found for score");
        }

        let tier = tier.unwrap();
        let borrow_limit = (collateral_value * (tier.borrow_limit_factor as i128)) / 10000i128;

        let credit_score = CreditScore {
            user: user.clone(),
            score,
            timestamp: env.ledger().timestamp(),
            borrow_limit,
            status: 0,
        };

        let mut scores: Vec<CreditScore> = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "scores"))
            .unwrap_or_else(|| Vec::new(&env));

        let existing = scores.iter().position(|s| s.user == user);

        if let Some(idx) = existing {
            scores.set(idx, credit_score.clone());
        } else {
            scores.push_back(credit_score.clone());
        }

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "scores"), &scores);

        env.events()
            .publish(
                (Symbol::new(&env, "score_updated"),),
                (user, score, borrow_limit),
            );
    }

    pub fn get_credit_score(env: Env, user: Address) -> Option<CreditScore> {
        let scores: Vec<CreditScore> = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "scores"))
            .unwrap_or_else(|| Vec::new(&env));

        let score = scores.iter().find(|s| s.user == user).cloned();

        if let Some(s) = score {
            let age = env.ledger().timestamp() - s.timestamp;

            if age > SCORE_MAX_AGE_SECONDS {
                return None;
            }

            Some(s)
        } else {
            None
        }
    }

    pub fn get_borrow_limit(env: Env, user: Address) -> Option<i128> {
        let score = Self::get_credit_score(&env, user);

        score.map(|s| s.borrow_limit)
    }

    pub fn is_score_fresh(env: Env, user: Address) -> bool {
        let score = Self::get_credit_score(&env, user);

        score.is_some()
    }

    pub fn dispute_score(env: Env, user: Address) {
        user.require_auth();

        let mut scores: Vec<CreditScore> = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "scores"))
            .unwrap_or_else(|| Vec::new(&env));

        let existing = scores.iter().position(|s| s.user == user);

        if let Some(idx) = existing {
            let mut score = scores.get(idx).unwrap();
            score.status = 1;
            scores.set(idx, score);

            env.storage()
                .instance()
                .set(&Symbol::new(&env, "scores"), &scores);

            env.events()
                .publish((Symbol::new(&env, "score_disputed"),), user);
        }
    }

    pub fn resolve_dispute(env: Env, user: Address, new_score: u32) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin"))
            .expect("Admin not set");

        admin.require_auth();

        Self::update_credit_score(&env, user.clone(), new_score, 0i128);

        let mut scores: Vec<CreditScore> = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "scores"))
            .unwrap_or_else(|| Vec::new(&env));

        let existing = scores.iter().position(|s| s.user == user);

        if let Some(idx) = existing {
            let mut score = scores.get(idx).unwrap();
            score.status = 0;
            scores.set(idx, score);

            env.storage()
                .instance()
                .set(&Symbol::new(&env, "scores"), &scores);
        }

        env.events()
            .publish((Symbol::new(&env, "dispute_resolved"),), user);
    }

    pub fn get_all_scores(env: Env) -> Vec<CreditScore> {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, "scores"))
            .unwrap_or_else(|| Vec::new(&env))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
}
