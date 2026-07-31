#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env, symbol_short, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum DcaError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    PlanNotFound = 4,
    InvalidAmount = 5,
    InvalidFrequency = 6,
    PlanNotActive = 7,
    PlanAlreadyPaused = 8,
    PlanNotPaused = 9,
    ExecutionNotDue = 10,
    InsufficientFunds = 11,
    PlanCompleted = 12,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum DcaFrequency {
    Daily = 0,
    Weekly = 1,
    Monthly = 2,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum DcaPlanStatus {
    Active = 0,
    Paused = 1,
    Completed = 2,
    Cancelled = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum DcaDirection {
    Buy = 0,
    Sell = 1,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct DcaPlan {
    pub id: u64,
    pub owner: Address,
    pub asset: Address,
    pub amount_per_execution: i128,
    pub frequency: DcaFrequency,
    pub direction: DcaDirection,
    pub total_executions: u32,
    pub max_executions: u32,
    pub funded_amount: i128,
    pub spent_amount: i128,
    pub status: DcaPlanStatus,
    pub next_execution_ledger: u64,
    pub created_at: u64,
    pub last_executed_at: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct DcaExecution {
    pub plan_id: u64,
    pub execution_number: u32,
    pub amount: i128,
    pub executed_at: u64,
}

#[contract]
pub struct DcaModule;

const DAY_LEDGERS: u64 = 17_280;
const WEEK_LEDGERS: u64 = DAY_LEDGERS * 7;
const MONTH_LEDGERS: u64 = DAY_LEDGERS * 30;

#[contractimpl]
impl DcaModule {
    pub fn initialize(env: Env, admin: Address) -> Result<(), DcaError> {
        if env.storage().instance().has(&symbol_short!("admin")) {
            return Err(DcaError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&symbol_short!("admin"), &admin);
        env.storage().instance().set(&symbol_short!("next_id"), &1u64);
        Ok(())
    }

    pub fn create_plan(
        env: Env,
        owner: Address,
        asset: Address,
        amount_per_execution: i128,
        frequency: DcaFrequency,
        direction: DcaDirection,
        max_executions: u32,
        funded_amount: i128,
    ) -> Result<u64, DcaError> {
        owner.require_auth();
        Self::require_initialized(&env)?;

        if amount_per_execution <= 0 {
            return Err(DcaError::InvalidAmount);
        }
        if max_executions == 0 {
            return Err(DcaError::InvalidFrequency);
        }

        let current_ledger = env.ledger().sequence() as u64;
        let interval = Self::frequency_to_ledgers(&frequency);
        let plan_id = Self::next_id(&env);

        let plan = DcaPlan {
            id: plan_id,
            owner: owner.clone(),
            asset,
            amount_per_execution,
            frequency,
            direction,
            total_executions: 0,
            max_executions,
            funded_amount,
            spent_amount: 0,
            status: DcaPlanStatus::Active,
            next_execution_ledger: current_ledger + interval,
            created_at: current_ledger,
            last_executed_at: 0,
        };

        env.storage().persistent().set(&plan_id, &plan);

        let mut user_plans = Self::get_user_plans_internal(&env, &owner);
        user_plans.push_back(plan_id);
        Self::set_user_plans(&env, &owner, &user_plans);

        env.events().publish(
            (symbol_short!("dca"), symbol_short!("create")),
            (plan_id, owner),
        );

        Ok(plan_id)
    }

    pub fn execute(env: Env, keeper: Address, plan_id: u64) -> Result<DcaExecution, DcaError> {
        keeper.require_auth();

        let mut plan: DcaPlan = env
            .storage()
            .persistent()
            .get(&plan_id)
            .ok_or(DcaError::PlanNotFound)?;

        if plan.status != DcaPlanStatus::Active {
            return Err(DcaError::PlanNotActive);
        }

        let current_ledger = env.ledger().sequence() as u64;
        if current_ledger < plan.next_execution_ledger {
            return Err(DcaError::ExecutionNotDue);
        }

        if plan.total_executions >= plan.max_executions {
            plan.status = DcaPlanStatus::Completed;
            env.storage().persistent().set(&plan_id, &plan);
            return Err(DcaError::PlanCompleted);
        }

        let remaining = plan.funded_amount - plan.spent_amount;
        if remaining < plan.amount_per_execution {
            return Err(DcaError::InsufficientFunds);
        }

        plan.total_executions += 1;
        plan.spent_amount += plan.amount_per_execution;
        plan.last_executed_at = current_ledger;
        plan.next_execution_ledger = current_ledger + Self::frequency_to_ledgers(&plan.frequency);

        if plan.total_executions >= plan.max_executions {
            plan.status = DcaPlanStatus::Completed;
        }

        let execution = DcaExecution {
            plan_id,
            execution_number: plan.total_executions,
            amount: plan.amount_per_execution,
            executed_at: current_ledger,
        };

        env.storage().persistent().set(&plan_id, &plan);

        env.events().publish(
            (symbol_short!("dca"), symbol_short!("execute")),
            (plan_id, plan.total_executions, plan.amount_per_execution),
        );

        Ok(execution)
    }

    pub fn pause(env: Env, owner: Address, plan_id: u64) -> Result<(), DcaError> {
        owner.require_auth();

        let mut plan: DcaPlan = env
            .storage()
            .persistent()
            .get(&plan_id)
            .ok_or(DcaError::PlanNotFound)?;

        if plan.owner != owner {
            return Err(DcaError::Unauthorized);
        }
        if plan.status == DcaPlanStatus::Paused {
            return Err(DcaError::PlanAlreadyPaused);
        }
        if plan.status != DcaPlanStatus::Active {
            return Err(DcaError::PlanNotActive);
        }

        plan.status = DcaPlanStatus::Paused;
        env.storage().persistent().set(&plan_id, &plan);

        Ok(())
    }

    pub fn resume(env: Env, owner: Address, plan_id: u64) -> Result<(), DcaError> {
        owner.require_auth();

        let mut plan: DcaPlan = env
            .storage()
            .persistent()
            .get(&plan_id)
            .ok_or(DcaError::PlanNotFound)?;

        if plan.owner != owner {
            return Err(DcaError::Unauthorized);
        }
        if plan.status != DcaPlanStatus::Paused {
            return Err(DcaError::PlanNotPaused);
        }

        let current_ledger = env.ledger().sequence() as u64;
        plan.status = DcaPlanStatus::Active;
        plan.next_execution_ledger = current_ledger + Self::frequency_to_ledgers(&plan.frequency);
        env.storage().persistent().set(&plan_id, &plan);

        Ok(())
    }

    pub fn cancel(env: Env, owner: Address, plan_id: u64) -> Result<i128, DcaError> {
        owner.require_auth();

        let mut plan: DcaPlan = env
            .storage()
            .persistent()
            .get(&plan_id)
            .ok_or(DcaError::PlanNotFound)?;

        if plan.owner != owner {
            return Err(DcaError::Unauthorized);
        }
        if plan.status == DcaPlanStatus::Completed || plan.status == DcaPlanStatus::Cancelled {
            return Err(DcaError::PlanNotActive);
        }

        let refund = plan.funded_amount - plan.spent_amount;
        plan.status = DcaPlanStatus::Cancelled;
        env.storage().persistent().set(&plan_id, &plan);

        env.events().publish(
            (symbol_short!("dca"), symbol_short!("cancel")),
            (plan_id, refund),
        );

        Ok(refund)
    }

    pub fn get_plan(env: Env, plan_id: u64) -> Option<DcaPlan> {
        env.storage().persistent().get(&plan_id)
    }

    pub fn get_user_plans(env: Env, owner: Address) -> Vec<u64> {
        Self::get_user_plans_internal(&env, &owner)
    }

    fn require_initialized(env: &Env) -> Result<(), DcaError> {
        if !env.storage().instance().has(&symbol_short!("admin")) {
            return Err(DcaError::NotInitialized);
        }
        Ok(())
    }

    fn next_id(env: &Env) -> u64 {
        let id: u64 = env.storage().instance().get(&symbol_short!("next_id")).unwrap_or(1);
        env.storage().instance().set(&symbol_short!("next_id"), &(id + 1));
        id
    }

    fn frequency_to_ledgers(freq: &DcaFrequency) -> u64 {
        match freq {
            DcaFrequency::Daily => DAY_LEDGERS,
            DcaFrequency::Weekly => WEEK_LEDGERS,
            DcaFrequency::Monthly => MONTH_LEDGERS,
        }
    }

    fn get_user_plans_internal(env: &Env, owner: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&(symbol_short!("plans"), owner.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn set_user_plans(env: &Env, owner: &Address, plans: &Vec<u64>) {
        env.storage()
            .persistent()
            .set(&(symbol_short!("plans"), owner.clone()), plans);
    }
}
