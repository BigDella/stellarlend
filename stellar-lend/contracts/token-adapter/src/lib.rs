#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, Env, IntoVal,
    Symbol, Vec,
};
use soroban_token_sdk::token::{Client as TokenClient, StellarAssetClient};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AdapterError {
    InsufficientBalance = 1,
    InsufficientAllowance = 2,
    TransferFailed = 3,
    ApproveFailed = 4,
    InvalidAmount = 5,
    DecimalMismatch = 6,
    DustAmount = 7,
    TokenNotRegistered = 8,
    Unauthorized = 9,
    Overflow = 10,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum TokenType {
    SorobanToken,
    NativeAsset,
    WrappedAsset,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TokenInfo {
    pub address: Address,
    pub token_type: TokenType,
    pub decimals: u32,
    pub name: Symbol,
    pub symbol: Symbol,
    pub min_transfer: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    TokenInfo(Address),
    RegisteredTokens,
    GlobalDecimals,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TransferResult {
    pub from: Address,
    pub to: Address,
    pub amount: i128,
    pub actual_amount: i128,
    pub dust_remaining: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct BatchResult {
    pub approve_result: bool,
    pub transfer_from_result: bool,
    pub total_moved: i128,
}

#[contractevent]
#[derive(Clone)]
pub struct TransferEvent {
    #[topic]
    from: Address,
    #[topic]
    to: Address,
    #[topic]
    token: Address,
    amount: i128,
}

#[contract]
pub struct TokenAdapterContract;

#[contractimpl]
impl TokenAdapterContract {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().persistent().has(&DataKey::Admin) {
            return;
        }
        admin.require_auth();
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::GlobalDecimals, &18u32);
    }

    pub fn register_token(
        env: Env,
        admin: Address,
        token_address: Address,
        token_type: TokenType,
        decimals: u32,
        name: Symbol,
        symbol: Symbol,
    ) -> Result<(), AdapterError> {
        Self::require_admin(&env, &admin)?;
        let min_transfer = 1i128;
        let info = TokenInfo {
            address: token_address.clone(),
            token_type,
            decimals,
            name,
            symbol,
            min_transfer,
        };
        env.storage()
            .persistent()
            .set(&DataKey::TokenInfo(token_address), &info);
        Ok(())
    }

    pub fn get_token_info(env: Env, token: Address) -> Option<TokenInfo> {
        env.storage().persistent().get(&DataKey::TokenInfo(token))
    }

    // === Core Operations ===

    pub fn transfer(
        env: Env,
        token: Address,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<TransferResult, AdapterError> {
        if amount <= 0 {
            return Err(AdapterError::InvalidAmount);
        }
        let info = Self::get_token_info(env.clone(), token.clone())
            .ok_or(AdapterError::TokenNotRegistered)?;
        let actual_amount = Self::apply_dust_filter(amount, info.min_transfer);
        if actual_amount <= 0 {
            return Err(AdapterError::DustAmount);
        }
        let balance = Self::balance_of(env.clone(), token.clone(), from.clone())?;
        if balance < actual_amount {
            return Err(AdapterError::InsufficientBalance);
        }
        Self::do_transfer(&env, &info, &from, &to, actual_amount)?;
        Ok(TransferResult {
            from,
            to,
            amount,
            actual_amount,
            dust_remaining: amount - actual_amount,
        })
    }

    pub fn transfer_from(
        env: Env,
        token: Address,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<TransferResult, AdapterError> {
        if amount <= 0 {
            return Err(AdapterError::InvalidAmount);
        }
        let info = Self::get_token_info(env.clone(), token.clone())
            .ok_or(AdapterError::TokenNotRegistered)?;
        let actual_amount = Self::apply_dust_filter(amount, info.min_transfer);
        if actual_amount <= 0 {
            return Err(AdapterError::DustAmount);
        }
        let allowance = Self::allowance(
            env.clone(),
            token.clone(),
            from.clone(),
            env.current_contract_address(),
        )?;
        if allowance < actual_amount {
            return Err(AdapterError::InsufficientAllowance);
        }
        Self::do_transfer_from(&env, &info, &from, &to, actual_amount)?;
        Ok(TransferResult {
            from,
            to,
            amount,
            actual_amount,
            dust_remaining: amount - actual_amount,
        })
    }

    pub fn approve(
        env: Env,
        token: Address,
        owner: Address,
        spender: Address,
        amount: i128,
    ) -> Result<(), AdapterError> {
        if amount < 0 {
            return Err(AdapterError::InvalidAmount);
        }
        let info = Self::get_token_info(env.clone(), token.clone())
            .ok_or(AdapterError::TokenNotRegistered)?;
        Self::do_approve(&env, &info, &owner, &spender, amount)
    }

    pub fn balance_of(env: Env, token: Address, address: Address) -> Result<i128, AdapterError> {
        let info = Self::get_token_info(env.clone(), token.clone())
            .ok_or(AdapterError::TokenNotRegistered)?;
        Self::do_balance_of(&env, &info, &address)
    }

    pub fn allowance(
        env: Env,
        token: Address,
        owner: Address,
        spender: Address,
    ) -> Result<i128, AdapterError> {
        let info = Self::get_token_info(env.clone(), token.clone())
            .ok_or(AdapterError::TokenNotRegistered)?;
        Self::do_allowance(&env, &info, &owner, &spender)
    }

    pub fn total_supply(env: Env, token: Address) -> Result<i128, AdapterError> {
        let info = Self::get_token_info(env.clone(), token.clone())
            .ok_or(AdapterError::TokenNotRegistered)?;
        Self::do_total_supply(&env, &info)
    }

    // === Batch Operations ===

    pub fn batch_approve_and_transfer(
        env: Env,
        token: Address,
        owner: Address,
        spender: Address,
        to: Address,
        amount: i128,
    ) -> Result<BatchResult, AdapterError> {
        Self::approve(
            env.clone(),
            token.clone(),
            owner.clone(),
            spender.clone(),
            amount.clone(),
        )?;
        let transfer_result = Self::transfer_from(env, token, spender, to, amount.clone())?;
        Ok(BatchResult {
            approve_result: true,
            transfer_from_result: true,
            total_moved: transfer_result.actual_amount,
        })
    }

    // === Decimal Normalization ===

    pub fn normalize_amount(
        env: Env,
        token: Address,
        amount: i128,
        from_decimals: u32,
    ) -> Result<i128, AdapterError> {
        let info = Self::get_token_info(env.clone(), token.clone())
            .ok_or(AdapterError::TokenNotRegistered)?;
        if from_decimals == info.decimals {
            return Ok(amount);
        }
        if from_decimals > info.decimals {
            let diff = from_decimals - info.decimals;
            let divisor = 10i128.pow(diff);
            Ok(amount / divisor)
        } else {
            let diff = info.decimals - from_decimals;
            let multiplier = 10i128.pow(diff);
            amount.checked_mul(multiplier).ok_or(AdapterError::Overflow)
        }
    }

    pub fn to_minimal_unit(env: Env, token: Address, amount: i128) -> Result<i128, AdapterError> {
        let info = Self::get_token_info(env.clone(), token.clone())
            .ok_or(AdapterError::TokenNotRegistered)?;
        let decimals = info.decimals;
        let scale = 10i128.pow(decimals);
        amount.checked_mul(scale).ok_or(AdapterError::Overflow)
    }

    pub fn from_minimal_unit(
        env: Env,
        token: Address,
        minimal: i128,
    ) -> Result<i128, AdapterError> {
        let info = Self::get_token_info(env.clone(), token.clone())
            .ok_or(AdapterError::TokenNotRegistered)?;
        let decimals = info.decimals;
        let scale = 10i128.pow(decimals);
        Ok(minimal / scale)
    }

    // === Admin ===

    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().persistent().get(&DataKey::Admin)
    }

    // === Internal Helpers ===

    fn require_admin(env: &Env, admin: &Address) -> Result<(), AdapterError> {
        let stored: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(AdapterError::Unauthorized)?;
        if admin != &stored {
            return Err(AdapterError::Unauthorized);
        }
        admin.require_auth();
        Ok(())
    }

    fn apply_dust_filter(amount: i128, min_transfer: i128) -> i128 {
        if min_transfer <= 0 || amount < min_transfer {
            0
        } else {
            amount
        }
    }

    fn do_transfer(
        env: &Env,
        info: &TokenInfo,
        from: &Address,
        to: &Address,
        amount: i128,
    ) -> Result<(), AdapterError> {
        match info.token_type {
            TokenType::SorobanToken => {
                let client = TokenClient::new(env, &info.address);
                client.transfer(from, to, &amount);
                Ok(())
            }
            TokenType::NativeAsset | TokenType::WrappedAsset => {
                let client = StellarAssetClient::new(env, &info.address);
                client.transfer(from, to, &amount);
                Ok(())
            }
        }
    }

    fn do_transfer_from(
        env: &Env,
        info: &TokenInfo,
        from: &Address,
        to: &Address,
        amount: i128,
    ) -> Result<(), AdapterError> {
        match info.token_type {
            TokenType::SorobanToken => {
                let client = TokenClient::new(env, &info.address);
                let contract_addr = env.current_contract_address();
                client.transfer_from(from, &contract_addr, &amount);
                client.transfer(&contract_addr, to, &amount);
                Ok(())
            }
            TokenType::NativeAsset | TokenType::WrappedAsset => {
                let client = StellarAssetClient::new(env, &info.address);
                let contract_addr = env.current_contract_address();
                client.transfer(from, &contract_addr, &amount);
                client.transfer(&contract_addr, to, &amount);
                Ok(())
            }
        }
    }

    fn do_approve(
        env: &Env,
        info: &TokenInfo,
        owner: &Address,
        spender: &Address,
        amount: i128,
    ) -> Result<(), AdapterError> {
        match info.token_type {
            TokenType::SorobanToken => {
                let client = TokenClient::new(env, &info.address);
                let ledger = env.ledger().sequence();
                client.approve(owner, spender, &amount, &(ledger + 100));
                Ok(())
            }
            TokenType::NativeAsset | TokenType::WrappedAsset => {
                let client = StellarAssetClient::new(env, &info.address);
                let ledger = env.ledger().sequence();
                client.approve(owner, spender, &amount, &(ledger + 100));
                Ok(())
            }
        }
    }

    fn do_balance_of(env: &Env, info: &TokenInfo, address: &Address) -> Result<i128, AdapterError> {
        match info.token_type {
            TokenType::SorobanToken => {
                let client = TokenClient::new(env, &info.address);
                Ok(client.balance(address))
            }
            TokenType::NativeAsset | TokenType::WrappedAsset => {
                let client = StellarAssetClient::new(env, &info.address);
                Ok(client.balance(address))
            }
        }
    }

    fn do_allowance(
        env: &Env,
        info: &TokenInfo,
        owner: &Address,
        spender: &Address,
    ) -> Result<i128, AdapterError> {
        match info.token_type {
            TokenType::SorobanToken => {
                let client = TokenClient::new(env, &info.address);
                Ok(client.allowance(owner, spender))
            }
            TokenType::NativeAsset | TokenType::WrappedAsset => {
                let client = StellarAssetClient::new(env, &info.address);
                Ok(client.allowance(owner, spender))
            }
        }
    }

    fn do_total_supply(env: &Env, info: &TokenInfo) -> Result<i128, AdapterError> {
        match info.token_type {
            TokenType::SorobanToken => {
                let client = TokenClient::new(env, &info.address);
                Ok(client.total_supply())
            }
            TokenType::NativeAsset | TokenType::WrappedAsset => {
                let client = StellarAssetClient::new(env, &info.address);
                Ok(client.balance(&env.current_contract_address()))
            }
        }
    }
}

#[cfg(test)]
mod lib_test;
