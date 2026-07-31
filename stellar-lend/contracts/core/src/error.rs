use core::fmt;

use soroban_sdk::contracterror;

/// Stable protocol-wide error codebook for shared contract logic.
///
/// The numeric values are intentionally stable so that clients can parse and
/// branch on errors without depending on string messages.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ProtocolError {
    Unauthorized = 1000,
    AlreadyInitialized = 1001,
    NotInitialized = 1002,
    InvalidAddress = 1003,
    InvalidAmount = 1004,
    InvalidConfiguration = 1005,
    InvalidState = 1006,
    InsufficientBalance = 1007,
    InsufficientCollateral = 1008,
    LimitExceeded = 1009,
    ReentrancyDetected = 1010,
    CrossContractFailure = 1011,
    SerializationFailure = 1012,
    Overflow = 1013,
    Underflow = 1014,
    UnsupportedOperation = 1015,
    NotFound = 1016,
    DeadlineExpired = 1017,
    DuplicateRequest = 1018,
    Unknown = 1999,
}

impl ProtocolError {
    pub const fn code(self) -> u32 {
        self as u32
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::Unauthorized => "Unauthorized action",
            Self::AlreadyInitialized => "Contract has already been initialized",
            Self::NotInitialized => "Contract state is not initialized",
            Self::InvalidAddress => "Address is invalid or missing",
            Self::InvalidAmount => "Amount is invalid",
            Self::InvalidConfiguration => "Configuration value is invalid",
            Self::InvalidState => "Operation is not valid for the current state",
            Self::InsufficientBalance => "Insufficient balance",
            Self::InsufficientCollateral => "Insufficient collateral",
            Self::LimitExceeded => "Requested value exceeds an allowed limit",
            Self::ReentrancyDetected => "Reentrancy was detected and blocked",
            Self::CrossContractFailure => "Cross-contract invocation failed",
            Self::SerializationFailure => "Failed to serialize or deserialize contract data",
            Self::Overflow => "Arithmetic overflow occurred",
            Self::Underflow => "Arithmetic underflow occurred",
            Self::UnsupportedOperation => "Operation is not supported",
            Self::NotFound => "Requested item was not found",
            Self::DeadlineExpired => "Deadline has expired",
            Self::DuplicateRequest => "Duplicate request was rejected",
            Self::Unknown => "An unknown contract error occurred",
        }
    }

    pub const fn handling_hint(self) -> &'static str {
        match self {
            Self::Unauthorized => "Check the signer, auth flow, or role assignment.",
            Self::AlreadyInitialized => "Skip initialization or treat the call as idempotent.",
            Self::NotInitialized => "Initialize the contract before retrying.",
            Self::InvalidAddress => "Validate addresses before submitting the transaction.",
            Self::InvalidAmount => "Clamp values to a positive non-zero range.",
            Self::InvalidConfiguration => {
                "Refresh configuration data or fix the submitted settings."
            }
            Self::InvalidState => "Reload state and retry only after the required lifecycle step.",
            Self::InsufficientBalance => "Surface the balance shortfall to the user.",
            Self::InsufficientCollateral => "Ask the user to add collateral or reduce exposure.",
            Self::LimitExceeded => "Reduce the requested amount or raise the configured limit.",
            Self::ReentrancyDetected => "Retry only after the original call completes.",
            Self::CrossContractFailure => {
                "Inspect the downstream contract and propagate the wrapped code."
            }
            Self::SerializationFailure => {
                "Treat as a client or state integrity issue and fail safely."
            }
            Self::Overflow => "Use smaller values or safer arithmetic before retrying.",
            Self::Underflow => "Use smaller deductions or validate the input range first.",
            Self::UnsupportedOperation => "Route the request to a supported contract entrypoint.",
            Self::NotFound => "Re-read the entity identifier and refresh local cache state.",
            Self::DeadlineExpired => "Submit a fresh transaction with a new deadline.",
            Self::DuplicateRequest => "Deduplicate the request before retrying.",
            Self::Unknown => "Log the numeric code and fail closed without panicking.",
        }
    }

    pub fn from_code(code: u32) -> Self {
        match code {
            1000 => Self::Unauthorized,
            1001 => Self::AlreadyInitialized,
            1002 => Self::NotInitialized,
            1003 => Self::InvalidAddress,
            1004 => Self::InvalidAmount,
            1005 => Self::InvalidConfiguration,
            1006 => Self::InvalidState,
            1007 => Self::InsufficientBalance,
            1008 => Self::InsufficientCollateral,
            1009 => Self::LimitExceeded,
            1010 => Self::ReentrancyDetected,
            1011 => Self::CrossContractFailure,
            1012 => Self::SerializationFailure,
            1013 => Self::Overflow,
            1014 => Self::Underflow,
            1015 => Self::UnsupportedOperation,
            1016 => Self::NotFound,
            1017 => Self::DeadlineExpired,
            1018 => Self::DuplicateRequest,
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

/// Internal validation failures that map to the public protocol error codebook.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    InvalidAddress,
    InvalidAmount,
    InvalidConfiguration,
    InvalidState,
    LimitExceeded,
    NotFound,
}

impl From<ValidationError> for ProtocolError {
    fn from(value: ValidationError) -> Self {
        match value {
            ValidationError::InvalidAddress => Self::InvalidAddress,
            ValidationError::InvalidAmount => Self::InvalidAmount,
            ValidationError::InvalidConfiguration => Self::InvalidConfiguration,
            ValidationError::InvalidState => Self::InvalidState,
            ValidationError::LimitExceeded => Self::LimitExceeded,
            ValidationError::NotFound => Self::NotFound,
        }
    }
}

/// Internal lifecycle failures that map to the public protocol error codebook.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum InitializationError {
    AlreadyInitialized,
    NotInitialized,
    InvalidConfiguration,
}

impl From<InitializationError> for ProtocolError {
    fn from(value: InitializationError) -> Self {
        match value {
            InitializationError::AlreadyInitialized => Self::AlreadyInitialized,
            InitializationError::NotInitialized => Self::NotInitialized,
            InitializationError::InvalidConfiguration => Self::InvalidConfiguration,
        }
    }
}

/// Internal storage and runtime failures that should never panic at the edge.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StorageError {
    SerializationFailure,
    NotInitialized,
    NotFound,
}

impl From<StorageError> for ProtocolError {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::SerializationFailure => Self::SerializationFailure,
            StorageError::NotInitialized => Self::NotInitialized,
            StorageError::NotFound => Self::NotFound,
        }
    }
}

/// Internal cross-contract failures that can be mapped into stable public codes.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CrossContractError {
    Failed,
    UnsupportedOperation,
    DeadlineExpired,
    DuplicateRequest,
}

impl From<CrossContractError> for ProtocolError {
    fn from(value: CrossContractError) -> Self {
        match value {
            CrossContractError::Failed => Self::CrossContractFailure,
            CrossContractError::UnsupportedOperation => Self::UnsupportedOperation,
            CrossContractError::DeadlineExpired => Self::DeadlineExpired,
            CrossContractError::DuplicateRequest => Self::DuplicateRequest,
        }
    }
}

/// Convenience alias for the most common shared error type.
pub type ErrorKind = ProtocolError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_error_codes_are_stable() {
        assert_eq!(ProtocolError::Unauthorized.code(), 1000);
        assert_eq!(ProtocolError::AlreadyInitialized.code(), 1001);
        assert_eq!(ProtocolError::Unknown.code(), 1999);
    }

    #[test]
    fn protocol_error_messages_are_human_readable() {
        assert_eq!(
            ProtocolError::InsufficientCollateral.message(),
            "Insufficient collateral"
        );
        assert_eq!(
            ProtocolError::DeadlineExpired.message(),
            "Deadline has expired"
        );
    }

    #[test]
    fn internal_errors_map_into_protocol_errors() {
        let validation: ProtocolError = ValidationError::LimitExceeded.into();
        let init: ProtocolError = InitializationError::AlreadyInitialized.into();
        let storage: ProtocolError = StorageError::SerializationFailure.into();
        let cross: ProtocolError = CrossContractError::DuplicateRequest.into();

        assert_eq!(validation, ProtocolError::LimitExceeded);
        assert_eq!(init, ProtocolError::AlreadyInitialized);
        assert_eq!(storage, ProtocolError::SerializationFailure);
        assert_eq!(cross, ProtocolError::DuplicateRequest);
    }

    #[test]
    fn unknown_codes_fail_closed() {
        assert_eq!(ProtocolError::from_code(42), ProtocolError::Unknown);
    }
}
