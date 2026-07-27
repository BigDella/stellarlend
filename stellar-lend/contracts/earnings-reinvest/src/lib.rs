#![no_std]

//! Earnings reinvestment planner.
//!
//! Lets a lender configure a strategy for automatically routing earned
//! interest back into lending pools instead of letting it sit idle. The
//! actual pool-selection logic for the `BestApy` strategy (comparing APYs
//! across pools) is resolved off-chain by the caller and passed in via
//! `target_pool` — this contract focuses on plan state, schedule/threshold
//! gating, and recording reinvestment events for cost-basis tracking.

use soroban_sdk::{contract, contracterror, contractevent, contractimpl, contracttype, Address, Env, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ReinvestError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    PlanNotFound = 4,
    InvalidThreshold = 5,
    InvalidWeights = 6,
    PlanPaused = 7,
    PlanNotPaused = 8,
    BelowThreshold = 9,
    PoolPaused = 10,
    GasExceedsEarnings = 11,
    ScheduleNotDue = 12,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ReinvestStrategy {
    SamePool = 0,
    BestApy = 1,
    Weighted = 2,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ReinvestSchedule {
    RealTime = 0,
    Daily = 1,
    Weekly = 2,
    Threshold = 3,
}

/// One target pool + share (in basis points, must sum to 10_000 across a plan) for the Weighted strategy.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeightedTarget {
    pub pool: Address,
    pub weight_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ReinvestPlan {
    pub id: u64,
    pub owner: Address,
    pub source_pool: Address,
    pub strategy: ReinvestStrategy,
    pub schedule: ReinvestSchedule,
    /// Minimum earned amount required before a sweep executes.
    pub threshold: i128,
    /// Only populated (and used) when `strategy == Weighted`.
    pub weighted_targets: Vec<WeightedTarget>,
    pub paused: bool,
    pub total_reinvested: i128,
    pub total_sweeps: u32,
    pub created_at: u64,
    pub last_swept_at: u64,
    /// Earliest ledger a Daily/Weekly-scheduled sweep may execute again; unused for RealTime/Threshold.
    pub next_eligible_ledger: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ReinvestEvent {
    pub plan_id: u64,
    pub pool: Address,
    pub amount: i128,
    /// Cost basis recorded for this reinvestment lot (equal to `amount` at time of reinvestment).
    pub cost_basis: i128,
    pub swept_at: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    NextId,
    Plan(u64),
    UserPlans(Address),
    History(u64),
}

const DAY_LEDGERS: u64 = 17_280;
const WEEK_LEDGERS: u64 = DAY_LEDGERS * 7;
const MAX_HISTORY_LEN: u32 = 50;
const BPS_DENOMINATOR: u32 = 10_000;

#[contractevent]
#[derive(Clone)]
pub struct PlanCreatedEvent {
    #[topic]
    pub plan_id: u64,
    #[topic]
    pub owner: Address,
}

#[contractevent]
#[derive(Clone)]
pub struct SweptEvent {
    #[topic]
    pub plan_id: u64,
    pub earned: i128,
    pub event_count: u32,
}

#[contract]
pub struct EarningsReinvestContract;

#[contractimpl]
impl EarningsReinvestContract {
    pub fn initialize(env: Env, admin: Address) -> Result<(), ReinvestError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(ReinvestError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextId, &1u64);
        Ok(())
    }

    /// Create a reinvestment plan for `owner`, sourced from `source_pool`.
    ///
    /// `weighted_targets` is only validated/used when `strategy == Weighted`, in which case
    /// its `weight_bps` values must sum to exactly 10_000 (100%).
    pub fn create_plan(
        env: Env,
        owner: Address,
        source_pool: Address,
        strategy: ReinvestStrategy,
        schedule: ReinvestSchedule,
        threshold: i128,
        weighted_targets: Vec<WeightedTarget>,
    ) -> Result<u64, ReinvestError> {
        owner.require_auth();
        Self::require_initialized(&env)?;

        if threshold < 0 {
            return Err(ReinvestError::InvalidThreshold);
        }

        if strategy == ReinvestStrategy::Weighted {
            if weighted_targets.is_empty() {
                return Err(ReinvestError::InvalidWeights);
            }
            let mut total_bps: u32 = 0;
            for target in weighted_targets.iter() {
                total_bps = total_bps
                    .checked_add(target.weight_bps)
                    .ok_or(ReinvestError::InvalidWeights)?;
            }
            if total_bps != BPS_DENOMINATOR {
                return Err(ReinvestError::InvalidWeights);
            }
        } else if !weighted_targets.is_empty() {
            return Err(ReinvestError::InvalidWeights);
        }

        let plan_id = Self::next_id(&env);
        let current_ledger = env.ledger().sequence() as u64;

        let plan = ReinvestPlan {
            id: plan_id,
            owner: owner.clone(),
            source_pool,
            strategy,
            schedule,
            threshold,
            weighted_targets,
            paused: false,
            total_reinvested: 0,
            total_sweeps: 0,
            created_at: current_ledger,
            last_swept_at: 0,
            next_eligible_ledger: 0,
        };

        env.storage().persistent().set(&DataKey::Plan(plan_id), &plan);

        let mut user_plans = Self::get_user_plans_internal(&env, &owner);
        user_plans.push_back(plan_id);
        env.storage()
            .persistent()
            .set(&DataKey::UserPlans(owner.clone()), &user_plans);

        PlanCreatedEvent { plan_id, owner }.publish(&env);

        Ok(plan_id)
    }

    /// Sweep earned interest into the plan's target pool(s), subject to pause state, the
    /// plan's threshold, schedule cadence, pool-paused state, and gas-vs-earnings viability.
    ///
    /// `target_pool` is the resolved best-APY pool when `strategy == BestApy` (ignored otherwise).
    /// `pool_paused` and `estimated_gas_cost` are supplied by the caller (an off-chain keeper),
    /// since this contract does not itself cross-call into lending pools.
    pub fn sweep(
        env: Env,
        keeper: Address,
        plan_id: u64,
        target_pool: Address,
        earned: i128,
        pool_paused: bool,
        estimated_gas_cost: i128,
    ) -> Result<Vec<ReinvestEvent>, ReinvestError> {
        keeper.require_auth();

        let mut plan: ReinvestPlan = env
            .storage()
            .persistent()
            .get(&DataKey::Plan(plan_id))
            .ok_or(ReinvestError::PlanNotFound)?;

        if plan.paused {
            return Err(ReinvestError::PlanPaused);
        }
        if pool_paused {
            return Err(ReinvestError::PoolPaused);
        }
        if earned < plan.threshold {
            return Err(ReinvestError::BelowThreshold);
        }
        if estimated_gas_cost >= earned {
            return Err(ReinvestError::GasExceedsEarnings);
        }

        let current_ledger = env.ledger().sequence() as u64;
        let schedule_gap = Self::schedule_gap_ledgers(&plan.schedule);
        if schedule_gap > 0 && current_ledger < plan.next_eligible_ledger {
            return Err(ReinvestError::ScheduleNotDue);
        }

        let events = Self::build_events(&env, &plan, &target_pool, earned, current_ledger);

        plan.total_reinvested += earned;
        plan.total_sweeps += 1;
        plan.last_swept_at = current_ledger;
        if schedule_gap > 0 {
            plan.next_eligible_ledger = current_ledger + schedule_gap;
        }
        env.storage().persistent().set(&DataKey::Plan(plan_id), &plan);

        Self::append_history(&env, plan_id, &events);

        SweptEvent {
            plan_id,
            earned,
            event_count: events.len(),
        }
        .publish(&env);

        Ok(events)
    }

    pub fn pause(env: Env, owner: Address, plan_id: u64) -> Result<(), ReinvestError> {
        let mut plan = Self::load_owned_plan(&env, &owner, plan_id)?;
        if plan.paused {
            return Err(ReinvestError::PlanPaused);
        }
        plan.paused = true;
        env.storage().persistent().set(&DataKey::Plan(plan_id), &plan);
        Ok(())
    }

    pub fn resume(env: Env, owner: Address, plan_id: u64) -> Result<(), ReinvestError> {
        let mut plan = Self::load_owned_plan(&env, &owner, plan_id)?;
        if !plan.paused {
            return Err(ReinvestError::PlanNotPaused);
        }
        plan.paused = false;
        env.storage().persistent().set(&DataKey::Plan(plan_id), &plan);
        Ok(())
    }

    pub fn get_plan(env: Env, plan_id: u64) -> Option<ReinvestPlan> {
        env.storage().persistent().get(&DataKey::Plan(plan_id))
    }

    pub fn get_user_plans(env: Env, owner: Address) -> Vec<u64> {
        Self::get_user_plans_internal(&env, &owner)
    }

    pub fn get_history(env: Env, plan_id: u64) -> Vec<ReinvestEvent> {
        env.storage()
            .persistent()
            .get(&DataKey::History(plan_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    fn load_owned_plan(env: &Env, owner: &Address, plan_id: u64) -> Result<ReinvestPlan, ReinvestError> {
        owner.require_auth();
        let plan: ReinvestPlan = env
            .storage()
            .persistent()
            .get(&DataKey::Plan(plan_id))
            .ok_or(ReinvestError::PlanNotFound)?;
        if &plan.owner != owner {
            return Err(ReinvestError::Unauthorized);
        }
        Ok(plan)
    }

    fn build_events(
        env: &Env,
        plan: &ReinvestPlan,
        target_pool: &Address,
        earned: i128,
        current_ledger: u64,
    ) -> Vec<ReinvestEvent> {
        let mut events = Vec::new(env);
        match plan.strategy {
            ReinvestStrategy::SamePool => {
                events.push_back(ReinvestEvent {
                    plan_id: plan.id,
                    pool: plan.source_pool.clone(),
                    amount: earned,
                    cost_basis: earned,
                    swept_at: current_ledger,
                });
            }
            ReinvestStrategy::BestApy => {
                events.push_back(ReinvestEvent {
                    plan_id: plan.id,
                    pool: target_pool.clone(),
                    amount: earned,
                    cost_basis: earned,
                    swept_at: current_ledger,
                });
            }
            ReinvestStrategy::Weighted => {
                let count = plan.weighted_targets.len();
                let mut allocated: i128 = 0;
                for (i, target) in plan.weighted_targets.iter().enumerate() {
                    let share = if i as u32 == count - 1 {
                        // Last target absorbs any rounding remainder so the full
                        // `earned` amount is always accounted for (no dust lost).
                        earned - allocated
                    } else {
                        earned * (target.weight_bps as i128) / (BPS_DENOMINATOR as i128)
                    };
                    allocated += share;
                    events.push_back(ReinvestEvent {
                        plan_id: plan.id,
                        pool: target.pool.clone(),
                        amount: share,
                        cost_basis: share,
                        swept_at: current_ledger,
                    });
                }
            }
        }
        events
    }

    fn append_history(env: &Env, plan_id: u64, new_events: &Vec<ReinvestEvent>) {
        let mut history: Vec<ReinvestEvent> = env
            .storage()
            .persistent()
            .get(&DataKey::History(plan_id))
            .unwrap_or_else(|| Vec::new(env));
        for event in new_events.iter() {
            if history.len() >= MAX_HISTORY_LEN {
                history.remove(0);
            }
            history.push_back(event);
        }
        env.storage().persistent().set(&DataKey::History(plan_id), &history);
    }

    fn require_initialized(env: &Env) -> Result<(), ReinvestError> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(ReinvestError::NotInitialized);
        }
        Ok(())
    }

    fn next_id(env: &Env) -> u64 {
        let id: u64 = env.storage().instance().get(&DataKey::NextId).unwrap_or(1);
        env.storage().instance().set(&DataKey::NextId, &(id + 1));
        id
    }

    fn schedule_gap_ledgers(schedule: &ReinvestSchedule) -> u64 {
        match schedule {
            ReinvestSchedule::RealTime | ReinvestSchedule::Threshold => 0,
            ReinvestSchedule::Daily => DAY_LEDGERS,
            ReinvestSchedule::Weekly => WEEK_LEDGERS,
        }
    }

    fn get_user_plans_internal(env: &Env, owner: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::UserPlans(owner.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }
}

#[cfg(test)]
mod lib_test;
