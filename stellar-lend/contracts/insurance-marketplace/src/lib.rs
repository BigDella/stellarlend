#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, Env, String, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotFound = 2,
    Unauthorized = 3,
    InvalidAmount = 4,
    Insolvent = 5,
    Expired = 6,
    TriggerNotCovered = 7,
    AlreadyClaimed = 8,
}

#[contracttype]
#[derive(Clone)]
pub struct Provider {
    pub owner: Address,
    pub collateral_token: Address,
    pub collateral: i128,
    pub available: i128,
    pub kyc_approved: bool,
    pub rating: u32,
}
#[contracttype]
#[derive(Clone)]
pub struct Policy {
    pub id: u64,
    pub provider: Address,
    pub coverage: i128,
    pub premium_bps: u32,
    pub duration_ledgers: u32,
    pub terms: String,
    pub triggers: Vec<u32>,
    pub active: bool,
}
#[contracttype]
#[derive(Clone)]
pub struct Coverage {
    pub id: u64,
    pub policy_id: u64,
    pub lender: Address,
    pub position_id: u64,
    pub amount: i128,
    pub expires_at: u32,
    pub claimed: bool,
}
#[contracttype]
#[derive(Clone)]
enum Key {
    Admin,
    Provider(Address),
    Policy(u64),
    Coverage(u64),
    Nonce,
}

#[contract]
pub struct InsuranceMarketplace;

#[contractimpl]
impl InsuranceMarketplace {
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&Key::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&Key::Admin, &admin);
        env.storage().instance().set(&Key::Nonce, &0u64);
        Ok(())
    }
    pub fn onboard_provider(
        env: Env,
        owner: Address,
        token: Address,
        collateral: i128,
        kyc_approved: bool,
    ) -> Result<(), Error> {
        owner.require_auth();
        if collateral <= 0 {
            return Err(Error::InvalidAmount);
        }
        token::Client::new(&env, &token).transfer(
            &owner,
            &env.current_contract_address(),
            &collateral,
        );
        env.storage().persistent().set(
            &Key::Provider(owner.clone()),
            &Provider {
                owner,
                collateral_token: token,
                collateral,
                available: collateral,
                kyc_approved,
                rating: 0,
            },
        );
        Ok(())
    }
    pub fn create_policy(
        env: Env,
        provider: Address,
        coverage: i128,
        premium_bps: u32,
        duration_ledgers: u32,
        terms: String,
        triggers: Vec<u32>,
    ) -> Result<u64, Error> {
        provider.require_auth();
        let p: Provider = env
            .storage()
            .persistent()
            .get(&Key::Provider(provider.clone()))
            .ok_or(Error::NotFound)?;
        if !p.kyc_approved {
            return Err(Error::Unauthorized);
        }
        if coverage <= 0 || coverage > p.available {
            return Err(Error::Insolvent);
        }
        let id: u64 = env.storage().instance().get(&Key::Nonce).unwrap_or(0);
        env.storage().instance().set(&Key::Nonce, &(id + 1));
        env.storage().persistent().set(
            &Key::Policy(id),
            &Policy {
                id,
                provider,
                coverage,
                premium_bps,
                duration_ledgers,
                terms,
                triggers,
                active: true,
            },
        );
        Ok(id)
    }
    pub fn purchase(
        env: Env,
        policy_id: u64,
        lender: Address,
        position_id: u64,
        amount: i128,
    ) -> Result<u64, Error> {
        lender.require_auth();
        let policy: Policy = env
            .storage()
            .persistent()
            .get(&Key::Policy(policy_id))
            .ok_or(Error::NotFound)?;
        let mut provider: Provider = env
            .storage()
            .persistent()
            .get(&Key::Provider(policy.provider.clone()))
            .ok_or(Error::NotFound)?;
        if !policy.active || amount <= 0 || amount > policy.coverage || amount > provider.available
        {
            return Err(Error::Insolvent);
        }
        let premium = amount
            .checked_mul(policy.premium_bps as i128)
            .ok_or(Error::InvalidAmount)?
            / 10_000;
        token::Client::new(&env, &provider.collateral_token).transfer(
            &lender,
            &policy.provider,
            &premium,
        );
        provider.available -= amount;
        env.storage()
            .persistent()
            .set(&Key::Provider(policy.provider), &provider);
        let id: u64 = env.storage().instance().get(&Key::Nonce).unwrap_or(0);
        env.storage().instance().set(&Key::Nonce, &(id + 1));
        env.storage().persistent().set(
            &Key::Coverage(id),
            &Coverage {
                id,
                policy_id,
                lender,
                position_id,
                amount,
                expires_at: env
                    .ledger()
                    .sequence()
                    .saturating_add(policy.duration_ledgers),
                claimed: false,
            },
        );
        Ok(id)
    }
    pub fn trigger_claim(env: Env, coverage_id: u64, trigger: u32) -> Result<i128, Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&Key::Admin)
            .ok_or(Error::NotFound)?;
        admin.require_auth();
        let mut coverage: Coverage = env
            .storage()
            .persistent()
            .get(&Key::Coverage(coverage_id))
            .ok_or(Error::NotFound)?;
        if coverage.claimed {
            return Err(Error::AlreadyClaimed);
        }
        if env.ledger().sequence() > coverage.expires_at {
            return Err(Error::Expired);
        }
        let policy: Policy = env
            .storage()
            .persistent()
            .get(&Key::Policy(coverage.policy_id))
            .ok_or(Error::NotFound)?;
        if !policy.triggers.iter().any(|item| item == trigger) {
            return Err(Error::TriggerNotCovered);
        }
        let mut provider: Provider = env
            .storage()
            .persistent()
            .get(&Key::Provider(policy.provider.clone()))
            .ok_or(Error::NotFound)?;
        token::Client::new(&env, &provider.collateral_token).transfer(
            &env.current_contract_address(),
            &coverage.lender,
            &coverage.amount,
        );
        coverage.claimed = true;
        provider.collateral -= coverage.amount;
        env.storage()
            .persistent()
            .set(&Key::Coverage(coverage_id), &coverage);
        env.storage()
            .persistent()
            .set(&Key::Provider(policy.provider), &provider);
        Ok(coverage.amount)
    }
    pub fn get_policy(env: Env, id: u64) -> Option<Policy> {
        env.storage().persistent().get(&Key::Policy(id))
    }
    pub fn get_coverage(env: Env, id: u64) -> Option<Coverage> {
        env.storage().persistent().get(&Key::Coverage(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, token, vec, Address, Env, String};

    #[test]
    fn purchases_coverage_and_pays_approved_claim() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let provider = Address::generate(&env);
        let lender = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(token_admin);
        let token_address = asset.address();
        let asset_client = token::StellarAssetClient::new(&env, &token_address);
        asset_client.mint(&provider, &10_000);
        asset_client.mint(&lender, &1_000);

        let contract_address = env.register(InsuranceMarketplace, ());
        let client = InsuranceMarketplaceClient::new(&env, &contract_address);
        client.initialize(&admin);
        client.onboard_provider(&provider, &token_address, &10_000, &true);

        let policy_id = client.create_policy(
            &provider,
            &5_000,
            &100,
            &100,
            &String::from_str(&env, "Oracle and contract risk"),
            &vec![&env, 1],
        );
        let coverage_id = client.purchase(&policy_id, &lender, &42, &5_000);

        assert_eq!(asset_client.balance(&provider), 50);
        assert_eq!(asset_client.balance(&contract_address), 10_000);

        assert_eq!(client.trigger_claim(&coverage_id, &1), 5_000);
        assert_eq!(asset_client.balance(&lender), 5_950);
        assert!(client.get_coverage(&coverage_id).unwrap().claimed);
    }

    #[test]
    fn rejects_uncovered_claim_trigger() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let provider = Address::generate(&env);
        let lender = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(Address::generate(&env));
        let token_address = asset.address();
        let asset_client = token::StellarAssetClient::new(&env, &token_address);
        asset_client.mint(&provider, &10_000);
        asset_client.mint(&lender, &1_000);
        let contract_address = env.register(InsuranceMarketplace, ());
        let client = InsuranceMarketplaceClient::new(&env, &contract_address);
        client.initialize(&admin);
        client.onboard_provider(&provider, &token_address, &10_000, &true);
        let policy_id = client.create_policy(
            &provider,
            &5_000,
            &100,
            &100,
            &String::from_str(&env, "Oracle failures only"),
            &vec![&env, 1],
        );
        let coverage_id = client.purchase(&policy_id, &lender, &42, &5_000);

        assert_eq!(
            client.try_trigger_claim(&coverage_id, &2),
            Err(Ok(Error::TriggerNotCovered))
        );
    }
}
