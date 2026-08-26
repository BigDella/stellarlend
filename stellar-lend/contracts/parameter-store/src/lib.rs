#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec};

pub mod hello_world_bridge;

pub const BPS_DIVISOR: i128 = 10_000;
pub const RISK_TIMELOCK_SECONDS: u64 = 48 * 3600;
pub const STANDARD_TIMELOCK_SECONDS: u64 = 24 * 3600;
pub const EMERGENCY_TIMELOCK_SECONDS: u64 = 4 * 3600;

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum ParameterType {
    LTV,
    LiquidationThreshold,
    CloseFactor,
    LiquidationIncentive,
    ReserveFactor,
    DebtCeiling,
    BaseInterestRate,
    Slope1,
    Slope2,
    OptimalUtilization,
}

impl ParameterType {
    pub fn min_timelock(&self) -> u64 {
        match self {
            ParameterType::LTV
            | ParameterType::LiquidationThreshold
            | ParameterType::CloseFactor
            | ParameterType::LiquidationIncentive => RISK_TIMELOCK_SECONDS,
            _ => STANDARD_TIMELOCK_SECONDS,
        }
    }

    pub fn is_risk_parameter(&self) -> bool {
        matches!(
            self,
            ParameterType::LTV
                | ParameterType::LiquidationThreshold
                | ParameterType::CloseFactor
                | ParameterType::LiquidationIncentive
        )
    }

    pub fn validate_range(&self, value: i128) -> bool {
        match self {
            ParameterType::LTV => value > 0 && value <= 9_000,
            ParameterType::LiquidationThreshold => value > 0 && value <= BPS_DIVISOR,
            ParameterType::CloseFactor => value > 0 && value <= BPS_DIVISOR,
            ParameterType::LiquidationIncentive => value >= 1_000 && value <= 2_000,
            ParameterType::ReserveFactor => value >= 0 && value <= BPS_DIVISOR,
            ParameterType::DebtCeiling => value >= 0,
            ParameterType::BaseInterestRate => value >= 0 && value <= 5_000,
            ParameterType::Slope1 => value >= 0 && value <= BPS_DIVISOR,
            ParameterType::Slope2 => value >= 0 && value <= 50_000,
            ParameterType::OptimalUtilization => value > 0 && value < BPS_DIVISOR,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct ParameterChange {
    pub parameter: ParameterType,
    pub old_value: i128,
    pub new_value: i128,
    pub timestamp: u64,
    pub effective_at: u64,
    pub changed_by: Address,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct ParameterProposal {
    pub id: u64,
    pub pool: Address,
    pub parameter: ParameterType,
    pub proposed_value: i128,
    pub proposer: Address,
    pub created_at: u64,
    pub effective_at: u64,
    pub accepted: bool,
    pub rejected: bool,
    pub is_emergency: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct PoolRegistration {
    pub pool: Address,
    pub registered_at: u64,
    pub active: bool,
}

#[contract]
pub struct ParameterStoreContract;

#[contractimpl]
impl ParameterStoreContract {
    pub fn initialize(env: Env, governance: Address, admin: Address) {
        env.storage().instance().set(&DataKey::Governance, &governance);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::ProposalCounter, &0u64);
        env.storage().instance().set(&DataKey::PoolCounter, &0u64);
        env.storage().instance().set(&DataKey::EmergencyOverrideActive, &false);
    }

    pub fn register_pool(env: Env, pool: Address) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();

        let counter: u64 = env.storage().instance().get(&DataKey::PoolCounter).unwrap_or(0);
        let pool_id = counter + 1;
        let registration = PoolRegistration {
            pool: pool.clone(),
            registered_at: env.ledger().timestamp(),
            active: true,
        };
        env.storage().instance().set(&DataKey::Pool(pool_id), &registration);
        env.storage().instance().set(&DataKey::PoolAddress(pool.clone()), &pool_id);
        env.storage().instance().set(&DataKey::PoolCounter, &pool_id);
        env.events().publish(("register_pool",), &pool);
    }

    pub fn propose_change(
        env: Env,
        pool: Address,
        parameter: ParameterType,
        value: i128,
        timelock_seconds: u64,
    ) -> u64 {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();

        assert!(
            env.storage().instance().has(&DataKey::PoolAddress(pool.clone())),
            "Pool not registered"
        );
        assert!(parameter.validate_range(value), "Parameter value out of range");

        let min_timelock = parameter.min_timelock();
        assert!(timelock_seconds >= min_timelock, "Timelock too short: min {min_timelock}s");

        let counter: u64 = env.storage().instance().get(&DataKey::ProposalCounter).unwrap_or(0);
        let proposal_id = counter + 1;
        let current_timestamp = env.ledger().timestamp();
        let effective_at = current_timestamp + timelock_seconds;

        let param_clone = parameter.clone();
        let proposal = ParameterProposal {
            id: proposal_id,
            pool: pool.clone(),
            parameter,
            proposed_value: value,
            proposer: governance.clone(),
            created_at: current_timestamp,
            effective_at,
            accepted: false,
            rejected: false,
            is_emergency: false,
        };

        env.storage().instance().set(&DataKey::Proposal(proposal_id), &proposal);
        env.storage().instance().set(&DataKey::ProposalCounter, &proposal_id);
        env.events().publish(("propose_change", &param_clone), &proposal_id);

        proposal_id
    }

    pub fn propose_emergency_change(
        env: Env,
        pool: Address,
        parameter: ParameterType,
        value: i128,
    ) -> u64 {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();

        assert!(parameter.is_risk_parameter(), "Emergency override only for risk params");
        assert!(
            env.storage().instance().has(&DataKey::PoolAddress(pool.clone())),
            "Pool not registered"
        );
        assert!(parameter.validate_range(value), "Parameter value out of range");

        let counter: u64 = env.storage().instance().get(&DataKey::ProposalCounter).unwrap_or(0);
        let proposal_id = counter + 1;
        let current_timestamp = env.ledger().timestamp();
        let effective_at = current_timestamp + EMERGENCY_TIMELOCK_SECONDS;

        let param_clone = parameter.clone();
        let proposal = ParameterProposal {
            id: proposal_id,
            pool: pool.clone(),
            parameter,
            proposed_value: value,
            proposer: governance.clone(),
            created_at: current_timestamp,
            effective_at,
            accepted: false,
            rejected: false,
            is_emergency: true,
        };

        env.storage().instance().set(&DataKey::Proposal(proposal_id), &proposal);
        env.storage().instance().set(&DataKey::ProposalCounter, &proposal_id);
        env.events().publish(("propose_emergency", &param_clone), &proposal_id);

        proposal_id
    }

    pub fn accept_proposal(env: Env, proposal_id: u64) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();

        let mut proposal: ParameterProposal = env
            .storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .expect("Proposal not found");

        let current_timestamp = env.ledger().timestamp();
        assert!(current_timestamp >= proposal.effective_at, "Timelock not elapsed");
        assert!(!proposal.accepted && !proposal.rejected, "Proposal already decided");

        proposal.accepted = true;
        env.storage().instance().set(&DataKey::Proposal(proposal_id), &proposal);

        let key = DataKey::Parameter(proposal.parameter.clone(), proposal.pool.clone());
        let old_value: i128 = env.storage().instance().get(&key).unwrap_or(0);

        let change = ParameterChange {
            parameter: proposal.parameter.clone(),
            old_value,
            new_value: proposal.proposed_value,
            timestamp: current_timestamp,
            effective_at: proposal.effective_at,
            changed_by: proposal.proposer.clone(),
        };

        env.storage().instance().set(&key, &proposal.proposed_value);

        let history_key = DataKey::ChangeHistory(proposal.parameter.clone(), proposal.pool.clone());
        let mut history: Vec<ParameterChange> = env
            .storage()
            .instance()
            .get(&history_key)
            .unwrap_or_else(|| Vec::new(&env));
        history.push_back(change);
        env.storage().instance().set(&history_key, &history);

        env.events().publish(("accept_proposal", &proposal.parameter), &proposal_id);
    }

    pub fn execute_emergency_override(env: Env, proposal_id: u64) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();

        let mut proposal: ParameterProposal = env
            .storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .expect("Proposal not found");

        assert!(proposal.is_emergency, "Not an emergency proposal");
        assert!(!proposal.accepted && !proposal.rejected, "Proposal already decided");

        let current_timestamp = env.ledger().timestamp();
        assert!(current_timestamp >= proposal.effective_at, "Emergency timelock not elapsed");

        proposal.accepted = true;
        env.storage().instance().set(&DataKey::Proposal(proposal_id), &proposal);

        let key = DataKey::Parameter(proposal.parameter.clone(), proposal.pool.clone());
        let old_value: i128 = env.storage().instance().get(&key).unwrap_or(0);

        let change = ParameterChange {
            parameter: proposal.parameter.clone(),
            old_value,
            new_value: proposal.proposed_value,
            timestamp: current_timestamp,
            effective_at: proposal.effective_at,
            changed_by: proposal.proposer.clone(),
        };

        env.storage().instance().set(&key, &proposal.proposed_value);
        env.storage().instance().set(&DataKey::EmergencyOverrideActive, &true);

        let history_key = DataKey::ChangeHistory(proposal.parameter.clone(), proposal.pool.clone());
        let mut history: Vec<ParameterChange> = env
            .storage()
            .instance()
            .get(&history_key)
            .unwrap_or_else(|| Vec::new(&env));
        history.push_back(change);
        env.storage().instance().set(&history_key, &history);

        env.events().publish(("emergency_override",), &proposal_id);
    }

    pub fn clear_emergency_override(env: Env) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();
        env.storage().instance().set(&DataKey::EmergencyOverrideActive, &false);
        env.events().publish(("clear_emergency",), &());
    }

    pub fn reject_proposal(env: Env, proposal_id: u64) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();
        let mut proposal: ParameterProposal = env
            .storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .expect("Proposal not found");
        assert!(!proposal.accepted && !proposal.rejected, "Proposal already decided");
        proposal.rejected = true;
        env.storage().instance().set(&DataKey::Proposal(proposal_id), &proposal);
        env.events().publish(("reject_proposal",), &proposal_id);
    }

    pub fn get_parameter(env: Env, parameter: ParameterType, pool: Address) -> i128 {
        let _pool_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PoolAddress(pool.clone()))
            .expect("Pool not registered");
        let key = DataKey::Parameter(parameter, pool.clone());
        env.storage().instance().get(&key).unwrap_or(0)
    }

    pub fn get_proposal(env: Env, proposal_id: u64) -> ParameterProposal {
        env.storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .expect("Proposal not found")
    }

    pub fn get_change_history(env: Env, parameter: ParameterType, pool: Address) -> Vec<ParameterChange> {
        let key = DataKey::ChangeHistory(parameter, pool);
        env.storage().instance().get(&key).unwrap_or_else(|| Vec::new(&env))
    }

    pub fn is_emergency_active(env: Env) -> bool {
        env.storage().instance().get(&DataKey::EmergencyOverrideActive).unwrap_or(false)
    }
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Governance,
    Admin,
    ProposalCounter,
    PoolCounter,
    Pool(u64),
    PoolAddress(Address),
    Proposal(u64),
    Parameter(ParameterType, Address),
    ChangeHistory(ParameterType, Address),
    EmergencyOverrideActive,
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke};
    use soroban_sdk::{IntoVal, Vec};

    struct TestEnv {
        env: Env,
        contract_id: Address,
        governance: Address,
        admin: Address,
    }

    fn setup() -> TestEnv {
        let env = Env::default();
        let governance = Address::generate(&env);
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ParameterStoreContract);
        let client = ParameterStoreContractClient::new(&env, &contract_id);
        client.initialize(&governance, &admin);
        TestEnv { env, contract_id, governance, admin }
    }

    fn with_governance_auth<T>(
        te: &TestEnv,
        fn_name: &str,
        args: impl IntoVal<Env, Vec<soroban_sdk::Val>>,
        f: impl FnOnce() -> T,
    ) -> T {
        te.env.mock_auths(&[MockAuth {
            address: &te.governance,
            invoke: &MockAuthInvoke {
                contract: &te.contract_id,
                fn_name,
                args: args.into_val(&te.env),
                sub_invokes: &[],
            },
        }]);
        f()
    }

    fn client(te: &TestEnv) -> ParameterStoreContractClient<'_> {
        ParameterStoreContractClient::new(&te.env, &te.contract_id)
    }

    #[test]
    fn test_initialize() {
        let te = setup();
        let stored: Address = te.env
            .as_contract(&te.contract_id, || te.env.storage().instance().get(&DataKey::Governance))
            .unwrap();
        assert_eq!(stored, te.governance);
    }

    #[test]
    fn test_register_pool() {
        let te = setup();
        let pool = Address::generate(&te.env);
        with_governance_auth(&te, "register_pool", (&pool,), || {
            client(&te).register_pool(&pool);
        });
        let stored: u64 = te.env
            .as_contract(&te.contract_id, || te.env.storage().instance().get(&DataKey::PoolCounter))
            .unwrap();
        assert_eq!(stored, 1);
    }

    #[test]
    fn test_propose_and_accept_change() {
        let te = setup();
        let pool = Address::generate(&te.env);
        with_governance_auth(&te, "register_pool", (&pool,), || {
            client(&te).register_pool(&pool);
        });

        with_governance_auth(&te, "propose_change", (&pool, &ParameterType::LiquidationThreshold, &8_000i128, &RISK_TIMELOCK_SECONDS), || {
            let proposal_id = client(&te).propose_change(
                &pool, &ParameterType::LiquidationThreshold, &8_000, &RISK_TIMELOCK_SECONDS,
            );
            assert_eq!(proposal_id, 1);
        });

        te.env.ledger().set_timestamp(RISK_TIMELOCK_SECONDS + 1);

        with_governance_auth(&te, "accept_proposal", (&1u64,), || {
            client(&te).accept_proposal(&1);
        });

        let history = client(&te).get_change_history(&ParameterType::LiquidationThreshold, &pool);
        assert_eq!(history.len(), 1);
    }

    #[test]
    #[should_panic(expected = "Timelock too short")]
    fn test_propose_timelock_too_short() {
        let te = setup();
        let pool = Address::generate(&te.env);
        with_governance_auth(&te, "register_pool", (&pool,), || {
            client(&te).register_pool(&pool);
        });
        with_governance_auth(&te, "propose_change", (&pool, &ParameterType::LTV, &7_000i128, &3600u64), || {
            client(&te).propose_change(&pool, &ParameterType::LTV, &7_000, &3600);
        });
    }

    #[test]
    #[should_panic(expected = "Timelock not elapsed")]
    fn test_accept_before_timelock() {
        let te = setup();
        let pool = Address::generate(&te.env);
        with_governance_auth(&te, "register_pool", (&pool,), || {
            client(&te).register_pool(&pool);
        });
        with_governance_auth(&te, "propose_change", (&pool, &ParameterType::LTV, &7_000i128, &RISK_TIMELOCK_SECONDS), || {
            client(&te).propose_change(&pool, &ParameterType::LTV, &7_000, &RISK_TIMELOCK_SECONDS);
        });
        with_governance_auth(&te, "accept_proposal", (&1u64,), || {
            client(&te).accept_proposal(&1);
        });
    }

    #[test]
    #[should_panic(expected = "Parameter value out of range")]
    fn test_ltv_out_of_range() {
        let te = setup();
        let pool = Address::generate(&te.env);
        with_governance_auth(&te, "register_pool", (&pool,), || {
            client(&te).register_pool(&pool);
        });
        with_governance_auth(&te, "propose_change", (&pool, &ParameterType::LTV, &9_500i128, &RISK_TIMELOCK_SECONDS), || {
            client(&te).propose_change(&pool, &ParameterType::LTV, &9_500, &RISK_TIMELOCK_SECONDS);
        });
    }

    #[test]
    fn test_emergency_proposal() {
        let te = setup();
        let pool = Address::generate(&te.env);
        with_governance_auth(&te, "register_pool", (&pool,), || {
            client(&te).register_pool(&pool);
        });
        with_governance_auth(&te, "propose_emergency_change", (&pool, &ParameterType::LiquidationThreshold, &7_500i128), || {
            client(&te).propose_emergency_change(&pool, &ParameterType::LiquidationThreshold, &7_500);
        });

        te.env.ledger().set_timestamp(EMERGENCY_TIMELOCK_SECONDS + 1);

        with_governance_auth(&te, "execute_emergency_override", (&1u64,), || {
            client(&te).execute_emergency_override(&1);
        });

        assert!(client(&te).is_emergency_active());
    }

    #[test]
    fn test_clear_emergency_override() {
        let te = setup();
        let pool = Address::generate(&te.env);
        with_governance_auth(&te, "register_pool", (&pool,), || {
            client(&te).register_pool(&pool);
        });
        with_governance_auth(&te, "propose_emergency_change", (&pool, &ParameterType::LTV, &5_000i128), || {
            client(&te).propose_emergency_change(&pool, &ParameterType::LTV, &5_000);
        });
        te.env.ledger().set_timestamp(EMERGENCY_TIMELOCK_SECONDS + 1);
        with_governance_auth(&te, "execute_emergency_override", (&1u64,), || {
            client(&te).execute_emergency_override(&1);
        });
        assert!(client(&te).is_emergency_active());

        with_governance_auth(&te, "clear_emergency_override", (), || {
            client(&te).clear_emergency_override();
        });
        assert!(!client(&te).is_emergency_active());
    }

    #[test]
    fn test_reject_proposal() {
        let te = setup();
        let pool = Address::generate(&te.env);
        with_governance_auth(&te, "register_pool", (&pool,), || {
            client(&te).register_pool(&pool);
        });
        with_governance_auth(&te, "propose_change", (&pool, &ParameterType::ReserveFactor, &1_000i128, &STANDARD_TIMELOCK_SECONDS), || {
            client(&te).propose_change(&pool, &ParameterType::ReserveFactor, &1_000, &STANDARD_TIMELOCK_SECONDS);
        });
        with_governance_auth(&te, "reject_proposal", (&1u64,), || {
            client(&te).reject_proposal(&1);
        });

        let proposal = client(&te).get_proposal(&1);
        assert!(proposal.rejected);
        assert!(!proposal.accepted);
    }

    #[test]
    fn test_parameter_type_validation() {
        assert!(ParameterType::LTV.validate_range(5_000));
        assert!(!ParameterType::LTV.validate_range(0));
        assert!(!ParameterType::LTV.validate_range(9_500));
        assert!(ParameterType::LiquidationIncentive.validate_range(1_000));
        assert!(!ParameterType::LiquidationIncentive.validate_range(500));
        assert!(!ParameterType::LiquidationIncentive.validate_range(2_500));
    }

    #[test]
    fn test_multiple_different_parameters() {
        let te = setup();
        let pool = Address::generate(&te.env);
        with_governance_auth(&te, "register_pool", (&pool,), || {
            client(&te).register_pool(&pool);
        });
        with_governance_auth(&te, "propose_change", (&pool, &ParameterType::LTV, &6_500i128, &RISK_TIMELOCK_SECONDS), || {
            client(&te).propose_change(&pool, &ParameterType::LTV, &6_500, &RISK_TIMELOCK_SECONDS);
        });
        with_governance_auth(&te, "propose_change", (&pool, &ParameterType::ReserveFactor, &500i128, &STANDARD_TIMELOCK_SECONDS), || {
            client(&te).propose_change(&pool, &ParameterType::ReserveFactor, &500, &STANDARD_TIMELOCK_SECONDS);
        });
    }
}
