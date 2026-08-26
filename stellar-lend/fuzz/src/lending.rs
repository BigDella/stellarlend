use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger},
    Address, Env, IntoVal, Symbol,
};

use stellarlend_lending::{LendingContract, LendingContractClient, PauseType};

use crate::encoding::{parse_actions, ActionBytes};

const MAX_ACTIONS: usize = 64;
const NUM_USERS: usize = 4;
const NUM_ASSETS: usize = 4;

/// Fuzz-only oracle with mutable prices per asset.
#[contract]
pub struct FuzzOracle;

#[contractimpl]
impl FuzzOracle {
    pub fn set_price(env: Env, asset: Address, price: i128) {
        env.storage().persistent().set(&asset, &price);
    }

    pub fn price(env: Env, asset: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&asset)
            .unwrap_or(100_000_000)
    }
}

#[derive(Clone)]
struct LendingHarness {
    env: Env,
    admin: Address,
    oracle: Address,
    contract_id: Address,
    users: [Address; NUM_USERS],
    assets: [Address; NUM_ASSETS],
}

impl LendingHarness {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        env.ledger().with_mut(|li| li.timestamp = 1);

        let admin = Address::generate(&env);
        let contract_id = env.register(LendingContract, ());
        let client = LendingContractClient::new(&env, &contract_id);

        // Basic initialization so we can reach deeper states quickly.
        let _ = client.initialize(&admin, &i128::MAX, &1);
        let _ = client.initialize_deposit_settings(&i128::MAX, &0);
        let _ = client.initialize_withdraw_settings(&0);

        let oracle = env.register(FuzzOracle, ());
        let _ = client.set_oracle(&admin, &oracle);

        let users = [
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        ];
        let assets = [
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        ];

        Self {
            env,
            admin,
            oracle,
            contract_id,
            users,
            assets,
        }
    }

    fn user(&self, idx: u8) -> Address {
        self.users[(idx as usize) % NUM_USERS].clone()
    }

    fn asset(&self, idx: u8) -> Address {
        self.assets[(idx as usize) % NUM_ASSETS].clone()
    }

    fn set_time_delta(&self, delta: u64) {
        if delta == 0 {
            return;
        }
        self.env.ledger().with_mut(|li| {
            li.timestamp = li.timestamp.saturating_add(delta);
        });
    }

    fn set_oracle_price(&self, asset: &Address, raw: i64) {
        // Prices are i128 with 8 decimals; keep them non-negative and within a sane range.
        let abs = (raw as i128).abs();
        let price = (abs % 1_000_000_000_000_i128).max(1); // [1, 1e12)
        let _ = self.env.invoke_contract::<()>(
            &self.oracle,
            &Symbol::new(&self.env, "set_price"),
            (asset.clone(), price).into_val(&self.env),
        );
    }

    fn bounded_amount(raw: i64) -> i128 {
        i128::from(raw.unsigned_abs() % 1_000_000_000).max(1)
    }

    fn liquidate_from_action(&self, client: &LendingContractClient<'_>, action: ActionBytes) {
        let liquidator = self.user(action.user());
        let borrower = self.user(action.asset_a());
        let debt_asset = self.asset(action.asset_b());
        let collateral_asset = self.asset(action.u32_param() as u8);
        let amt = Self::bounded_amount(action.i64_a());
        let _ = client.try_liquidate(&liquidator, &borrower, &debt_asset, &collateral_asset, &amt);
    }

    fn act(&self, client: &LendingContractClient<'_>, action: ActionBytes) {
        match action.kind() % 11 {
            // 0: deposit(vault)
            0 => {
                let user = self.user(action.user());
                let asset = self.asset(action.asset_a());
                let amt = i128::from(action.i64_a());
                let _ = client.try_deposit(&user, &asset, &amt);
            }
            // 1: withdraw(vault)
            1 => {
                let user = self.user(action.user());
                let asset = self.asset(action.asset_a());
                let amt = i128::from(action.i64_a());
                let _ = client.try_withdraw(&user, &asset, &amt);
            }
            // 2: borrow
            2 => {
                let user = self.user(action.user());
                let debt_asset = self.asset(action.asset_a());
                let collateral_asset = self.asset(action.asset_b());
                let borrow_amt = i128::from(action.i64_a());
                let collateral_amt = i128::from(action.i64_b());
                let _ = client.try_borrow(
                    &user,
                    &debt_asset,
                    &borrow_amt,
                    &collateral_asset,
                    &collateral_amt,
                );
            }
            // 3: repay
            3 => {
                let user = self.user(action.user());
                let asset = self.asset(action.asset_a());
                let amt = i128::from(action.i64_a());
                let _ = client.try_repay(&user, &asset, &amt);
            }
            // 4: deposit_collateral (borrow module)
            4 => {
                let user = self.user(action.user());
                let asset = self.asset(action.asset_a());
                let amt = i128::from(action.i64_a());
                let _ = client.try_deposit_collateral(&user, &asset, &amt);
            }
            // 5: set_pause
            5 => {
                let pause_type = match action.u32_param() % 5 {
                    0 => PauseType::Deposit,
                    1 => PauseType::Borrow,
                    2 => PauseType::Repay,
                    3 => PauseType::Withdraw,
                    _ => PauseType::Liquidation,
                };
                let paused = (action.u64_tail() & 1) == 1;
                let _ = client.try_set_pause(&self.admin, &pause_type, &paused);
            }
            // 6: set_liquidation_threshold_bps
            6 => {
                let bps = (action.i64_a().unsigned_abs() as i128) % 20_000;
                let _ = client.try_set_liquidation_threshold_bps(&self.admin, &bps);
            }
            // 7: oracle set_price
            7 => {
                let asset = self.asset(action.asset_a());
                self.set_oracle_price(&asset, action.i64_a());
            }
            // 8: advance time
            8 => {
                // Cap to ~1 year per step to keep things reasonable.
                let delta = action.u64_tail() % 31_536_000;
                self.set_time_delta(delta);
            }
            // 9: call views/getters
            9 => {
                let user = self.user(action.user());
                let _ = client.get_user_position(&user);
                let _ = client.get_health_factor(&user);
                let _ = client.get_collateral_value(&user);
                let _ = client.get_debt_value(&user);
            }
            // 10: liquidate
            _ => self.liquidate_from_action(client, action),
        }
    }

    fn act_critical(&self, client: &LendingContractClient<'_>, action: ActionBytes) {
        match action.kind() % 6 {
            // 0: deposit vault collateral
            0 => {
                let user = self.user(action.user());
                let asset = self.asset(action.asset_a());
                let amt = Self::bounded_amount(action.i64_a());
                let _ = client.try_deposit(&user, &asset, &amt);
            }
            // 1: borrow with over-collateralized input so the fuzzer reaches debt states.
            1 => {
                let user = self.user(action.user());
                let debt_asset = self.asset(action.asset_a());
                let collateral_asset = self.asset(action.asset_b());
                let borrow_amt = Self::bounded_amount(action.i64_a());
                let extra_collateral = Self::bounded_amount(action.i64_b());
                let collateral_amt = borrow_amt
                    .saturating_mul(2)
                    .saturating_add(extra_collateral);
                let _ = client.try_borrow(
                    &user,
                    &debt_asset,
                    &borrow_amt,
                    &collateral_asset,
                    &collateral_amt,
                );
            }
            // 2: repay debt.
            2 => {
                let user = self.user(action.user());
                let asset = self.asset(action.asset_a());
                let amt = Self::bounded_amount(action.i64_a());
                let _ = client.try_repay(&user, &asset, &amt);
            }
            // 3: liquidate debt positions.
            3 => self.liquidate_from_action(client, action),
            // 4: mutate oracle prices to stress collateral and liquidation views.
            4 => {
                let asset = self.asset(action.asset_a());
                self.set_oracle_price(&asset, action.i64_a());
            }
            // 5: advance ledger time and query the accounting views.
            _ => {
                self.set_time_delta(action.u64_tail() % 604_800);
                let user = self.user(action.user());
                let _ = client.get_user_debt(&user);
                let _ = client.get_user_position(&user);
                let _ = client.get_health_factor(&user);
            }
        }
    }

    fn assert_invariants(&self, client: &LendingContractClient<'_>) {
        for u in self.users.iter() {
            let pos = client.get_user_position(u);
            let debt = client.get_user_debt(u);

            assert!(pos.collateral_balance >= 0);
            assert!(pos.debt_balance >= 0);
            assert!(debt.borrowed_amount >= 0);
            assert!(debt.interest_accrued >= 0);

            // If the user has no debt, health factor must be the sentinel (even if oracle is unset).
            if pos.debt_balance == 0 {
                assert_eq!(pos.health_factor, 100_000_000);
            }

            // Position summary must match individual getters.
            assert_eq!(pos.collateral_balance, client.get_collateral_balance(u));
            assert_eq!(pos.debt_balance, client.get_debt_balance(u));
        }
    }
}

pub fn run(data: &[u8]) {
    let h = LendingHarness::new();
    let client = LendingContractClient::new(&h.env, &h.contract_id);
    for action in parse_actions(data, MAX_ACTIONS) {
        h.act(&client, action);
    }
    h.assert_invariants(&client);
}

pub fn run_critical(data: &[u8]) {
    let h = LendingHarness::new();
    let client = LendingContractClient::new(&h.env, &h.contract_id);
    for action in parse_actions(data, MAX_ACTIONS) {
        h.act_critical(&client, action);
        h.assert_invariants(&client);
    }
}
