use crate::types::{AuditEntry, DataKey, MultisigConfig, Proposal, WalletError};
use soroban_sdk::{Address, Env, Vec};

pub fn get_config(env: &Env) -> Result<MultisigConfig, WalletError> {
    env.storage()
        .instance()
        .get(&DataKey::Config)
        .ok_or(WalletError::NotInitialized)
}

pub fn set_config(env: &Env, config: &MultisigConfig) {
    env.storage().instance().set(&DataKey::Config, config);
}

pub fn get_admins(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Admins)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_admins(env: &Env, admins: &Vec<Address>) {
    env.storage().instance().set(&DataKey::Admins, admins);
}

pub fn get_next_proposal_id(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::NextProposalId)
        .unwrap_or(0)
}

pub fn increment_proposal_id(env: &Env) -> u64 {
    let id = get_next_proposal_id(env);
    env.storage()
        .instance()
        .set(&DataKey::NextProposalId, &(id + 1));
    id
}

pub fn get_proposal(env: &Env, id: u64) -> Option<Proposal> {
    env.storage().persistent().get(&DataKey::Proposal(id))
}

pub fn set_proposal(env: &Env, id: u64, proposal: &Proposal) {
    env.storage()
        .persistent()
        .set(&DataKey::Proposal(id), proposal);
}

pub fn get_approvals(env: &Env, id: u64) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::Approvals(id))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_approvals(env: &Env, id: u64, approvals: &Vec<Address>) {
    env.storage()
        .persistent()
        .set(&DataKey::Approvals(id), approvals);
}

pub fn add_audit_entry(env: &Env, id: u64, entry: AuditEntry) {
    let mut trail: Vec<AuditEntry> = env
        .storage()
        .persistent()
        .get(&DataKey::AuditTrail(id))
        .unwrap_or_else(|| Vec::new(env));
    trail.push_back(entry);
    env.storage()
        .persistent()
        .set(&DataKey::AuditTrail(id), &trail);
}

pub fn get_audit_trail(env: &Env, id: u64) -> Vec<AuditEntry> {
    env.storage()
        .persistent()
        .get(&DataKey::AuditTrail(id))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn get_guardians(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Guardians)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_guardians(env: &Env, guardians: &Vec<Address>) {
    env.storage().instance().set(&DataKey::Guardians, guardians);
}

pub fn get_guardian_threshold(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::GuardianThreshold)
        .unwrap_or(0)
}

pub fn set_guardian_threshold(env: &Env, threshold: u32) {
    env.storage()
        .instance()
        .set(&DataKey::GuardianThreshold, &threshold);
}

pub fn get_recovery_request(env: &Env) -> Option<crate::types::RecoveryRequest> {
    env.storage().instance().get(&DataKey::RecoveryRequest)
}

pub fn set_recovery_request(env: &Env, request: Option<crate::types::RecoveryRequest>) {
    match request {
        Some(r) => env.storage().instance().set(&DataKey::RecoveryRequest, &r),
        None => env.storage().instance().remove(&DataKey::RecoveryRequest),
    }
}

pub fn get_last_activity(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::LastActivity)
        .unwrap_or(0)
}

pub fn set_last_activity(env: &Env, timestamp: u64) {
    env.storage()
        .instance()
        .set(&DataKey::LastActivity, &timestamp);
}

pub fn get_pending_guardian_invites(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::PendingGuardianInvites)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_pending_guardian_invites(env: &Env, guardians: &Vec<Address>) {
    env.storage()
        .instance()
        .set(&DataKey::PendingGuardianInvites, guardians);
}

pub fn get_guardian_acceptance(env: &Env, guardian: &Address) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::GuardianAcceptances(guardian.clone()))
        .unwrap_or(false)
}

pub fn set_guardian_acceptance(env: &Env, guardian: Address, accepted: bool) {
    env.storage()
        .instance()
        .set(&DataKey::GuardianAcceptances(guardian), &accepted);
}
