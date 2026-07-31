use crate::oracle::{OracleConfig, OracleIncidentKind};
use crate::{HelloContract, HelloContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, Env,
};

fn create_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn setup(env: &Env) -> (Address, HelloContractClient<'_>) {
    let contract_id = env.register(HelloContract, ());
    let client = HelloContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (admin, client)
}

fn monitoring_config() -> OracleConfig {
    OracleConfig {
        max_deviation_bps: 10000,
        max_staleness_seconds: 3600,
        cache_ttl_seconds: 0,
        min_price: 1,
        max_price: i128::MAX,
        twap_window_seconds: 0,
        max_observations: 64,
        min_sources: 1,
        outlier_deviation_bps: 10000,
        breaker_deviation_bps: 10000,
        breaker_cooldown_seconds: 60,
    }
}

#[test]
fn source_deviation_above_two_percent_generates_alert_report() {
    let env = create_env();
    let (admin, client) = setup(&env);
    let asset = Address::generate(&env);
    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let sources = vec![&env, s1.clone(), s2.clone()];

    client.configure_oracle(&admin, &monitoring_config());
    client.set_oracle_sources(&admin, &asset, &sources);
    client.update_price_feed(&admin, &asset, &100_000_000, &8, &s1);
    client.update_price_feed(&admin, &asset, &102_500_000, &8, &s2);

    let report = client.get_oracle_incident_report(&asset).unwrap();
    assert_eq!(report.kind, OracleIncidentKind::SourceDeviationAlert);
    assert!(report.observed_bps > 200);
    assert_eq!(
        client.get_oracle_circuit_breaker_state(&asset).open_until,
        0
    );
}

#[test]
fn source_deviation_above_ten_percent_pauses_oracle() {
    let env = create_env();
    let (admin, client) = setup(&env);
    let asset = Address::generate(&env);
    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let sources = vec![&env, s1.clone(), s2.clone()];

    client.configure_oracle(&admin, &monitoring_config());
    client.set_oracle_sources(&admin, &asset, &sources);
    client.update_price_feed(&admin, &asset, &100_000_000, &8, &s1);
    client.update_price_feed(&admin, &asset, &111_000_000, &8, &s2);

    let report = client.get_oracle_incident_report(&asset).unwrap();
    assert_eq!(report.kind, OracleIncidentKind::SourceDeviationPause);
    assert!(report.observed_bps > 1000);
    assert!(client.get_oracle_circuit_breaker_state(&asset).open_until > env.ledger().timestamp());
    assert!(client.try_get_price(&asset).is_err());
}

#[test]
fn volatility_above_twenty_percent_in_ten_minutes_pauses_oracle() {
    let env = create_env();
    let (admin, client) = setup(&env);
    let asset = Address::generate(&env);
    let oracle = Address::generate(&env);

    client.configure_oracle(&admin, &monitoring_config());
    client.update_price_feed(&admin, &asset, &100_000_000, &8, &oracle);
    env.ledger().with_mut(|li| li.timestamp += 300);
    client.update_price_feed(&admin, &asset, &121_000_000, &8, &oracle);

    let report = client.get_oracle_incident_report(&asset).unwrap();
    assert_eq!(report.kind, OracleIncidentKind::VolatilityPause);
    assert!(report.observed_bps > 2000);
    assert!(client.try_get_price(&asset).is_err());
}

#[test]
fn breaker_gradually_unpauses_after_stable_updates() {
    let env = create_env();
    let (admin, client) = setup(&env);
    let asset = Address::generate(&env);
    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let sources = vec![&env, s1.clone(), s2.clone()];

    client.configure_oracle(&admin, &monitoring_config());
    client.set_oracle_sources(&admin, &asset, &sources);
    client.update_price_feed(&admin, &asset, &100_000_000, &8, &s1);
    client.update_price_feed(&admin, &asset, &111_000_000, &8, &s2);
    env.ledger().with_mut(|li| li.timestamp += 61);

    assert!(client.try_get_price(&asset).is_err());

    client.update_price_feed(&admin, &asset, &100_000_000, &8, &s1);
    client.update_price_feed(&admin, &asset, &101_000_000, &8, &s1);
    client.update_price_feed(&admin, &asset, &100_500_000, &8, &s1);

    let report = client.get_oracle_incident_report(&asset).unwrap();
    assert_eq!(report.kind, OracleIncidentKind::PriceStabilized);
    assert_eq!(
        client.get_oracle_circuit_breaker_state(&asset).open_until,
        0
    );
}
