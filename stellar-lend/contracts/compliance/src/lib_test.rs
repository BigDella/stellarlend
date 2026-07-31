#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

use crate::{ComplianceContract, ComplianceContractClient, ComplianceError, TransactionLimits};

fn setup() -> (Env, Address, ComplianceContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(ComplianceContract, ());
    let client = ComplianceContractClient::new(&env, &contract_id);
    (env, admin, client)
}

#[test]
fn test_initialize() {
    let (env, admin, client) = setup();
    let result = client.initialize(&admin);
    assert!(result.is_ok());
}

#[test]
fn test_initialize_cannot_double_init() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();
    let result = client.initialize(&admin);
    assert_eq!(result.unwrap_err(), ComplianceError::Unauthorized);
}

#[test]
fn test_add_and_check_sanction() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let target = Address::generate(&env);
    let source = Symbol::new(&env, "OFAC");
    let reason = Symbol::new(&env, "sanctioned_entity");

    client.add_sanction(&admin, &target, &source, &reason, &None);
    assert!(client.check_sanctioned(&target));
}

#[test]
fn test_remove_sanction() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let target = Address::generate(&env);
    let source = Symbol::new(&env, "OFAC");
    let reason = Symbol::new(&env, "sanctioned_entity");

    client.add_sanction(&admin, &target, &source, &reason, &None);
    assert!(client.check_sanctioned(&target));

    client.remove_sanction(&admin, &target);
    assert!(!client.check_sanctioned(&target));
}

#[test]
fn test_double_sanction_fails() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let target = Address::generate(&env);
    let source = Symbol::new(&env, "OFAC");
    let reason = Symbol::new(&env, "sanctioned_entity");

    client.add_sanction(&admin, &target, &source, &reason, &None);
    let result = client.add_sanction(&admin, &target, &source, &reason, &None);
    assert_eq!(result.unwrap_err(), ComplianceError::AlreadySanctioned);
}

#[test]
fn test_remove_unsanctioned_fails() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let target = Address::generate(&env);
    let result = client.remove_sanction(&admin, &target);
    assert_eq!(result.unwrap_err(), ComplianceError::AddressNotSanctioned);
}

#[test]
fn test_set_and_check_kyc() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let user = Address::generate(&env);
    let jurisdiction = Symbol::new(&env, "US");
    let provider = Symbol::new(&env, "Jumio");

    client.set_kyc_verification(&admin, &user, &1, &jurisdiction, &provider, &31536000);
    assert!(client.check_kyc(&user));

    let kyc = client.get_kyc(&user).unwrap();
    assert_eq!(kyc.verified, true);
    assert_eq!(kyc.tier, 1);
    assert_eq!(kyc.jurisdiction, jurisdiction);
    assert_eq!(kyc.kyc_provider, provider);
}

#[test]
fn test_revoke_kyc() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let user = Address::generate(&env);
    let jurisdiction = Symbol::new(&env, "US");
    let provider = Symbol::new(&env, "Jumio");

    client.set_kyc_verification(&admin, &user, &1, &jurisdiction, &provider, &31536000);
    assert!(client.check_kyc(&user));

    client.revoke_kyc(&admin, &user);
    assert!(!client.check_kyc(&user));
}

#[test]
fn test_unauthorized_admin_fails() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let unauthorized = Address::generate(&env);
    let target = Address::generate(&env);
    let source = Symbol::new(&env, "OFAC");
    let reason = Symbol::new(&env, "sanctioned_entity");

    let result = client.add_sanction(&unauthorized, &target, &source, &reason, &None);
    assert_eq!(result.unwrap_err(), ComplianceError::Unauthorized);
}

#[test]
fn test_set_and_get_tx_limits() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let user = Address::generate(&env);
    let limits = TransactionLimits {
        daily_limit: 500_000_000_000,
        weekly_limit: 2_000_000_000_000,
        max_single_tx: 100_000_000_000,
    };

    client.set_tx_limits(&admin, &user, &limits);
    let result = client.get_tx_limits(&user);
    assert_eq!(result.daily_limit, 500_000_000_000);
    assert_eq!(result.weekly_limit, 2_000_000_000_000);
    assert_eq!(result.max_single_tx, 100_000_000_000);
}

#[test]
fn test_check_transaction_passes() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let asset = Address::generate(&env);

    let result = client.check_transaction(&from, &to, &100_000_000, &asset);
    assert!(result.is_ok());
}

#[test]
fn test_check_transaction_sanctioned_fails() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let asset = Address::generate(&env);

    let source = Symbol::new(&env, "OFAC");
    let reason = Symbol::new(&env, "sanctioned");
    client.add_sanction(&admin, &from, &source, &reason, &None);

    let result = client.check_transaction(&from, &to, &100_000_000, &asset);
    assert_eq!(result.unwrap_err(), ComplianceError::AddressSanctioned);
}

#[test]
fn test_check_transaction_too_large_fails() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let asset = Address::generate(&env);

    let limits = TransactionLimits {
        daily_limit: 1_000_000_000_000,
        weekly_limit: 5_000_000_000_000,
        max_single_tx: 100,
    };
    client.set_tx_limits(&admin, &from, &limits);

    let result = client.check_transaction(&from, &to, &200, &asset);
    assert_eq!(result.unwrap_err(), ComplianceError::TransactionTooLarge);
}

#[test]
fn test_file_and_get_sar() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let target = Address::generate(&env);
    let asset = Address::generate(&env);
    let reason = Symbol::new(&env, "unusual_activity");

    let sar_id = client.file_sar(&admin, &target, &reason, &500_000, &asset);
    assert_eq!(sar_id, 0);

    let sar = client.get_sar(&0).unwrap();
    assert_eq!(sar.address, target);
    assert_eq!(sar.amount, 500_000);
    assert_eq!(sar.sar_id, 0);

    let sar_id2 = client.file_sar(&admin, &target, &reason, &1_000_000, &asset);
    assert_eq!(sar_id2, 1);
}

#[test]
fn test_add_and_remove_restricted_jurisdiction() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let jurisdiction = Symbol::new(&env, "KP");
    client.add_restricted_jurisdiction(&admin, &jurisdiction);

    let config = client.get_config();
    assert!(config.restricted_jurisdictions.contains(&jurisdiction));

    client.remove_restricted_jurisdiction(&admin, &jurisdiction);
    let config = client.get_config();
    assert!(!config.restricted_jurisdictions.contains(&jurisdiction));
}

#[test]
fn test_pause_unpause() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    client.pause(&admin);
    let config = client.get_config();
    assert!(config.paused);

    let from = Address::generate(&env);
    let to = Address::generate(&env);
    let asset = Address::generate(&env);
    let result = client.check_transaction(&from, &to, &100, &asset);
    assert_eq!(result.unwrap_err(), ComplianceError::CompliancePaused);

    client.unpause(&admin);
    let config = client.get_config();
    assert!(!config.paused);
}

#[test]
fn test_geographic_restriction() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let user = Address::generate(&env);
    let jurisdiction = Symbol::new(&env, "KP");
    let provider = Symbol::new(&env, "Jumio");

    client.add_restricted_jurisdiction(&admin, &jurisdiction);
    client.set_kyc_verification(&admin, &user, &1, &jurisdiction, &provider, &31536000);

    let to = Address::generate(&env);
    let asset = Address::generate(&env);
    let result = client.check_transaction(&user, &to, &100, &asset);
    assert_eq!(result.unwrap_err(), ComplianceError::GeographicRestricted);
}

#[test]
fn test_get_config_defaults() {
    let (env, admin, client) = setup();
    client.initialize(&admin).unwrap();

    let config = client.get_config();
    assert_eq!(config.admin, admin);
    assert!(!config.paused);
    assert_eq!(config.default_limits.daily_limit, 1_000_000_000_000);
    assert_eq!(config.default_limits.weekly_limit, 5_000_000_000_000);
    assert_eq!(config.default_limits.max_single_tx, 500_000_000_000);
}
