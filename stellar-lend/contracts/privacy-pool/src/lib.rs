#![no_std]
extern crate alloc;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, Bytes, BytesN, Env, Vec,
};

use stealth_address::StealthAddressRegistryClient;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PrivacyPoolError {
    InvalidAmount = 1,
    DepositPaused = 2,
    WithdrawPaused = 3,
    InvalidProof = 4,
    NullifierAlreadyUsed = 5,
    CommitmentNotFound = 6,
    InsufficientAnonymitySet = 7,
    Unauthorized = 8,
    MerkleTreeFull = 9,
    ComplianceRequired = 10,
    DisclosureInvalid = 11,
    Overflow = 12,
    AlreadyInitialized = 13,
    NotInitialized = 14,
    AssetNotSupported = 15,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrivacyPoolConfig {
    pub admin: Address,
    pub asset: Address,
    pub merkle_root: BytesN<32>,
    pub next_leaf_index: u32,
    pub tree_depth: u32,
    pub min_anonymity_set: u32,
    pub deposit_paused: bool,
    pub withdraw_paused: bool,
    pub compliance_required: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitmentNote {
    pub commitment: BytesN<32>,
    pub amount: i128,
    pub depositor: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawalProof {
    pub nullifier: BytesN<32>,
    pub commitment: BytesN<32>,
    pub recipient: Address,
    pub amount: i128,
    pub merkle_root: BytesN<32>,
    pub path_indices: Vec<u32>,
    pub siblings: Vec<BytesN<32>>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceDisclosure {
    pub depositor: Address,
    pub commitment: BytesN<32>,
}

#[contracttype]
#[derive(Clone)]
pub enum PrivacyPoolDataKey {
    Config,
    Commitment(u32),
    Nullifier(BytesN<32>),
    TotalDeposits,
    TotalWithdrawals,
    AnonymitySetSize,
    ComplianceDepositor(BytesN<32>),
    FilledSubtree(u32),
    ZeroHash(u32),
}

const MIN_ANONYMITY_SET: u32 = 10;

#[contract]
pub struct PrivacyPool;

#[contractimpl]
impl PrivacyPool {
    pub fn initialize(
        env: Env,
        admin: Address,
        asset: Address,
        tree_depth: u32,
        compliance_required: bool,
    ) -> Result<(), PrivacyPoolError> {
        if env.storage().persistent().has(&PrivacyPoolDataKey::Config) {
            return Err(PrivacyPoolError::AlreadyInitialized);
        }

        if !(10..=32).contains(&tree_depth) {
            return Err(PrivacyPoolError::InvalidProof);
        }

        for level in 0..tree_depth {
            let zh = compute_zero_hash(&env, level);
            env.storage()
                .persistent()
                .set(&PrivacyPoolDataKey::ZeroHash(level), &zh);
        }

        let config = PrivacyPoolConfig {
            admin,
            asset: asset.clone(),
            merkle_root: compute_zero_hash(&env, tree_depth - 1),
            next_leaf_index: 0,
            tree_depth,
            min_anonymity_set: MIN_ANONYMITY_SET,
            deposit_paused: false,
            withdraw_paused: false,
            compliance_required,
        };

        env.storage()
            .persistent()
            .set(&PrivacyPoolDataKey::Config, &config);

        PrivacyPoolInitializedEvent {
            asset,
            tree_depth,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(())
    }

    pub fn shielded_deposit(
        env: Env,
        from: Address,
        commitment: BytesN<32>,
        amount: i128,
        stealth_address_registry: Address,
        recipient: Address,
        ephemeral_public_key: BytesN<32>,
    ) -> Result<u32, PrivacyPoolError> {
        let config = get_config(&env)?;

        if config.deposit_paused {
            return Err(PrivacyPoolError::DepositPaused);
        }

        if amount <= 0 {
            return Err(PrivacyPoolError::InvalidAmount);
        }

        from.require_auth();

        let stealth_client = StealthAddressRegistryClient::new(&env, &stealth_address_registry);
        if !stealth_client.is_registered(&recipient) {
            return Err(PrivacyPoolError::AssetNotSupported);
        }

        let leaf_index = config.next_leaf_index;
        let max_leaves = 1u32.checked_shl(config.tree_depth).unwrap_or(u32::MAX);
        if leaf_index >= max_leaves {
            return Err(PrivacyPoolError::MerkleTreeFull);
        }

        let token_client = token::Client::new(&env, &config.asset);
        token_client.transfer(&from, env.current_contract_address(), &amount);

        let note = CommitmentNote {
            commitment: commitment.clone(),
            amount,
            depositor: recipient.clone(),
            timestamp: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&PrivacyPoolDataKey::Commitment(leaf_index), &note);

        let new_root = insert_leaf(&env, &config, leaf_index, &commitment);

        let mut new_config = config;
        new_config.next_leaf_index = leaf_index + 1;
        new_config.merkle_root = new_root.clone();
        save_config(&env, &new_config);

        update_total_deposits(&env, amount);

        env.storage().persistent().set(
            &PrivacyPoolDataKey::ComplianceDepositor(commitment.clone()),
            &recipient,
        );

        let anon_size = get_anonymity_set_size(&env);
        let new_anon_size = if leaf_index < MIN_ANONYMITY_SET {
            anon_size + 1
        } else {
            anon_size
        };
        env.storage()
            .persistent()
            .set(&PrivacyPoolDataKey::AnonymitySetSize, &new_anon_size);

        let _stealth_addr =
            stealth_client.compute_stealth_address(&recipient, &ephemeral_public_key);

        ShieldedDepositEvent {
            leaf_index,
            commitment,
            amount,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(leaf_index)
    }

    pub fn shielded_withdraw(
        env: Env,
        proof: WithdrawalProof,
        compliance_disclosure: Option<ComplianceDisclosure>,
    ) -> Result<(), PrivacyPoolError> {
        let config = get_config(&env)?;

        if config.withdraw_paused {
            return Err(PrivacyPoolError::WithdrawPaused);
        }

        if env
            .storage()
            .persistent()
            .has(&PrivacyPoolDataKey::Nullifier(proof.nullifier.clone()))
        {
            return Err(PrivacyPoolError::NullifierAlreadyUsed);
        }

        let anon_size = get_anonymity_set_size(&env);
        if anon_size < config.min_anonymity_set {
            return Err(PrivacyPoolError::InsufficientAnonymitySet);
        }

        if proof.amount <= 0 {
            return Err(PrivacyPoolError::InvalidAmount);
        }

        if proof.merkle_root != config.merkle_root {
            return Err(PrivacyPoolError::CommitmentNotFound);
        }

        if proof.path_indices.len() != config.tree_depth {
            return Err(PrivacyPoolError::InvalidProof);
        }

        if proof.siblings.len() != config.tree_depth {
            return Err(PrivacyPoolError::InvalidProof);
        }

        if config.compliance_required {
            let disclosure = compliance_disclosure
                .as_ref()
                .ok_or(PrivacyPoolError::ComplianceRequired)?;

            verify_compliance_disclosure(&env, disclosure)?;
        }

        let valid = verify_merkle_proof(
            &env,
            &config,
            &proof.commitment,
            &proof.siblings,
            &proof.path_indices,
        );
        if !valid {
            return Err(PrivacyPoolError::InvalidProof);
        }

        env.storage().persistent().set(
            &PrivacyPoolDataKey::Nullifier(proof.nullifier.clone()),
            &true,
        );

        update_total_withdrawals(&env, proof.amount);

        let token_client = token::Client::new(&env, &config.asset);
        token_client.transfer(
            &env.current_contract_address(),
            &proof.recipient,
            &proof.amount,
        );

        ShieldedWithdrawEvent {
            nullifier: proof.nullifier,
            recipient: proof.recipient,
            amount: proof.amount,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_merkle_root(env: Env) -> Result<BytesN<32>, PrivacyPoolError> {
        let config = get_config(&env)?;
        Ok(config.merkle_root)
    }

    pub fn get_commitment(env: Env, leaf_index: u32) -> Option<CommitmentNote> {
        env.storage()
            .persistent()
            .get(&PrivacyPoolDataKey::Commitment(leaf_index))
    }

    pub fn is_nullifier_used(env: Env, nullifier: BytesN<32>) -> bool {
        env.storage()
            .persistent()
            .has(&PrivacyPoolDataKey::Nullifier(nullifier))
    }

    pub fn get_total_deposits(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get(&PrivacyPoolDataKey::TotalDeposits)
            .unwrap_or(0)
    }

    pub fn get_total_withdrawals(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get(&PrivacyPoolDataKey::TotalWithdrawals)
            .unwrap_or(0)
    }

    pub fn get_anonymity_set_size(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&PrivacyPoolDataKey::AnonymitySetSize)
            .unwrap_or(0)
    }

    pub fn get_config(env: Env) -> Result<PrivacyPoolConfig, PrivacyPoolError> {
        get_config(&env)
    }

    pub fn set_pause_deposit(
        env: Env,
        admin: Address,
        paused: bool,
    ) -> Result<(), PrivacyPoolError> {
        admin.require_auth();
        let mut config = get_config(&env)?;
        if admin != config.admin {
            return Err(PrivacyPoolError::Unauthorized);
        }
        config.deposit_paused = paused;
        save_config(&env, &config);
        Ok(())
    }

    pub fn set_pause_withdraw(
        env: Env,
        admin: Address,
        paused: bool,
    ) -> Result<(), PrivacyPoolError> {
        admin.require_auth();
        let mut config = get_config(&env)?;
        if admin != config.admin {
            return Err(PrivacyPoolError::Unauthorized);
        }
        config.withdraw_paused = paused;
        save_config(&env, &config);
        Ok(())
    }

    pub fn get_filled_subtree(env: Env, level: u32) -> Option<BytesN<32>> {
        env.storage()
            .persistent()
            .get(&PrivacyPoolDataKey::FilledSubtree(level))
    }

    pub fn get_zero_hash(env: Env, level: u32) -> Option<BytesN<32>> {
        env.storage()
            .persistent()
            .get(&PrivacyPoolDataKey::ZeroHash(level))
    }

    pub fn compute_merkle_root(
        env: Env,
        commitment: BytesN<32>,
        siblings: Vec<BytesN<32>>,
        path_indices: Vec<u32>,
    ) -> Result<BytesN<32>, PrivacyPoolError> {
        let config = get_config(&env)?;
        let mut current = commitment;
        for i in 0..config.tree_depth {
            let sibling = siblings.get(i).unwrap();
            let index = path_indices.get(i).unwrap_or(0);
            current = if index == 0 {
                hash_pair(&env, &current, &sibling)
            } else {
                hash_pair(&env, &sibling, &current)
            };
        }
        Ok(current)
    }

    pub fn set_compliance_required(
        env: Env,
        admin: Address,
        required: bool,
    ) -> Result<(), PrivacyPoolError> {
        admin.require_auth();
        let mut config = get_config(&env)?;
        if admin != config.admin {
            return Err(PrivacyPoolError::Unauthorized);
        }
        config.compliance_required = required;
        save_config(&env, &config);
        Ok(())
    }
}

fn get_config(env: &Env) -> Result<PrivacyPoolConfig, PrivacyPoolError> {
    env.storage()
        .persistent()
        .get(&PrivacyPoolDataKey::Config)
        .ok_or(PrivacyPoolError::NotInitialized)
}

fn save_config(env: &Env, config: &PrivacyPoolConfig) {
    env.storage()
        .persistent()
        .set(&PrivacyPoolDataKey::Config, config);
}

fn update_total_deposits(env: &Env, amount: i128) {
    let current: i128 = env
        .storage()
        .persistent()
        .get(&PrivacyPoolDataKey::TotalDeposits)
        .unwrap_or(0);
    env.storage()
        .persistent()
        .set(&PrivacyPoolDataKey::TotalDeposits, &(current + amount));
}

fn update_total_withdrawals(env: &Env, amount: i128) {
    let current: i128 = env
        .storage()
        .persistent()
        .get(&PrivacyPoolDataKey::TotalWithdrawals)
        .unwrap_or(0);
    env.storage()
        .persistent()
        .set(&PrivacyPoolDataKey::TotalWithdrawals, &(current + amount));
}

fn get_anonymity_set_size(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&PrivacyPoolDataKey::AnonymitySetSize)
        .unwrap_or(0)
}

fn hash_pair(env: &Env, left: &BytesN<32>, right: &BytesN<32>) -> BytesN<32> {
    let mut input = Bytes::new(env);
    let left_arr: [u8; 32] = left.clone().into();
    input.append(&Bytes::from_array(env, &left_arr));
    let right_arr: [u8; 32] = right.clone().into();
    input.append(&Bytes::from_array(env, &right_arr));
    let digest = env.crypto().keccak256(&input);
    digest.into()
}

fn compute_zero_hash(env: &Env, level: u32) -> BytesN<32> {
    if level == 0 {
        let zero_leaf = BytesN::from_array(env, &[0u8; 32]);
        hash_pair(env, &zero_leaf, &zero_leaf)
    } else {
        let child = compute_zero_hash(env, level - 1);
        hash_pair(env, &child, &child)
    }
}

fn insert_leaf(
    env: &Env,
    config: &PrivacyPoolConfig,
    leaf_index: u32,
    commitment: &BytesN<32>,
) -> BytesN<32> {
    let mut current_hash = commitment.clone();
    let mut index = leaf_index;

    for level in 0..config.tree_depth {
        if index & 1 == 0 {
            let zero = env
                .storage()
                .persistent()
                .get::<_, BytesN<32>>(&PrivacyPoolDataKey::ZeroHash(level))
                .unwrap_or(BytesN::from_array(env, &[0u8; 32]));
            env.storage()
                .persistent()
                .set(&PrivacyPoolDataKey::FilledSubtree(level), &current_hash);
            current_hash = hash_pair(env, &current_hash, &zero);
        } else {
            let left = env
                .storage()
                .persistent()
                .get::<_, BytesN<32>>(&PrivacyPoolDataKey::FilledSubtree(level))
                .unwrap_or(BytesN::from_array(env, &[0u8; 32]));
            current_hash = hash_pair(env, &left, &current_hash);
        }
        index >>= 1;
    }

    current_hash
}

fn verify_merkle_proof(
    env: &Env,
    config: &PrivacyPoolConfig,
    commitment: &BytesN<32>,
    siblings: &Vec<BytesN<32>>,
    path_indices: &Vec<u32>,
) -> bool {
    let mut current_hash = commitment.clone();

    for i in 0..config.tree_depth {
        let sibling = siblings.get(i).unwrap();
        let index = path_indices.get(i).unwrap_or(0);

        current_hash = if index == 0 {
            hash_pair(env, &current_hash, &sibling)
        } else {
            hash_pair(env, &sibling, &current_hash)
        };
    }

    current_hash == config.merkle_root
}

fn verify_compliance_disclosure(
    env: &Env,
    disclosure: &ComplianceDisclosure,
) -> Result<(), PrivacyPoolError> {
    let stored: Option<Address> =
        env.storage()
            .persistent()
            .get(&PrivacyPoolDataKey::ComplianceDepositor(
                disclosure.commitment.clone(),
            ));

    match stored {
        Some(depositor) if depositor == disclosure.depositor => Ok(()),
        _ => Err(PrivacyPoolError::DisclosureInvalid),
    }
}

use soroban_sdk::contractevent;

#[contractevent]
#[derive(Clone, Debug)]
pub struct ShieldedDepositEvent {
    pub leaf_index: u32,
    pub commitment: BytesN<32>,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct ShieldedWithdrawEvent {
    pub nullifier: BytesN<32>,
    pub recipient: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct PrivacyPoolInitializedEvent {
    pub asset: Address,
    pub tree_depth: u32,
    pub timestamp: u64,
}

#[cfg(test)]
mod tests;
