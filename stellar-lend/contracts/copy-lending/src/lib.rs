#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, xdr::ToXdr, Address, BytesN, Env, Map, String, Vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Strategy {
    pub leader: Address,
    pub allocation: Map<Address, i128>,
    pub rebalance_frequency: u64,
    pub risk_level: u32,
    pub description: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FollowRelation {
    pub follower: Address,
    pub leader: Address,
    pub invested_amount: i128,
    pub proportional_allocation: i128,
    pub total_profit: i128,
    pub leader_profit_share: i128,
    pub started_at: u32,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaderStats {
    pub total_followers: u32,
    pub total_follower_value: i128,
    pub total_returns: i128,
    pub apy: i128,
    pub risk_adjusted_returns: i128,
    pub volatility: i128,
}

#[contracttype]
pub enum CopyLendingDataKey {
    Strategy(Address),
    Follow(BytesN<32>),
    LeaderStats(Address),
    OptOut(Address),
    FollowerCount(Address),
    FollowerList(Address),
}

fn get_follow_key(env: &Env, follower: &Address, leader: &Address) -> BytesN<32> {
    let mut combined: Vec<Address> = Vec::new(env);
    combined.push_back(follower.clone());
    combined.push_back(leader.clone());
    let xdr = combined.to_xdr(env);
    env.crypto().sha256(&xdr).into()
}

#[contract]
pub struct CopyLendingContract;

#[contractimpl]
impl CopyLendingContract {
    pub fn set_strategy(env: Env, leader: Address, strategy: Strategy) {
        leader.require_auth();
        env.storage()
            .instance()
            .set(&CopyLendingDataKey::Strategy(leader.clone()), &strategy);
    }

    pub fn get_strategy(env: Env, leader: Address) -> Option<Strategy> {
        env.storage()
            .instance()
            .get(&CopyLendingDataKey::Strategy(leader))
    }

    pub fn follow(env: Env, follower: Address, leader: Address, amount: i128) -> FollowRelation {
        follower.require_auth();

        let min_investment: i128 = 1_000_000;
        if amount < min_investment {
            panic!("amount below minimum investment");
        }

        let opt_out: bool = env
            .storage()
            .instance()
            .get(&CopyLendingDataKey::OptOut(leader.clone()))
            .unwrap_or(false);
        if opt_out {
            panic!("leader has opted out of copying");
        }

        let follow_key = get_follow_key(&env, &follower, &leader);
        if env
            .storage()
            .instance()
            .has(&CopyLendingDataKey::Follow(follow_key.clone()))
        {
            panic!("already following this leader");
        }

        let ledger_seq = env.ledger().sequence();

        let relation = FollowRelation {
            follower: follower.clone(),
            leader: leader.clone(),
            invested_amount: amount,
            proportional_allocation: 0,
            total_profit: 0,
            leader_profit_share: 0,
            started_at: ledger_seq,
            active: true,
        };

        env.storage()
            .instance()
            .set(&CopyLendingDataKey::Follow(follow_key), &relation);

        let count: u32 = env
            .storage()
            .instance()
            .get(&CopyLendingDataKey::FollowerCount(leader.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &CopyLendingDataKey::FollowerCount(leader.clone()),
            &(count + 1),
        );

        relation
    }

    pub fn unfollow(env: Env, follower: Address, leader: Address) -> Option<FollowRelation> {
        follower.require_auth();

        let follow_key = get_follow_key(&env, &follower, &leader);
        let mut relation: FollowRelation = env
            .storage()
            .instance()
            .get(&CopyLendingDataKey::Follow(follow_key.clone()))?;

        relation.active = false;
        env.storage()
            .instance()
            .set(&CopyLendingDataKey::Follow(follow_key), &relation);

        let count: u32 = env
            .storage()
            .instance()
            .get(&CopyLendingDataKey::FollowerCount(leader.clone()))
            .unwrap_or(1);
        if count > 0 {
            env.storage().instance().set(
                &CopyLendingDataKey::FollowerCount(leader.clone()),
                &(count - 1),
            );
        }

        Some(relation)
    }

    pub fn get_follow_relation(
        env: Env,
        follower: Address,
        leader: Address,
    ) -> Option<FollowRelation> {
        let follow_key = get_follow_key(&env, &follower, &leader);
        env.storage()
            .instance()
            .get(&CopyLendingDataKey::Follow(follow_key))
    }

    pub fn get_leader_stats(env: Env, leader: Address) -> Option<LeaderStats> {
        env.storage()
            .instance()
            .get(&CopyLendingDataKey::LeaderStats(leader))
    }

    pub fn update_leader_stats(env: Env, leader: Address, stats: LeaderStats) {
        leader.require_auth();
        env.storage()
            .instance()
            .set(&CopyLendingDataKey::LeaderStats(leader), &stats);
    }

    pub fn set_opt_out(env: Env, leader: Address, opt_out: bool) {
        leader.require_auth();
        env.storage()
            .instance()
            .set(&CopyLendingDataKey::OptOut(leader), &opt_out);
    }

    pub fn is_opted_out(env: Env, leader: Address) -> bool {
        env.storage()
            .instance()
            .get(&CopyLendingDataKey::OptOut(leader))
            .unwrap_or(false)
    }

    pub fn get_follower_count(env: Env, leader: Address) -> u32 {
        env.storage()
            .instance()
            .get(&CopyLendingDataKey::FollowerCount(leader))
            .unwrap_or(0)
    }
}
