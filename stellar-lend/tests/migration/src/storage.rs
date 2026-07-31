use soroban_sdk::{contract, contractimpl, Bytes, Env, String as SorobanString};

#[contract]
pub struct TestStorage;

#[contractimpl]
impl TestStorage {
    pub fn noop(_env: Env) {}
}

pub struct StorageContext {
    pub env: Env,
    contract_id: soroban_sdk::Address,
}

impl StorageContext {
    pub fn new(env: &Env) -> Self {
        let contract_id = env.register(TestStorage, ());
        Self {
            env: env.clone(),
            contract_id,
        }
    }

    pub fn data_save(&self, key: &SorobanString, value: &Bytes) {
        self.env.as_contract(&self.contract_id, || {
            self.env.storage().persistent().set(key, value);
        });
    }

    pub fn data_load(&self, key: &SorobanString) -> Bytes {
        self.env.as_contract(&self.contract_id, || {
            self.env
                .storage()
                .persistent()
                .get(key)
                .unwrap_or(Bytes::new(&self.env))
        })
    }

    pub fn schema_version(&self) -> u32 {
        self.env.as_contract(&self.contract_id, || {
            let key = SorobanString::from_str(&self.env, "__schema_version");
            self.env
                .storage()
                .persistent()
                .get(&key)
                .unwrap_or(0u32)
        })
    }

    pub fn bump_schema_version(&self, new_version: u32) -> Result<u32, String> {
        self.env.as_contract(&self.contract_id, || {
            let key = SorobanString::from_str(&self.env, "__schema_version");
            let current: u32 = self
                .env
                .storage()
                .persistent()
                .get(&key)
                .unwrap_or(0);
            if new_version <= current {
                return Err(format!(
                    "InvalidVersion: {} <= {}",
                    new_version, current
                ));
            }
            self.env.storage().persistent().set(&key, &new_version);
            Ok(new_version)
        })
    }

    pub fn set_schema_version(&self, version: u32) {
        self.env.as_contract(&self.contract_id, || {
            let key = SorobanString::from_str(&self.env, "__schema_version");
            self.env.storage().persistent().set(&key, &version);
        });
    }

    pub fn save_with_count(&self, key: &SorobanString, value: &Bytes) {
        self.env.as_contract(&self.contract_id, || {
            let count_key = SorobanString::from_str(&self.env, "__entry_count");
            let current: u32 = self
                .env
                .storage()
                .persistent()
                .get(&count_key)
                .unwrap_or(0);
            self.env
                .storage()
                .persistent()
                .set(&count_key, &(current + 1));
            self.env.storage().persistent().set(key, value);
        });
    }
}
