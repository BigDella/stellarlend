#![no_std]

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol, Vec};

mod storage;
mod types;

#[cfg(test)]
mod test;

use crate::storage::{
    add_audit_entry, get_admins, get_approvals, get_config, get_next_proposal_id, get_proposal,
    increment_proposal_id, set_admins, set_approvals, set_config, set_proposal,
};
use crate::types::{
    AuditEntry, MultisigConfig, Proposal, ProposalStatus, Transaction, WalletError,
};

/// Emergency recovery timeout: 90 days without any admin activity.
const EMERGENCY_RECOVERY_TIMEOUT: u64 = 90 * 24 * 60 * 60;

#[contract]
pub struct InstitutionalWallet;

#[contractimpl]
impl InstitutionalWallet {
    /// Initialize the wallet with a set of admins and a threshold.
    pub fn initialize(env: Env, admins: Vec<Address>, threshold: u32) -> Result<(), WalletError> {
        if env.storage().instance().has(&crate::types::DataKey::Config) {
            return Err(WalletError::AlreadyInitialized);
        }

        if admins.is_empty() {
            return Err(WalletError::InvalidAdmins);
        }

        if threshold == 0 || threshold > admins.len() {
            return Err(WalletError::InvalidThreshold);
        }

        let config = MultisigConfig { threshold };
        set_config(&env, &config);
        set_admins(&env, &admins);
        crate::storage::set_last_activity(&env, env.ledger().timestamp());

        Ok(())
    }

    /// Propose a batch of transactions.
    pub fn propose(
        env: Env,
        proposer: Address,
        description: String,
        batch: Vec<Transaction>,
    ) -> Result<u64, WalletError> {
        proposer.require_auth();

        let admins = get_admins(&env);
        if !admins.contains(proposer.clone()) {
            return Err(WalletError::Unauthorized);
        }

        if batch.is_empty() {
            return Err(WalletError::InvalidBatch);
        }

        let id = increment_proposal_id(&env);
        let now = env.ledger().timestamp();

        let proposal = Proposal {
            id,
            proposer: proposer.clone(),
            description,
            batch,
            status: ProposalStatus::Active,
            created_at: now,
        };

        set_proposal(&env, id, &proposal);
        crate::storage::set_last_activity(&env, now);

        // Auto-approve by proposer
        let mut approvals = Vec::new(&env);
        approvals.push_back(proposer.clone());
        set_approvals(&env, id, &approvals);

        add_audit_entry(
            &env,
            id,
            AuditEntry {
                actor: proposer,
                action: symbol_short!("propose"),
                timestamp: now,
            },
        );

        Ok(id)
    }

    /// Approve an active proposal.
    pub fn approve(env: Env, approver: Address, proposal_id: u64) -> Result<(), WalletError> {
        approver.require_auth();

        let admins = get_admins(&env);
        if !admins.contains(approver.clone()) {
            return Err(WalletError::Unauthorized);
        }

        let proposal = get_proposal(&env, proposal_id).ok_or(WalletError::ProposalNotFound)?;
        if proposal.status != ProposalStatus::Active {
            return Err(WalletError::ProposalNotActive);
        }

        let mut approvals = get_approvals(&env, proposal_id);
        if approvals.contains(approver.clone()) {
            return Err(WalletError::AlreadyVoted);
        }

        approvals.push_back(approver.clone());
        set_approvals(&env, proposal_id, &approvals);
        crate::storage::set_last_activity(&env, env.ledger().timestamp());

        add_audit_entry(
            &env,
            proposal_id,
            AuditEntry {
                actor: approver,
                action: symbol_short!("approve"),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Execute a proposal if threshold is met.
    pub fn execute(env: Env, executor: Address, proposal_id: u64) -> Result<(), WalletError> {
        executor.require_auth();

        let admins = get_admins(&env);
        if !admins.contains(executor.clone()) {
            return Err(WalletError::Unauthorized);
        }

        let mut proposal = get_proposal(&env, proposal_id).ok_or(WalletError::ProposalNotFound)?;
        if proposal.status != ProposalStatus::Active {
            return Err(WalletError::ProposalNotActive);
        }

        let config = get_config(&env)?;
        let approvals = get_approvals(&env, proposal_id);

        if approvals.len() < config.threshold {
            return Err(WalletError::InsufficientApprovals);
        }

        // Execute batch
        for tx in proposal.batch.iter() {
            env.invoke_contract::<()>(&tx.contract, &tx.function, tx.args);
        }

        proposal.status = ProposalStatus::Executed;
        set_proposal(&env, proposal_id, &proposal);
        crate::storage::set_last_activity(&env, env.ledger().timestamp());

        add_audit_entry(
            &env,
            proposal_id,
            AuditEntry {
                actor: executor,
                action: symbol_short!("execute"),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Add a new admin to the wallet (must be called via multisig execute).
    pub fn add_admin(env: Env, new_admin: Address) -> Result<(), WalletError> {
        env.current_contract_address().require_auth();

        let mut admins = get_admins(&env);
        if admins.contains(new_admin.clone()) {
            return Err(WalletError::InvalidAdmins);
        }

        admins.push_back(new_admin);
        set_admins(&env, &admins);
        Ok(())
    }

    /// Remove an admin from the wallet (must be called via multisig execute).
    pub fn remove_admin(env: Env, admin: Address) -> Result<(), WalletError> {
        env.current_contract_address().require_auth();

        let admins = get_admins(&env);
        let mut new_admins = Vec::new(&env);
        let mut found = false;
        for a in admins.iter() {
            if a == admin {
                found = true;
            } else {
                new_admins.push_back(a);
            }
        }

        if !found {
            return Err(WalletError::InvalidAdmins);
        }

        let config = get_config(&env)?;
        if new_admins.len() < config.threshold as usize {
            return Err(WalletError::InvalidThreshold);
        }

        set_admins(&env, &new_admins);
        Ok(())
    }

    /// Set a new approval threshold (must be called via multisig execute).
    pub fn set_threshold(env: Env, threshold: u32) -> Result<(), WalletError> {
        env.current_contract_address().require_auth();

        let admins = get_admins(&env);
        if threshold == 0 || threshold > admins.len() {
            return Err(WalletError::InvalidThreshold);
        }

        let mut config = get_config(&env)?;
        config.threshold = threshold;
        set_config(&env, &config);
        Ok(())
    }

    /// Propose guardians for designation (creates pending invites).
    pub fn propose_guardians(
        env: Env,
        caller: Address,
        guardians: Vec<Address>,
        threshold: u32,
    ) -> Result<(), WalletError> {
        env.current_contract_address().require_auth();

        if guardians.is_empty() || threshold == 0 || threshold > guardians.len() {
            return Err(WalletError::InvalidThreshold);
        }

        // Store as pending invites instead of directly assigning
        crate::storage::set_pending_guardian_invites(&env, &guardians);
        crate::storage::set_guardian_threshold(&env, threshold);

        add_audit_entry(
            &env,
            0,
            AuditEntry {
                actor: caller,
                action: symbol_short!("invite"),
                timestamp: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    /// Accept guardian designation (guardian must call this).
    pub fn accept_guardian(env: Env, guardian: Address) -> Result<(), WalletError> {
        guardian.require_auth();

        let pending = crate::storage::get_pending_guardian_invites(&env);
        if !pending.contains(guardian.clone()) {
            return Err(WalletError::Unauthorized);
        }

        let mut acceptances: Vec<Address> = crate::storage::get_guardians(&env);
        if acceptances.contains(guardian.clone()) {
            return Err(WalletError::InvalidAdmins);
        }

        acceptances.push_back(guardian.clone());
        crate::storage::set_guardians(&env, &acceptances);
        crate::storage::set_guardian_acceptance(&env, guardian.clone(), true);

        add_audit_entry(
            &env,
            0,
            AuditEntry {
                actor: guardian,
                action: symbol_short!("accept"),
                timestamp: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    /// Rotate guardians with existing guardian consent.
    pub fn rotate_guardians(
        env: Env,
        caller: Address,
        new_guardians: Vec<Address>,
        new_threshold: u32,
    ) -> Result<(), WalletError> {
        caller.require_auth();

        let guardians = crate::storage::get_guardians(&env);
        if !guardians.contains(caller.clone()) {
            return Err(WalletError::Unauthorized);
        }

        if new_guardians.is_empty() || new_threshold == 0 || new_threshold > new_guardians.len() {
            return Err(WalletError::InvalidThreshold);
        }

        // Collect guardian approvals for rotation
        let mut approvals: Vec<Address> = env
            .storage()
            .instance()
            .get(&types::DataKey::GuardianApprovals)
            .unwrap_or_else(|| Vec::new(&env));
        if approvals.contains(caller.clone()) {
            return Err(WalletError::AlreadyVoted);
        }

        let threshold = crate::storage::get_guardian_threshold(&env);
        approvals.push_back(caller.clone());
        env.storage()
            .instance()
            .set(&types::DataKey::GuardianApprovals, &approvals);

        if approvals.len() < threshold as usize {
            return Ok(()); // Need more approvals
        }

        // Threshold met — execute rotation
        crate::storage::set_guardians(&env, &new_guardians);
        crate::storage::set_guardian_threshold(&env, new_threshold);
        crate::storage::set_pending_guardian_invites(&env, &new_guardians);

        // Reset approvals
        env.storage()
            .instance()
            .remove(&types::DataKey::GuardianApprovals);

        add_audit_entry(
            &env,
            0,
            AuditEntry {
                actor: caller,
                action: symbol_short!("rotate"),
                timestamp: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    /// Start a recovery request by a guardian.
    pub fn start_recovery(
        env: Env,
        guardian: Address,
        new_admins: Vec<Address>,
        new_threshold: u32,
    ) -> Result<(), WalletError> {
        guardian.require_auth();

        // Check emergency timeout — if 90 days without activity, any guardian can trigger
        let last_activity = crate::storage::get_last_activity(&env);
        let now = env.ledger().timestamp();
        let emergency_active = last_activity + EMERGENCY_RECOVERY_TIMEOUT < now;

        let guardians = crate::storage::get_guardians(&env);
        let is_guardian = guardians.contains(guardian.clone());

        // In emergency mode, any accepted guardian can initiate
        // In normal mode, must be an accepted guardian
        if !is_guardian {
            return Err(WalletError::Unauthorized);
        }

        if crate::storage::get_recovery_request(&env).is_some() {
            return Err(WalletError::RecoveryAlreadyExists);
        }

        if new_admins.is_empty() || new_threshold == 0 || new_threshold > new_admins.len() {
            return Err(WalletError::InvalidThreshold);
        }

        let request = crate::types::RecoveryRequest {
            new_admins,
            new_threshold,
            initiated_at: now,
        };

        crate::storage::set_recovery_request(&env, Some(request));

        // Reset guardian approvals for this recovery
        let mut approvals = Vec::new(&env);
        approvals.push_back(guardian.clone());
        env.storage()
            .instance()
            .set(&types::DataKey::GuardianApprovals, &approvals);

        add_audit_entry(
            &env,
            0,
            AuditEntry {
                actor: guardian,
                action: symbol_short!("recover"),
                timestamp: now,
            },
        );
        Ok(())
    }

    /// Approve a pending recovery (by another guardian).
    pub fn approve_recovery(env: Env, guardian: Address) -> Result<(), WalletError> {
        guardian.require_auth();

        let guardians = crate::storage::get_guardians(&env);
        if !guardians.contains(guardian.clone()) {
            return Err(WalletError::Unauthorized);
        }

        let request =
            crate::storage::get_recovery_request(&env).ok_or(WalletError::RecoveryNotActive)?;
        let now = env.ledger().timestamp();

        // Recovery expires after emergency timeout
        if now > request.initiated_at + EMERGENCY_RECOVERY_TIMEOUT {
            crate::storage::set_recovery_request(&env, None);
            return Err(WalletError::RecoveryNotActive);
        }

        let mut approvals: Vec<Address> = env
            .storage()
            .instance()
            .get(&types::DataKey::GuardianApprovals)
            .unwrap_or_else(|| Vec::new(&env));
        if approvals.contains(guardian.clone()) {
            return Err(WalletError::AlreadyVoted);
        }

        approvals.push_back(guardian.clone());
        env.storage()
            .instance()
            .set(&types::DataKey::GuardianApprovals, &approvals);

        add_audit_entry(
            &env,
            0,
            AuditEntry {
                actor: guardian,
                action: symbol_short!("aprvRec"),
                timestamp: now,
            },
        );
        Ok(())
    }

    /// Cancel a recovery request by the original owner (admin).
    pub fn cancel_recovery_by_owner(env: Env, owner: Address) -> Result<(), WalletError> {
        owner.require_auth();

        let admins = get_admins(&env);
        if !admins.contains(owner.clone()) {
            return Err(WalletError::Unauthorized);
        }

        let request =
            crate::storage::get_recovery_request(&env).ok_or(WalletError::RecoveryNotActive)?;
        let now = env.ledger().timestamp();

        // Can only cancel during the challenge period (before threshold is met)
        let threshold = crate::storage::get_guardian_threshold(&env);
        let approvals: Vec<Address> = env
            .storage()
            .instance()
            .get(&types::DataKey::GuardianApprovals)
            .unwrap_or_else(|| Vec::new(&env));

        if approvals.len() >= threshold as usize {
            return Err(WalletError::ExecutionFailed); // Too late, recovery already approved
        }

        crate::storage::set_recovery_request(&env, None);
        env.storage()
            .instance()
            .remove(&types::DataKey::GuardianApprovals);

        add_audit_entry(
            &env,
            0,
            AuditEntry {
                actor: owner,
                action: symbol_short!("cnclRec"),
                timestamp: now,
            },
        );
        Ok(())
    }

    /// Execute recovery after guardian threshold is met.
    pub fn execute_recovery(env: Env, guardian: Address) -> Result<(), WalletError> {
        guardian.require_auth();

        let guardians = crate::storage::get_guardians(&env);
        if !guardians.contains(guardian.clone()) {
            return Err(WalletError::Unauthorized);
        }

        let request =
            crate::storage::get_recovery_request(&env).ok_or(WalletError::ProposalNotFound)?;
        let now = env.ledger().timestamp();

        // Check emergency timeout mode
        let last_activity = crate::storage::get_last_activity(&env);
        let emergency_active = last_activity + EMERGENCY_RECOVERY_TIMEOUT < now;

        // Get guardian approvals
        let threshold = crate::storage::get_guardian_threshold(&env);
        let approvals: Vec<Address> = env
            .storage()
            .instance()
            .get(&types::DataKey::GuardianApprovals)
            .unwrap_or_else(|| Vec::new(&env));

        if !emergency_active {
            // Normal mode: enforce recovery delay (24h) and guardian threshold
            if now < request.initiated_at + 86400 {
                return Err(WalletError::ExecutionFailed);
            }

            if approvals.len() < threshold as usize {
                return Err(WalletError::InsufficientApprovals);
            }
        }
        // Emergency mode: skip delay, just need guardian auth

        set_admins(&env, &request.new_admins);
        let config = MultisigConfig {
            threshold: request.new_threshold,
        };
        set_config(&env, &config);

        crate::storage::set_recovery_request(&env, None);
        env.storage()
            .instance()
            .remove(&types::DataKey::GuardianApprovals);
        crate::storage::set_last_activity(&env, now);

        add_audit_entry(
            &env,
            0,
            AuditEntry {
                actor: guardian,
                action: symbol_short!("execRec"),
                timestamp: now,
            },
        );
        Ok(())
    }

    // --- View Functions ---

    pub fn get_proposal(env: Env, id: u64) -> Option<Proposal> {
        get_proposal(&env, id)
    }

    pub fn get_audit_trail(env: Env, id: u64) -> Vec<AuditEntry> {
        crate::storage::get_audit_trail(&env, id)
    }

    pub fn get_admins(env: Env) -> Vec<Address> {
        get_admins(&env)
    }

    pub fn get_threshold(env: Env) -> u32 {
        get_config(&env).map(|c| c.threshold).unwrap_or(0)
    }

    pub fn get_guardians(env: Env) -> Vec<Address> {
        crate::storage::get_guardians(&env)
    }

    pub fn get_guardian_threshold(env: Env) -> u32 {
        crate::storage::get_guardian_threshold(&env)
    }

    pub fn get_pending_guardian_invites(env: Env) -> Vec<Address> {
        crate::storage::get_pending_guardian_invites(&env)
    }

    pub fn is_guardian_accepted(env: Env, guardian: Address) -> bool {
        crate::storage::get_guardian_acceptance(&env, &guardian)
    }

    pub fn get_recovery_request(env: Env) -> Option<crate::types::RecoveryRequest> {
        crate::storage::get_recovery_request(&env)
    }

    pub fn get_guardian_approvals(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&types::DataKey::GuardianApprovals)
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn get_last_activity(env: Env) -> u64 {
        crate::storage::get_last_activity(&env)
    }
}
