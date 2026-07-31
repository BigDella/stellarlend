//! # ERC-20 Token Adapter Implementation
//!
//! Provides a standardized adapter for ERC-20 compatible tokens,
//! enabling the lending protocol to interact with any ERC-20 compliant token.

use crate::token_adapter::{AdapterConfig, AdapterError, TokenAdapterType};
use soroban_sdk::{token::Client as TokenClient, Address, Env, Vec};

/// ERC-20 adapter for standard token interactions
pub struct ERC20Adapter {
    config: AdapterConfig,
}

impl ERC20Adapter {
    /// Create a new ERC-20 adapter
    pub fn new(token_address: Address) -> Self {
        Self {
            config: AdapterConfig {
                adapter_type: TokenAdapterType::ERC20,
                token_address,
                enabled: true,
                metadata: Vec::new(&Env::default()),
            },
        }
    }

    /// Create from existing configuration
    pub fn from_config(config: AdapterConfig) -> Result<Self, AdapterError> {
        if config.adapter_type != TokenAdapterType::ERC20 {
            return Err(AdapterError::TokenNotSupported);
        }
        Ok(Self { config })
    }

    /// Get the underlying token address
    pub fn token_address(&self) -> &Address {
        &self.config.token_address
    }
}

/// ERC-20 Token interface methods
///
/// These methods interact with standard ERC-20 token contracts
pub mod erc20 {
    use super::*;

    /// Transfer tokens from the current contract to another address
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

    /// Get the balance of a specific address
    pub fn balance_of(env: &Env, token: &Address, address: &Address) -> Result<i128, AdapterError> {
        let token_client = TokenClient::new(env, token);
        Ok(token_client.balance(address))
    }

    /// Get the total supply of the token
    pub fn total_supply(env: &Env, token: &Address) -> Result<i128, AdapterError> {
        let token_client = TokenClient::new(env, token);
        Ok(token_client.total_supply())
    }

    /// Approve a spender to use a certain amount of tokens
    pub fn approve(
        env: &Env,
        token: &Address,
        owner: &Address,
        spender: &Address,
        amount: i128,
    ) -> Result<(), AdapterError> {
        if amount < 0 {
            return Err(AdapterError::InvalidConfig);
        }
        let token_client = TokenClient::new(env, token);
        token_client.approve(owner, spender, &amount, &(env.ledger().sequence() + 100));
        Ok(())
    }

    /// Get the allowance for a spender
    pub fn allowance(
        env: &Env,
        token: &Address,
        owner: &Address,
        spender: &Address,
    ) -> Result<i128, AdapterError> {
        let token_client = TokenClient::new(env, token);
        Ok(token_client.allowance(owner, spender))
    }

    /// Transfer tokens using allowance
    pub fn transfer_from(
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
        token_client.transfer_from(from, to, &amount);
        Ok(())
    }
}

impl super::TokenAdapterTrait for ERC20Adapter {
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
        erc20::transfer(env, &self.config.token_address, from, to, amount)
    }

    fn balance_of(&self, env: &Env, address: &Address) -> Result<i128, AdapterError> {
        erc20::balance_of(env, &self.config.token_address, address)
    }

    fn total_supply(&self, env: &Env) -> Result<i128, AdapterError> {
        erc20::total_supply(env, &self.config.token_address)
    }

    fn approve(
        &self,
        env: &Env,
        owner: &Address,
        spender: &Address,
        amount: i128,
    ) -> Result<(), AdapterError> {
        erc20::approve(env, &self.config.token_address, owner, spender, amount)
    }

    fn allowance(
        &self,
        env: &Env,
        owner: &Address,
        spender: &Address,
    ) -> Result<i128, AdapterError> {
        erc20::allowance(env, &self.config.token_address, owner, spender)
    }

    fn transfer_from(
        &self,
        env: &Env,
        from: &Address,
        to: &Address,
        amount: i128,
    ) -> Result<(), AdapterError> {
        erc20::transfer_from(env, &self.config.token_address, from, to, amount)
    }
}

/// Verify if a token is ERC-20 compatible
pub fn verify_erc20_compatibility(
    env: &Env,
    token_address: &Address,
) -> Result<bool, AdapterError> {
    // Verify that the token contract implements required ERC-20 functions
    // This is a basic check - in production, would verify each function exists

    // Check if token supports basic ERC-20 functions
    // In Soroban, this would use the token interface
    match erc20::balance_of(
        env,
        token_address,
        &Address::from_contract_id(env, &env.current_contract()),
    ) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
