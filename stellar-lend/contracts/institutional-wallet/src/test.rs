#![cfg(test)]

use super::*;
use crate::types::{ProposalStatus, Transaction};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    vec, Address, Env, String,
};

#[test]
fn test_initialize() {
    let env = Env::default();
    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admins = vec![&env, admin1.clone(), admin2.clone()];

    let contract_id = env.register_contract(None, InstitutionalWallet);
    let client = InstitutionalWalletClient::new(&env, &contract_id);

    client.initialize(&admins, &2);

    assert_eq!(client.get_threshold(), 2);
    assert_eq!(client.get_admins().len(), 2);
}

#[test]
fn test_propose_and_approve() {
    let env = Env::default();
    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admins = vec![&env, admin1.clone(), admin2.clone()];

    let contract_id = env.register_contract(None, InstitutionalWallet);
    let client = InstitutionalWalletClient::new(&env, &contract_id);

    client.initialize(&admins, &2);

    let tx = Transaction {
        contract: Address::generate(&env),
        function: symbol_short!("test"),
        args: vec![&env],
    };
    let batch = vec![&env, tx];

    env.mock_all_auths();
    let proposal_id = client.propose(&admin1, &String::from_str(&env, "Test proposal"), &batch);

    let proposal = client.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Active);
    assert_eq!(proposal.proposer, admin1);

    // Second admin approves
    client.approve(&admin2, &proposal_id);

    let audit = client.get_audit_trail(&proposal_id);
    assert_eq!(audit.len(), 2); // Propose + Approve
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #9)")]
fn test_execute_insufficient_approvals() {
    let env = Env::default();
    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admins = vec![&env, admin1.clone(), admin2.clone()];

    let contract_id = env.register_contract(None, InstitutionalWallet);
    let client = InstitutionalWalletClient::new(&env, &contract_id);

    client.initialize(&admins, &2);

    let tx = Transaction {
        contract: Address::generate(&env),
        function: symbol_short!("test"),
        args: vec![&env],
    };
    let batch = vec![&env, tx];

    env.mock_all_auths();
    let proposal_id = client.propose(&admin1, &String::from_str(&env, "Test proposal"), &batch);

    // Try to execute with only 1 approval (proposer's)
    client.execute(&admin1, &proposal_id);
}

#[test]
fn test_execute_success() {
    let env = Env::default();
    env.mock_all_auths();

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admins = vec![&env, admin1.clone(), admin2.clone()];

    let contract_id = env.register_contract(None, InstitutionalWallet);
    let client = InstitutionalWalletClient::new(&env, &contract_id);

    client.initialize(&admins, &2);

    // Mock a target contract for execution
    // (For simplicity in this test, we'll just check that it doesn't fail
    // when calling a dummy address, although it would in a real environment
    // without a contract registered there. But Soroban host might error if
    // address has no contract. Let's register a dummy one.)

    #[contract]
    pub struct DummyTarget;
    #[contractimpl]
    impl DummyTarget {
        pub fn test(env: Env) {}
    }
    let target_id = env.register_contract(None, DummyTarget);

    let tx = Transaction {
        contract: target_id,
        function: symbol_short!("test"),
        args: vec![&env],
    };
    let batch = vec![&env, tx];

    let proposal_id = client.propose(&admin1, &String::from_str(&env, "Test proposal"), &batch);

    client.approve(&admin2, &proposal_id);
    client.execute(&admin1, &proposal_id);

    let proposal = client.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Executed);
}

#[test]
fn test_recovery_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let admin1 = Address::generate(&env);
    let admins = vec![&env, admin1.clone()];

    let contract_id = env.register_contract(None, InstitutionalWallet);
    let client = InstitutionalWalletClient::new(&env, &contract_id);

    client.initialize(&admins, &1);

    let guardian = Address::generate(&env);
    let guardians = vec![&env, guardian.clone()];

    // Propose setting guardians
    let tx = Transaction {
        contract: contract_id.clone(),
        function: Symbol::new(&env, "set_guardians"),
        args: (guardians.clone(), 1u32).into_val(&env),
    };
    let batch = vec![&env, tx];
    let proposal_id = client.propose(&admin1, &String::from_str(&env, "Set guardians"), &batch);
    client.execute(&admin1, &proposal_id);

    // Start recovery
    let new_admin = Address::generate(&env);
    let new_admins = vec![&env, new_admin.clone()];
    client.start_recovery(&guardian, &new_admins, &1);

    // Fast forward 25 hours
    env.ledger().set(soroban_sdk::testutils::LedgerInfo {
        timestamp: 86400 + 3600,
        protocol_version: 20,
        sequence_number: 10,
        network_id: [0u8; 32],
        base_reserve: 10,
        min_persistent_entry_ttl: 100,
        min_temp_entry_ttl: 10,
        max_entry_ttl: 1000,
    });

    client.execute_recovery(&guardian);

    assert_eq!(client.get_admins().get(0).unwrap(), new_admin);
}
