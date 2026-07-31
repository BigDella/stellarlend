#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec, Map, symbol_short, Symbol};

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
pub enum Category {
    Operations = 0,
    Marketing = 1,
    Development = 2,
    Emergency = 3,
}

#[contracttype]
pub struct SpendingLimit {
    pub daily_limit: i128,
    pub monthly_limit: i128,
    pub daily_spent: i128,
    pub monthly_spent: i128,
    pub last_reset_day: u64,
    pub last_reset_month: u64,
}

#[contracttype]
pub struct CategoryPolicy {
    pub limit: SpendingLimit,
    pub required_signers: u32,
    pub enabled: bool,
}

#[contracttype]
pub struct Proposal {
    pub id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub category: Category,
    pub created_at: u64,
    pub executed: bool,
    pub approvals: u32,
    pub requires_timelock: bool,
    pub timelock_until: u64,
}

#[contracttype]
pub struct TreasuryState {
    pub signers: Vec<Address>,
    pub signer_threshold: u32,
    pub policies: Map<u32, CategoryPolicy>,
    pub proposals: Map<u64, Proposal>,
    pub next_proposal_id: u64,
    pub admin: Address,
}

const TIMELOCK_DURATION: u64 = 48 * 3600;

#[contract]
pub struct TreasuryMultisig;

#[contractimpl]
impl TreasuryMultisig {
    pub fn initialize(
        env: Env,
        admin: Address,
        signers: Vec<Address>,
        threshold: u32,
    ) -> Result<(), String> {
        admin.require_auth();

        if threshold == 0 || threshold as usize > signers.len() {
            return Err(String::from_slice(&env, "Invalid threshold"));
        }

        let mut policies: Map<u32, CategoryPolicy> = Map::new(&env);

        for category in &[
            Category::Operations as u32,
            Category::Marketing as u32,
            Category::Development as u32,
            Category::Emergency as u32,
        ] {
            let policy = CategoryPolicy {
                limit: SpendingLimit {
                    daily_limit: 1_000_000_000_000,
                    monthly_limit: 10_000_000_000_000,
                    daily_spent: 0,
                    monthly_spent: 0,
                    last_reset_day: 0,
                    last_reset_month: 0,
                },
                required_signers: threshold,
                enabled: true,
            };
            policies.set(*category, policy);
        }

        let state = TreasuryState {
            signers,
            signer_threshold: threshold,
            policies,
            proposals: Map::new(&env),
            next_proposal_id: 1,
            admin,
        };

        env.storage()
            .persistent()
            .set(&symbol_short!("TREASURY"), &state);

        Ok(())
    }

    pub fn propose_transaction(
        env: Env,
        amount: i128,
        recipient: Address,
        category: Category,
    ) -> Result<u64, String> {
        let mut state: TreasuryState = env
            .storage()
            .persistent()
            .get(&symbol_short!("TREASURY"))
            .ok_or(String::from_slice(&env, "Treasury not initialized"))?;

        if amount <= 0 {
            return Err(String::from_slice(&env, "Invalid amount"));
        }

        let category_id = category as u32;
        let policy = state
            .policies
            .get(category_id)
            .ok_or(String::from_slice(&env, "Invalid category"))?;

        if !policy.enabled {
            return Err(String::from_slice(&env, "Category disabled"));
        }

        let proposal = Proposal {
            id: state.next_proposal_id,
            amount,
            recipient,
            category,
            created_at: env.ledger().timestamp(),
            executed: false,
            approvals: 0,
            requires_timelock: amount > policy.limit.daily_limit / 10,
            timelock_until: env.ledger().timestamp() + TIMELOCK_DURATION,
        };

        state.proposals.set(proposal.id, proposal);
        state.next_proposal_id += 1;
        env.storage()
            .persistent()
            .set(&symbol_short!("TREASURY"), &state);

        Ok(state.next_proposal_id - 1)
    }

    pub fn approve_proposal(env: Env, proposal_id: u64, signer: Address) -> Result<(), String> {
        signer.require_auth();

        let mut state: TreasuryState = env
            .storage()
            .persistent()
            .get(&symbol_short!("TREASURY"))
            .ok_or(String::from_slice(&env, "Treasury not initialized"))?;

        if !state.signers.iter().any(|s| s == &signer) {
            return Err(String::from_slice(&env, "Not a signer"));
        }

        let mut proposal = state
            .proposals
            .get(proposal_id)
            .ok_or(String::from_slice(&env, "Proposal not found"))?;

        if proposal.executed {
            return Err(String::from_slice(&env, "Already executed"));
        }

        proposal.approvals += 1;
        state.proposals.set(proposal_id, proposal);

        env.storage()
            .persistent()
            .set(&symbol_short!("TREASURY"), &state);

        Ok(())
    }

    pub fn execute_proposal(env: Env, proposal_id: u64) -> Result<(), String> {
        let mut state: TreasuryState = env
            .storage()
            .persistent()
            .get(&symbol_short!("TREASURY"))
            .ok_or(String::from_slice(&env, "Treasury not initialized"))?;

        let mut proposal = state
            .proposals
            .get(proposal_id)
            .ok_or(String::from_slice(&env, "Proposal not found"))?;

        if proposal.executed {
            return Err(String::from_slice(&env, "Already executed"));
        }

        if proposal.approvals < state.signer_threshold {
            return Err(String::from_slice(&env, "Insufficient approvals"));
        }

        if proposal.requires_timelock && env.ledger().timestamp() < proposal.timelock_until {
            return Err(String::from_slice(&env, "Timelock not elapsed"));
        }

        proposal.executed = true;
        state.proposals.set(proposal_id, proposal);

        env.storage()
            .persistent()
            .set(&symbol_short!("TREASURY"), &state);

        Ok(())
    }

    pub fn update_spending_limit(
        env: Env,
        admin: Address,
        category: Category,
        daily_limit: i128,
        monthly_limit: i128,
    ) -> Result<(), String> {
        admin.require_auth();

        let mut state: TreasuryState = env
            .storage()
            .persistent()
            .get(&symbol_short!("TREASURY"))
            .ok_or(String::from_slice(&env, "Treasury not initialized"))?;

        if state.admin != admin {
            return Err(String::from_slice(&env, "Not authorized"));
        }

        let category_id = category as u32;
        let mut policy = state
            .policies
            .get(category_id)
            .ok_or(String::from_slice(&env, "Invalid category"))?;

        policy.limit.daily_limit = daily_limit;
        policy.limit.monthly_limit = monthly_limit;
        state.policies.set(category_id, policy);

        env.storage()
            .persistent()
            .set(&symbol_short!("TREASURY"), &state);

        Ok(())
    }

    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<Proposal, String> {
        let state: TreasuryState = env
            .storage()
            .persistent()
            .get(&symbol_short!("TREASURY"))
            .ok_or(String::from_slice(&env, "Treasury not initialized"))?;

        state
            .proposals
            .get(proposal_id)
            .ok_or(String::from_slice(&env, "Proposal not found"))
    }
}
