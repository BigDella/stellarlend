#![allow(unused_variables)]
pub use shared_events::*;

use soroban_sdk::{contractevent, Address, Env, String, Symbol, Vec};

use crate::types::{AssetStatus, ProposalType, VoteType};

// ============================================================================
// Core Lending Events (Existing)
// ============================================================================





























// ============================================================================
// Asset-Specific Events (Carbon Asset Style)
// ============================================================================

#[allow(dead_code)]


#[allow(dead_code)]


#[allow(dead_code)]


#[allow(dead_code)]


#[allow(dead_code)]


#[allow(dead_code)]


#[allow(dead_code)]


// ============================================================================
// Governance Events
// ============================================================================



























#[allow(dead_code)]


// ============================================================================
// Multisig Events
// ============================================================================

#[allow(dead_code)]


// ============================================================================
// Guardian & Recovery Events
// ============================================================================





#[allow(dead_code)]








// ============================================================================
// Treasury Events
// ============================================================================









// ============================================================================
// Core Lending Emitter Helpers
// ============================================================================

pub fn emit_deposit(e: &Env, event: DepositEvent) {
    event.publish(e);
}

pub fn emit_withdrawal(e: &Env, event: WithdrawalEvent) {
    event.publish(e);
}

pub fn emit_borrow(e: &Env, event: BorrowEvent) {
    event.publish(e);
}

pub fn emit_repay(e: &Env, event: RepayEvent) {
    event.publish(e);
}

pub fn emit_liquidation(e: &Env, event: LiquidationEvent) {
    event.publish(e);
}

pub fn emit_flash_loan_initiated(e: &Env, event: FlashLoanInitiatedEvent) {
    event.publish(e);
}

pub fn emit_flash_loan_repaid(e: &Env, event: FlashLoanRepaidEvent) {
    event.publish(e);
}

pub fn emit_admin_action(e: &Env, event: AdminActionEvent) {
    event.publish(e);
}

pub fn emit_price_updated(e: &Env, event: PriceUpdatedEvent) {
    event.publish(e);
}

pub fn emit_risk_params_updated(e: &Env, event: RiskParamsUpdatedEvent) {
    event.publish(e);
}

pub fn emit_pause_state_changed(e: &Env, event: PauseStateChangedEvent) {
    event.publish(e);
}

pub fn emit_position_updated(e: &Env, event: PositionUpdatedEvent) {
    event.publish(e);
}

pub fn emit_analytics_updated(e: &Env, event: AnalyticsUpdatedEvent) {
    event.publish(e);
}

pub fn emit_user_activity_tracked(e: &Env, event: UserActivityTrackedEvent) {
    event.publish(e);
}

// ============================================================================
// Asset-Specific Emitter Helpers
// ============================================================================

#[allow(dead_code)]
pub fn emit_mint(e: &Env, event: MintEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_transfer(e: &Env, event: TransferEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_status_change(e: &Env, event: StatusChangeEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_quality_score_updated(e: &Env, event: QualityScoreUpdatedEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_approve(e: &Env, event: ApproveEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_sep41_transfer(e: &Env, event: Sep41TransferEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_sep41_burn(e: &Env, event: Sep41BurnEvent) {
    event.publish(e);
}

// ============================================================================
// Governance Emitter Helpers
// ============================================================================

#[allow(dead_code)]
pub fn emit_governance_initialized(e: &Env, event: GovernanceInitializedEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_proposal_created(e: &Env, event: ProposalCreatedEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_vote_cast(e: &Env, event: VoteCastEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_proposal_queued(e: &Env, event: ProposalQueuedEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_proposal_executed(e: &Env, event: ProposalExecutedEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_proposal_failed(e: &Env, event: ProposalFailedEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_proposal_cancelled(e: &Env, event: ProposalCancelledEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_proposal_approved(e: &Env, event: ProposalApprovedEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_governance_config_updated(e: &Env, event: GovernanceConfigUpdatedEvent) {
    event.publish(e);
}

// ============================================================================
// Multisig Emitter Helpers
// ============================================================================

#[allow(dead_code)]
pub fn emit_multisig_config_updated(e: &Env, event: MultisigConfigUpdatedEvent) {
    event.publish(e);
}

// ============================================================================
// Guardian & Recovery Emitter Helpers
// ============================================================================

#[allow(dead_code)]
pub fn emit_guardian_added(e: &Env, event: GuardianAddedEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_guardian_removed(e: &Env, event: GuardianRemovedEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_guardian_threshold_updated(e: &Env, event: GuardianThresholdUpdatedEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_recovery_started(e: &Env, event: RecoveryStartedEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_recovery_approved(e: &Env, event: RecoveryApprovedEvent) {
    event.publish(e);
}

#[allow(dead_code)]
pub fn emit_recovery_executed(e: &Env, event: RecoveryExecutedEvent) {
    event.publish(e);
}

// ============================================================================
// Treasury Emitter Helpers
// ============================================================================

pub fn emit_treasury_set(e: &Env, event: TreasurySetEvent) {
    event.publish(e);
}

pub fn emit_reserves_claimed(e: &Env, event: ReservesClaimedEvent) {
    event.publish(e);
}

pub fn emit_fee_config_updated(e: &Env, event: FeeConfigUpdatedEvent) {
    event.publish(e);
}

pub fn emit_liquidation_fee_collected(e: &Env, event: LiquidationFeeCollectedEvent) {
    event.publish(e);
}

/// Combo flash-loan + liquidation execution (Issue #661).
#[contractevent]
#[derive(Clone, Debug)]
pub struct FlashLoanLiquidationComboEvent {
    #[topic]
    pub liquidator: Address,
    #[topic]
    pub borrower: Address,
    pub debt_asset: Address,
    pub debt_amount: i128,
    pub collateral_seized: i128,
    pub flash_fee: i128,
    pub profit: i128,
    pub timestamp: u64,
}

pub fn emit_flash_loan_liquidation_combo(e: &Env, event: FlashLoanLiquidationComboEvent) {
    event.publish(e);
}

// ============================================================================
// Credit Scoring Events (from origin/main)
// ============================================================================



// ============================================================================
// Timelock Events (from origin/main)
// ============================================================================







// ============================================================================
// Circuit Breaker Events (from origin/main)
// ============================================================================









// ============================================================================
// Liquidation Queue Events (from HEAD)
// ============================================================================









// ============================================================================
// Emergency Withdrawal Events
// ============================================================================







pub fn emit_emergency_triggered(e: &Env, state: crate::types::EmergencyState) {
    EmergencyTriggeredEvent {
        trigger: state.trigger,
        started_at: state.started_at,
        window_opens_at: state.window_opens_at,
        window_closes_at: state.window_closes_at,
        timestamp: e.ledger().timestamp(),
    }
    .publish(e);
}

pub fn emit_emergency_cancelled(e: &Env) {
    EmergencyCancelledEvent {
        timestamp: e.ledger().timestamp(),
    }
    .publish(e);
}

pub fn emit_emergency_withdrawal(e: &Env, withdrawal: crate::types::EmergencyWithdrawal) {
    EmergencyWithdrawalEvent {
        user: withdrawal.user,
        asset: withdrawal.asset,
        amount: withdrawal.amount,
        loss_share_bps: withdrawal.loss_share_bps,
        timestamp: withdrawal.withdrawn_at,
    }
    .publish(e);
}

pub fn emit_batch_liquidation(e: &Env, event: BatchLiquidationEvent) {
    event.publish(e);
}
