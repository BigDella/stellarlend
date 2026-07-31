#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, Env, String,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PrincipalTokenError {
    Unauthorized = 1,
    AlreadyInitialized = 2,
    NotInitialized = 3,
    InsufficientBalance = 4,
    InsufficientAllowance = 5,
    InvalidAmount = 6,
    Overflow = 7,
    BurnFailed = 8,
    MintFailed = 9,
    TokenNotActive = 10,
    MaturityReached = 11,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    Name,
    Symbol,
    Decimals,
    Balance(Address),
    Allowance(Address, Address),
    TotalSupply,
    MaturityDate,
    UnderlyingAsset,
    YieldSplitter,
    Active,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct TransferEvent {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct ApprovalEvent {
    #[topic]
    pub owner: Address,
    #[topic]
    pub spender: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct MintEvent {
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct BurnEvent {
    #[topic]
    pub from: Address,
    pub amount: i128,
}

#[contract]
pub struct PrincipalToken;

#[contractimpl]
impl PrincipalToken {
    pub fn initialize(
        env: Env,
        admin: Address,
        name: String,
        symbol: String,
        underlying_asset: Address,
        yield_splitter: Address,
        maturity_date: u64,
    ) -> Result<(), PrincipalTokenError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(PrincipalTokenError::AlreadyInitialized);
        }
        admin.require_auth();

        let decimals: u32 = 7;
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::Symbol, &symbol);
        env.storage().instance().set(&DataKey::Decimals, &decimals);
        env.storage()
            .instance()
            .set(&DataKey::UnderlyingAsset, &underlying_asset);
        env.storage()
            .instance()
            .set(&DataKey::YieldSplitter, &yield_splitter);
        env.storage()
            .instance()
            .set(&DataKey::MaturityDate, &maturity_date);
        env.storage().instance().set(&DataKey::TotalSupply, &0i128);
        env.storage().instance().set(&DataKey::Active, &true);
        Ok(())
    }

    pub fn name(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Name)
            .unwrap()
    }

    pub fn symbol(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Symbol)
            .unwrap()
    }

    pub fn decimals(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::Decimals)
            .unwrap_or(7)
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(id))
            .unwrap_or(0)
    }

    pub fn total_supply(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0)
    }

    pub fn allowance(env: Env, owner: Address, spender: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Allowance(owner, spender))
            .unwrap_or(0)
    }

    pub fn transfer(
        env: Env,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), PrincipalTokenError> {
        from.require_auth();
        Self::do_transfer(&env, &from, &to, amount)?;

        TransferEvent { from, to, amount }.publish(&env);
        Ok(())
    }

    pub fn transfer_from(
        env: Env,
        spender: Address,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), PrincipalTokenError> {
        spender.require_auth();
        let current_allowance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Allowance(from.clone(), spender.clone()))
            .unwrap_or(0);

        if current_allowance < amount {
            return Err(PrincipalTokenError::InsufficientAllowance);
        }

        let new_allowance = current_allowance
            .checked_sub(amount)
            .ok_or(PrincipalTokenError::Overflow)?;

        env.storage()
            .persistent()
            .set(&DataKey::Allowance(from.clone(), spender.clone()), &new_allowance);
        Self::do_transfer(&env, &from, &to, amount)?;

        TransferEvent { from, to, amount }.publish(&env);
        Ok(())
    }

    pub fn approve(
        env: Env,
        owner: Address,
        spender: Address,
        amount: i128,
    ) -> Result<(), PrincipalTokenError> {
        owner.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::Allowance(owner.clone(), spender.clone()), &amount);

        ApprovalEvent { owner, spender, amount }.publish(&env);
        Ok(())
    }

    pub fn mint(
        env: Env,
        to: Address,
        amount: i128,
    ) -> Result<(), PrincipalTokenError> {
        let splitter: Address = env
            .storage()
            .instance()
            .get(&DataKey::YieldSplitter)
            .ok_or(PrincipalTokenError::Unauthorized)?;
        splitter.require_auth();

        if amount <= 0 {
            return Err(PrincipalTokenError::InvalidAmount);
        }
        let active = env
            .storage()
            .instance()
            .get(&DataKey::Active)
            .unwrap_or(false);
        if !active {
            return Err(PrincipalTokenError::TokenNotActive);
        }

        let mut balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0);
        balance = balance
            .checked_add(amount)
            .ok_or(PrincipalTokenError::Overflow)?;
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to.clone()), &balance);

        let mut supply: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0);
        supply = supply
            .checked_add(amount)
            .ok_or(PrincipalTokenError::Overflow)?;
        env.storage()
            .instance()
            .set(&DataKey::TotalSupply, &supply);

        MintEvent { to, amount }.publish(&env);
        Ok(())
    }

    pub fn burn(
        env: Env,
        from: Address,
        amount: i128,
    ) -> Result<(), PrincipalTokenError> {
        let splitter: Address = env
            .storage()
            .instance()
            .get(&DataKey::YieldSplitter)
            .ok_or(PrincipalTokenError::Unauthorized)?;
        splitter.require_auth();

        if amount <= 0 {
            return Err(PrincipalTokenError::InvalidAmount);
        }

        let mut balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(from.clone()))
            .unwrap_or(0);
        if balance < amount {
            return Err(PrincipalTokenError::InsufficientBalance);
        }
        balance = balance
            .checked_sub(amount)
            .ok_or(PrincipalTokenError::Overflow)?;
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from.clone()), &balance);

        let mut supply: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0);
        supply = supply
            .checked_sub(amount)
            .ok_or(PrincipalTokenError::Overflow)?;
        env.storage()
            .instance()
            .set(&DataKey::TotalSupply, &supply);

        BurnEvent { from, amount }.publish(&env);
        Ok(())
    }

    pub fn get_underlying_asset(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::UnderlyingAsset)
    }

    pub fn get_maturity_date(env: Env) -> Option<u64> {
        env.storage().instance().get(&DataKey::MaturityDate)
    }

    pub fn get_yield_splitter(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::YieldSplitter)
    }

    pub fn is_active(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Active)
            .unwrap_or(false)
    }

    pub fn set_active(
        env: Env,
        admin: Address,
        active: bool,
    ) -> Result<(), PrincipalTokenError> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(PrincipalTokenError::Unauthorized)?;
        if admin != stored_admin {
            return Err(PrincipalTokenError::Unauthorized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Active, &active);
        Ok(())
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    fn do_transfer(
        env: &Env,
        from: &Address,
        to: &Address,
        amount: i128,
    ) -> Result<(), PrincipalTokenError> {
        if amount <= 0 {
            return Err(PrincipalTokenError::InvalidAmount);
        }

        let mut from_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(from.clone()))
            .unwrap_or(0);
        if from_balance < amount {
            return Err(PrincipalTokenError::InsufficientBalance);
        }
        from_balance = from_balance
            .checked_sub(amount)
            .ok_or(PrincipalTokenError::Overflow)?;
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from.clone()), &from_balance);

        let mut to_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0);
        to_balance = to_balance
            .checked_add(amount)
            .ok_or(PrincipalTokenError::Overflow)?;
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to.clone()), &to_balance);

        Ok(())
    }
}

#[cfg(test)]
mod test;
