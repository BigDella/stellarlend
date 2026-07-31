//! # Token Adapter Interface
//!
//! Provides standardized adapters for different token implementations,
//! enabling support for various token standards without core logic changes.
//!
//! ## Adapter Types
//! - **ERC20Adapter**: For standard ERC-20 compatible tokens
//! - **NativeAdapter**: For native blockchain assets (XLM)
//! - **WrappedAdapter**: For wrapped assets (e.g., Stellar's wrapped XLM)

use soroban_sdk::{Address, Env, Vec};

/// Errors that can occur during adapter operations
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AdapterError {
    /// Adapter operation failed
    AdapterFailed = 1,
    /// Token is not supported by this adapter
    TokenNotSupported = 2,
    /// Adapter verification failed
    VerificationFailed = 3,
    /// Invalid adapter configuration
    InvalidConfig = 4,
    /// Adapter operation not implemented
    NotImplemented = 5,
}

/// Token adapter types supported by the protocol
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TokenAdapterType {
    /// Standard ERC-20 compatible token
    ERC20 = 0,
    /// Native blockchain asset (XLM)
    Native = 1,
    /// Wrapped token (e.g., wxlm)
    Wrapped = 2,
    /// Unknown or custom token type
    Unknown = 3,
}

/// Adapter configuration for token interactions
#[contracttype]
#[derive(Clone)]
pub struct AdapterConfig {
    /// Type of adapter
    pub adapter_type: TokenAdapterType,
    /// Token contract address
    pub token_address: Address,
    /// Whether adapter is enabled
    pub enabled: bool,
    /// Adapter-specific metadata
    pub metadata: Vec<u8>,
}

/// Result of adapter verification
#[contracttype]
#[derive(Clone)]
pub struct AdapterVerificationResult {
    /// Whether the adapter is valid
    pub is_valid: bool,
    /// Adapter type detected
    pub adapter_type: TokenAdapterType,
    /// Token standard version
    pub token_standard: u32,
    /// Additional verification data
    pub verification_data: Vec<u8>,
}

#[contracttype]
#[derive(Clone)]
pub enum AdapterDataKey {
    Adapter(Address),
    AdapterList,
}

/// TokenAdapterTrait - Core trait for all token adapters
///
/// This trait defines the standard interface that all token adapters
/// must implement to be compatible with the lending protocol.
pub trait TokenAdapterTrait {
    /// Get the adapter type
    fn get_adapter_type(&self) -> TokenAdapterType;

    /// Get the token address this adapter handles
    fn get_token_address(&self) -> Address;

    /// Check if the adapter is enabled
    fn is_enabled(&self) -> bool;

    /// Transfer tokens from one address to another
    fn transfer(
        &self,
        env: &Env,
        from: &Address,
        to: &Address,
        amount: i128,
    ) -> Result<(), AdapterError>;

    /// Get the balance of an address
    fn balance_of(&self, env: &Env, address: &Address) -> Result<i128, AdapterError>;

    /// Get the total supply of the token
    fn total_supply(&self, env: &Env) -> Result<i128, AdapterError>;

    /// Approve spender to transfer tokens from the owner
    fn approve(
        &self,
        env: &Env,
        owner: &Address,
        spender: &Address,
        amount: i128,
    ) -> Result<(), AdapterError>;

    /// Get approved allowance
    fn allowance(
        &self,
        env: &Env,
        owner: &Address,
        spender: &Address,
    ) -> Result<i128, AdapterError>;

    /// Transfer tokens from one address to another (with allowance)
    fn transfer_from(
        &self,
        env: &Env,
        from: &Address,
        to: &Address,
        amount: i128,
    ) -> Result<(), AdapterError>;
}

/// Factory for creating token adapters
pub mod factory {
    use super::*;

    /// Create an adapter based on token type detection
    pub fn create_adapter(
        env: &Env,
        token_address: Address,
    ) -> Result<AdapterConfig, AdapterError> {
        // Detect token type and create appropriate adapter
        let adapter_type = detect_token_type(env, &token_address)?;

        Ok(AdapterConfig {
            adapter_type,
            token_address,
            enabled: true,
            metadata: Vec::new(env),
        })
    }

    /// Detect the type of token at the given address
    fn detect_token_type(
        _env: &Env,
        _token_address: &Address,
    ) -> Result<TokenAdapterType, AdapterError> {
        // Token type detection logic
        // In practice, this would query the token contract to determine its type
        // For now, we default to ERC20 as the most common type
        Ok(TokenAdapterType::ERC20)
    }

    /// Register a new adapter with the protocol
    pub fn register_adapter(env: &Env, config: AdapterConfig) -> Result<(), AdapterError> {
        if !config.enabled {
            return Err(AdapterError::InvalidConfig);
        }
        crate::token_adapter_verify::register_adapter(env, config)
    }
}
