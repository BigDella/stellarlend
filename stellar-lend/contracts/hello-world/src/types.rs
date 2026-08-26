use soroban_sdk::{contracttype, Address, Bytes, String, Symbol, Vec};

// ========================================================================
// Proposal Types
// ========================================================================

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum ProposalStatus {
    Pending,
    Active,
    Succeeded,
    Defeated,
    Expired,
    Queued,
    Executed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum VoteType {
    For,
    Against,
    Abstain,
}

/// Proposal type for protocol parameter changes
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum ProposalType {
    /// Change minimum collateral ratio
    MinCollateralRatio(i128),
    /// Change risk parameters (min_cr, liq_threshold, close_factor, liq_incentive)
    RiskParams(Option<i128>, Option<i128>, Option<i128>, Option<i128>),
    /// Pause/unpause operation
    PauseSwitch(Symbol, bool),
    /// Emergency pause
    EmergencyPause(bool),
    /// Generic action for future extensions
    GenericAction(Action),
    /// Change interest rate configuration
    InterestRateConfig(InterestRateParams),
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct InterestRateParams {
    pub base_rate_bps: Option<i128>,
    pub kink_utilization_bps: Option<i128>,
    pub multiplier_bps: Option<i128>,
    pub jump_multiplier_bps: Option<i128>,
    pub rate_floor_bps: Option<i128>,
    pub rate_ceiling_bps: Option<i128>,
    pub spread_bps: Option<i128>,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub proposal_type: ProposalType,
    pub description: String,
    pub status: ProposalStatus,
    pub start_time: u64,
    pub end_time: u64,
    pub execution_time: Option<u64>,
    pub voting_threshold: i128, // In basis points (e.g., 5000 = 50%)
    pub for_votes: i128,
    pub against_votes: i128,
    pub abstain_votes: i128,
    pub total_voting_power: i128,
    pub created_at: u64,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct VoteInfo {
    pub voter: Address,
    pub proposal_id: u64,
    pub vote_type: VoteType,
    pub voting_power: i128,
    pub timestamp: u64,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct VotePowerSnapshot {
    pub proposal_id: u64,
    pub voter: Address,
    pub balance: i128,
    pub snapshot_time: u64,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct VoteLock {
    pub voter: Address,
    pub locked_until: u64,
    pub locked_amount: i128,
    pub proposal_id: u64,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct DelegationRecord {
    pub delegator: Address,
    pub delegatee: Address,
    pub delegated_at: u64,
    pub depth: u32,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct GovernanceAnalytics {
    pub total_proposals: u64,
    pub total_votes: u64,
    pub suspicious_proposals: u64,
    pub last_suspicious_at: u64,
    pub max_single_voter_power: i128,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct ProposalOutcome {
    pub proposal_id: u64,
    pub succeeded: bool,
    pub for_votes: i128,
    pub against_votes: i128,
    pub abstain_votes: i128,
    pub quorum_reached: bool,
    pub quorum_required: i128,
}

// ========================================================================
// Proposal Simulation + Optimization
// ========================================================================

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct ProposalSimulationResult {
    pub proposal_id: u64,
    pub now: u64,
    pub would_succeed: bool,
    pub quorum_required: i128,
    pub quorum_reached: bool,
    pub threshold_votes: i128,
    pub threshold_met: bool,
    pub for_votes: i128,
    pub against_votes: i128,
    pub abstain_votes: i128,
    pub total_voting_power: i128,
    pub note: String,
}

/// Dry-run execution preview with state diff and impact metrics (Issue #662).
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct StateDiffEntry {
    pub field: String,
    pub current_value: i128,
    pub proposed_value: i128,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct ProposalDryRunResult {
    pub proposal_id: u64,
    pub would_succeed: bool,
    pub tvl_delta: i128,
    pub apy_delta_bps: i128,
    pub risk_score_delta: i128,
    pub gas_units_estimate: u64,
    pub diffs: Vec<StateDiffEntry>,
    pub simulated_at: u64,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct ParameterOptimizationRecommendation {
    pub generated_at: u64,
    pub suggested_quorum_bps: u32,
    pub suggested_vote_threshold_bps: i128,
    pub suggested_voting_period: u64,
    pub transparency_note: String,
}

/// Asset status for carbon credit or tokenized assets
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum AssetStatus {
    Issued,
    Listed,
    Retired,
    Invalidated,
}

// ========================================================================
// Governance Configuration
// ========================================================================

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct GovernanceConfig {
    pub voting_period: u64,             // Duration in seconds
    pub execution_delay: u64,           // Delay before execution
    pub quorum_bps: u32,                // Quorum in basis points
    pub proposal_threshold: i128,       // Min tokens to create proposal
    pub vote_token: Address,            // Token used for voting
    pub timelock_duration: u64,         // Max time before expiration
    pub default_voting_threshold: i128, // Default 50% in basis points
}

// ========================================================================
// Multisig Types
// ========================================================================

#[derive(Clone, Debug)]
#[contracttype]
pub struct MultisigConfig {
    pub admins: Vec<Address>,
    pub threshold: u32,
}

// ========================================================================
// Social Recovery Types
// ========================================================================

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct RecoveryRequest {
    pub old_admin: Address,
    pub new_admin: Address,
    pub initiator: Address,
    pub initiated_at: u64,
    pub expires_at: u64,
}

// ========================================================================
// Action Type (for generic execution)
// ========================================================================

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct Action {
    pub target: Address,
    pub method: Symbol,
    pub args: Vec<Bytes>,
    pub value: i128,
}

// ========================================================================
// Constants
// ========================================================================

pub const BASIS_POINTS_SCALE: i128 = 10_000; // 100% = 10,000 basis points
pub const DEFAULT_VOTING_PERIOD: u64 = 7 * 24 * 60 * 60; // 7 days
pub const DEFAULT_EXECUTION_DELAY: u64 = 2 * 24 * 60 * 60; // 2 days
pub const DEFAULT_QUORUM_BPS: u32 = 4_000; // 40% default quorum
pub const DEFAULT_VOTING_THRESHOLD: i128 = 5_000; // 50% default threshold
pub const DEFAULT_TIMELOCK_DURATION: u64 = 7 * 24 * 60 * 60; // 7 days
pub const DEFAULT_RECOVERY_PERIOD: u64 = 3 * 24 * 60 * 60; // 3 days
pub const MIN_TIMELOCK_DELAY: u64 = 24 * 60 * 60; // 24 hours
pub const DELEGATION_DEADLINE: u64 = 24 * 60 * 60; // 24 hours
pub const MAX_DELEGATION_DEPTH: u32 = 3;
pub const PROPOSAL_RATE_LIMIT: u32 = 5;
pub const PROPOSAL_RATE_WINDOW: u64 = 24 * 60 * 60; // 24 hours
