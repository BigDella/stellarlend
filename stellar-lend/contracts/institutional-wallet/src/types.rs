use soroban_sdk::{contracterror, contracttype, Address, BytesN, String, Val, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum WalletError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    InvalidThreshold = 4,
    InvalidAdmins = 5,
    ProposalNotFound = 6,
    AlreadyVoted = 7,
    ProposalNotActive = 8,
    InsufficientApprovals = 9,
    ExecutionFailed = 10,
    InvalidBatch = 11,
    GuardianAcceptanceRequired = 12,
    GuardianNotAccepted = 13,
    RecoveryNotActive = 14,
    RecoveryAlreadyExists = 15,
    GuardianRotationFailed = 16,
    EmergencyTimeoutActive = 17,
    RecoveryCancelledByOwner = 18,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Config,
    Admins,
    NextProposalId,
    Proposal(u64),
    Approvals(u64),
    AuditTrail(u64),
    Guardians,
    GuardianThreshold,
    RecoveryRequest,
    PendingGuardianInvites,
    GuardianAcceptances(Address),
    LastActivity,
    GuardianApprovals,
    RecoveryCancelRequest,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryRequest {
    pub new_admins: Vec<Address>,
    pub new_threshold: u32,
    pub initiated_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultisigConfig {
    pub threshold: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transaction {
    pub contract: Address,
    pub function: soroban_sdk::Symbol,
    pub args: Vec<Val>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    Active,
    Executed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub description: String,
    pub batch: Vec<Transaction>,
    pub status: ProposalStatus,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEntry {
    pub actor: Address,
    pub action: soroban_sdk::Symbol,
    pub timestamp: u64,
}
