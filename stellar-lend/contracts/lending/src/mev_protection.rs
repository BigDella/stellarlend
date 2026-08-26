//! Lightweight MEV guard primitives for the lending contract.
//!
//! The main liquidation batch auction lives in the core `hello-world` contract,
//! while this module gives the lending contract a compact commit/reveal and
//! gas-bid analysis surface for large transactions.

use soroban_sdk::{contracterror, contracttype, Address, Env, Symbol};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MevGuardError {
    BadConfig = 1,
    NotFound = 2,
    Unauthorized = 3,
    NotReady = 4,
    Expired = 5,
    SlippageExceeded = 6,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LendingMevCommitment {
    pub owner: Address,
    pub operation: Symbol,
    pub amount: i128,
    pub quoted_output: i128,
    pub min_output: i128,
    pub max_slippage_bps: i128,
    pub reveal_after: u64,
    pub deadline: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LendingGasBidStats {
    pub samples: u64,
    pub avg_bid_microlumens: i128,
    pub max_bid_microlumens: i128,
    pub avg_inclusion_delay_ledgers: u64,
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone)]
enum MevGuardKey {
    NextCommitId,
    Commit(u64),
    Gas(Symbol),
}

const MAX_BPS: i128 = 10_000;
const DEFAULT_REVEAL_DELAY_SECS: u64 = 30;

pub fn commit_large_tx(
    env: &Env,
    owner: Address,
    operation: Symbol,
    amount: i128,
    quoted_output: i128,
    min_output: i128,
    max_slippage_bps: i128,
    deadline: u64,
) -> Result<u64, MevGuardError> {
    owner.require_auth();
    if amount <= 0
        || quoted_output <= 0
        || min_output < 0
        || min_output > quoted_output
        || !(0..=MAX_BPS).contains(&max_slippage_bps)
        || deadline <= env.ledger().timestamp()
    {
        return Err(MevGuardError::BadConfig);
    }

    let id = next_commit_id(env);
    let commitment = LendingMevCommitment {
        owner,
        operation,
        amount,
        quoted_output,
        min_output,
        max_slippage_bps,
        reveal_after: env
            .ledger()
            .timestamp()
            .saturating_add(DEFAULT_REVEAL_DELAY_SECS),
        deadline,
    };
    env.storage()
        .persistent()
        .set(&MevGuardKey::Commit(id), &commitment);
    Ok(id)
}

pub fn reveal_large_tx(
    env: &Env,
    owner: Address,
    commit_id: u64,
    actual_output: i128,
) -> Result<LendingMevCommitment, MevGuardError> {
    owner.require_auth();
    let commitment = get_commit(env, commit_id).ok_or(MevGuardError::NotFound)?;
    if commitment.owner != owner {
        return Err(MevGuardError::Unauthorized);
    }

    let now = env.ledger().timestamp();
    if now < commitment.reveal_after {
        return Err(MevGuardError::NotReady);
    }
    if now > commitment.deadline {
        return Err(MevGuardError::Expired);
    }

    let slippage_floor = commitment
        .quoted_output
        .saturating_mul(MAX_BPS.saturating_sub(commitment.max_slippage_bps))
        .saturating_div(MAX_BPS);
    if actual_output < commitment.min_output.max(slippage_floor) {
        return Err(MevGuardError::SlippageExceeded);
    }

    env.storage()
        .persistent()
        .remove(&MevGuardKey::Commit(commit_id));
    Ok(commitment)
}

pub fn get_commit(env: &Env, commit_id: u64) -> Option<LendingMevCommitment> {
    env.storage()
        .persistent()
        .get(&MevGuardKey::Commit(commit_id))
}

pub fn record_gas_bid(
    env: &Env,
    reporter: Address,
    operation: Symbol,
    bid_microlumens: i128,
    inclusion_delay_ledgers: u64,
) -> Result<LendingGasBidStats, MevGuardError> {
    reporter.require_auth();
    if bid_microlumens <= 0 {
        return Err(MevGuardError::BadConfig);
    }

    let key = MevGuardKey::Gas(operation);
    let mut stats = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(LendingGasBidStats {
            samples: 0,
            avg_bid_microlumens: 0,
            max_bid_microlumens: 0,
            avg_inclusion_delay_ledgers: 0,
            last_updated: 0,
        });
    let next_samples = stats.samples.saturating_add(1);
    stats.avg_bid_microlumens = stats
        .avg_bid_microlumens
        .saturating_mul(i128::from(stats.samples))
        .saturating_add(bid_microlumens)
        .saturating_div(i128::from(next_samples));
    stats.max_bid_microlumens = stats.max_bid_microlumens.max(bid_microlumens);
    stats.avg_inclusion_delay_ledgers = stats
        .avg_inclusion_delay_ledgers
        .saturating_mul(stats.samples)
        .saturating_add(inclusion_delay_ledgers)
        .saturating_div(next_samples);
    stats.samples = next_samples;
    stats.last_updated = env.ledger().timestamp();
    env.storage().persistent().set(&key, &stats);
    Ok(stats)
}

pub fn get_gas_bid_stats(env: &Env, operation: Symbol) -> LendingGasBidStats {
    env.storage()
        .persistent()
        .get(&MevGuardKey::Gas(operation))
        .unwrap_or(LendingGasBidStats {
            samples: 0,
            avg_bid_microlumens: 0,
            max_bid_microlumens: 0,
            avg_inclusion_delay_ledgers: 0,
            last_updated: 0,
        })
}

fn next_commit_id(env: &Env) -> u64 {
    let id = env
        .storage()
        .persistent()
        .get::<MevGuardKey, u64>(&MevGuardKey::NextCommitId)
        .unwrap_or(1);
    env.storage()
        .persistent()
        .set(&MevGuardKey::NextCommitId, &id.saturating_add(1));
    id
}
