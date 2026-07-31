use soroban_sdk::{contractimpl, Address, Env};

pub struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn initialize(
        env: Env,
        admin: Address,
        decimal: u32,
        name: soroban_sdk::String,
        symbol: soroban_sdk::String,
    ) {
        env.storage().instance().set(&"admin", &admin);
        env.storage().instance().set(&"decimal", &decimal);
        env.storage().instance().set(&"name", &name);
        env.storage().instance().set(&"symbol", &symbol);
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage().persistent().get(&id).unwrap_or(0)
    }

    pub fn transfer(_env: Env, _from: Address, _to: Address, _amount: i128) {}

    pub fn mint(env: Env, to: Address, amount: i128) {
        let balance: i128 = env.storage().persistent().get(&to).unwrap_or(0);
        env.storage().persistent().set(&to, &(balance + amount));
    }

    pub fn burn(env: Env, from: Address, amount: i128) {
        let balance: i128 = env.storage().persistent().get(&from).unwrap_or(0);
        env.storage().persistent().set(&from, &(balance - amount));
    }
}

pub struct MockOracle;

#[contractimpl]
impl MockOracle {
    pub fn initialize(env: Env, admin: Address) {
        env.storage().instance().set(&"admin", &admin);
    }

    pub fn get_price(_env: Env, _asset: Address) -> i128 {
        1_000_000
    }

    pub fn set_price(env: Env, asset: Address, price: i128) {
        env.storage().persistent().set(&asset, &price);
    }

    pub fn get_last_updated(_env: Env, _asset: Address) -> u64 {
        0
    }
}

pub fn register_mock_token(env: &Env) -> Address {
    env.register(MockToken, ())
}

pub fn register_mock_oracle(env: &Env) -> Address {
    env.register(MockOracle, ())
}
