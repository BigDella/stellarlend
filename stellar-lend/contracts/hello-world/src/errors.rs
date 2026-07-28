use soroban_sdk::contracterror;

use crate::admin::AdminError;
use crate::analytics::AnalyticsError;
use crate::borrow::BorrowError;
use crate::cross_asset::CrossAssetError;
use crate::debt_token::DebtTokenError;
use crate::deposit::DepositError;
use crate::flash_loan::FlashLoanError;
use crate::interest_rate::InterestRateError;
use crate::liquidate::LiquidationError;
use crate::mev_protection::MevProtectionError;
use crate::rate_limiter::RateLimitError;
use crate::rebalancing::RebalancingError;
use crate::repay::RepayError;
use crate::reserve::ReserveError;
use crate::risk_management::RiskManagementError;
use crate::risk_params::RiskParamsError;
use crate::treasury::TreasuryError;
use crate::withdraw::WithdrawError;
use crate::emergency_withdrawal::EmergencyWithdrawalError;

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GovernanceError {
    ProposalNotFound = 100,
    ProposalNotActive = 101,
    NotInVotingPeriod = 102,
    AlreadyVoted = 103,
    NoVotingPower = 104,
    InsufficientProposalPower = 105,
    VotingNotEnded = 106,
    InvalidProposalStatus = 107,
    ProposalExpired = 108,
    NotQueued = 109,
    InvalidExecutionTime = 110,
    ExecutionTooEarly = 111,
    AlreadyExecuted = 112,
    InvalidQuorum = 113,
    InvalidVotingPeriod = 114,
    CannotExecute = 115,
    QuorumNotMet = 116,
    ProposalDefeated = 117,
    InvalidAction = 118,
    ThresholdNotMet = 119,
    ProposalAlreadyFailed = 120,
    ProposalNotReady = 121,
    ExecutionFailed = 122,
    InvalidMultisigConfig = 123,
    InsufficientApprovals = 124,
    InvalidProposalType = 125,
    GuardianAlreadyExists = 126,
    GuardianNotFound = 127,
    InvalidGuardianConfig = 128,
    RecoveryInProgress = 129,
    NoRecoveryInProgress = 130,
    Unauthorized = 131,
    AlreadyInitialized = 132,
    NotInitialized = 133,
    InvalidProposal = 134,
    InputTooLong = 135,
    SelfDelegation = 136,
    VotesLocked = 137,
    AlreadyDelegated = 138,
    DelegationDepthExceeded = 139,
    ProposalRateLimitExceeded = 140,
    // Timelock errors
    TimelockNotFound = 141,
    TimelockNotReady = 142,
    TimelockExpired = 143,
    InvalidTimelockStatus = 144,
    InvalidTimelockConfig = 145,
    InvalidTimelockDelay = 146,
    /// #675 — recovery approved but the mandatory delay/cancellation window hasn't elapsed
    RecoveryNotReady = 147,
    /// #674 — per-action-type timelock delay configuration is invalid
    InvalidActionTypeDelay = 148,
    /// #674 — guardian emergency override already approved by this guardian
    EmergencyOverrideAlreadyApproved = 149,
    /// #674 — not enough guardian approvals yet for an emergency override
    InsufficientEmergencyApprovals = 150,
}

/// Unified public contract error type for the lending interface.
///
/// Internal module error enums keep their existing numeric values. Public entrypoints
/// convert them into this compact, stable interface codebook.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum LendingError {
    /// Caller is not authorized to perform the requested action.
    Unauthorized = 1,
    /// Amount input is zero, negative, or otherwise invalid.
    InvalidAmount = 2,
    /// Asset reference is missing, malformed, or unsupported for the operation.
    InvalidAsset = 3,
    /// Generic invalid parameter/configuration input.
    InvalidParameter = 4,
    /// User or contract balance is too low to complete the operation.
    InsufficientBalance = 5,
    /// User collateral is too low for the requested action.
    InsufficientCollateral = 6,
    /// Action would violate the required collateral ratio.
    InsufficientCollateralRatio = 7,
    /// Arithmetic overflow or underflow occurred.
    Overflow = 8,
    /// Protocol or operation-level pause is active.
    ProtocolPaused = 9,
    /// Reentrant execution was detected and blocked.
    Reentrancy = 10,
    /// Required state/config has not been initialized.
    NotInitialized = 11,
    /// Initialization was attempted more than once.
    AlreadyInitialized = 12,
    /// Requested state was not found.
    DataNotFound = 13,
    /// Division by zero occurred during a calculation.
    DivisionByZero = 14,
    /// Repayment was attempted with no outstanding debt.
    NoDebt = 15,
    /// Asset exists but is disabled for the requested action.
    AssetNotEnabled = 16,
    /// Request exceeded a protocol-enforced limit or bound.
    LimitExceeded = 17,
    /// Requested action is invalid for the current protocol state.
    InvalidState = 18,
    /// Required oracle or pricing information is unavailable.
    PriceUnavailable = 19,
    /// Contract liquidity is too low for the requested flash loan.
    InsufficientLiquidity = 20,
    /// Flash loan callback address is invalid.
    InvalidCallback = 21,
    /// Flash loan callback execution failed.
    CallbackFailed = 22,
    /// Flash loan was not fully repaid within the required flow.
    NotRepaid = 23,
    /// Treasury address has not been configured.
    TreasuryNotSet = 24,
    /// Requested reserve withdrawal exceeds available reserves.
    InsufficientReserve = 25,
    /// Fee configuration value is outside the allowed range.
    InvalidFee = 26,
    /// Action requires governance flow rather than direct execution.
    GovernanceRequired = 27,
    /// Generic governance failure surfaced through the public interface.
    GovernanceError = 28,
    /// Commit/reveal confirmation is required before the sensitive action can execute.
    CommitRequired = 29,
    /// Requested commit record does not exist.
    CommitNotFound = 30,
    /// Commit exists but cannot be revealed yet.
    CommitNotReady = 31,
    /// Commit expired before reveal.
    CommitExpired = 32,
    /// Protected execution would exceed the user's declared fee cap.
    FeeCapExceeded = 33,
    /// Requested resource or entity was not found.
    NotFound = 34,
    /// Entity already exists.
    AlreadyExists = 35,
}

macro_rules! impl_from_error {
    ($source:ty, { $($from:path => $to:path,)+ }) => {
        impl From<$source> for LendingError {
            fn from(error: $source) -> Self {
                match error {
                    $($from => $to,)+
                }
            }
        }
    };
}

impl_from_error!(AdminError, {
    AdminError::Unauthorized => LendingError::Unauthorized,
    AdminError::InvalidParameter => LendingError::InvalidParameter,
    AdminError::AdminAlreadySet => LendingError::AlreadyInitialized,
});

impl_from_error!(AnalyticsError, {
    AnalyticsError::NotInitialized => LendingError::NotInitialized,
    AnalyticsError::InvalidParameter => LendingError::InvalidParameter,
    AnalyticsError::Overflow => LendingError::Overflow,
    AnalyticsError::DataNotFound => LendingError::DataNotFound,
});

impl_from_error!(BorrowError, {
    BorrowError::InvalidAmount => LendingError::InvalidAmount,
    BorrowError::InvalidAsset => LendingError::InvalidAsset,
    BorrowError::InsufficientCollateral => LendingError::InsufficientCollateral,
    BorrowError::BorrowPaused => LendingError::ProtocolPaused,
    BorrowError::InsufficientCollateralRatio => LendingError::InsufficientCollateralRatio,
    BorrowError::Overflow => LendingError::Overflow,
    BorrowError::Reentrancy => LendingError::Reentrancy,
    BorrowError::MaxBorrowExceeded => LendingError::LimitExceeded,
    BorrowError::AssetNotEnabled => LendingError::AssetNotEnabled,
});

impl_from_error!(DepositError, {
    DepositError::InvalidAmount => LendingError::InvalidAmount,
    DepositError::InvalidAsset => LendingError::InvalidAsset,
    DepositError::InsufficientBalance => LendingError::InsufficientBalance,
    DepositError::DepositPaused => LendingError::ProtocolPaused,
    DepositError::AssetNotEnabled => LendingError::AssetNotEnabled,
    DepositError::Overflow => LendingError::Overflow,
    DepositError::Reentrancy => LendingError::Reentrancy,
    DepositError::Unauthorized => LendingError::Unauthorized,
});

impl_from_error!(FlashLoanError, {
    FlashLoanError::InvalidAmount => LendingError::InvalidAmount,
    FlashLoanError::InvalidAsset => LendingError::InvalidAsset,
    FlashLoanError::InsufficientLiquidity => LendingError::InsufficientLiquidity,
    FlashLoanError::FlashLoanPaused => LendingError::ProtocolPaused,
    FlashLoanError::NotRepaid => LendingError::NotRepaid,
    FlashLoanError::InsufficientRepayment => LendingError::InsufficientBalance,
    FlashLoanError::Overflow => LendingError::Overflow,
    FlashLoanError::Reentrancy => LendingError::Reentrancy,
    FlashLoanError::InvalidCallback => LendingError::InvalidCallback,
    FlashLoanError::CallbackFailed => LendingError::CallbackFailed,
});

impl_from_error!(ReserveError, {
    ReserveError::Unauthorized => LendingError::Unauthorized,
    ReserveError::InvalidReserveFactor => LendingError::InvalidParameter,
    ReserveError::InsufficientReserve => LendingError::InsufficientReserve,
    ReserveError::InvalidAsset => LendingError::InvalidAsset,
    ReserveError::InvalidTreasury => LendingError::InvalidParameter,
    ReserveError::InvalidAmount => LendingError::InvalidAmount,
    ReserveError::Overflow => LendingError::Overflow,
    ReserveError::TreasuryNotSet => LendingError::TreasuryNotSet,
});

impl From<GovernanceError> for LendingError {
    fn from(error: GovernanceError) -> Self {
        match error {
            GovernanceError::Unauthorized => LendingError::Unauthorized,
            GovernanceError::AlreadyInitialized => LendingError::AlreadyInitialized,
            GovernanceError::NotInitialized => LendingError::NotInitialized,
            GovernanceError::InvalidQuorum => LendingError::InvalidParameter,
            GovernanceError::InvalidVotingPeriod => LendingError::InvalidParameter,
            _ => LendingError::GovernanceError,
        }
    }
}

impl_from_error!(InterestRateError, {
    InterestRateError::Unauthorized => LendingError::Unauthorized,
    InterestRateError::InvalidParameter => LendingError::InvalidParameter,
    InterestRateError::ParameterChangeTooLarge => LendingError::LimitExceeded,
    InterestRateError::Overflow => LendingError::Overflow,
    InterestRateError::DivisionByZero => LendingError::DivisionByZero,
    InterestRateError::AlreadyInitialized => LendingError::AlreadyInitialized,
});

impl_from_error!(LiquidationError, {
    LiquidationError::InvalidAmount => LendingError::InvalidAmount,
    LiquidationError::InvalidAsset => LendingError::InvalidAsset,
    LiquidationError::NotLiquidatable => LendingError::InvalidState,
    LiquidationError::LiquidationPaused => LendingError::ProtocolPaused,
    LiquidationError::ExceedsCloseFactor => LendingError::LimitExceeded,
    LiquidationError::InsufficientBalance => LendingError::InsufficientBalance,
    LiquidationError::Overflow => LendingError::Overflow,
    LiquidationError::InvalidCollateralAsset => LendingError::InvalidAsset,
    LiquidationError::InvalidDebtAsset => LendingError::InvalidAsset,
    LiquidationError::PriceNotAvailable => LendingError::PriceUnavailable,
    LiquidationError::InsufficientLiquidation => LendingError::InvalidState,
    LiquidationError::Reentrancy => LendingError::Reentrancy,
});

impl_from_error!(RateLimitError, {
    RateLimitError::RateLimited => LendingError::LimitExceeded,
    RateLimitError::InvalidConfig => LendingError::InvalidParameter,
    RateLimitError::Unauthorized => LendingError::Unauthorized,
    RateLimitError::Overflow => LendingError::Overflow,
});

impl_from_error!(MevProtectionError, {
    MevProtectionError::InvalidConfig => LendingError::InvalidParameter,
    MevProtectionError::CommitNotFound => LendingError::CommitNotFound,
    MevProtectionError::CommitNotReady => LendingError::CommitNotReady,
    MevProtectionError::CommitExpired => LendingError::CommitExpired,
    MevProtectionError::Unauthorized => LendingError::Unauthorized,
    MevProtectionError::FeeCapExceeded => LendingError::FeeCapExceeded,
    MevProtectionError::InvalidAmount => LendingError::InvalidAmount,
    MevProtectionError::InvalidOperation => LendingError::InvalidState,
    MevProtectionError::DeadlineExpired => LendingError::CommitExpired,
    MevProtectionError::SlippageExceeded => LendingError::LimitExceeded,
    MevProtectionError::AuctionWindowOpen => LendingError::InvalidState,
    MevProtectionError::NoBidsInAuction => LendingError::DataNotFound,
});

impl_from_error!(RepayError, {
    RepayError::InvalidAmount => LendingError::InvalidAmount,
    RepayError::InvalidAsset => LendingError::InvalidAsset,
    RepayError::InsufficientBalance => LendingError::InsufficientBalance,
    RepayError::RepayPaused => LendingError::ProtocolPaused,
    RepayError::NoDebt => LendingError::NoDebt,
    RepayError::Overflow => LendingError::Overflow,
    RepayError::Reentrancy => LendingError::Reentrancy,
});

impl_from_error!(RiskManagementError, {
    RiskManagementError::Unauthorized => LendingError::Unauthorized,
    RiskManagementError::InvalidParameter => LendingError::InvalidParameter,
    RiskManagementError::ParameterChangeTooLarge => LendingError::LimitExceeded,
    RiskManagementError::InsufficientCollateralRatio => LendingError::InsufficientCollateralRatio,
    RiskManagementError::OperationPaused => LendingError::ProtocolPaused,
    RiskManagementError::EmergencyPaused => LendingError::ProtocolPaused,
    RiskManagementError::InvalidCollateralRatio => LendingError::InvalidParameter,
    RiskManagementError::InvalidLiquidationThreshold => LendingError::InvalidParameter,
    RiskManagementError::InvalidCloseFactor => LendingError::InvalidParameter,
    RiskManagementError::InvalidLiquidationIncentive => LendingError::InvalidParameter,
    RiskManagementError::Overflow => LendingError::Overflow,
    RiskManagementError::GovernanceRequired => LendingError::GovernanceRequired,
    RiskManagementError::AlreadyInitialized => LendingError::AlreadyInitialized,
});

impl_from_error!(RiskParamsError, {
    RiskParamsError::Unauthorized => LendingError::Unauthorized,
    RiskParamsError::InvalidParameter => LendingError::InvalidParameter,
    RiskParamsError::ParameterChangeTooLarge => LendingError::LimitExceeded,
    RiskParamsError::InvalidCollateralRatio => LendingError::InvalidParameter,
    RiskParamsError::InvalidLiquidationThreshold => LendingError::InvalidParameter,
    RiskParamsError::InvalidCloseFactor => LendingError::InvalidParameter,
    RiskParamsError::InvalidLiquidationIncentive => LendingError::InvalidParameter,
});

impl_from_error!(TreasuryError, {
    TreasuryError::Unauthorized => LendingError::Unauthorized,
    TreasuryError::InvalidAmount => LendingError::InvalidAmount,
    TreasuryError::InsufficientReserve => LendingError::InsufficientReserve,
    TreasuryError::Overflow => LendingError::Overflow,
    TreasuryError::TreasuryNotSet => LendingError::TreasuryNotSet,
    TreasuryError::InvalidFee => LendingError::InvalidFee,
});

impl_from_error!(WithdrawError, {
    WithdrawError::InvalidAmount => LendingError::InvalidAmount,
    WithdrawError::InvalidAsset => LendingError::InvalidAsset,
    WithdrawError::InsufficientCollateral => LendingError::InsufficientCollateral,
    WithdrawError::WithdrawPaused => LendingError::ProtocolPaused,
    WithdrawError::InsufficientCollateralRatio => LendingError::InsufficientCollateralRatio,
    WithdrawError::Overflow => LendingError::Overflow,
    WithdrawError::Reentrancy => LendingError::Reentrancy,
    WithdrawError::Undercollateralized => LendingError::InvalidState,
});

impl_from_error!(RebalancingError, {
    RebalancingError::Unauthorized => LendingError::Unauthorized,
    RebalancingError::InvalidConfig => LendingError::InvalidParameter,
    RebalancingError::AlreadyHealthy => LendingError::InvalidState,
    RebalancingError::GasCostTooHigh => LendingError::LimitExceeded,
    RebalancingError::SlippageTooHigh => LendingError::LimitExceeded,
    RebalancingError::SwapTooSmall => LendingError::InvalidAmount,
    RebalancingError::CooldownActive => LendingError::LimitExceeded,
    RebalancingError::Undercollateralized => LendingError::InsufficientCollateralRatio,
    RebalancingError::AmmFailed => LendingError::InvalidState,
    RebalancingError::InsufficientLiquidity => LendingError::InsufficientLiquidity,
    RebalancingError::Overflow => LendingError::Overflow,
});

impl_from_error!(DebtTokenError, {
    DebtTokenError::TokenNotFound => LendingError::DataNotFound,
    DebtTokenError::Unauthorized => LendingError::Unauthorized,
    DebtTokenError::TransferPaused => LendingError::ProtocolPaused,
    DebtTokenError::TransferBlocked => LendingError::Unauthorized,
    DebtTokenError::LiquidationInProgress => LendingError::InvalidState,
    DebtTokenError::InvalidTokenId => LendingError::InvalidParameter,
    DebtTokenError::Undercollateralized => LendingError::InsufficientCollateralRatio,
    DebtTokenError::Overflow => LendingError::Overflow,
    DebtTokenError::ZeroAddress => LendingError::InvalidParameter,
    DebtTokenError::AlreadyTokenized => LendingError::AlreadyExists,
    DebtTokenError::PositionNotFound => LendingError::DataNotFound,
});

impl From<CrossAssetError> for LendingError {
    fn from(error: CrossAssetError) -> Self {
        match error {
            CrossAssetError::AssetNotConfigured => LendingError::DataNotFound,
            CrossAssetError::AssetDisabled => LendingError::AssetNotEnabled,
            CrossAssetError::InsufficientCollateral => LendingError::InsufficientCollateral,
            CrossAssetError::ExceedsBorrowCapacity => LendingError::InsufficientCollateralRatio,
            CrossAssetError::UnhealthyPosition => LendingError::InsufficientCollateralRatio,
            CrossAssetError::SupplyCapExceeded => LendingError::LimitExceeded,
            CrossAssetError::BorrowCapExceeded => LendingError::LimitExceeded,
            CrossAssetError::InvalidPrice => LendingError::PriceUnavailable,
            CrossAssetError::PriceStale => LendingError::PriceUnavailable,
            CrossAssetError::NotAuthorized => LendingError::Unauthorized,
        }
    }
}

impl_from_error!(EmergencyWithdrawalError, {
    EmergencyWithdrawalError::NotActive => LendingError::InvalidState,
    EmergencyWithdrawalError::AlreadyActive => LendingError::AlreadyExists,
    EmergencyWithdrawalError::WindowNotOpen => LendingError::InvalidState,
    EmergencyWithdrawalError::NotAuthorized => LendingError::Unauthorized,
    EmergencyWithdrawalError::InsufficientBalance => LendingError::InsufficientBalance,
    EmergencyWithdrawalError::ExceedsWithdrawalCap => LendingError::LimitExceeded,
    EmergencyWithdrawalError::InvalidParameter => LendingError::InvalidParameter,
    EmergencyWithdrawalError::AlreadyWithdrawn => LendingError::AlreadyExists,
});
