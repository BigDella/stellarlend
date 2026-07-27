#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

mod token {
    use soroban_sdk::symbol_short;

    pub const NAME: Symbol = symbol_short!("NAME");
    pub const SYMBOL: Symbol = symbol_short!("SYMBOL");
    pub const DECIMALS: Symbol = symbol_short!("DECIMALS");
    pub const TOTAL_SUPPLY: Symbol = symbol_short!("SUPPLY");
    pub const BALANCE: Symbol = symbol_short!("BALANCE");
    pub const ALLOWANCE: Symbol = symbol_short!("ALLOWANCE");
}

mod storage {
    use soroban_sdk::symbol_short;

    pub const CONFIG: Symbol = symbol_short!("CONFIG");
    pub const INITIALIZED: Symbol = symbol_short!("INIT");
    pub const TRANSFER_LOCKED: Symbol = symbol_short!("TLOCK");
    pub const INTEREST_INDEX: Symbol = symbol_short!("IDX");
    pub const POSITIONS: Symbol = symbol_short!("POSITION");
    pub const HOLDERS: Symbol = symbol_short!("HOLDERS");
}

const ONE: i128 = 10_000;

#[derive(Clone)]
#[contracttype]
pub struct DebtTokenConfig {
    pub admin: Address,
    pub lending_pool: Address,
    pub underlying_asset: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u32,
    pub interest_index: i128,
    pub total_principal: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct Position {
    pub owner: Address,
    pub principal: i128,
    pub minted_tokens: i128,
    pub deposit_timestamp: u64,
    pub last_interest_update: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct DebtTokenAnalytics {
    pub total_supply: i128,
    pub total_principal: i128,
    pub accrued_interest: i128,
    pub interest_index: i128,
    pub holders_count: u32,
    pub avg_holding_period_secs: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct TransferEvent {
    pub from: Address,
    pub to: Address,
    pub amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct ApprovalEvent {
    pub owner: Address,
    pub spender: Address,
    pub amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct MintEvent {
    pub to: Address,
    pub amount: i128,
    pub principal: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct BurnEvent {
    pub from: Address,
    pub amount: i128,
    pub principal_redeemed: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct InterestAccruedEvent {
    pub account: Address,
    pub interest: i128,
    pub new_index: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct TransferLockEvent {
    pub locked: bool,
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum DebtTokenError {
    AlreadyInitialized = 1,
    InsufficientBalance = 2,
    InsufficientAllowance = 3,
    InvalidAmount = 4,
    Overflow = 5,
    Unauthorized = 6,
    TransferLocked = 7,
    RedemptionFailed = 8,
    InterestCalculationFailed = 9,
}

#[contract]
pub struct StellarLendDebtToken;

#[contractimpl]
impl StellarLendDebtToken {
    pub fn initialize(
        env: Env,
        admin: Address,
        lending_pool: Address,
        underlying_asset: Address,
        name: String,
        symbol: String,
    ) -> Result<(), DebtTokenError> {
        if env.storage().instance().has(&storage::INITIALIZED) {
            return Err(DebtTokenError::AlreadyInitialized);
        }

        let config = DebtTokenConfig {
            admin,
            lending_pool,
            underlying_asset,
            name: name.clone(),
            symbol: symbol.clone(),
            decimals: 9,
            interest_index: ONE,
            total_principal: 0,
        };

        env.storage().instance().set(&storage::CONFIG, &config);
        env.storage().instance().set(&storage::INITIALIZED, &true);
        env.storage().instance().set(&storage::TRANSFER_LOCKED, &false);

        env.storage()
            .instance()
            .set(&token::NAME, &name);
        env.storage()
            .instance()
            .set(&token::SYMBOL, &symbol);
        env.storage().instance().set(&token::DECIMALS, &9u32);
        env.storage()
            .instance()
            .set(&token::TOTAL_SUPPLY, &0i128);

        Ok(())
    }

    pub fn deposit_and_mint(
        env: Env,
        depositor: Address,
        principal_amount: i128,
    ) -> Result<i128, DebtTokenError> {
        depositor.require_auth();

        if principal_amount <= 0 {
            return Err(DebtTokenError::InvalidAmount);
        }

        let mut config: DebtTokenConfig = env
            .storage()
            .instance()
            .get(&storage::CONFIG)
            .ok_or(DebtTokenError::Unauthorized)?;

        let current_index = config.interest_index;
        let tokens_to_mint =
            Self::calculate_tokens_for_principal(&env, principal_amount, current_index)?;

        let position_key = (storage::POSITIONS, depositor.clone());
        let mut position: Position = env
            .storage()
            .persistent()
            .get(&position_key)
            .unwrap_or(Position {
                owner: depositor.clone(),
                principal: 0,
                minted_tokens: 0,
                deposit_timestamp: env.ledger().timestamp(),
                last_interest_update: env.ledger().timestamp(),
            });

        position.principal = position
            .principal
            .checked_add(principal_amount)
            .ok_or(DebtTokenError::Overflow)?;
        position.minted_tokens = position
            .minted_tokens
            .checked_add(tokens_to_mint)
            .ok_or(DebtTokenError::Overflow)?;

        env.storage()
            .persistent()
            .set(&position_key, &position);

        let current_balance = Self::read_balance(&env, &depositor)?;
        let new_balance = current_balance
            .checked_add(tokens_to_mint)
            .ok_or(DebtTokenError::Overflow)?;
        Self::write_balance(&env, &depositor, new_balance);

        let current_total = Self::read_total_supply(&env)?;
        let new_total = current_total
            .checked_add(tokens_to_mint)
            .ok_or(DebtTokenError::Overflow)?;
        env.storage()
            .instance()
            .set(&token::TOTAL_SUPPLY, &new_total);

        config.total_principal = config
            .total_principal
            .checked_add(principal_amount)
            .ok_or(DebtTokenError::Overflow)?;
        env.storage().instance().set(&storage::CONFIG, &config);

        env.events().publish(
            (symbol_short!("MINT"),),
            MintEvent {
                to: depositor,
                amount: tokens_to_mint,
                principal: principal_amount,
            },
        );

        Ok(tokens_to_mint)
    }

    pub fn redeem(
        env: Env,
        redeemer: Address,
        token_amount: i128,
    ) -> Result<i128, DebtTokenError> {
        redeemer.require_auth();

        if token_amount <= 0 {
            return Err(DebtTokenError::InvalidAmount);
        }

        let current_balance = Self::read_balance(&env, &redeemer)?;
        if current_balance < token_amount {
            return Err(DebtTokenError::InsufficientBalance);
        }

        let mut config: DebtTokenConfig = env
            .storage()
            .instance()
            .get(&storage::CONFIG)
            .ok_or(DebtTokenError::Unauthorized)?;

        let position_key = (storage::POSITIONS, redeemer.clone());
        let mut position: Position = env
            .storage()
            .persistent()
            .get(&position_key)
            .ok_or(DebtTokenError::RedemptionFailed)?;

        let current_index = config.interest_index;
        let deposit_index = Self::get_deposit_index(&env, &position, current_index)?;
        let accrued_interest =
            Self::calculate_accrued_interest(position.principal, current_index, deposit_index)?;

        let principal_to_redeem =
            Self::calculate_principal_for_tokens(&env, token_amount, current_index)?;

        if principal_to_redeem > position.principal {
            return Err(DebtTokenError::InsufficientBalance);
        }

        let new_balance = current_balance
            .checked_sub(token_amount)
            .ok_or(DebtTokenError::InsufficientBalance)?;
        Self::write_balance(&env, &redeemer, new_balance);

        let current_total = Self::read_total_supply(&env)?;
        let new_total = current_total
            .checked_sub(token_amount)
            .ok_or(DebtTokenError::Overflow)?;
        env.storage()
            .instance()
            .set(&token::TOTAL_SUPPLY, &new_total);

        position.principal = position
            .principal
            .checked_sub(principal_to_redeem)
            .ok_or(DebtTokenError::Overflow)?;
        position.minted_tokens = position
            .minted_tokens
            .checked_sub(token_amount)
            .ok_or(DebtTokenError::Overflow)?;

        let interest_earned = if position.principal == 0 {
            accrued_interest
        } else {
            0
        };

        env.storage()
            .persistent()
            .set(&position_key, &position);

        config.total_principal = config
            .total_principal
            .checked_sub(principal_to_redeem)
            .ok_or(DebtTokenError::Overflow)?;
        env.storage().instance().set(&storage::CONFIG, &config);

        env.events().publish(
            (symbol_short!("BURN"),),
            BurnEvent {
                from: redeemer,
                amount: token_amount,
                principal_redeemed: principal_to_redeem,
            },
        );

        Ok(principal_to_redeem
            .checked_add(interest_earned)
            .ok_or(DebtTokenError::Overflow)?)
    }

    pub fn transfer(
        env: Env,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), DebtTokenError> {
        from.require_auth();

        if env
            .storage()
            .instance()
            .get::<_, bool>(&storage::TRANSFER_LOCKED)
            .unwrap_or(false)
        {
            return Err(DebtTokenError::TransferLocked);
        }

        if amount <= 0 {
            return Err(DebtTokenError::InvalidAmount);
        }

        let from_balance = Self::read_balance(&env, &from)?;
        if from_balance < amount {
            return Err(DebtTokenError::InsufficientBalance);
        }

        Self::accrue_interest_for_account(&env, &from)?;

        let new_from_balance = from_balance
            .checked_sub(amount)
            .ok_or(DebtTokenError::InsufficientBalance)?;
        Self::write_balance(&env, &from, new_from_balance);

        let to_balance = Self::read_balance(&env, &to)?;
        let new_to_balance = to_balance
            .checked_add(amount)
            .ok_or(DebtTokenError::Overflow)?;
        Self::write_balance(&env, &to, new_to_balance);

        Self::reassign_tokens(&env, &from, &to, amount)?;

        env.events().publish(
            (symbol_short!("TRANSFER"),),
            TransferEvent {
                from,
                to,
                amount,
            },
        );

        Ok(())
    }

    pub fn transfer_from(
        env: Env,
        owner: Address,
        spender: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), DebtTokenError> {
        spender.require_auth();

        if env
            .storage()
            .instance()
            .get::<_, bool>(&storage::TRANSFER_LOCKED)
            .unwrap_or(false)
        {
            return Err(DebtTokenError::TransferLocked);
        }

        if amount <= 0 {
            return Err(DebtTokenError::InvalidAmount);
        }

        let allowance = Self::read_allowance(&env, &owner, &spender)?;
        if allowance < amount {
            return Err(DebtTokenError::InsufficientAllowance);
        }

        let owner_balance = Self::read_balance(&env, &owner)?;
        if owner_balance < amount {
            return Err(DebtTokenError::InsufficientBalance);
        }

        Self::accrue_interest_for_account(&env, &owner)?;

        let new_allowance = allowance
            .checked_sub(amount)
            .ok_or(DebtTokenError::InsufficientAllowance)?;
        Self::write_allowance(&env, &owner, &spender, new_allowance);

        let new_owner_balance = owner_balance
            .checked_sub(amount)
            .ok_or(DebtTokenError::InsufficientBalance)?;
        Self::write_balance(&env, &owner, new_owner_balance);

        let to_balance = Self::read_balance(&env, &to)?;
        let new_to_balance = to_balance
            .checked_add(amount)
            .ok_or(DebtTokenError::Overflow)?;
        Self::write_balance(&env, &to, new_to_balance);

        Self::reassign_tokens(&env, &owner, &to, amount)?;

        env.events().publish(
            (symbol_short!("TRANSFER"),),
            TransferEvent {
                from: owner,
                to,
                amount,
            },
        );

        Ok(())
    }

    pub fn approve(
        env: Env,
        owner: Address,
        spender: Address,
        amount: i128,
    ) -> Result<(), DebtTokenError> {
        owner.require_auth();

        if amount < 0 {
            return Err(DebtTokenError::InvalidAmount);
        }

        Self::write_allowance(&env, &owner, &spender, amount);

        env.events().publish(
            (symbol_short!("APPROVE"),),
            ApprovalEvent {
                owner,
                spender,
                amount,
            },
        );

        Ok(())
    }

    pub fn balance(env: Env, account: Address) -> i128 {
        Self::read_balance(&env, &account).unwrap_or(0)
    }

    pub fn total_supply(env: Env) -> i128 {
        Self::read_total_supply(&env).unwrap_or(0)
    }

    pub fn allowance(env: Env, owner: Address, spender: Address) -> i128 {
        Self::read_allowance(&env, &owner, &spender).unwrap_or(0)
    }

    pub fn name(env: Env) -> String {
        env.storage()
            .instance()
            .get(&token::NAME)
            .unwrap_or_else(|| symbol_short!("DEBT").into())
    }

    pub fn symbol(env: Env) -> String {
        env.storage()
            .instance()
            .get(&token::SYMBOL)
            .unwrap_or_else(|| symbol_short!("dToken").into())
    }

    pub fn decimals(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&token::DECIMALS)
            .unwrap_or(9u32)
    }

    pub fn accrued_interest(env: Env, account: Address) -> i128 {
        let config: DebtTokenConfig = match env.storage().instance().get(&storage::CONFIG) {
            Some(c) => c,
            None => return 0,
        };

        let position_key = (storage::POSITIONS, account);
        let position: Position = match env.storage().persistent().get(&position_key) {
            Some(p) => p,
            None => return 0,
        };

        let current_index = config.interest_index;
        match Self::get_deposit_index(&env, &position, current_index) {
            Ok(deposit_index) => {
                Self::calculate_accrued_interest(position.principal, current_index, deposit_index)
                    .unwrap_or(0)
            }
            Err(_) => 0,
        }
    }

    pub fn get_position(env: Env, account: Address) -> Position {
        let position_key = (storage::POSITIONS, account);
        env.storage()
            .persistent()
            .get(&position_key)
            .unwrap_or(Position {
                owner: account,
                principal: 0,
                minted_tokens: 0,
                deposit_timestamp: 0,
                last_interest_update: 0,
            })
    }

    pub fn update_interest_index(env: Env) -> Result<i128, DebtTokenError> {
        let config: DebtTokenConfig = env
            .storage()
            .instance()
            .get(&storage::CONFIG)
            .ok_or(DebtTokenError::Unauthorized)?;

        config.lending_pool.require_auth();

        let total_supply = Self::read_total_supply(&env).unwrap_or(0);
        let total_principal = config.total_principal;

        if total_principal <= 0 || total_supply <= 0 {
            return Ok(config.interest_index);
        }

        let new_index = total_supply
            .checked_mul(ONE)
            .ok_or(DebtTokenError::Overflow)?
            .checked_div(total_principal)
            .ok_or(DebtTokenError::Overflow)?;

        let mut updated_config = config.clone();
        updated_config.interest_index = new_index;
        env.storage()
            .instance()
            .set(&storage::CONFIG, &updated_config);

        Ok(new_index)
    }

    pub fn lock_transfers(env: Env, locked: bool) -> Result<(), DebtTokenError> {
        let config: DebtTokenConfig = env
            .storage()
            .instance()
            .get(&storage::CONFIG)
            .ok_or(DebtTokenError::Unauthorized)?;

        config.admin.require_auth();

        env.storage()
            .instance()
            .set(&storage::TRANSFER_LOCKED, &locked);

        env.events()
            .publish((symbol_short!("TLOCK"),), TransferLockEvent { locked });

        Ok(())
    }

    pub fn get_analytics(env: Env) -> DebtTokenAnalytics {
        let config: DebtTokenConfig = env
            .storage()
            .instance()
            .get(&storage::CONFIG)
            .unwrap_or(DebtTokenConfig {
                admin: Address::from_str(&env, &soroban_sdk::Bytes::new()),
                lending_pool: Address::from_str(&env, &soroban_sdk::Bytes::new()),
                underlying_asset: Address::from_str(&env, &soroban_sdk::Bytes::new()),
                name: symbol_short!("DEBT"),
                symbol: symbol_short!("dToken"),
                decimals: 9,
                interest_index: ONE,
                total_principal: 0,
            });

        let total_supply = Self::read_total_supply(&env).unwrap_or(0);
        let total_principal = config.total_principal;
        let accrued_interest = if total_principal > 0 && config.interest_index > ONE {
            total_principal
                .checked_mul(config.interest_index.checked_sub(ONE).unwrap_or(0))
                .unwrap_or(0)
                .checked_div(ONE)
                .unwrap_or(0)
        } else {
            0
        };

        let holders_count = env
            .storage()
            .persistent()
            .get::<_, u32>(&storage::HOLDERS)
            .unwrap_or(0);

        let avg_holding_period_secs = Self::calculate_avg_holding_period(&env);

        DebtTokenAnalytics {
            total_supply,
            total_principal,
            accrued_interest,
            interest_index: config.interest_index,
            holders_count,
            avg_holding_period_secs,
        }
    }

    pub fn nav_per_token(env: Env) -> i128 {
        let config: DebtTokenConfig = match env.storage().instance().get(&storage::CONFIG) {
            Some(c) => c,
            None => return ONE,
        };

        let total_supply = Self::read_total_supply(&env).unwrap_or(0);
        if total_supply <= 0 {
            return ONE;
        }

        let total_value = config
            .total_principal
            .checked_mul(config.interest_index)
            .unwrap_or(0)
            .checked_div(ONE)
            .unwrap_or(0);

        total_value
            .checked_mul(ONE)
            .unwrap_or(0)
            .checked_div(total_supply)
            .unwrap_or(ONE)
    }

    fn read_balance(env: &Env, account: &Address) -> Result<i128, DebtTokenError> {
        let key = (token::BALANCE, account.clone());
        Ok(env.storage().instance().get(&key).unwrap_or(0))
    }

    fn write_balance(env: &Env, account: &Address, amount: i128) {
        let key = (token::BALANCE, account.clone());
        env.storage().instance().set(&key, &amount);
    }

    fn read_total_supply(env: &Env) -> Result<i128, DebtTokenError> {
        Ok(env
            .storage()
            .instance()
            .get(&token::TOTAL_SUPPLY)
            .unwrap_or(0))
    }

    fn read_allowance(
        env: &Env,
        owner: &Address,
        spender: &Address,
    ) -> Result<i128, DebtTokenError> {
        let key = (token::ALLOWANCE, owner.clone(), spender.clone());
        Ok(env.storage().instance().get(&key).unwrap_or(0))
    }

    fn write_allowance(env: &Env, owner: &Address, spender: &Address, amount: i128) {
        let key = (token::ALLOWANCE, owner.clone(), spender.clone());
        env.storage().instance().set(&key, &amount);
    }

    fn calculate_tokens_for_principal(
        _env: &Env,
        principal: i128,
        index: i128,
    ) -> Result<i128, DebtTokenError> {
        principal
            .checked_mul(ONE)
            .ok_or(DebtTokenError::Overflow)?
            .checked_div(index)
            .ok_or(DebtTokenError::Overflow)
    }

    fn calculate_principal_for_tokens(
        _env: &Env,
        tokens: i128,
        index: i128,
    ) -> Result<i128, DebtTokenError> {
        tokens
            .checked_mul(index)
            .ok_or(DebtTokenError::Overflow)?
            .checked_div(ONE)
            .ok_or(DebtTokenError::Overflow)
    }

    fn calculate_accrued_interest(
        principal: i128,
        current_index: i128,
        deposit_index: i128,
    ) -> Result<i128, DebtTokenError> {
        if deposit_index <= 0 {
            return Ok(0);
        }

        let index_diff = current_index
            .checked_sub(deposit_index)
            .ok_or(DebtTokenError::InterestCalculationFailed)?;

        if index_diff <= 0 {
            return Ok(0);
        }

        principal
            .checked_mul(index_diff)
            .ok_or(DebtTokenError::Overflow)?
            .checked_div(deposit_index)
            .ok_or(DebtTokenError::Overflow)
    }

    fn get_deposit_index(
        _env: &Env,
        position: &Position,
        current_index: i128,
    ) -> Result<i128, DebtTokenError> {
        if position.principal <= 0 {
            return Ok(current_index);
        }

        if position.minted_tokens <= 0 {
            return Ok(current_index);
        }

        position
            .minted_tokens
            .checked_mul(ONE)
            .ok_or(DebtTokenError::Overflow)?
            .checked_div(position.principal)
            .ok_or(DebtTokenError::Overflow)
    }

    fn accrue_interest_for_account(env: &Env, account: &Address) -> Result<(), DebtTokenError> {
        let config: DebtTokenConfig = env
            .storage()
            .instance()
            .get(&storage::CONFIG)
            .ok_or(DebtTokenError::Unauthorized)?;

        let position_key = (storage::POSITIONS, account.clone());
        let mut position: Position = match env.storage().persistent().get(&position_key) {
            Some(p) => p,
            None => return Ok(()),
        };

        if position.principal <= 0 {
            return Ok(());
        }

        let current_index = config.interest_index;
        let deposit_index = Self::get_deposit_index(env, &position, current_index)?;
        let interest =
            Self::calculate_accrued_interest(position.principal, current_index, deposit_index)?;

        if interest > 0 {
            let new_minted = position
                .minted_tokens
                .checked_add(interest)
                .ok_or(DebtTokenError::Overflow)?;
            position.minted_tokens = new_minted;
            position.last_interest_update = env.ledger().timestamp();

            let current_balance = Self::read_balance(env, account)?;
            let new_balance = current_balance
                .checked_add(interest)
                .ok_or(DebtTokenError::Overflow)?;
            Self::write_balance(env, account, new_balance);

            let current_total = Self::read_total_supply(env)?;
            let new_total = current_total
                .checked_add(interest)
                .ok_or(DebtTokenError::Overflow)?;
            env.storage()
                .instance()
                .set(&token::TOTAL_SUPPLY, &new_total);

            env.events().publish(
                (symbol_short!("INTACC"),),
                InterestAccruedEvent {
                    account: account.clone(),
                    interest,
                    new_index: current_index,
                },
            );
        }

        position.last_interest_update = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&position_key, &position);

        Ok(())
    }

    fn reassign_tokens(
        env: &Env,
        from: &Address,
        to: &Address,
        amount: i128,
    ) -> Result<(), DebtTokenError> {
        let from_key = (storage::POSITIONS, from.clone());
        let to_key = (storage::POSITIONS, to.clone());

        let mut from_position: Position = env
            .storage()
            .persistent()
            .get(&from_key)
            .unwrap_or(Position {
                owner: from.clone(),
                principal: 0,
                minted_tokens: 0,
                deposit_timestamp: env.ledger().timestamp(),
                last_interest_update: env.ledger().timestamp(),
            });

        let mut to_position: Position = env
            .storage()
            .persistent()
            .get(&to_key)
            .unwrap_or(Position {
                owner: to.clone(),
                principal: 0,
                minted_tokens: 0,
                deposit_timestamp: env.ledger().timestamp(),
                last_interest_update: env.ledger().timestamp(),
            });

        let config: DebtTokenConfig = env
            .storage()
            .instance()
            .get(&storage::CONFIG)
            .ok_or(DebtTokenError::Unauthorized)?;

        let current_index = config.interest_index;
        let from_deposit_index = Self::get_deposit_index(env, &from_position, current_index)?;
        let principal_transferred =
            Self::calculate_principal_for_tokens(env, amount, current_index)?;

        from_position.principal = from_position
            .principal
            .checked_sub(principal_transferred)
            .ok_or(DebtTokenError::Overflow)?;
        from_position.minted_tokens = from_position
            .minted_tokens
            .checked_sub(amount)
            .ok_or(DebtTokenError::Overflow)?;

        to_position.principal = to_position
            .principal
            .checked_add(principal_transferred)
            .ok_or(DebtTokenError::Overflow)?;
        to_position.minted_tokens = to_position
            .minted_tokens
            .checked_add(amount)
            .ok_or(DebtTokenError::Overflow)?;
        to_position.last_interest_update = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&from_key, &from_position);
        env.storage()
            .persistent()
            .set(&to_key, &to_position);

        Ok(())
    }

    fn calculate_avg_holding_period(env: &Env) -> u64 {
        let holders: u32 = env
            .storage()
            .persistent()
            .get(&storage::HOLDERS)
            .unwrap_or(0);

        if holders == 0 {
            return 0;
        }

        let current_time = env.ledger().timestamp();
        let avg_period = current_time / (holders as u64).max(1);
        avg_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

    fn create_env() -> Env {
        Env::default()
    }

    fn setup_contract(env: &Env) -> (Address, Address, Address) {
        let admin = Address::generate(env);
        let lending_pool = Address::generate(env);
        let underlying_asset = Address::generate(env);

        let contract_id = env.register_contract(None, StellarLendDebtToken);
        let client = StellarLendDebtTokenClient::new(env, &contract_id);

        let name = String::from_str(env, "StellarLend Debt Token");
        let symbol = String::from_str(env, "dToken");

        client.initialize(&admin, &lending_pool, &underlying_asset, &name, &symbol);

        (admin, lending_pool, underlying_asset)
    }

    #[test]
    fn test_initialize() {
        let env = create_env();
        let (admin, lending_pool, underlying_asset) = setup_contract(&env);

        let contract_id = env.register_contract(None, StellarLendDebtToken);
        let client = StellarLendDebtTokenClient::new(&env, &contract_id);

        assert_eq!(client.name(), String::from_str(&env, "StellarLend Debt Token"));
        assert_eq!(client.symbol(), String::from_str(&env, "dToken"));
        assert_eq!(client.decimals(), 9u32);
        assert_eq!(client.total_supply(), 0);
    }

    #[test]
    fn test_deposit_and_mint() {
        let env = create_env();
        let (admin, lending_pool, underlying_asset) = setup_contract(&env);

        let contract_id = env.register_contract(None, StellarLendDebtToken);
        let client = StellarLendDebtTokenClient::new(&env, &contract_id);

        let depositor = Address::generate(&env);
        let principal = 1_000_000_000i128;

        let minted = client.deposit_and_mint(&depositor, &principal);

        assert!(minted > 0);
        assert_eq!(client.balance(&depositor), minted);
        assert_eq!(client.total_supply(), minted);

        let position = client.get_position(&depositor);
        assert_eq!(position.principal, principal);
        assert_eq!(position.minted_tokens, minted);
    }

    #[test]
    fn test_transfer() {
        let env = create_env();
        let (admin, lending_pool, underlying_asset) = setup_contract(&env);

        let contract_id = env.register_contract(None, StellarLendDebtToken);
        let client = StellarLendDebtTokenClient::new(&env, &contract_id);

        let from = Address::generate(&env);
        let to = Address::generate(&env);
        let principal = 1_000_000_000i128;

        let minted = client.deposit_and_mint(&from, &principal);

        client.transfer(&from, &to, &(minted / 2));

        assert_eq!(client.balance(&from), minted - minted / 2);
        assert_eq!(client.balance(&to), minted / 2);
    }

    #[test]
    fn test_approve_and_transfer_from() {
        let env = create_env();
        let (admin, lending_pool, underlying_asset) = setup_contract(&env);

        let contract_id = env.register_contract(None, StellarLendDebtToken);
        let client = StellarLendDebtTokenClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let spender = Address::generate(&env);
        let to = Address::generate(&env);
        let principal = 1_000_000_000i128;

        let minted = client.deposit_and_mint(&owner, &principal);

        client.approve(&owner, &spender, &(minted / 2));
        assert_eq!(client.allowance(&owner, &spender), minted / 2);

        client.transfer_from(&owner, &spender, &to, &(minted / 2));

        assert_eq!(client.balance(&owner), minted - minted / 2);
        assert_eq!(client.balance(&to), minted / 2);
        assert_eq!(client.allowance(&owner, &spender), 0);
    }

    #[test]
    fn test_redeem() {
        let env = create_env();
        let (admin, lending_pool, underlying_asset) = setup_contract(&env);

        let contract_id = env.register_contract(None, StellarLendDebtToken);
        let client = StellarLendDebtTokenClient::new(&env, &contract_id);

        let redeemer = Address::generate(&env);
        let principal = 1_000_000_000i128;

        let minted = client.deposit_and_mint(&redeemer, &principal);

        let redeemed = client.redeem(&redeemer, &minted);

        assert!(redeemed >= principal);
        assert_eq!(client.balance(&redeemer), 0);
        assert_eq!(client.total_supply(), 0);
    }

    #[test]
    fn test_lock_transfers() {
        let env = create_env();
        let (admin, lending_pool, underlying_asset) = setup_contract(&env);

        let contract_id = env.register_contract(None, StellarLendDebtToken);
        let client = StellarLendDebtTokenClient::new(&env, &contract_id);

        client.lock_transfers(&true);

        let from = Address::generate(&env);
        let to = Address::generate(&env);
        let principal = 1_000_000_000i128;

        client.deposit_and_mint(&from, &principal);

        let result = client.try_transfer(&from, &to, &100);
        assert!(result.is_err());
    }

    #[test]
    fn test_insufficient_balance() {
        let env = create_env();
        let (admin, lending_pool, underlying_asset) = setup_contract(&env);

        let contract_id = env.register_contract(None, StellarLendDebtToken);
        let client = StellarLendDebtTokenClient::new(&env, &contract_id);

        let from = Address::generate(&env);
        let to = Address::generate(&env);

        let result = client.try_transfer(&from, &to, &1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_analytics() {
        let env = create_env();
        let (admin, lending_pool, underlying_asset) = setup_contract(&env);

        let contract_id = env.register_contract(None, StellarLendDebtToken);
        let client = StellarLendDebtTokenClient::new(&env, &contract_id);

        let depositor = Address::generate(&env);
        let principal = 1_000_000_000i128;

        client.deposit_and_mint(&depositor, &principal);

        let analytics = client.get_analytics();
        assert_eq!(analytics.total_principal, principal);
        assert!(analytics.total_supply > 0);
    }

    #[test]
    fn test_nav_per_token() {
        let env = create_env();
        let (admin, lending_pool, underlying_asset) = setup_contract(&env);

        let contract_id = env.register_contract(None, StellarLendDebtToken);
        let client = StellarLendDebtTokenClient::new(&env, &contract_id);

        let nav = client.nav_per_token();
        assert_eq!(nav, ONE);

        let depositor = Address::generate(&env);
        let principal = 1_000_000_000i128;

        client.deposit_and_mint(&depositor, &principal);

        let nav = client.nav_per_token();
        assert!(nav > 0);
    }

    #[test]
    fn test_get_position() {
        let env = create_env();
        let (admin, lending_pool, underlying_asset) = setup_contract(&env);

        let contract_id = env.register_contract(None, StellarLendDebtToken);
        let client = StellarLendDebtTokenClient::new(&env, &contract_id);

        let account = Address::generate(&env);
        let position = client.get_position(&account);

        assert_eq!(position.principal, 0);
        assert_eq!(position.minted_tokens, 0);

        let principal = 1_000_000_000i128;
        client.deposit_and_mint(&account, &principal);

        let position = client.get_position(&account);
        assert_eq!(position.principal, principal);
        assert!(position.minted_tokens > 0);
    }
}
