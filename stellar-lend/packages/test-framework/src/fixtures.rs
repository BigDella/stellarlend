use soroban_sdk::{Address, Env, Vec as SorobanVec};

pub struct ContractFixture {
    pub env: Env,
    pub admin: Address,
    pub governance: Address,
    pub oracle_addresses: SorobanVec<Address>,
}

impl ContractFixture {
    pub fn new(env: Env) -> Self {
        ContractFixture {
            env,
            admin: Address::generate(&env),
            governance: Address::generate(&env),
            oracle_addresses: SorobanVec::new(&env),
        }
    }

    pub fn with_admin(mut self, admin: Address) -> Self {
        self.admin = admin;
        self
    }

    pub fn with_governance(mut self, governance: Address) -> Self {
        self.governance = governance;
        self
    }

    pub fn with_oracles(mut self, count: usize) -> Self {
        for _ in 0..count {
            self.oracle_addresses
                .push_back(Address::generate(&self.env));
        }
        self
    }
}

pub struct FixtureBuilder {
    env: Env,
}

impl FixtureBuilder {
    pub fn new(env: Env) -> Self {
        FixtureBuilder { env }
    }

    pub fn build(self) -> ContractFixture {
        ContractFixture::new(self.env)
    }

    pub fn with_oracle_count(self, count: usize) -> ContractFixture {
        ContractFixture::new(self.env).with_oracles(count)
    }
}
