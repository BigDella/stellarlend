use crate::{AdapterError, DataKey, TokenAdapterContract, TokenType};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

fn create_test_env() -> (Env, Address) {
    let env = Env::default();
    let admin = Address::generate(&env);
    (env, admin)
}

fn register_test_token(env: &Env, admin: &Address, token_type: TokenType) -> Address {
    let token_address = Address::generate(env);
    TokenAdapterContract::register_token(
        env.clone(),
        admin.clone(),
        token_address.clone(),
        token_type,
        18u32,
        Symbol::new(env, "Test Token"),
        Symbol::new(env, "TST"),
    )
    .unwrap();
    token_address
}

#[test]
fn test_initialize() {
    let (env, admin) = create_test_env();
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &env.register_contract(None, TokenAdapterContract),
            fn_name: "initialize",
            args: (&admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    TokenAdapterContract::initialize(env.clone(), admin.clone());
    assert_eq!(TokenAdapterContract::get_admin(env), Some(admin));
}

#[test]
fn test_initialize_idempotent() {
    let (env, admin) = create_test_env();
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &env.register_contract(None, TokenAdapterContract),
            fn_name: "initialize",
            args: (&admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    TokenAdapterContract::initialize(env.clone(), admin.clone());
    TokenAdapterContract::initialize(env.clone(), admin.clone());
    assert_eq!(TokenAdapterContract::get_admin(env), Some(admin));
}

#[test]
fn test_register_token() {
    let (env, admin) = create_test_env();
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &env.register_contract(None, TokenAdapterContract),
            fn_name: "initialize",
            args: (&admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    TokenAdapterContract::initialize(env.clone(), admin.clone());

    let token_address = Address::generate(&env);
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &env.register_contract(None, TokenAdapterContract),
            fn_name: "register_token",
            args: (
                &admin,
                &token_address,
                TokenType::SorobanToken,
                18u32,
                Symbol::new(&env, "Test"),
                Symbol::new(&env, "TST"),
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);

    TokenAdapterContract::register_token(
        env.clone(),
        admin.clone(),
        token_address.clone(),
        TokenType::SorobanToken,
        18u32,
        Symbol::new(&env, "Test"),
        Symbol::new(&env, "TST"),
    )
    .unwrap();

    let info = TokenAdapterContract::get_token_info(env, token_address);
    assert!(info.is_some());
    let info = info.unwrap();
    assert_eq!(info.token_type, TokenType::SorobanToken);
    assert_eq!(info.decimals, 18u32);
}

#[test]
fn test_register_token_unauthorized() {
    let (env, admin) = create_test_env();
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &env.register_contract(None, TokenAdapterContract),
            fn_name: "initialize",
            args: (&admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    TokenAdapterContract::initialize(env.clone(), admin.clone());

    let unauthorized = Address::generate(&env);
    let token_address = Address::generate(&env);
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &unauthorized,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &env.register_contract(None, TokenAdapterContract),
            fn_name: "register_token",
            args: (
                &unauthorized,
                &token_address,
                TokenType::SorobanToken,
                18u32,
                Symbol::new(&env, "Test"),
                Symbol::new(&env, "TST"),
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = TokenAdapterContract::register_token(
        env.clone(),
        unauthorized,
        token_address,
        TokenType::SorobanToken,
        18u32,
        Symbol::new(&env, "Test"),
        Symbol::new(&env, "TST"),
    );
    assert_eq!(result, Err(AdapterError::Unauthorized));
}

#[test]
fn test_transfer_invalid_amount() {
    let (env, admin) = create_test_env();
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &env.register_contract(None, TokenAdapterContract),
            fn_name: "initialize",
            args: (&admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    TokenAdapterContract::initialize(env.clone(), admin.clone());
    let token_address = register_test_token(&env, &admin, TokenType::SorobanToken);

    let from = Address::generate(&env);
    let to = Address::generate(&env);

    let result = TokenAdapterContract::transfer(env, token_address, from, to, 0);
    assert_eq!(result, Err(AdapterError::InvalidAmount));

    let result = TokenAdapterContract::transfer(env, token_address, from, to, -1);
    assert_eq!(result, Err(AdapterError::InvalidAmount));
}

#[test]
fn test_transfer_token_not_registered() {
    let (env, _admin) = create_test_env();
    let unregistered = Address::generate(&env);
    let from = Address::generate(&env);
    let to = Address::generate(&env);

    let result = TokenAdapterContract::transfer(env, unregistered, from, to, 100);
    assert_eq!(result, Err(AdapterError::TokenNotRegistered));
}

#[test]
fn test_approve_invalid_amount() {
    let (env, admin) = create_test_env();
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &env.register_contract(None, TokenAdapterContract),
            fn_name: "initialize",
            args: (&admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    TokenAdapterContract::initialize(env.clone(), admin.clone());
    let token_address = register_test_token(&env, &admin, TokenType::SorobanToken);

    let owner = Address::generate(&env);
    let spender = Address::generate(&env);

    let result = TokenAdapterContract::approve(env, token_address, owner, spender, -1);
    assert_eq!(result, Err(AdapterError::InvalidAmount));
}

#[test]
fn test_normalize_amount_same_decimals() {
    let (env, admin) = create_test_env();
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &env.register_contract(None, TokenAdapterContract),
            fn_name: "initialize",
            args: (&admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    TokenAdapterContract::initialize(env.clone(), admin.clone());
    let token_address = register_test_token(&env, &admin, TokenType::SorobanToken);

    let result = TokenAdapterContract::normalize_amount(env, token_address, 1000, 18);
    assert_eq!(result, Ok(1000));
}

#[test]
fn test_normalize_amount_downscale() {
    let (env, admin) = create_test_env();
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &env.register_contract(None, TokenAdapterContract),
            fn_name: "initialize",
            args: (&admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    TokenAdapterContract::initialize(env.clone(), admin.clone());
    let token_address = register_test_token(&env, &admin, TokenType::SorobanToken);

    let result =
        TokenAdapterContract::normalize_amount(env, token_address, 1_000_000_000_000_000_000, 18);
    assert_eq!(result, Ok(1));
}

#[test]
fn test_normalize_amount_upscale() {
    let (env, admin) = create_test_env();
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &env.register_contract(None, TokenAdapterContract),
            fn_name: "initialize",
            args: (&admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    TokenAdapterContract::initialize(env.clone(), admin.clone());
    let token_address = register_test_token(&env, &admin, TokenType::SorobanToken);

    let result = TokenAdapterContract::normalize_amount(env, token_address, 1, 6);
    assert_eq!(result, Ok(1_000_000_000_000));
}

#[test]
fn test_to_minimal_unit() {
    let (env, admin) = create_test_env();
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &env.register_contract(None, TokenAdapterContract),
            fn_name: "initialize",
            args: (&admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    TokenAdapterContract::initialize(env.clone(), admin.clone());
    let token_address = register_test_token(&env, &admin, TokenType::SorobanToken);

    let result = TokenAdapterContract::to_minimal_unit(env, token_address, 1);
    assert_eq!(result, Ok(1_000_000_000_000_000_000));
}

#[test]
fn test_from_minimal_unit() {
    let (env, admin) = create_test_env();
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &env.register_contract(None, TokenAdapterContract),
            fn_name: "initialize",
            args: (&admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    TokenAdapterContract::initialize(env.clone(), admin.clone());
    let token_address = register_test_token(&env, &admin, TokenType::SorobanToken);

    let result =
        TokenAdapterContract::from_minimal_unit(env, token_address, 1_000_000_000_000_000_000);
    assert_eq!(result, Ok(1));
}

#[test]
fn test_get_admin() {
    let (env, _admin) = create_test_env();
    assert_eq!(TokenAdapterContract::get_admin(env), None);
}

#[test]
fn test_get_token_info() {
    let (env, _admin) = create_test_env();
    let unregistered = Address::generate(&env);
    assert_eq!(
        TokenAdapterContract::get_token_info(env, unregistered),
        None
    );
}
