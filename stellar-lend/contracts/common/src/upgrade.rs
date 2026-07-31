use crate::events::{
    UpgradeApprovalRecordedEvent, UpgradeApproverAddedEvent, UpgradeApproverRemovedEvent,
    UpgradeEmergencyProposedEvent, UpgradeExecutedEvent, UpgradeInitEvent, UpgradeProposedEvent,
    UpgradeRollbackEvent, UpgradeTimelockQueuedEvent,
};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, BytesN, Env,
    Vec,
};

/// Standard upgrade timelock: 48 hours in seconds.
pub const STANDARD_TIMELOCK_SECS: u64 = 172_800;

/// Emergency upgrade timelock: 4 hours in seconds.
pub const EMERGENCY_TIMELOCK_SECS: u64 = 14_400;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum UpgradeError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAuthorized = 3,
    ProposalNotFound = 4,
    InvalidVersion = 5,
    InvalidStatus = 6,
    AlreadyApproved = 7,
    NotEnoughApprovals = 8,
    InvalidThreshold = 9,
    TimelockNotElapsed = 10,
    StorageLayoutMismatch = 11,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpgradeStage {
    Proposed,
    Approved,
    /// Proposal has passed multisig but is waiting for the timelock to elapse.
    TimelockQueued,
    Executed,
    RolledBack,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeProposal {
    pub id: u64,
    pub proposer: Address,
    pub new_wasm_hash: BytesN<32>,
    pub new_version: u32,
    pub approvals: Vec<Address>,
    pub stage: UpgradeStage,
    pub prev_wasm_hash: Option<BytesN<32>>,
    pub prev_version: Option<u32>,
    /// Ledger timestamp after which `upgrade_execute` may be called.
    pub execute_after: Option<u64>,
    /// True when this proposal was queued via the emergency path (4 h timelock).
    pub is_emergency: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeStatus {
    pub id: u64,
    pub stage: UpgradeStage,
    pub approval_count: u32,
    pub required_approvals: u32,
    pub target_version: u32,
    /// Earliest timestamp at which execution is allowed (None = no timelock yet).
    pub execute_after: Option<u64>,
    pub is_emergency: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum UpgradeKey {
    Admin,
    Approvers,
    RequiredApprovals,
    NextProposalId,
    CurrentWasmHash,
    CurrentVersion,
    Proposal(u64),
}

#[contract]
pub struct UpgradeManager;

#[contractimpl]
impl UpgradeManager {
    /// Initializes the upgrade manager with an admin and current implementation hash.
    pub fn init(env: Env, admin: Address, current_wasm_hash: BytesN<32>, required_approvals: u32) {
        admin.require_auth();
        if env.storage().persistent().has(&UpgradeKey::Admin) {
            panic_with_error!(&env, UpgradeError::AlreadyInitialized);
        }
        if required_approvals == 0 {
            panic_with_error!(&env, UpgradeError::InvalidThreshold);
        }

        let mut approvers = Vec::new(&env);
        approvers.push_back(admin.clone());

        env.storage().persistent().set(&UpgradeKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&UpgradeKey::Approvers, &approvers);
        env.storage()
            .persistent()
            .set(&UpgradeKey::RequiredApprovals, &required_approvals);
        env.storage()
            .persistent()
            .set(&UpgradeKey::NextProposalId, &1u64);
        env.storage()
            .persistent()
            .set(&UpgradeKey::CurrentWasmHash, &current_wasm_hash);
        env.storage()
            .persistent()
            .set(&UpgradeKey::CurrentVersion, &0u32);

        UpgradeInitEvent {
            admin: admin.clone(),
            required_approvals,
        }
        .publish(&env);
    }

    /// Adds an upgrade approver. Only admin can call.
    pub fn add_approver(env: Env, caller: Address, approver: Address) {
        caller.require_auth();
        Self::assert_initialized(&env);
        Self::assert_admin(&env, &caller);

        let mut approvers = Self::approvers(&env);
        if !approvers.contains(&approver) {
            approvers.push_back(approver.clone());
            env.storage()
                .persistent()
                .set(&UpgradeKey::Approvers, &approvers);
        }

        UpgradeApproverAddedEvent {
            caller: caller.clone(),
            approver: approver.clone(),
        }
        .publish(&env);
    }

    /// Removes an upgrade approver. Only admin can call.
    ///
    /// Safety guardrails:
    /// - Admin cannot remove themselves as an approver.
    /// - The approver set cannot be reduced below `required_approvals`.
    pub fn remove_approver(env: Env, caller: Address, approver: Address) {
        caller.require_auth();
        Self::assert_initialized(&env);
        Self::assert_admin(&env, &caller);

        let admin: Address = env
            .storage()
            .persistent()
            .get(&UpgradeKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, UpgradeError::NotInitialized));

        if approver == admin {
            panic_with_error!(&env, UpgradeError::NotAuthorized);
        }

        let required = Self::required_approvals(env.clone());
        let mut approvers = Self::approvers(&env);

        let mut idx: Option<u32> = None;
        for i in 0..approvers.len() {
            if approvers.get(i).unwrap() == approver {
                idx = Some(i);
                break;
            }
        }

        if let Some(i) = idx {
            approvers.remove(i);
            if approvers.len() < required {
                panic_with_error!(&env, UpgradeError::InvalidThreshold);
            }
            env.storage()
                .persistent()
                .set(&UpgradeKey::Approvers, &approvers);

            UpgradeApproverRemovedEvent {
                caller: caller.clone(),
                approver: approver.clone(),
            }
            .publish(&env);
        }
    }

    /// Proposes a new implementation hash and target version.
    pub fn upgrade_propose(
        env: Env,
        caller: Address,
        new_wasm_hash: BytesN<32>,
        new_version: u32,
    ) -> u64 {
        caller.require_auth();
        Self::assert_initialized(&env);
        Self::assert_admin(&env, &caller);

        let current_version = Self::current_version(env.clone());
        if new_version <= current_version {
            panic_with_error!(&env, UpgradeError::InvalidVersion);
        }

        let mut approvals = Vec::new(&env);
        approvals.push_back(caller.clone());

        let required = Self::required_approvals(env.clone());
        let stage = if approvals.len() >= required {
            UpgradeStage::Approved
        } else {
            UpgradeStage::Proposed
        };

        let id: u64 = env
            .storage()
            .persistent()
            .get(&UpgradeKey::NextProposalId)
            .unwrap_or(1);
        let proposal = UpgradeProposal {
            id,
            proposer: caller.clone(),
            new_wasm_hash,
            new_version,
            approvals,
            stage,
            prev_wasm_hash: None,
            prev_version: None,
            execute_after: None,
            is_emergency: false,
        };

        env.storage()
            .persistent()
            .set(&UpgradeKey::Proposal(id), &proposal);
        env.storage()
            .persistent()
            .set(&UpgradeKey::NextProposalId, &(id + 1));

        UpgradeProposedEvent {
            caller: caller.clone(),
            id,
            new_version,
        }
        .publish(&env);
        id
    }

    /// Approves a proposal. Caller must be in approvers.
    pub fn upgrade_approve(env: Env, caller: Address, proposal_id: u64) -> u32 {
        caller.require_auth();
        Self::assert_initialized(&env);
        Self::assert_approver(&env, &caller);

        let mut proposal = Self::proposal(env.clone(), proposal_id);
        if proposal.stage != UpgradeStage::Proposed && proposal.stage != UpgradeStage::Approved {
            panic_with_error!(&env, UpgradeError::InvalidStatus);
        }
        if proposal.approvals.contains(&caller) {
            panic_with_error!(&env, UpgradeError::AlreadyApproved);
        }

        proposal.approvals.push_back(caller.clone());
        if proposal.approvals.len() >= Self::required_approvals(env.clone()) {
            proposal.stage = UpgradeStage::Approved;
        }
        let count = proposal.approvals.len();

        env.storage()
            .persistent()
            .set(&UpgradeKey::Proposal(proposal_id), &proposal);
        UpgradeApprovalRecordedEvent {
            caller: caller.clone(),
            proposal_id,
            approval_count: count,
        }
        .publish(&env);
        count
    }

    /// Queues an approved proposal into the timelock.
    ///
    /// Only callable once per proposal. Starts the 48 h (standard) countdown.
    /// Emergency proposals use the 4 h path — call `upgrade_propose_emergency` instead.
    pub fn upgrade_queue_timelock(env: Env, caller: Address, proposal_id: u64) {
        caller.require_auth();
        Self::assert_initialized(&env);
        Self::assert_approver(&env, &caller);

        let mut proposal = Self::proposal(env.clone(), proposal_id);
        if proposal.stage != UpgradeStage::Approved {
            panic_with_error!(&env, UpgradeError::InvalidStatus);
        }

        let execute_after = env
            .ledger()
            .timestamp()
            .saturating_add(STANDARD_TIMELOCK_SECS);
        proposal.stage = UpgradeStage::TimelockQueued;
        proposal.execute_after = Some(execute_after);

        env.storage()
            .persistent()
            .set(&UpgradeKey::Proposal(proposal_id), &proposal);

        UpgradeTimelockQueuedEvent {
            caller: caller.clone(),
            proposal_id,
            execute_after,
            is_emergency: false,
        }
        .publish(&env);
    }

    /// Proposes an emergency upgrade with a 4 h timelock instead of 48 h.
    ///
    /// Requires admin. Counts the proposer's approval automatically; if the
    /// threshold is already met the proposal jumps straight to `TimelockQueued`.
    pub fn upgrade_propose_emergency(
        env: Env,
        caller: Address,
        new_wasm_hash: BytesN<32>,
        new_version: u32,
    ) -> u64 {
        caller.require_auth();
        Self::assert_initialized(&env);
        Self::assert_admin(&env, &caller);

        let current_version = Self::current_version(env.clone());
        if new_version <= current_version {
            panic_with_error!(&env, UpgradeError::InvalidVersion);
        }

        let mut approvals = Vec::new(&env);
        approvals.push_back(caller.clone());

        let required = Self::required_approvals(env.clone());
        let execute_after = env
            .ledger()
            .timestamp()
            .saturating_add(EMERGENCY_TIMELOCK_SECS);
        let stage = if approvals.len() >= required {
            UpgradeStage::TimelockQueued
        } else {
            UpgradeStage::Proposed
        };

        let id: u64 = env
            .storage()
            .persistent()
            .get(&UpgradeKey::NextProposalId)
            .unwrap_or(1);

        let proposal = UpgradeProposal {
            id,
            proposer: caller.clone(),
            new_wasm_hash,
            new_version,
            approvals,
            stage: stage.clone(),
            prev_wasm_hash: None,
            prev_version: None,
            execute_after: if stage == UpgradeStage::TimelockQueued {
                Some(execute_after)
            } else {
                None
            },
            is_emergency: true,
        };

        env.storage()
            .persistent()
            .set(&UpgradeKey::Proposal(id), &proposal);
        env.storage()
            .persistent()
            .set(&UpgradeKey::NextProposalId, &(id + 1));

        UpgradeEmergencyProposedEvent {
            caller: caller.clone(),
            id,
            new_version,
            execute_after,
        }
        .publish(&env);

        id
    }

    /// Executes an approved (and timelock-elapsed) proposal. Caller must be an approver.
    pub fn upgrade_execute(env: Env, caller: Address, proposal_id: u64) {
        caller.require_auth();
        Self::assert_initialized(&env);
        Self::assert_approver(&env, &caller);

        let mut proposal = Self::proposal(env.clone(), proposal_id);
        if proposal.stage == UpgradeStage::Executed || proposal.stage == UpgradeStage::RolledBack {
            panic_with_error!(&env, UpgradeError::InvalidStatus);
        }
        if proposal.approvals.len() < Self::required_approvals(env.clone()) {
            panic_with_error!(&env, UpgradeError::NotEnoughApprovals);
        }

        // Enforce timelock: proposal must be in TimelockQueued and past its execute_after.
        if proposal.stage == UpgradeStage::TimelockQueued {
            let execute_after = proposal
                .execute_after
                .unwrap_or_else(|| panic_with_error!(&env, UpgradeError::InvalidStatus));
            if env.ledger().timestamp() < execute_after {
                panic_with_error!(&env, UpgradeError::TimelockNotElapsed);
            }
        } else if proposal.stage != UpgradeStage::Approved {
            // Proposals that have enough approvals but were never queued must go through
            // upgrade_queue_timelock first before execution is allowed.
            panic_with_error!(&env, UpgradeError::InvalidStatus);
        }

        let current_hash = Self::current_wasm_hash(env.clone());
        let current_version = Self::current_version(env.clone());
        proposal.prev_wasm_hash = Some(current_hash.clone());
        proposal.prev_version = Some(current_version);
        proposal.stage = UpgradeStage::Executed;

        env.storage()
            .persistent()
            .set(&UpgradeKey::CurrentWasmHash, &proposal.new_wasm_hash);
        env.storage()
            .persistent()
            .set(&UpgradeKey::CurrentVersion, &proposal.new_version);
        env.storage()
            .persistent()
            .set(&UpgradeKey::Proposal(proposal_id), &proposal);

        UpgradeExecutedEvent {
            caller: caller.clone(),
            proposal_id,
            new_version: proposal.new_version,
        }
        .publish(&env);
    }

    /// Rolls back an executed proposal to the previous hash/version. Only admin can call.
    pub fn upgrade_rollback(env: Env, caller: Address, proposal_id: u64) {
        caller.require_auth();
        Self::assert_initialized(&env);
        Self::assert_admin(&env, &caller);

        let mut proposal = Self::proposal(env.clone(), proposal_id);
        if proposal.stage != UpgradeStage::Executed {
            panic_with_error!(&env, UpgradeError::InvalidStatus);
        }

        let prev_hash = proposal
            .prev_wasm_hash
            .clone()
            .unwrap_or_else(|| panic_with_error!(&env, UpgradeError::InvalidStatus));
        let prev_version = proposal
            .prev_version
            .unwrap_or_else(|| panic_with_error!(&env, UpgradeError::InvalidStatus));

        env.storage()
            .persistent()
            .set(&UpgradeKey::CurrentWasmHash, &prev_hash);
        env.storage()
            .persistent()
            .set(&UpgradeKey::CurrentVersion, &prev_version);

        proposal.stage = UpgradeStage::RolledBack;
        env.storage()
            .persistent()
            .set(&UpgradeKey::Proposal(proposal_id), &proposal);

        UpgradeRollbackEvent {
            caller: caller.clone(),
            proposal_id,
            prev_version,
        }
        .publish(&env);
    }

    /// Returns the status of a proposal.
    pub fn upgrade_status(env: Env, proposal_id: u64) -> UpgradeStatus {
        Self::assert_initialized(&env);
        let proposal = Self::proposal(env.clone(), proposal_id);
        UpgradeStatus {
            id: proposal.id,
            stage: proposal.stage,
            approval_count: proposal.approvals.len(),
            required_approvals: Self::required_approvals(env),
            target_version: proposal.new_version,
            execute_after: proposal.execute_after,
            is_emergency: proposal.is_emergency,
        }
    }

    /// Returns the current active implementation hash.
    pub fn current_wasm_hash(env: Env) -> BytesN<32> {
        Self::assert_initialized(&env);
        env.storage()
            .persistent()
            .get(&UpgradeKey::CurrentWasmHash)
            .unwrap_or_else(|| panic_with_error!(&env, UpgradeError::NotInitialized))
    }

    /// Returns the currently active version.
    pub fn current_version(env: Env) -> u32 {
        Self::assert_initialized(&env);
        env.storage()
            .persistent()
            .get(&UpgradeKey::CurrentVersion)
            .unwrap_or(0)
    }

    /// Returns the configured approval threshold.
    pub fn required_approvals(env: Env) -> u32 {
        Self::assert_initialized(&env);
        env.storage()
            .persistent()
            .get(&UpgradeKey::RequiredApprovals)
            .unwrap_or(0)
    }

    /// Returns true when `addr` has approval rights.
    pub fn is_approver(env: Env, addr: Address) -> bool {
        if !env.storage().persistent().has(&UpgradeKey::Admin) {
            return false;
        }
        Self::approvers(&env).contains(&addr)
    }

    fn proposal(env: Env, proposal_id: u64) -> UpgradeProposal {
        env.storage()
            .persistent()
            .get(&UpgradeKey::Proposal(proposal_id))
            .unwrap_or_else(|| panic_with_error!(&env, UpgradeError::ProposalNotFound))
    }

    fn approvers(env: &Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&UpgradeKey::Approvers)
            .unwrap_or_else(|| Vec::new(env))
    }

    fn assert_initialized(env: &Env) {
        if !env.storage().persistent().has(&UpgradeKey::Admin) {
            panic_with_error!(env, UpgradeError::NotInitialized);
        }
    }

    fn assert_admin(env: &Env, caller: &Address) {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&UpgradeKey::Admin)
            .unwrap_or_else(|| panic_with_error!(env, UpgradeError::NotInitialized));
        if *caller != admin {
            panic_with_error!(env, UpgradeError::NotAuthorized);
        }
    }

    fn assert_approver(env: &Env, caller: &Address) {
        if !Self::approvers(env).contains(caller) {
            panic_with_error!(env, UpgradeError::NotAuthorized);
        }
    }
}
