#![allow(unused)]
use soroban_sdk::{Address, Env, Vec};

use crate::governance::{
    emit_guardian_added_event, emit_guardian_removed_event, emit_recovery_approved_event,
    emit_recovery_cancelled_event, emit_recovery_executed_event, emit_recovery_started_event,
    GovernanceDataKey, GovernanceError, RecoveryRequest,
};
use crate::types::DEFAULT_RECOVERY_DELAY;

const DEFAULT_RECOVERY_PERIOD: u64 = 3 * 24 * 60 * 60;

fn require_multisig_admin(env: &Env, caller: &Address) -> Result<(), GovernanceError> {
    let admins: Vec<Address> = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::MultisigAdmins)
        .ok_or(GovernanceError::Unauthorized)?;
    if !admins.contains(caller.clone()) {
        return Err(GovernanceError::Unauthorized);
    }
    Ok(())
}

pub fn set_guardians(
    env: &Env,
    caller: Address,
    guardians: Vec<Address>,
    threshold: u32,
) -> Result<(), GovernanceError> {
    require_multisig_admin(env, &caller)?;

    if guardians.is_empty() {
        return Err(GovernanceError::InvalidGuardianConfig);
    }
    if threshold == 0 || threshold > guardians.len() {
        return Err(GovernanceError::InvalidGuardianConfig);
    }

    for i in 0..guardians.len() {
        for j in (i + 1)..guardians.len() {
            if guardians.get(i).unwrap() == guardians.get(j).unwrap() {
                return Err(GovernanceError::InvalidGuardianConfig);
            }
        }
    }

    env.storage()
        .persistent()
        .set(&GovernanceDataKey::Guardians, &guardians);
    env.storage()
        .persistent()
        .set(&GovernanceDataKey::GuardianThreshold, &threshold);

    for g in guardians.iter() {
        emit_guardian_added_event(env, &g);
    }

    Ok(())
}

pub fn add_guardian(env: &Env, caller: Address, guardian: Address) -> Result<(), GovernanceError> {
    require_multisig_admin(env, &caller)?;

    let mut guardians: Vec<Address> = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Guardians)
        .unwrap_or_else(|| Vec::new(env));

    if guardians.contains(guardian.clone()) {
        return Err(GovernanceError::GuardianAlreadyExists);
    }

    guardians.push_back(guardian.clone());
    env.storage()
        .persistent()
        .set(&GovernanceDataKey::Guardians, &guardians);

    emit_guardian_added_event(env, &guardian);
    Ok(())
}

pub fn remove_guardian(
    env: &Env,
    caller: Address,
    guardian: Address,
) -> Result<(), GovernanceError> {
    require_multisig_admin(env, &caller)?;

    let guardians: Vec<Address> = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Guardians)
        .ok_or(GovernanceError::GuardianNotFound)?;

    let mut new_guardians = Vec::new(env);
    let mut found = false;
    for g in guardians.iter() {
        if g == guardian {
            found = true;
        } else {
            new_guardians.push_back(g);
        }
    }

    if !found {
        return Err(GovernanceError::GuardianNotFound);
    }

    env.storage()
        .persistent()
        .set(&GovernanceDataKey::Guardians, &new_guardians);
    emit_guardian_removed_event(env, &guardian);
    Ok(())
}

pub fn set_guardian_threshold(
    env: &Env,
    caller: Address,
    threshold: u32,
) -> Result<(), GovernanceError> {
    require_multisig_admin(env, &caller)?;

    let guardians: Vec<Address> = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Guardians)
        .unwrap_or_else(|| Vec::new(env));

    if threshold == 0 || threshold > guardians.len() {
        return Err(GovernanceError::InvalidGuardianConfig);
    }

    env.storage()
        .persistent()
        .set(&GovernanceDataKey::GuardianThreshold, &threshold);
    Ok(())
}

pub fn start_recovery(
    env: &Env,
    initiator: Address,
    old_admin: Address,
    new_admin: Address,
) -> Result<(), GovernanceError> {
    let guardians: Vec<Address> = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Guardians)
        .ok_or(GovernanceError::Unauthorized)?;

    if !guardians.contains(initiator.clone()) {
        return Err(GovernanceError::Unauthorized);
    }

    if env
        .storage()
        .persistent()
        .has(&GovernanceDataKey::RecoveryRequest)
    {
        return Err(GovernanceError::RecoveryInProgress);
    }

    let now = env.ledger().timestamp();
    let recovery = RecoveryRequest {
        old_admin: old_admin.clone(),
        new_admin: new_admin.clone(),
        initiator: initiator.clone(),
        initiated_at: now,
        expires_at: now + DEFAULT_RECOVERY_PERIOD,
        // #675 — fixed delay from initiation, independent of when guardian
        // approvals come in. Gives the account owner (or any guardian) a
        // guaranteed minimum window to notice and cancel a fraudulent or
        // mistaken recovery attempt before it can execute, regardless of
        // how quickly the approval threshold is reached.
        ready_at: now + DEFAULT_RECOVERY_DELAY,
    };

    env.storage()
        .persistent()
        .set(&GovernanceDataKey::RecoveryRequest, &recovery);

    let mut approvals = Vec::new(env);
    approvals.push_back(initiator.clone());
    env.storage()
        .persistent()
        .set(&GovernanceDataKey::RecoveryApprovals, &approvals);

    emit_recovery_started_event(env, &old_admin, &new_admin, &initiator);
    Ok(())
}

pub fn approve_recovery(env: &Env, approver: Address) -> Result<(), GovernanceError> {
    let guardians: Vec<Address> = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Guardians)
        .ok_or(GovernanceError::Unauthorized)?;

    if !guardians.contains(approver.clone()) {
        return Err(GovernanceError::Unauthorized);
    }

    let recovery: RecoveryRequest = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::RecoveryRequest)
        .ok_or(GovernanceError::NoRecoveryInProgress)?;

    let now = env.ledger().timestamp();
    if now > recovery.expires_at {
        env.storage()
            .persistent()
            .remove(&GovernanceDataKey::RecoveryRequest);
        return Err(GovernanceError::ProposalExpired);
    }

    let mut approvals: Vec<Address> = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::RecoveryApprovals)
        .unwrap_or_else(|| Vec::new(env));

    if approvals.contains(approver.clone()) {
        return Err(GovernanceError::AlreadyVoted);
    }

    approvals.push_back(approver.clone());
    env.storage()
        .persistent()
        .set(&GovernanceDataKey::RecoveryApprovals, &approvals);

    emit_recovery_approved_event(env, &approver);
    Ok(())
}

pub fn execute_recovery(env: &Env, executor: Address) -> Result<(), GovernanceError> {
    let recovery: RecoveryRequest = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::RecoveryRequest)
        .ok_or(GovernanceError::NoRecoveryInProgress)?;

    let now = env.ledger().timestamp();
    if now > recovery.expires_at {
        env.storage()
            .persistent()
            .remove(&GovernanceDataKey::RecoveryRequest);
        return Err(GovernanceError::ProposalExpired);
    }

    // #675 — recovery delay: even with enough approvals, execution cannot
    // happen before `ready_at`. This is the actual cancellation window.
    if now < recovery.ready_at {
        return Err(GovernanceError::RecoveryNotReady);
    }

    let threshold: u32 = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::GuardianThreshold)
        .unwrap_or(1u32);

    let approvals: Vec<Address> = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::RecoveryApprovals)
        .unwrap_or_else(|| Vec::new(env));

    if approvals.len() < threshold {
        return Err(GovernanceError::InsufficientApprovals);
    }

    let admins: Vec<Address> = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::MultisigAdmins)
        .unwrap_or_else(|| Vec::new(env));

    let mut new_admins = Vec::new(env);
    for a in admins.iter() {
        if a != recovery.old_admin {
            new_admins.push_back(a);
        }
    }
    new_admins.push_back(recovery.new_admin.clone());
    env.storage()
        .persistent()
        .set(&GovernanceDataKey::MultisigAdmins, &new_admins);

    env.storage()
        .persistent()
        .remove(&GovernanceDataKey::RecoveryRequest);
    env.storage()
        .persistent()
        .remove(&GovernanceDataKey::RecoveryApprovals);

    emit_recovery_executed_event(env, &recovery.old_admin, &recovery.new_admin, &executor);
    Ok(())
}

/// #675 — cancel a pending recovery request during the delay/pending window.
/// Callable by the account being protected (`recovery.old_admin`) or by any
/// guardian, so a fraudulent or mistaken recovery attempt can be stopped by
/// either the legitimate owner noticing it, or a guardian who didn't approve
/// it flagging it. Once cancelled, the request and its approvals are cleared
/// entirely — a new recovery must be started fresh.
pub fn cancel_recovery(env: &Env, caller: Address) -> Result<(), GovernanceError> {
    caller.require_auth();

    let recovery: RecoveryRequest = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::RecoveryRequest)
        .ok_or(GovernanceError::NoRecoveryInProgress)?;

    let is_protected_account = caller == recovery.old_admin;
    let is_guardian = env
        .storage()
        .persistent()
        .get::<_, Vec<Address>>(&GovernanceDataKey::Guardians)
        .map(|guardians| guardians.contains(caller.clone()))
        .unwrap_or(false);

    if !is_protected_account && !is_guardian {
        return Err(GovernanceError::Unauthorized);
    }

    env.storage()
        .persistent()
        .remove(&GovernanceDataKey::RecoveryRequest);
    env.storage()
        .persistent()
        .remove(&GovernanceDataKey::RecoveryApprovals);

    emit_recovery_cancelled_event(env, &recovery.old_admin, &recovery.new_admin, &caller);
    Ok(())
}

pub fn get_guardians(env: &Env) -> Option<Vec<Address>> {
    env.storage()
        .persistent()
        .get(&GovernanceDataKey::Guardians)
}

pub fn get_guardian_threshold(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&GovernanceDataKey::GuardianThreshold)
        .unwrap_or(1u32)
}

pub fn get_recovery_request(env: &Env) -> Option<RecoveryRequest> {
    env.storage()
        .persistent()
        .get(&GovernanceDataKey::RecoveryRequest)
}

pub fn get_recovery_approvals(env: &Env) -> Option<Vec<Address>> {
    env.storage()
        .persistent()
        .get(&GovernanceDataKey::RecoveryApprovals)
}
