#![no_std]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(deprecated)]

use soroban_sdk::{contractevent, contracttype, Address, Env, String, Symbol, Vec};

/// Asset status as used in the protocol
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssetStatus {
    Active,
    Frozen,
    Deprecated,
}

/// Proposal types for Governance
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalType {
    Standard,
    Emergency,
    ParameterChange,
}

/// Vote types for Governance
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VoteType {
    Against,
    For,
    Abstain,
}

/// Emergency Trigger
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmergencyTrigger {
    Admin,
    CircuitBreaker,
    OracleFailure,
}

pub enum RiskAlertSeverity {
    Warning = 1,
    Critical = 2,
    Emergency = 3,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct UpgradeInitEvent {
    #[topic]
    pub admin: Address,
    pub required_approvals: u32,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct UpgradeApproverAddedEvent {
    #[topic]
    pub caller: Address,
    pub approver: Address,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct UpgradeApproverRemovedEvent {
    #[topic]
    pub caller: Address,
    pub approver: Address,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct UpgradeProposedEvent {
    #[topic]
    pub caller: Address,
    pub id: u64,
    pub new_version: u32,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct UpgradeApprovalRecordedEvent {
    #[topic]
    pub caller: Address,
    pub proposal_id: u64,
    pub approval_count: u32,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct UpgradeExecutedEvent {
    #[topic]
    pub caller: Address,
    pub proposal_id: u64,
    pub new_version: u32,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct UpgradeRollbackEvent {
    #[topic]
    pub caller: Address,
    pub proposal_id: u64,
    pub prev_version: u32,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct UpgradeTimelockQueuedEvent {
    #[topic]
    pub caller: Address,
    pub proposal_id: u64,
    pub execute_after: u64,
    pub is_emergency: bool,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct UpgradeEmergencyProposedEvent {
    #[topic]
    pub caller: Address,
    pub id: u64,
    pub new_version: u32,
    pub execute_after: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct DepositEvent {
    #[topic]
    pub user: Address,
    pub asset: Option<Address>,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct WithdrawalEvent {
    #[topic]
    pub user: Address,
    pub asset: Option<Address>,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct BorrowEvent {
    #[topic]
    pub user: Address,
    pub asset: Option<Address>,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct RepayEvent {
    #[topic]
    pub user: Address,
    pub asset: Option<Address>,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct LiquidationEvent {
    #[topic]
    pub liquidator: Address,
    #[topic]
    pub borrower: Address,
    pub debt_asset: Option<Address>,
    pub collateral_asset: Option<Address>,
    pub debt_liquidated: i128,
    pub collateral_seized: i128,
    pub incentive_amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct FlashLoanInitiatedEvent {
    #[topic]
    pub user: Address,
    #[topic]
    pub asset: Address,
    pub amount: i128,
    pub fee: i128,
    pub callback: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct FlashLoanRepaidEvent {
    #[topic]
    pub user: Address,
    #[topic]
    pub asset: Address,
    pub amount: i128,
    pub fee: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct AdminActionEvent {
    pub actor: Address,
    pub action: Symbol,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct PriceUpdatedEvent {
    pub actor: Address,
    #[topic]
    pub asset: Address,
    pub price: i128,
    pub decimals: u32,
    pub oracle: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct RiskParamsUpdatedEvent {
    pub actor: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct PauseStateChangedEvent {
    pub actor: Address,
    pub operation: Symbol,
    pub paused: bool,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct PositionUpdatedEvent {
    #[topic]
    pub user: Address,
    pub collateral: i128,
    pub debt: i128,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct AnalyticsUpdatedEvent {
    #[topic]
    pub user: Address,
    pub activity_type: String,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct UserActivityTrackedEvent {
    #[topic]
    pub user: Address,
    pub operation: Symbol,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct MintEvent {
    pub token_id: u32,
    pub owner: Address,
    pub project_id: String,
    pub vintage_year: u64,
    pub methodology_id: u32,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct TransferEvent {
    pub token_id: u32,
    pub from: Address,
    pub to: Address,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct StatusChangeEvent {
    pub token_id: u32,
    pub old_status: Option<AssetStatus>,
    pub new_status: AssetStatus,
    pub changed_by: Address,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct QualityScoreUpdatedEvent {
    pub token_id: u32,
    pub old_score: i128,
    pub new_score: i128,
    pub updated_by: Address,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct ApproveEvent {
    pub from: Address,
    pub spender: Address,
    pub amount: i128,
    pub live_until_ledger: u32,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct Sep41TransferEvent {
    pub from: Address,
    pub to: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct Sep41BurnEvent {
    pub from: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct GovernanceInitializedEvent {
    #[topic]
    pub admin: Address,
    pub vote_token: Address,
    pub voting_period: u64,
    pub quorum_bps: u32,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct ProposalCreatedEvent {
    pub proposal_id: u64,
    pub proposer: Address,
    pub proposal_type: ProposalType,
    pub description: String,
    pub start_time: u64,
    pub end_time: u64,
    pub created_at: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct VoteCastEvent {
    pub proposal_id: u64,
    pub voter: Address,
    pub vote_type: VoteType,
    pub voting_power: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct VotePowerSnapshotTakenEvent {
    pub proposal_id: u64,
    pub voter: Address,
    pub balance: i128,
    pub snapshot_time: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct VoteLockedEvent {
    pub voter: Address,
    pub proposal_id: u64,
    pub locked_amount: i128,
    pub locked_until: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct VoteDelegatedEvent {
    pub delegator: Address,
    pub delegatee: Address,
    pub delegated_at: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct VoteDelegationRevokedEvent {
    pub delegator: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct SuspiciousGovActivityEvent {
    pub proposal_id: u64,
    pub voter: Address,
    pub voter_power: i128,
    pub total_supply_estimate: i128,
    pub reason: Symbol,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct ProposalQueuedEvent {
    pub proposal_id: u64,
    pub execution_time: u64,
    pub for_votes: i128,
    pub against_votes: i128,
    pub quorum_reached: bool,
    pub threshold_met: bool,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct ProposalExecutedEvent {
    pub proposal_id: u64,
    pub executor: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct ProposalFailedEvent {
    pub proposal_id: u64,
    pub for_votes: i128,
    pub against_votes: i128,
    pub quorum_reached: bool,
    pub threshold_met: bool,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct ProposalCancelledEvent {
    pub proposal_id: u64,
    #[topic]
    pub caller: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct ProposalApprovedEvent {
    pub proposal_id: u64,
    pub approver: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct GovernanceConfigUpdatedEvent {
    #[topic]
    pub admin: Address,
    pub voting_period: Option<u64>,
    pub execution_delay: Option<u64>,
    pub quorum_bps: Option<u32>,
    pub proposal_threshold: Option<i128>,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct MultisigConfigUpdatedEvent {
    #[topic]
    pub admin: Address,
    pub admins: Vec<Address>,
    pub threshold: u32,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct GuardianAddedEvent {
    pub guardian: Address,
    pub added_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct GuardianRemovedEvent {
    pub guardian: Address,
    pub removed_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct GuardianThresholdUpdatedEvent {
    #[topic]
    pub admin: Address,
    pub old_threshold: u32,
    pub new_threshold: u32,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct RecoveryStartedEvent {
    pub old_admin: Address,
    pub new_admin: Address,
    pub initiator: Address,
    pub expires_at: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct RecoveryApprovedEvent {
    pub approver: Address,
    pub current_approvals: u32,
    pub threshold: u32,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct RecoveryExecutedEvent {
    pub old_admin: Address,
    pub new_admin: Address,
    pub executor: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct TreasurySetEvent {
    #[topic]
    pub admin: Address,
    pub treasury: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct ReservesClaimedEvent {
    #[topic]
    pub admin: Address,
    pub asset: Option<Address>,
    pub recipient: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct FeeConfigUpdatedEvent {
    #[topic]
    pub admin: Address,
    pub interest_fee_bps: i128,
    pub liquidation_fee_bps: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct LiquidationFeeCollectedEvent {
    pub asset: Option<Address>,
    pub fee_amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct CreditScoreUpdatedEvent {
    #[topic]
    pub user: Address,
    pub old_score: i128,
    pub new_score: i128,
    pub reason: String,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct TimelockQueuedEvent {
    pub operation_id: u64,
    pub proposer: Address,
    pub ready_at: u64,
    pub expires_at: u64,
    pub delay: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct TimelockExecutedEvent {
    pub operation_id: u64,
    pub executor: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct TimelockCancelledEvent {
    pub operation_id: u64,
    #[topic]
    pub caller: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct CircuitBreakerActivatedEvent {
    pub activated_by: Address,
    pub emergency_mode: bool,
    pub timestamp: u64,
}

#[contractevent(topics = ["cb_deactivated"])]
#[derive(Clone, Debug)]
pub struct CircuitBreakerDeactivatedEvent {
    pub deactivated_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct WhitelistAddedEvent {
    #[topic]
    pub liquidator: Address,
    pub added_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct WhitelistRemovedEvent {
    #[topic]
    pub liquidator: Address,
    pub removed_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct LiquidationQueuedEvent {
    pub entry_id: u64,
    #[topic]
    pub borrower: Address,
    #[topic]
    pub liquidator: Address,
    pub health_factor: i128,
    pub priority_score: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct LiquidationProcessedEvent {
    pub entry_id: u64,
    #[topic]
    pub borrower: Address,
    #[topic]
    pub liquidator: Address,
    pub executor: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct LiquidationCancelledEvent {
    pub entry_id: u64,
    #[topic]
    pub caller: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct BatchLiquidationEvent {
    #[topic]
    pub liquidator: Address,
    pub total_positions: u32,
    pub successful: u32,
    pub failed: u32,
    pub total_debt_liquidated: i128,
    pub total_collateral_seized: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct EmergencyTriggeredEvent {
    pub trigger: EmergencyTrigger,
    pub started_at: u64,
    pub window_opens_at: u64,
    pub window_closes_at: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct EmergencyCancelledEvent {
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct EmergencyWithdrawalEvent {
    #[topic]
    pub user: Address,
    pub asset: Option<Address>,
    pub amount: i128,
    pub loss_share_bps: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["deposit_event"])]
#[derive(Clone, Debug)]
pub struct BorrowCollateralDepositEvent {
    #[topic]
    pub user: Address,
    #[topic]
    pub asset: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["deposit_event"])]
#[derive(Clone, Debug)]
pub struct VaultDepositEvent {
    #[topic]
    pub user: Address,
    #[topic]
    pub asset: Address,
    pub amount: i128,
    pub new_balance: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct WithdrawEvent {
    #[topic]
    pub user: Address,
    #[topic]
    pub asset: Address,
    pub amount: i128,
    pub remaining_balance: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct FlashLoanEvent {
    pub receiver: Address,
    #[topic]
    pub asset: Address,
    pub amount: i128,
    pub fee: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct PegDeviationEvent {
    #[topic]
    pub asset: Address,
    pub price: i128,
    pub target_price: i128,
    pub deviation_bps: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct StabilityFeeAppliedEvent {
    #[topic]
    pub asset: Address,
    pub fee_bps: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["interest_rate_model_updated"])]
#[derive(Clone, Debug)]
pub struct InterestRateModelUpdatedEvent {
    #[topic]
    pub caller: Address,
    pub previous: u32,
    pub updated: u32,
    pub timestamp: u64,
}

#[contractevent(topics = ["risk_util_alert"])]
#[derive(Clone, Debug)]
pub struct RiskUtilizationAlertEvent {
    pub severity: u32,
    pub utilization_bps: u32,
    pub total_debt: i128,
    pub debt_ceiling: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["borrow_commit_create"])]
#[derive(Clone, Debug)]
pub struct BorrowCommitmentCreatedEvent {
    #[topic]
    pub commitment_id: u64,
    pub owner: Address,
    pub borrow_asset: Address,
    pub borrow_amount: i128,
    pub expiry: u64,
}

#[contractevent(topics = ["borrow_commit_cancel"])]
#[derive(Clone, Debug)]
pub struct BorrowCommitmentCancelledEvent {
    #[topic]
    pub commitment_id: u64,
    pub owner: Address,
}

#[contractevent(topics = ["borrow_commit_exec"])]
#[derive(Clone, Debug)]
pub struct BorrowCommitmentExecutedEvent {
    #[topic]
    pub commitment_id: u64,
    pub owner: Address,
    pub borrowed_amount: i128,
    pub collateral_amount: i128,
}

#[contractevent(topics = ["ds_init"], data_format = "single-value")]
#[derive(Clone, Debug)]
pub struct DataStoreInitEvent {
    #[topic]
    pub admin: Address,
}

#[contractevent(topics = ["writer"], data_format = "single-value")]
#[derive(Clone, Debug)]
pub struct DataStoreWriterChangeEvent {
    #[topic]
    pub caller: Address,
    #[topic]
    pub writer: Address,
}

#[contractevent(topics = ["ds_save"], data_format = "single-value")]
#[derive(Clone, Debug)]
pub struct DataStoreSaveEvent {
    #[topic]
    pub caller: Address,
    #[topic]
    pub key: String,
    pub value_len: u32,
}

#[contractevent(topics = ["ds_bkup"], data_format = "single-value")]
#[derive(Clone, Debug)]
pub struct DataStoreBackupEvent {
    #[topic]
    pub caller: Address,
    #[topic]
    pub backup_name: String,
    pub key_count: u32,
}

#[contractevent(topics = ["ds_rest"], data_format = "single-value")]
#[derive(Clone, Debug)]
pub struct DataStoreRestoreEvent {
    #[topic]
    pub caller: Address,
    #[topic]
    pub backup_name: String,
    pub entry_count: u32,
}

#[contractevent(topics = ["ds_migr"], data_format = "single-value")]
#[derive(Clone, Debug)]
pub struct DataStoreMigrateEvent {
    #[topic]
    pub caller: Address,
    #[topic]
    pub new_version: u32,
    pub memo: Option<String>,
}


/// Emits a standardized event across the protocol.
#[macro_export]
macro_rules! emit_event {
    ($env:expr, $module:expr, $action:expr, $caller:expr, $asset:expr, $amount:expr) => {
        {
            let topics = (
                soroban_sdk::Symbol::new($env, "PROTOCOL_EVENT"),
                soroban_sdk::Symbol::new($env, $module),
                soroban_sdk::Symbol::new($env, $action),
                $caller.clone(),
                $asset.clone(),
            );
            let data = ($amount, 1u32);
            $env.events().publish(topics, data);
        }
    };
}
