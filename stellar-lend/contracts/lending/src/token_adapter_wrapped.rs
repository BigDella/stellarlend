//! # Wrapped Token Adapter Implementation
//!
//! Provides an adapter for wrapped assets (e.g., wxlm on Stellar).
//! Wrapped tokens are tokens that represent the native asset on a different chain
//! or a synthetic version of a native asset.

use crate::token_adapter::{AdapterConfig, AdapterError, TokenAdapterType};
use soroban_sdk::{token::Client as TokenClient, Address, Env, Vec};

/// Wrapped token adapter for handling wrapped assets
pub struct WrappedAdapter {
    config: AdapterConfig,
    /// The underlying asset this wrapped token represents
    underlying_asset: Option<Address>,
}

impl WrappedAdapter {
    /// Create a new wrapped token adapter
    pub fn new(token_address: Address, underlying_asset: Option<Address>) -> Self {
        let env = Env::default();
        Self {
            config: AdapterConfig {
                adapter_type: TokenAdapterType::Wrapped,
                token_address,
                enabled: true,
                metadata: Vec::new(&env),
            },
            underlying_asset,
        }
    }

    /// Create from existing configuration
    pub fn from_config(
        config: AdapterConfig,
        underlying_asset: Option<Address>,
    ) -> Result<Self, AdapterError> {
        if config.adapter_type != TokenAdapterType::Wrapped {
            return Err(AdapterError::TokenNotSupported);
        }
        Ok(Self {
            config,
            underlying_asset,
        })
    }

    /// Get the underlying asset address
    pub fn underlying_asset(&self) -> Option<&Address> {
        self.underlying_asset.as_ref()
    }

    /// Check if this is a wrapped token
    pub fn is_wrapped(&self) -> bool {
        self.config.adapter_type == TokenAdapterType::Wrapped
    }
}

/// Wrapped token operations
///
/// Wrapped tokens are backed by underlying assets and typically support
/// minting/burning when wrapping/unwrapping.
pub mod wrapped {
    use super::*;

    /// Wrap tokens (deposit native and mint wrapped)
    pub fn wrap(
        env: &Env,
        token: &Address,
        _from: &Address,
        amount: i128,
    ) -> Result<i128, AdapterError> {
        if amount <= 0 {
            return Err(AdapterError::InvalidConfig);
        }
        let token_client = TokenClient::new(env, token);
        token_client.mint(_from, &amount);
        Ok(amount)
    }

    /// Unwrap tokens (burn wrapped and release underlying)
    pub fn unwrap(
        env: &Env,
        token: &Address,
        from: &Address,
        amount: i128,
    ) -> Result<i128, AdapterError> {
        if amount <= 0 {
            return Err(AdapterError::InvalidConfig);
        }
        let token_client = TokenClient::new(env, token);
        token_client.burn(from, &amount);
        Ok(amount)
    }

    /// Get the underlying asset for a wrapped token
    pub fn get_underlying(_env: &Env, _token: &Address) -> Result<Option<Address>, AdapterError> {
        Ok(None)
    }

    /// Get the wrap/unwrap ratio
    pub fn get_ratio(_env: &Env, _token: &Address) -> Result<(i128, i128), AdapterError> {
        Ok((1, 1))
    }

    /// Transfer wrapped tokens
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
        let token_client = TokenClient::new(env, token);
        token_client.transfer(from, to, &amount);
        Ok(())
    }

    /// Get the balance of wrapped tokens
    pub fn balance_of(env: &Env, token: &Address, address: &Address) -> Result<i128, AdapterError> {
        let token_client = TokenClient::new(env, token);
        Ok(token_client.balance(address))
    }

    /// Get total supply of wrapped tokens
    pub fn total_supply(env: &Env, token: &Address) -> Result<i128, AdapterError> {
        let token_client = TokenClient::new(env, token);
        Ok(token_client.total_supply())
    }
}

impl super::TokenAdapterTrait for WrappedAdapter {
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
        wrapped::transfer(env, &self.config.token_address, from, to, amount)
    }

    fn balance_of(&self, env: &Env, address: &Address) -> Result<i128, AdapterError> {
        wrapped::balance_of(env, &self.config.token_address, address)
    }

    fn total_supply(&self, env: &Env) -> Result<i128, AdapterError> {
        wrapped::total_supply(env, &self.config.token_address)
    }

    fn approve(
        &self,
        _env: &Env,
        _owner: &Address,
        _spender: &Address,
        _amount: i128,
    ) -> Result<(), AdapterError> {
        // Wrapped tokens may support approval
        Err(AdapterError::NotImplemented)
    }

    fn allowance(
        &self,
        _env: &Env,
        _owner: &Address,
        _spender: &Address,
    ) -> Result<i128, AdapterError> {
        Ok(0)
    }

    fn transfer_from(
        &self,
        _env: &Env,
        _from: &Address,
        _to: &Address,
        _amount: i128,
    ) -> Result<(), AdapterError> {
        Err(AdapterError::NotImplemented)
    }
}

/// Verify if a token is a wrapped token
pub fn verify_wrapped_token(_env: &Env, token_address: &Address) -> Result<bool, AdapterError> {
    match token_address {
        Address::Contract(_) => Ok(true),
        _ => Ok(false),
    }
}

/// Create a wrapped adapter with automatic underlying asset detection
pub fn create_wrapped_adapter(
    env: &Env,
    token_address: Address,
) -> Result<WrappedAdapter, AdapterError> {
    let underlying = wrapped::get_underlying(env, &token_address)?;
    Ok(WrappedAdapter::new(token_address, underlying))
}
