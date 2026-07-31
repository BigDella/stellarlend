//! Conditional borrow commitments: scheduled borrows that anyone may execute once oracle
//! triggers and expiry have been satisfied. Intended for keepers / automation.

use soroban_sdk::{contracterror, contracttype, Address, Env, IntoVal, Symbol, Vec};

use crate::borrow::{
    borrow_from_commitment, get_debt_ceiling, get_min_borrow_amount, get_oracle, get_total_debt,
    validate_collateral_ratio,
};
use crate::events::{
    BorrowCommitmentCancelledEvent, BorrowCommitmentCreatedEvent, BorrowCommitmentExecutedEvent,
};

pub const MAX_TRIGGERS: u32 = 4;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum CommitmentError {
    NotFound = 1,
    Unauthorized = 2,
    BadConfig = 3,
    Expired = 4,
    NotExecutable = 5,
    OracleNotConfigured = 6,
    TriggerNotMet = 7,
    BelowMinPartialFill = 8,
    BorrowFailed = 9,
    TooManyTriggers = 10,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TriggerCombiner {
    /// Any single trigger fires execution (OR).
    Any,
    /// All triggers must be satisfied (AND).
    All,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceTrigger {
    pub asset: Address,
    pub trigger_price: i128,
    /// When true: execute when oracle price >= trigger_price; when false: when price <= trigger_price.
    pub fire_when_at_or_above: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommitmentStatus {
    Pending,
    Executed,
    Cancelled,
    Expired,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BorrowCommitment {
    pub owner: Address,
    pub triggers: Vec<PriceTrigger>,
    pub combiner: TriggerCombiner,
    pub borrow_asset: Address,
    pub collateral_asset: Address,
    /// Requested borrow amount (may be reduced for partial execution against debt ceiling).
    pub borrow_amount: i128,
    pub collateral_amount: i128,
    /// Minimum filled borrow as ratio of requested amount (basis points, 10000 = full size only).
    pub min_fill_bps: u32,
    pub expiry_timestamp: u64,
    pub status: CommitmentStatus,
}

#[contracttype]
#[derive(Clone)]
pub(crate) enum CommitmentStorageKey {
    NextId,
    Commitment(u64),
}

fn read_price(env: &Env, asset: &Address) -> Result<i128, CommitmentError> {
    let oracle = get_oracle(env).ok_or(CommitmentError::OracleNotConfigured)?;
    let price: i128 = env.invoke_contract(
        &oracle,
        &Symbol::new(env, "price"),
        (asset.clone(),).into_val(env),
    );
    Ok(price)
}

fn trigger_met(env: &Env, t: &PriceTrigger) -> Result<bool, CommitmentError> {
    let price = read_price(env, &t.asset)?;
    Ok(if t.fire_when_at_or_above {
        price >= t.trigger_price
    } else {
        price <= t.trigger_price
    })
}

fn triggers_satisfied(
    env: &Env,
    triggers: &Vec<PriceTrigger>,
    combiner: &TriggerCombiner,
) -> Result<bool, CommitmentError> {
    if triggers.is_empty() {
        return Err(CommitmentError::BadConfig);
    }

    let len = triggers.len();
    match combiner {
        TriggerCombiner::Any => {
            let mut i = 0u32;
            while i < len {
                let t = triggers.get(i).ok_or(CommitmentError::BadConfig)?;
                if trigger_met(env, &t)? {
                    return Ok(true);
                }
                i += 1;
            }
            Ok(false)
        }
        TriggerCombiner::All => {
            let mut i = 0u32;
            while i < len {
                let t = triggers.get(i).ok_or(CommitmentError::BadConfig)?;
                if !trigger_met(env, &t)? {
                    return Ok(false);
                }
                i += 1;
            }
            Ok(true)
        }
    }
}

pub fn create_borrow_commitment(
    env: &Env,
    owner: Address,
    triggers: Vec<PriceTrigger>,
    combiner: TriggerCombiner,
    borrow_asset: Address,
    collateral_asset: Address,
    borrow_amount: i128,
    collateral_amount: i128,
    min_fill_bps: u32,
    expiry_timestamp: u64,
) -> Result<u64, CommitmentError> {
    owner.require_auth();

    if triggers.is_empty() {
        return Err(CommitmentError::BadConfig);
    }
    if triggers.len() > MAX_TRIGGERS {
        return Err(CommitmentError::TooManyTriggers);
    }
    if borrow_amount <= 0 || collateral_amount <= 0 || min_fill_bps == 0 || min_fill_bps > 10000 {
        return Err(CommitmentError::BadConfig);
    }

    validate_collateral_ratio(collateral_amount, borrow_amount)
        .map_err(|_| CommitmentError::BadConfig)?;

    let now = env.ledger().timestamp();
    if expiry_timestamp <= now {
        return Err(CommitmentError::BadConfig);
    }

    let id: u64 = env
        .storage()
        .persistent()
        .get(&CommitmentStorageKey::NextId)
        .unwrap_or(0);

    let c = BorrowCommitment {
        owner,
        triggers,
        combiner,
        borrow_asset,
        collateral_asset,
        borrow_amount,
        collateral_amount,
        min_fill_bps,
        expiry_timestamp,
        status: CommitmentStatus::Pending,
    };

    env.storage()
        .persistent()
        .set(&CommitmentStorageKey::Commitment(id), &c);
    env.storage()
        .persistent()
        .set(&CommitmentStorageKey::NextId, &(id + 1));

    BorrowCommitmentCreatedEvent {
        commitment_id: id,
        owner: c.owner.clone(),
        borrow_asset: c.borrow_asset.clone(),
        borrow_amount: c.borrow_amount,
        expiry: c.expiry_timestamp,
    }
    .publish(env);

    Ok(id)
}

pub fn cancel_borrow_commitment(
    env: &Env,
    owner: Address,
    commitment_id: u64,
) -> Result<(), CommitmentError> {
    owner.require_auth();

    let mut c: BorrowCommitment = env
        .storage()
        .persistent()
        .get(&CommitmentStorageKey::Commitment(commitment_id))
        .ok_or(CommitmentError::NotFound)?;

    if c.owner != owner {
        return Err(CommitmentError::Unauthorized);
    }

    if c.status != CommitmentStatus::Pending {
        return Err(CommitmentError::NotExecutable);
    }

    c.status = CommitmentStatus::Cancelled;
    env.storage()
        .persistent()
        .set(&CommitmentStorageKey::Commitment(commitment_id), &c);

    BorrowCommitmentCancelledEvent {
        commitment_id,
        owner,
    }
    .publish(env);

    Ok(())
}

pub fn execute_borrow_commitment(env: &Env, commitment_id: u64) -> Result<(), CommitmentError> {
    let mut c: BorrowCommitment = env
        .storage()
        .persistent()
        .get(&CommitmentStorageKey::Commitment(commitment_id))
        .ok_or(CommitmentError::NotFound)?;

    if c.status != CommitmentStatus::Pending {
        return Err(CommitmentError::NotExecutable);
    }

    let now = env.ledger().timestamp();
    if now > c.expiry_timestamp {
        c.status = CommitmentStatus::Expired;
        env.storage()
            .persistent()
            .set(&CommitmentStorageKey::Commitment(commitment_id), &c);
        return Err(CommitmentError::Expired);
    }

    if !triggers_satisfied(env, &c.triggers, &c.combiner)? {
        return Err(CommitmentError::TriggerNotMet);
    }

    let total_debt = get_total_debt(env);
    let debt_ceiling = get_debt_ceiling(env);
    let room = debt_ceiling.saturating_sub(total_debt);

    let fill_amount = c.borrow_amount.min(room);

    let min_allowed = c
        .borrow_amount
        .saturating_mul(i128::from(c.min_fill_bps))
        .saturating_div(10000);

    if fill_amount < min_allowed {
        return Err(CommitmentError::BelowMinPartialFill);
    }

    let min_borrow = get_min_borrow_amount(env);
    if fill_amount < min_borrow {
        return Err(CommitmentError::BorrowFailed);
    }

    let collateral_fill = c
        .collateral_amount
        .saturating_mul(fill_amount)
        .saturating_div(c.borrow_amount);

    if collateral_fill <= 0 {
        return Err(CommitmentError::BadConfig);
    }

    validate_collateral_ratio(collateral_fill, fill_amount)
        .map_err(|_| CommitmentError::BorrowFailed)?;

    borrow_from_commitment(
        env,
        c.owner.clone(),
        c.borrow_asset.clone(),
        fill_amount,
        c.collateral_asset.clone(),
        collateral_fill,
    )
    .map_err(|_| CommitmentError::BorrowFailed)?;

    c.status = CommitmentStatus::Executed;
    env.storage()
        .persistent()
        .set(&CommitmentStorageKey::Commitment(commitment_id), &c);

    BorrowCommitmentExecutedEvent {
        commitment_id,
        owner: c.owner.clone(),
        borrowed_amount: fill_amount,
        collateral_amount: collateral_fill,
    }
    .publish(env);

    Ok(())
}

pub fn get_borrow_commitment(env: &Env, commitment_id: u64) -> Option<BorrowCommitment> {
    env.storage()
        .persistent()
        .get(&CommitmentStorageKey::Commitment(commitment_id))
}
