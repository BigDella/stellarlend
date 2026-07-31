use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, Env, Symbol, TryFromVal,
};

fn setup(env: &Env, ceiling: i128) -> (LendingContractClient<'_>, Address, Address, Address) {
    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let user = Address::generate(env);
    let asset = Address::generate(env);

    client.initialize(&admin, &ceiling, &1000);
    (client, admin, user, asset)
}

#[test]
fn test_rates_zero_utilization() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, _user, _asset) = setup(&env, 100_000);

    assert_eq!(client.get_utilization_bps(), 0);
    assert_eq!(client.get_borrow_rate_bps(), 100);
    assert_eq!(client.get_supply_rate_bps(), 0);
}

#[test]
fn test_rates_below_kink() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, user, asset) = setup(&env, 100_000);
    let collateral_asset = Address::generate(&env);

    client.borrow(&user, &asset, &40_000, &collateral_asset, &60_000);

    assert_eq!(client.get_utilization_bps(), 4000);

    // base 100 + (4000/8000)*2000 = 100 + 1000 = 1100
    assert_eq!(client.get_borrow_rate_bps(), 1100);
    assert_eq!(client.get_supply_rate_bps(), 900);
}

#[test]
fn test_rates_above_kink_jump_slope() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, user, asset) = setup(&env, 100_000);
    let collateral_asset = Address::generate(&env);

    client.borrow(&user, &asset, &90_000, &collateral_asset, &135_000);

    assert_eq!(client.get_utilization_bps(), 9000);

    // base 100 + slope 2000 + ((9000-8000)/(10000-8000))*10000
    // = 2100 + (1000/2000)*10000 = 2100 + 5000 = 7100
    assert_eq!(client.get_borrow_rate_bps(), 7100);
    assert_eq!(client.get_supply_rate_bps(), 6900);
}

#[test]
fn test_rate_model_update_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _user, _asset) = setup(&env, 100_000);

    client.update_interest_rate_model(
        &admin,
        &InterestRateConfigUpdate {
            model: None,
            base_rate_bps: Some(200),
            kink_utilization_bps: None,
            slope_bps: None,
            jump_slope_bps: None,
            rate_floor_bps: None,
            rate_ceiling_bps: None,
            spread_bps: None,
        },
    );

    let events = env.events().all();
    let last = events.last().unwrap();
    let topic0 = last.1.get(0).unwrap();
    let sym: Symbol = Symbol::try_from_val(&env, &topic0).unwrap();
    assert_eq!(sym, Symbol::new(&env, "interest_rate_model_updated"));

    assert_eq!(client.get_borrow_rate_bps(), 200);
}

#[test]
fn test_can_switch_between_prebuilt_rate_models() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, user, asset) = setup(&env, 100_000);
    let collateral_asset = Address::generate(&env);
    client.borrow(&user, &asset, &90_000, &collateral_asset, &135_000);

    client.update_interest_rate_model(
        &admin,
        &InterestRateConfigUpdate {
            model: Some(InterestRateModelKind::Linear as u32),
            base_rate_bps: None,
            kink_utilization_bps: None,
            slope_bps: None,
            jump_slope_bps: None,
            rate_floor_bps: None,
            rate_ceiling_bps: None,
            spread_bps: None,
        },
    );
    assert_eq!(
        client.get_interest_rate_model(),
        InterestRateModelKind::Linear
    );
    assert_eq!(client.get_borrow_rate_bps(), 1900);

    client.update_interest_rate_model(
        &admin,
        &InterestRateConfigUpdate {
            model: Some(InterestRateModelKind::Jump as u32),
            base_rate_bps: None,
            kink_utilization_bps: None,
            slope_bps: None,
            jump_slope_bps: None,
            rate_floor_bps: None,
            rate_ceiling_bps: None,
            spread_bps: None,
        },
    );
    assert_eq!(
        client.get_interest_rate_model(),
        InterestRateModelKind::Jump
    );
    assert_eq!(client.get_borrow_rate_bps(), 6900);

    client.update_interest_rate_model(
        &admin,
        &InterestRateConfigUpdate {
            model: Some(InterestRateModelKind::Exponential as u32),
            base_rate_bps: None,
            kink_utilization_bps: None,
            slope_bps: None,
            jump_slope_bps: None,
            rate_floor_bps: None,
            rate_ceiling_bps: None,
            spread_bps: None,
        },
    );
    assert_eq!(
        client.get_interest_rate_model(),
        InterestRateModelKind::Exponential
    );
    assert_eq!(client.get_borrow_rate_bps(), 9010);
}
