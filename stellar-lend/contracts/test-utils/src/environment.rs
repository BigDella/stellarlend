use soroban_sdk::{Address, Env, String as SorobanString};

pub struct TestEnv {
    pub env: Env,
    pub admin: Address,
    pub users: Vec<Address>,
}

impl TestEnv {
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let users = Vec::new();

        Self { env, admin, users }
    }

    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.env.ledger().with_mut(|li| li.timestamp = timestamp);
        self
    }

    pub fn with_ledger_sequence(mut self, sequence: u32) -> Self {
        self.env
            .ledger()
            .with_mut(|li| li.sequence_number = sequence);
        self
    }

    pub fn generate_user(&mut self) -> Address {
        let user = Address::generate(&self.env);
        self.users.push(user.clone());
        user
    }

    pub fn generate_users(&mut self, count: usize) -> Vec<Address> {
        let mut generated = Vec::new();
        for _ in 0..count {
            generated.push(self.generate_user());
        }
        generated
    }

    pub fn advance_time(&mut self, seconds: u64) {
        let current = self.env.ledger().timestamp();
        self.env
            .ledger()
            .with_mut(|li| li.timestamp = current + seconds);
    }

    pub fn advance_ledger(&mut self, blocks: u32) {
        let current = self.env.ledger().sequence();
        self.env
            .ledger()
            .with_mut(|li| li.sequence_number = current + blocks);
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_string(env: &Env, value: &str) -> SorobanString {
    SorobanString::from_str(env, value)
}

pub fn setup_test_env() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    (env, admin)
}

pub fn setup_test_env_with_users(user_count: usize) -> (Env, Address, Vec<Address>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let mut users = Vec::new();
    for _ in 0..user_count {
        users.push(Address::generate(&env));
    }
    (env, admin, users)
}
