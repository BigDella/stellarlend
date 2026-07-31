#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env, String, symbol_short, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum PositionNftError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    TokenNotFound = 4,
    NotOwner = 5,
    InvalidSplit = 6,
    IncompatibleMerge = 7,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum PositionType {
    Deposit = 0,
    Borrow = 1,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PositionNft {
    pub token_id: u64,
    pub owner: Address,
    pub position_type: PositionType,
    pub asset: Address,
    pub amount: i128,
    pub health_factor_bps: u32,
    pub apy_bps: u32,
    pub minted_at: u64,
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct NftMetadata {
    pub name: String,
    pub description: String,
    pub position_type: PositionType,
    pub asset: Address,
    pub amount: i128,
    pub health_factor_bps: u32,
    pub apy_bps: u32,
}

#[contract]
pub struct PositionNftContract;

#[contractimpl]
impl PositionNftContract {
    pub fn initialize(env: Env, admin: Address) -> Result<(), PositionNftError> {
        if env.storage().instance().has(&symbol_short!("admin")) {
            return Err(PositionNftError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&symbol_short!("admin"), &admin);
        env.storage().instance().set(&symbol_short!("next_id"), &1u64);
        Ok(())
    }

    pub fn mint(
        env: Env,
        owner: Address,
        position_type: PositionType,
        asset: Address,
        amount: i128,
        health_factor_bps: u32,
        apy_bps: u32,
    ) -> Result<u64, PositionNftError> {
        owner.require_auth();
        Self::require_initialized(&env)?;

        let token_id = Self::next_id(&env);

        let nft = PositionNft {
            token_id,
            owner: owner.clone(),
            position_type,
            asset,
            amount,
            health_factor_bps,
            apy_bps,
            minted_at: env.ledger().sequence() as u64,
            last_updated: env.ledger().sequence() as u64,
        };

        env.storage().persistent().set(&token_id, &nft);

        let mut owner_tokens = Self::get_owner_tokens(&env, &owner);
        owner_tokens.push_back(token_id);
        Self::set_owner_tokens(&env, &owner, &owner_tokens);

        env.events().publish(
            (symbol_short!("nft"), symbol_short!("mint")),
            (token_id, owner),
        );

        Ok(token_id)
    }

    pub fn transfer(
        env: Env,
        from: Address,
        to: Address,
        token_id: u64,
    ) -> Result<(), PositionNftError> {
        from.require_auth();

        let mut nft: PositionNft = env
            .storage()
            .persistent()
            .get(&token_id)
            .ok_or(PositionNftError::TokenNotFound)?;

        if nft.owner != from {
            return Err(PositionNftError::NotOwner);
        }

        let mut from_tokens = Self::get_owner_tokens(&env, &from);
        let mut new_from = Vec::new(&env);
        for i in 0..from_tokens.len() {
            let id = from_tokens.get(i).unwrap();
            if id != token_id {
                new_from.push_back(id);
            }
        }
        Self::set_owner_tokens(&env, &from, &new_from);

        nft.owner = to.clone();
        nft.last_updated = env.ledger().sequence() as u64;
        env.storage().persistent().set(&token_id, &nft);

        let mut to_tokens = Self::get_owner_tokens(&env, &to);
        to_tokens.push_back(token_id);
        Self::set_owner_tokens(&env, &to, &to_tokens);

        env.events().publish(
            (symbol_short!("nft"), symbol_short!("transfer")),
            (token_id, from, to),
        );

        Ok(())
    }

    pub fn split(
        env: Env,
        owner: Address,
        token_id: u64,
        split_amount: i128,
    ) -> Result<u64, PositionNftError> {
        owner.require_auth();

        let mut original: PositionNft = env
            .storage()
            .persistent()
            .get(&token_id)
            .ok_or(PositionNftError::TokenNotFound)?;

        if original.owner != owner {
            return Err(PositionNftError::NotOwner);
        }
        if split_amount <= 0 || split_amount >= original.amount {
            return Err(PositionNftError::InvalidSplit);
        }

        let new_id = Self::next_id(&env);

        let new_nft = PositionNft {
            token_id: new_id,
            owner: owner.clone(),
            position_type: original.position_type.clone(),
            asset: original.asset.clone(),
            amount: split_amount,
            health_factor_bps: original.health_factor_bps,
            apy_bps: original.apy_bps,
            minted_at: env.ledger().sequence() as u64,
            last_updated: env.ledger().sequence() as u64,
        };

        original.amount -= split_amount;
        original.last_updated = env.ledger().sequence() as u64;

        env.storage().persistent().set(&token_id, &original);
        env.storage().persistent().set(&new_id, &new_nft);

        let mut owner_tokens = Self::get_owner_tokens(&env, &owner);
        owner_tokens.push_back(new_id);
        Self::set_owner_tokens(&env, &owner, &owner_tokens);

        env.events().publish(
            (symbol_short!("nft"), symbol_short!("split")),
            (token_id, new_id, split_amount),
        );

        Ok(new_id)
    }

    pub fn merge(
        env: Env,
        owner: Address,
        token_id_a: u64,
        token_id_b: u64,
    ) -> Result<(), PositionNftError> {
        owner.require_auth();

        let mut nft_a: PositionNft = env
            .storage()
            .persistent()
            .get(&token_id_a)
            .ok_or(PositionNftError::TokenNotFound)?;
        let nft_b: PositionNft = env
            .storage()
            .persistent()
            .get(&token_id_b)
            .ok_or(PositionNftError::TokenNotFound)?;

        if nft_a.owner != owner || nft_b.owner != owner {
            return Err(PositionNftError::NotOwner);
        }
        if nft_a.asset != nft_b.asset || nft_a.position_type != nft_b.position_type {
            return Err(PositionNftError::IncompatibleMerge);
        }

        nft_a.amount += nft_b.amount;
        nft_a.last_updated = env.ledger().sequence() as u64;
        env.storage().persistent().set(&token_id_a, &nft_a);
        env.storage().persistent().remove(&token_id_b);

        let mut owner_tokens = Self::get_owner_tokens(&env, &owner);
        let mut filtered = Vec::new(&env);
        for i in 0..owner_tokens.len() {
            let id = owner_tokens.get(i).unwrap();
            if id != token_id_b {
                filtered.push_back(id);
            }
        }
        Self::set_owner_tokens(&env, &owner, &filtered);

        env.events().publish(
            (symbol_short!("nft"), symbol_short!("merge")),
            (token_id_a, token_id_b),
        );

        Ok(())
    }

    pub fn get_position(env: Env, token_id: u64) -> Option<PositionNft> {
        env.storage().persistent().get(&token_id)
    }

    pub fn get_tokens_by_owner(env: Env, owner: Address) -> Vec<u64> {
        Self::get_owner_tokens(&env, &owner)
    }

    pub fn get_metadata(env: Env, token_id: u64) -> Option<NftMetadata> {
        let nft: PositionNft = env.storage().persistent().get(&token_id)?;
        let type_str = match nft.position_type {
            PositionType::Deposit => "Deposit Position",
            PositionType::Borrow => "Borrow Position",
        };
        Some(NftMetadata {
            name: String::from_str(&env, type_str),
            description: String::from_str(&env, "StellarLend Position NFT"),
            position_type: nft.position_type,
            asset: nft.asset,
            amount: nft.amount,
            health_factor_bps: nft.health_factor_bps,
            apy_bps: nft.apy_bps,
        })
    }

    fn require_initialized(env: &Env) -> Result<(), PositionNftError> {
        if !env.storage().instance().has(&symbol_short!("admin")) {
            return Err(PositionNftError::NotInitialized);
        }
        Ok(())
    }

    fn next_id(env: &Env) -> u64 {
        let id: u64 = env.storage().instance().get(&symbol_short!("next_id")).unwrap_or(1);
        env.storage().instance().set(&symbol_short!("next_id"), &(id + 1));
        id
    }

    fn get_owner_tokens(env: &Env, owner: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&(symbol_short!("tokens"), owner.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn set_owner_tokens(env: &Env, owner: &Address, tokens: &Vec<u64>) {
        env.storage()
            .persistent()
            .set(&(symbol_short!("tokens"), owner.clone()), tokens);
    }
}
