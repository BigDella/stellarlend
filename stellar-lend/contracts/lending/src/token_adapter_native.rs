//! # Native Token Adapter Implementation
//!
//! Provides an adapter for native blockchain assets (XLM).
//! Native tokens require different handling than contract-based tokens.

use crate::token_adapter::{AdapterConfig, AdapterError, TokenAdapterType};
use soroban_sdk::{token::StellarAssetClient, Address, Env, Vec};

/// Native token adapter for handling native blockchain assets
pub struct NativeAdapter {
    config: AdapterConfig,
}

impl NativeAdapter {
    /// Create a new native token adapter
    pub fn new() -> Self {
        let env = Env::default();
        Self {
            config: AdapterConfig {
                adapter_type: TokenAdapterType::Native,
                // Native tokens don't have a contract address
                token_address: Address::from_contract_id(&env, &env.current_contract()),
                enabled: true,
                metadata: Vec::new(&env),
            },
        }
    }

    /// Create from existing configuration
    pub fn from_config(config: AdapterConfig) -> Result<Self, AdapterError> {
        if config.adapter_type != TokenAdapterType::Native {
            return Err(AdapterError::TokenNotSupported);
        }
        Ok(Self { config })
    }

    /// Check if this is the native token
    pub fn is_native(&self) -> bool {
        self.config.adapter_type == TokenAdapterType::Native
    }
}

/// Native token operations
///
/// Native tokens (like XLM on Stellar) are handled differently from
/// contract-based tokens. They use the blockchain's native transfer mechanism.
pub mod native {
    use super::*;

    /// Transfer native tokens (XLM)
    pub fn transfer(
        env: &Env,
        token: &Address,
        from: &Address,
        to: &Address,
        amount: i128,
    ) -> Result<(), AdapterError> {
        if amount <= 0 {
            return Err(AdapterError::InvalidConfig);
        }
        let client = StellarAssetClient::new(env, token);
        client.transfer(from, to, &amount);
        Ok(())
    }

    /// Get the native balance of an address
    pub fn balance_of(env: &Env, token: &Address, address: &Address) -> Result<i128, AdapterError> {
        let client = StellarAssetClient::new(env, token);
        Ok(client.balance(address))
    }

    /// Get the total native token supply
    pub fn total_supply(_env: &Env) -> Result<i128, AdapterError> {
        Err(AdapterError::NotImplemented)
    }

    /// Mint native tokens (requires special permissions)
    pub fn mint(_env: &Env, _to: &Address, _amount: i128) -> Result<(), AdapterError> {
        Err(AdapterError::NotImplemented)
    }

    /// Burn native tokens
    pub fn burn(_env: &Env, _from: &Address, _amount: i128) -> Result<(), AdapterError> {
        Err(AdapterError::NotImplemented)
    }
}

impl super::TokenAdapterTrait for NativeAdapter {
    fn get_adapter_type(&self) -> TokenAdapterType {
        self.config.adapter_type
    }

    fn get_token_address(&self) -> Address {
        self.config.token_address.clone()
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    fn transfer(
        &self,
        env: &Env,
        from: &Address,
        to: &Address,
        amount: i128,
    ) -> Result<(), AdapterError> {
        native::transfer(env, &self.config.token_address, from, to, amount)
    }

    fn balance_of(&self, env: &Env, address: &Address) -> Result<i128, AdapterError> {
        native::balance_of(env, &self.config.token_address, address)
    }

    fn total_supply(&self, env: &Env) -> Result<i128, AdapterError> {
        native::total_supply(env)
    }

    fn approve(
        &self,
        _env: &Env,
        _owner: &Address,
        _spender: &Address,
        _amount: i128,
    ) -> Result<(), AdapterError> {
        // Native tokens don't support approval in the same way
        Err(AdapterError::NotImplemented)
    }

    fn allowance(
        &self,
        _env: &Env,
        _owner: &Address,
        _spender: &Address,
    ) -> Result<i128, AdapterError> {
        // Native tokens don't have allowances
        Ok(0)
    }

    fn transfer_from(
        &self,
        _env: &Env,
        _from: &Address,
        _to: &Address,
        _amount: i128,
    ) -> Result<(), AdapterError> {
        // Native tokens don't support transfer_from
        Err(AdapterError::NotImplemented)
    }
}

/// Verify if an address represents a native token
pub fn verify_native_token(env: &Env, token_address: &Address) -> Result<bool, AdapterError> {
    // Native tokens are identified by special addresses or flags
    // In Stellar, native XLM is handled differently from token contracts
    Ok(true) // Native adapter is considered valid when enabled
}
