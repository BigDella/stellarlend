use soroban_sdk::{Address, Env, String, Vec};
use stellar_lend::hello_world::HelloContractClient;

pub fn setup_governance_contract(
    env: &Env,
    admin: &Address,
    vote_token: &Address,
) -> HelloContractClient<'static> {
    let contract = HelloContractClient::new(env, &Address::generate(env));

    contract.governance_initialize(
        admin,
        vote_token,
        &Some(100000),
        &Some(100000),
        &Some(5000),
        &Some(0),
        &Some(86400),
        &Some(6000),
    );

    contract
}

pub fn setup_vote_token(env: &Env, token: &Address, user: &Address, amount: i128) {
    // In a real implementation, this would mint tokens to the user
    // For testing purposes, we assume tokens are available
}
