#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Address, Bytes, BytesN, Env, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum StealthError {
    AlreadyRegistered = 1,
    NotRegistered = 2,
    Unauthorized = 3,
    InvalidPublicKey = 4,
    InvalidViewTag = 5,
    RegistrationPaused = 6,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StealthMetaAddress {
    pub spend_public_key: BytesN<32>,
    pub view_public_key: BytesN<32>,
    pub scheme_id: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StealthAddress {
    pub stealth_public_key: BytesN<32>,
    pub ephemeral_public_key: BytesN<32>,
    pub view_tag: BytesN<16>,
}

#[contracttype]
#[derive(Clone)]
pub enum StealthDataKey {
    MetaAddress(Address),
    RegisteredCount,
    RecipientsList,
    StealthAddressMeta(Address),
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct StealthRegisteredEvent {
    pub user: Address,
    pub spend_public_key: BytesN<32>,
    pub view_public_key: BytesN<32>,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct StealthAddressGeneratedEvent {
    pub recipient: Address,
    pub stealth_address: BytesN<32>,
    pub ephemeral_public_key: BytesN<32>,
    pub timestamp: u64,
}

use soroban_sdk::contractevent;

const MAX_REGISTRANTS: u32 = 10000;

#[contract]
pub struct StealthAddressRegistry;

#[contractimpl]
impl StealthAddressRegistry {
    pub fn register(
        env: Env,
        user: Address,
        spend_public_key: BytesN<32>,
        view_public_key: BytesN<32>,
    ) -> Result<(), StealthError> {
        user.require_auth();

        if env
            .storage()
            .persistent()
            .has(&StealthDataKey::MetaAddress(user.clone()))
        {
            return Err(StealthError::AlreadyRegistered);
        }

        let count: u32 = env
            .storage()
            .persistent()
            .get(&StealthDataKey::RegisteredCount)
            .unwrap_or(0);

        if count >= MAX_REGISTRANTS {
            return Err(StealthError::RegistrationPaused);
        }

        let meta = StealthMetaAddress {
            spend_public_key: spend_public_key.clone(),
            view_public_key: view_public_key.clone(),
            scheme_id: 1,
        };

        env.storage()
            .persistent()
            .set(&StealthDataKey::MetaAddress(user.clone()), &meta);

        env.storage()
            .persistent()
            .set(&StealthDataKey::RegisteredCount, &(count + 1));

        let mut list: Vec<Address> = env
            .storage()
            .persistent()
            .get(&StealthDataKey::RecipientsList)
            .unwrap_or(Vec::new(&env));

        list.push_back(user.clone());

        env.storage()
            .persistent()
            .set(&StealthDataKey::RecipientsList, &list);

        StealthRegisteredEvent {
            user,
            spend_public_key,
            view_public_key,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_meta_address(env: Env, user: Address) -> Option<StealthMetaAddress> {
        env.storage()
            .persistent()
            .get(&StealthDataKey::MetaAddress(user))
    }

    pub fn compute_stealth_address(
        env: Env,
        recipient: Address,
        ephemeral_public_key: BytesN<32>,
    ) -> Result<StealthAddress, StealthError> {
        let meta = env
            .storage()
            .persistent()
            .get::<_, StealthMetaAddress>(&StealthDataKey::MetaAddress(recipient.clone()))
            .ok_or(StealthError::NotRegistered)?;

        let shared_secret =
            compute_shared_secret(&env, &meta.spend_public_key, &ephemeral_public_key);
        let stealth_public_key = derive_stealth_key(&env, &meta.spend_public_key, &shared_secret);
        let view_tag = compute_view_tag(&env, &shared_secret);

        let addr = StealthAddress {
            stealth_public_key: stealth_public_key.clone(),
            ephemeral_public_key: ephemeral_public_key.clone(),
            view_tag,
        };

        env.storage().persistent().set(
            &StealthDataKey::StealthAddressMeta(recipient.clone()),
            &addr,
        );

        StealthAddressGeneratedEvent {
            recipient: recipient.clone(),
            stealth_address: stealth_public_key,
            ephemeral_public_key,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(addr)
    }

    pub fn get_stealth_address(env: Env, user: Address) -> Option<StealthAddress> {
        env.storage()
            .persistent()
            .get(&StealthDataKey::StealthAddressMeta(user))
    }

    pub fn is_registered(env: Env, user: Address) -> bool {
        env.storage()
            .persistent()
            .has(&StealthDataKey::MetaAddress(user))
    }

    pub fn get_registered_count(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&StealthDataKey::RegisteredCount)
            .unwrap_or(0)
    }

    pub fn get_all_recipients(env: Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&StealthDataKey::RecipientsList)
            .unwrap_or(Vec::new(&env))
    }
}

fn compute_shared_secret(
    env: &Env,
    spend_public_key: &BytesN<32>,
    ephemeral_public_key: &BytesN<32>,
) -> BytesN<32> {
    let mut input = Bytes::new(env);
    let spend_arr: [u8; 32] = spend_public_key.clone().into();
    input.append(&Bytes::from_array(env, &spend_arr));
    let ephemeral_arr: [u8; 32] = ephemeral_public_key.clone().into();
    input.append(&Bytes::from_array(env, &ephemeral_arr));
    let digest = env.crypto().keccak256(&input);
    digest.into()
}

fn derive_stealth_key(
    env: &Env,
    spend_public_key: &BytesN<32>,
    shared_secret: &BytesN<32>,
) -> BytesN<32> {
    let mut input = Bytes::new(env);
    let spend_arr: [u8; 32] = spend_public_key.clone().into();
    input.append(&Bytes::from_array(env, &spend_arr));
    let secret_arr: [u8; 32] = shared_secret.clone().into();
    input.append(&Bytes::from_array(env, &secret_arr));
    input.append(&Bytes::from_array(env, &[0u8; 1]));
    let digest = env.crypto().keccak256(&input);
    digest.into()
}

fn compute_view_tag(env: &Env, shared_secret: &BytesN<32>) -> BytesN<16> {
    let mut input = Bytes::new(env);
    let secret_arr: [u8; 32] = shared_secret.clone().into();
    input.append(&Bytes::from_array(env, &secret_arr));
    input.append(&Bytes::from_array(env, &[1u8; 1]));
    let digest = env.crypto().keccak256(&input);
    let arr: [u8; 32] = digest.into();
    let mut tag = [0u8; 16];
    tag.copy_from_slice(&arr[..16]);
    BytesN::from_array(env, &tag)
}

#[cfg(test)]
mod tests;
