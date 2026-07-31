#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, Vec};

#[derive(Clone)]
#[contracttype]
pub struct StakePosition {
    pub user: Address,
    pub amount: i128,
    pub staked_at: u64,
    pub last_claim: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct RewardDistribution {
    pub period_start: u64,
    pub period_end: u64,
    pub total_rewards: i128,
    pub distributed: i128,
}

#[contract]
pub struct Staking;

#[contractimpl]
impl Staking {
    pub fn initialize(env: Env, admin: Address, pool_token: Address, reward_token: Address) {
        admin.require_auth();

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "admin"), &admin);
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "pool_token"), &pool_token);
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "reward_token"), &reward_token);

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "total_staked"), &0i128);
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "reward_rate"), &0i128);
    }

    pub fn stake(env: Env, user: Address, amount: i128) {
        user.require_auth();

        if amount <= 0 {
            panic!("Stake amount must be positive");
        }

        let mut stakes: Vec<StakePosition> = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "stakes"))
            .unwrap_or_else(|| Vec::new(&env));

        let existing_stake = stakes.iter().find(|s| s.user == user);

        if let Some(stake) = existing_stake {
            let mut updated_stake = stake.clone();
            updated_stake.amount += amount;
            stakes.push_back(updated_stake);
        } else {
            let new_stake = StakePosition {
                user: user.clone(),
                amount,
                staked_at: env.ledger().timestamp(),
                last_claim: env.ledger().timestamp(),
            };
            stakes.push_back(new_stake);
        }

        let total_staked: i128 = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "total_staked"))
            .unwrap_or(0);

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "total_staked"), &(total_staked + amount));

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "stakes"), &stakes);

        env.events()
            .publish((Symbol::new(&env, "staked"),), (user, amount));
    }

    pub fn unstake(env: Env, user: Address, amount: i128) {
        user.require_auth();

        if amount <= 0 {
            panic!("Unstake amount must be positive");
        }

        let mut stakes: Vec<StakePosition> = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "stakes"))
            .unwrap_or_else(|| Vec::new(&env));

        let stake = stakes.iter().find(|s| s.user == user);

        if let Some(s) = stake {
            if s.amount < amount {
                panic!("Insufficient stake balance");
            }

            let mut updated_stake = s.clone();
            updated_stake.amount -= amount;

            if updated_stake.amount == 0 {
                stakes.retain(|s| s.user != user);
            } else {
                stakes.push_back(updated_stake);
            }

            let total_staked: i128 = env
                .storage()
                .instance()
                .get(&Symbol::new(&env, "total_staked"))
                .unwrap_or(0);

            env.storage()
                .instance()
                .set(&Symbol::new(&env, "total_staked"), &(total_staked - amount));

            env.storage()
                .instance()
                .set(&Symbol::new(&env, "stakes"), &stakes);

            env.events()
                .publish((Symbol::new(&env, "unstaked"),), (user, amount));
        } else {
            panic!("No stake found for user");
        }
    }

    pub fn claim_rewards(env: Env, user: Address) -> i128 {
        user.require_auth();

        let stakes: Vec<StakePosition> = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "stakes"))
            .unwrap_or_else(|| Vec::new(&env));

        let stake = stakes.iter().find(|s| s.user == user);

        if let Some(s) = stake {
            let reward_rate: i128 = env
                .storage()
                .instance()
                .get(&Symbol::new(&env, "reward_rate"))
                .unwrap_or(0);

            let current_time = env.ledger().timestamp();
            let time_delta = (current_time - s.last_claim) as i128;

            let rewards = (s.amount * reward_rate * time_delta) / 1_000_000_000i128;

            if rewards > 0 {
                env.events()
                    .publish((Symbol::new(&env, "rewards_claimed"),), (user, rewards));
            }

            rewards
        } else {
            panic!("No stake found for user");
        }
    }

    pub fn get_stake(env: Env, user: Address) -> Option<StakePosition> {
        let stakes: Vec<StakePosition> = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "stakes"))
            .unwrap_or_else(|| Vec::new(&env));

        stakes.iter().find(|s| s.user == user).cloned()
    }

    pub fn set_reward_rate(env: Env, rate: i128) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin"))
            .expect("Admin not set");

        admin.require_auth();

        if rate < 0 {
            panic!("Reward rate cannot be negative");
        }

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "reward_rate"), &rate);

        env.events()
            .publish((Symbol::new(&env, "reward_rate_updated"),), rate);
    }

    pub fn get_total_staked(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, "total_staked"))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
}
