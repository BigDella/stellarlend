use super::*;
use soroban_sdk::{testutils::Address as _, Env};

fn setup_test() -> (Env, StealthAddressRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(StealthAddressRegistry, ());
    let client = StealthAddressRegistryClient::new(&env, &contract_id);

    (env, client)
}

fn generate_key(env: &Env) -> BytesN<32> {
    let arr: [u8; 32] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f, 0x20,
    ];
    BytesN::from_array(env, &arr)
}

#[test]
fn test_register_success() {
    let (env, client) = setup_test();
    let user = Address::generate(&env);
    let spend_key = generate_key(&env);
    let view_key = generate_key(&env);

    client.register(&user, &spend_key, &view_key);

    assert!(client.is_registered(&user));
    assert_eq!(client.get_registered_count(), 1);

    let meta = client.get_meta_address(&user).unwrap();
    assert_eq!(meta.spend_public_key, spend_key);
    assert_eq!(meta.view_public_key, view_key);
    assert_eq!(meta.scheme_id, 1);
}

#[test]
fn test_register_duplicate_fails() {
    let (env, client) = setup_test();
    let user = Address::generate(&env);
    let spend_key = generate_key(&env);
    let view_key = generate_key(&env);

    client.register(&user, &spend_key, &view_key);

    let result = client.try_register(&user, &spend_key, &view_key);
    assert_eq!(result, Err(Ok(StealthError::AlreadyRegistered)));
    assert_eq!(client.get_registered_count(), 1);
}

#[test]
fn test_is_registered() {
    let (env, client) = setup_test();
    let user = Address::generate(&env);
    let other = Address::generate(&env);
    let spend_key = generate_key(&env);
    let view_key = generate_key(&env);

    assert!(!client.is_registered(&user));
    assert!(!client.is_registered(&other));

    client.register(&user, &spend_key, &view_key);

    assert!(client.is_registered(&user));
    assert!(!client.is_registered(&other));
}

#[test]
fn test_get_registered_count() {
    let (env, client) = setup_test();
    assert_eq!(client.get_registered_count(), 0);

    for i in 0..5u8 {
        let user = Address::generate(&env);
        let mut spend_arr = [0u8; 32];
        spend_arr[0] = i + 1;
        let spend_key = BytesN::from_array(&env, &spend_arr);
        let mut view_arr = [0u8; 32];
        view_arr[31] = i + 1;
        let view_key = BytesN::from_array(&env, &view_arr);

        client.register(&user, &spend_key, &view_key);
        assert_eq!(client.get_registered_count(), (i as u32) + 1);
    }
}

#[test]
fn test_compute_stealth_address() {
    let (env, client) = setup_test();
    let user = Address::generate(&env);
    let spend_key = generate_key(&env);
    let view_key = generate_key(&env);

    client.register(&user, &spend_key, &view_key);

    let ephemeral_key = {
        let arr: [u8; 32] = [
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
            0x99, 0x00, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0xab, 0xcd, 0xef, 0x01,
            0x23, 0x45, 0x67, 0x89,
        ];
        BytesN::from_array(&env, &arr)
    };

    let addr = client.compute_stealth_address(&user, &ephemeral_key);
    assert_eq!(addr.ephemeral_public_key, ephemeral_key);
    assert_ne!(addr.stealth_public_key, spend_key);

    let stored = client.get_stealth_address(&user).unwrap();
    assert_eq!(stored.stealth_public_key, addr.stealth_public_key);
}

#[test]
fn test_get_all_recipients() {
    let (env, client) = setup_test();

    let initial = client.get_all_recipients();
    assert_eq!(initial.len(), 0);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let spend_key = generate_key(&env);
    let view_key = generate_key(&env);

    client.register(&user1, &spend_key, &view_key);
    client.register(&user2, &spend_key, &view_key);

    let recipients = client.get_all_recipients();
    assert_eq!(recipients.len(), 2);
    assert_eq!(recipients.get(0).unwrap(), user1);
    assert_eq!(recipients.get(1).unwrap(), user2);
}

#[test]
fn test_get_meta_address_not_registered() {
    let (env, client) = setup_test();
    let user = Address::generate(&env);

    let meta = client.get_meta_address(&user);
    assert!(meta.is_none());
}

#[test]
fn test_deterministic_stealth_address() {
    let (env, client) = setup_test();
    let user = Address::generate(&env);
    let spend_key = generate_key(&env);
    let view_key = generate_key(&env);

    client.register(&user, &spend_key, &view_key);

    let ephemeral_key = generate_key(&env);

    let addr1 = client.compute_stealth_address(&user, &ephemeral_key);
    let addr2 = client.compute_stealth_address(&user, &ephemeral_key);

    assert_eq!(addr1.stealth_public_key, addr2.stealth_public_key);
    assert_eq!(addr1.view_tag, addr2.view_tag);
}

#[test]
fn test_register_multiple_users() {
    let (env, client) = setup_test();

    for i in 0u8..15u8 {
        let user = Address::generate(&env);
        let mut spend_arr = [0u8; 32];
        spend_arr[0] = i + 1;
        let spend_key = BytesN::from_array(&env, &spend_arr);
        let mut view_arr = [0u8; 32];
        view_arr[31] = i + 1;
        let view_key = BytesN::from_array(&env, &view_arr);

        client.register(&user, &spend_key, &view_key);
        assert!(client.is_registered(&user));
    }

    assert_eq!(client.get_registered_count(), 15);
}
