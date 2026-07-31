use soroban_sdk::{contracterror, contracttype, Address, BytesN, Env, Symbol};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SignatureError {
    InvalidSignature = 1,
    InvalidNonce = 2,
    InvalidChainId = 3,
    InvalidContractAddress = 4,
    InvalidVersion = 5,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignaturePayload {
    pub chain_id: u32,
    pub contract_address: Address,
    pub version: u32,
    pub nonce: u64,
    pub payload_hash: BytesN<32>,
}

pub fn verify_signature_payload(
    env: &Env,
    payload: &SignaturePayload,
    expected_version: u32,
) -> Result<(), SignatureError> {
    // 1. Verify chain ID
    // Soroban currently doesn't expose a global chain ID natively in the same way EVM does,
    // but typically protocols store it or infer it. We assume the contract has configured it.
    // For this example, we verify it matches a configured chain ID (or pass if not strict).
    let expected_chain_id: u32 = env
        .storage()
        .instance()
        .get(&Symbol::new(env, "CHAIN_ID"))
        .unwrap_or(0);

    if expected_chain_id != 0 && payload.chain_id != expected_chain_id {
        return Err(SignatureError::InvalidChainId);
    }

    // 2. Verify Contract Address
    if payload.contract_address != env.current_contract_address() {
        return Err(SignatureError::InvalidContractAddress);
    }

    // 3. Verify Version
    if payload.version != expected_version {
        return Err(SignatureError::InvalidVersion);
    }

    Ok(())
}

pub fn consume_nonce(env: &Env, user: &Address, expected_nonce: u64) -> Result<(), SignatureError> {
    let key = (Symbol::new(env, "Nonce"), user.clone());
    let current_nonce: u64 = env.storage().persistent().get(&key).unwrap_or(0);

    if current_nonce != expected_nonce {
        return Err(SignatureError::InvalidNonce);
    }

    env.storage().persistent().set(&key, &(current_nonce + 1));
    Ok(())
}

pub fn get_nonce(env: &Env, user: &Address) -> u64 {
    let key = (Symbol::new(env, "Nonce"), user.clone());
    env.storage().persistent().get(&key).unwrap_or(0)
}
