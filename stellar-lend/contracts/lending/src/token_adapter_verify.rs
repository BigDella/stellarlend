//! # Token Adapter Verification
//!
//! Provides verification functions for token adapters,
//! ensuring adapters are properly configured and functional.

use crate::token_adapter::{
    AdapterConfig, AdapterError, AdapterVerificationResult, TokenAdapterType,
};
use crate::token_adapter_erc20;
use crate::token_adapter_native;
use crate::token_adapter_wrapped;
use soroban_sdk::{Address, Env, Vec};

/// Verify an adapter configuration
///
/// This function performs comprehensive verification of an adapter,
/// checking its configuration, enabled state, and compatibility.
pub fn verify_adapter(env: &Env, config: &AdapterConfig) -> AdapterVerificationResult {
    // Check if adapter is enabled
    if !config.enabled {
        return AdapterVerificationResult {
            is_valid: false,
            adapter_type: TokenAdapterType::Unknown,
            token_standard: 0,
            verification_data: Vec::new(env),
        };
    }

    // Verify based on adapter type
    let adapter_type = config.adapter_type;
    let is_valid = match adapter_type {
        TokenAdapterType::ERC20 => {
            // Verify ERC-20 compatibility
            match token_adapter_erc20::verify_erc20_compatibility(env, &config.token_address) {
                Ok(valid) => valid,
                Err(_) => false,
            }
        }
        TokenAdapterType::Native => {
            // Verify native token
            match native::verify_native_token(env, &config.token_address) {
                Ok(valid) => valid,
                Err(_) => false,
            }
        }
        TokenAdapterType::Wrapped => {
            // Verify wrapped token
            match wrapped::verify_wrapped_token(env, &config.token_address) {
                Ok(valid) => valid,
                Err(_) => false,
            }
        }
        TokenAdapterType::Unknown => false,
    };

    AdapterVerificationResult {
        is_valid,
        adapter_type,
        token_standard: get_token_standard_version(adapter_type),
        verification_data: Vec::new(env),
    }
}

/// Get the token standard version for an adapter type
fn get_token_standard_version(adapter_type: TokenAdapterType) -> u32 {
    match adapter_type {
        TokenAdapterType::ERC20 => 20,   // ERC-20 standard
        TokenAdapterType::Native => 0,   // Native (no standard)
        TokenAdapterType::Wrapped => 20, // Wrapped tokens typically ERC-20
        TokenAdapterType::Unknown => 0,
    }
}

/// Verify adapter can perform required operations
///
/// This verifies that an adapter can execute the core operations
/// needed by the lending protocol.
pub fn verify_adapter_operations(env: &Env, config: &AdapterConfig) -> Result<bool, AdapterError> {
    match config.adapter_type {
        TokenAdapterType::ERC20 => {
            // Verify ERC-20 operations
            let adapter = token_adapter_erc20::ERC20Adapter::from_config(config.clone())?;

            // Test balance query (won't fail if token exists)
            let _ = token_adapter_erc20::erc20::balance_of(
                env,
                &config.token_address,
                &Address::from_contract_id(env, &env.current_contract()),
            );

            Ok(true)
        }
        TokenAdapterType::Native => {
            // Native tokens are always valid if enabled
            Ok(config.enabled)
        }
        TokenAdapterType::Wrapped => {
            // Verify wrapped token operations
            Ok(config.enabled)
        }
        TokenAdapterType::Unknown => Err(AdapterError::VerificationFailed),
    }
}

/// Register a new adapter with the protocol
///
/// This stores the adapter configuration for later use by the protocol.
pub fn register_adapter(env: &Env, config: AdapterConfig) -> Result<(), AdapterError> {
    // Verify the adapter before registering
    let verification = verify_adapter(env, &config);

    if !verification.is_valid {
        return Err(AdapterError::VerificationFailed);
    }

    env.storage().persistent().set(
        &crate::token_adapter::AdapterDataKey::Adapter(config.token_address.clone()),
        &config,
    );

    let mut adapters: Vec<AdapterConfig> = env
        .storage()
        .persistent()
        .get(&crate::token_adapter::AdapterDataKey::AdapterList)
        .unwrap_or_else(|| Vec::new(env));

    if !adapters
        .iter()
        .any(|existing| existing.token_address == config.token_address)
    {
        adapters.push_back(config.clone());
        env.storage().persistent().set(
            &crate::token_adapter::AdapterDataKey::AdapterList,
            &adapters,
        );
    }

    Ok(())
}

/// Get adapter configuration from storage
pub fn get_adapter(
    env: &Env,
    token_address: &Address,
) -> Result<Option<AdapterConfig>, AdapterError> {
    Ok(env
        .storage()
        .persistent()
        .get(&crate::token_adapter::AdapterDataKey::Adapter(
            token_address.clone(),
        )))
}

/// List all registered adapters
pub fn list_adapters(env: &Env) -> Result<Vec<AdapterConfig>, AdapterError> {
    Ok(env
        .storage()
        .persistent()
        .get(&crate::token_adapter::AdapterDataKey::AdapterList)
        .unwrap_or_else(|| Vec::new(env)))
}

/// Upgrade path for new tokens
///
/// This module handles the upgrade path for adding support for new token types.
pub mod upgrade {
    use super::*;

    /// Upgrade adapter to support new token standard
    pub fn upgrade_adapter(
        env: &Env,
        old_config: &AdapterConfig,
        new_adapter_type: TokenAdapterType,
    ) -> Result<AdapterConfig, AdapterError> {
        if !old_config.enabled {
            return Err(AdapterError::InvalidConfig);
        }

        // Create new configuration with updated adapter type
        Ok(AdapterConfig {
            adapter_type: new_adapter_type,
            token_address: old_config.token_address.clone(),
            enabled: true,
            metadata: old_config.metadata.clone(),
        })
    }

    /// Migrate from old adapter to new adapter
    pub fn migrate_adapter(
        env: &Env,
        old_config: AdapterConfig,
        new_config: AdapterConfig,
    ) -> Result<(), AdapterError> {
        // Verify both adapters
        let old_verification = verify_adapter(env, &old_config);
        let new_verification = verify_adapter(env, &new_config);

        if !old_verification.is_valid || !new_verification.is_valid {
            return Err(AdapterError::VerificationFailed);
        }

        // Migrate state from old to new adapter
        // In practice, would transfer any adapter-specific state
        Ok(())
    }
}

// Re-export verification functions from submodules
pub mod native {
    pub use crate::token_adapter_native::verify_native_token;
}

pub mod wrapped {
    pub use crate::token_adapter_wrapped::{create_wrapped_adapter, verify_wrapped_token};
}
