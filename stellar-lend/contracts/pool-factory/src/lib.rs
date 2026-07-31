#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, Env, Symbol, Vec};

#[derive(Clone)]
#[contracttype]
pub struct PoolConfig {
    pub asset: Address,
    pub oracle: Address,
    pub ltv_bps: u32,
    pub liquidation_threshold_bps: u32,
    pub interest_model: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct Pool {
    pub address: Address,
    pub config: PoolConfig,
    pub created_at: u64,
    pub created_by: Address,
}

#[contract]
pub struct PoolFactory;

#[contractimpl]
impl PoolFactory {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "admin"), &admin);

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "pool_count"), &0u64);
    }

    pub fn create_pool(
        env: Env,
        asset: Address,
        oracle: Address,
        ltv_bps: u32,
        liquidation_threshold_bps: u32,
        interest_model: Address,
    ) -> Address {
        let caller = env.current_contract_address();

        if ltv_bps > 10000 || liquidation_threshold_bps > 10000 {
            panic!("Invalid LTV or liquidation threshold");
        }

        if ltv_bps > liquidation_threshold_bps {
            panic!("LTV must be less than or equal to liquidation threshold");
        }

        let config = PoolConfig {
            asset,
            oracle,
            ltv_bps,
            liquidation_threshold_bps,
            interest_model,
        };

        let pool_count: u64 = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "pool_count"))
            .unwrap_or(0);

        let pool_index = pool_count + 1;

        let pool_address = Address::from_contract_id(&env.contract_id());

        let pool = Pool {
            address: pool_address.clone(),
            config: config.clone(),
            created_at: env.ledger().timestamp(),
            created_by: caller.clone(),
        };

        let pools_key = Symbol::new(&env, "pools");
        let mut pools: Vec<Pool> = env
            .storage()
            .instance()
            .get(&pools_key)
            .unwrap_or_else(|| Vec::new(&env));

        pools.push_back(pool);

        env.storage().instance().set(&pools_key, &pools);

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "pool_count"), &pool_index);

        env.events()
            .publish((Symbol::new(&env, "pool_created"),), pool_address.clone());

        pool_address
    }

    pub fn get_pool_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, "pool_count"))
            .unwrap_or(0)
    }

    pub fn get_pools(env: Env) -> Vec<Pool> {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, "pools"))
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn get_pool_by_index(env: Env, index: u32) -> Option<Pool> {
        let pools: Vec<Pool> = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "pools"))
            .unwrap_or_else(|| Vec::new(&env));

        if (index as usize) < pools.len() {
            Some(pools.get(index as usize).unwrap())
        } else {
            None
        }
    }

    pub fn update_pool_config(env: Env, pool_index: u32, config: PoolConfig) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin"))
            .expect("Admin not set");

        admin.require_auth();

        if config.ltv_bps > 10000 || config.liquidation_threshold_bps > 10000 {
            panic!("Invalid LTV or liquidation threshold");
        }

        let pools_key = Symbol::new(&env, "pools");
        let mut pools: Vec<Pool> = env
            .storage()
            .instance()
            .get(&pools_key)
            .unwrap_or_else(|| Vec::new(&env));

        if (pool_index as usize) < pools.len() {
            let mut pool = pools.get(pool_index as usize).unwrap();
            pool.config = config;
            pools.set(pool_index as usize, pool);
            env.storage().instance().set(&pools_key, &pools);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Env};

    #[test]
    fn test_initialize() {
        let env = Env::default();
        let contract = PoolFactoryClient::new(&env, &env.register_contract(None, PoolFactory));
        let admin = Address::random(&env);

        contract.initialize(&admin);

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin"))
            .unwrap();

        assert_eq!(stored_admin, admin);
    }

    #[test]
    fn test_create_pool() {
        let env = Env::default();
        let contract = PoolFactoryClient::new(&env, &env.register_contract(None, PoolFactory));
        let admin = Address::random(&env);
        let asset = Address::random(&env);
        let oracle = Address::random(&env);
        let interest_model = Address::random(&env);

        contract.initialize(&admin);

        let _pool = contract.create_pool(&asset, &oracle, &5000, &7500, &interest_model);

        let count = contract.get_pool_count();
        assert_eq!(count, 1);
    }
}
