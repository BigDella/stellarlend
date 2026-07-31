#![no_std]

use lending_types::{CommonError, Position, ProtocolConfig, UserPosition};
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct LendingCoreContract;

#[contractimpl]
impl LendingCoreContract {
    pub fn initialize(env: Env, admin: Address, debt_ceiling: i128) -> Result<(), CommonError> {
        if Self::get_admin(env.clone()).is_some() {
            return Err(CommonError::Unauthorized);
        }

        let config = ProtocolConfig {
            admin: admin.clone(),
            oracle: None,
            debt_ceiling,
            min_borrow_amount: 100,
            liquidation_threshold_bps: 8_000,
        };

        env.storage().instance().set(&"config", &config);
        Ok(())
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        let config: Option<ProtocolConfig> = env.storage().instance().get(&"config");
        config.map(|c| c.admin)
    }

    pub fn get_position(env: Env, user: Address) -> Option<Position> {
        env.storage().persistent().get(&user)
    }

    pub fn update_position(env: Env, user: Address, position: Position) -> Result<(), CommonError> {
        env.storage().persistent().set(&user, &position);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};
    use test_utils::TestEnv;

    #[test]
    fn test_initialize() {
        let test_env = TestEnv::new();
        let contract_id = test_env.env.register(LendingCoreContract, ());
        let client = LendingCoreContractClient::new(&test_env.env, &contract_id);

        let result = client.initialize(&test_env.admin, &1_000_000_000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_admin() {
        let test_env = TestEnv::new();
        let contract_id = test_env.env.register(LendingCoreContract, ());
        let client = LendingCoreContractClient::new(&test_env.env, &contract_id);

        client.initialize(&test_env.admin, &1_000_000_000);
        let admin = client.get_admin();
        assert_eq!(admin, Some(test_env.admin));
    }
}
