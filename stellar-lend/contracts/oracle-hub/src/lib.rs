#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, Env, Vec};

pub const VERSION: u32 = 1;
pub const MAX_FEEDS_PER_ASSET: u32 = 5;
pub const OUTLIER_DEVIATION_BPS: i128 = 2_000;
pub const DEFAULT_STALE_THRESHOLD_SECONDS: u64 = 3600;

// ── Priorities ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
#[contracttype]
pub enum FeedPriority {
    Primary = 0,
    Secondary = 1,
    Fallback = 2,
}

// ── Data structures ──────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct PriceFeed {
    pub asset: Bytes,
    pub oracle_address: Address,
    pub priority: FeedPriority,
    pub enabled: bool,
    pub stale_threshold_seconds: u64,
    pub registered_at: u64,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct PricePoint {
    pub asset: Bytes,
    pub price: i128,
    pub timestamp: u64,
    pub confidence: u32,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct AggregatedPrice {
    pub price: i128,
    pub timestamp: u64,
    pub confidence: u32,
    pub num_feeds: u32,
    pub num_active_feeds: u32,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct FeedStatus {
    pub asset: Bytes,
    pub status: FeedStatusCode,
    pub last_update: u64,
    pub is_stale: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum FeedStatusCode {
    Active = 0,
    Stale = 1,
    Disabled = 2,
    Frozen = 3,
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct OracleHubContract;

#[contractimpl]
impl OracleHubContract {
    pub fn initialize(env: Env, governance: Address, admin: Address) {
        env.storage().instance().set(&DataKey::Governance, &governance);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::FeedCount, &0u32);
        env.storage().instance().set(&DataKey::Frozen, &false);
        env.storage().instance().set(&DataKey::Version, &VERSION);
    }

    pub fn version(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Version).unwrap_or(0)
    }

    pub fn upgrade(env: Env, new_version: u32) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();
        assert!(new_version > VERSION, "Version must increase");
        let old_version: u32 = env.storage().instance().get(&DataKey::Version).unwrap_or(0);
        env.storage().instance().set(&DataKey::Version, &new_version);
        env.events().publish(("upgrade",), (&old_version, &new_version));
    }

    // ── Feed management ────────────────────────────────────────────────────

    pub fn register_feed(
        env: Env,
        asset: Bytes,
        oracle_address: Address,
        priority: FeedPriority,
        stale_threshold_seconds: u64,
    ) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();

        let threshold = if stale_threshold_seconds == 0 {
            DEFAULT_STALE_THRESHOLD_SECONDS
        } else {
            stale_threshold_seconds
        };

        let feed = PriceFeed {
            asset: asset.clone(),
            oracle_address: oracle_address.clone(),
            priority,
            enabled: true,
            stale_threshold_seconds: threshold,
            registered_at: env.ledger().timestamp(),
        };

        let feed_key = DataKey::Feed(asset.clone(), priority as u32);
        env.storage().instance().set(&feed_key, &feed);

        let count: u32 = env.storage().instance().get(&DataKey::FeedCount).unwrap_or(0);
        env.storage().instance().set(&DataKey::FeedCount, &(count + 1));
        env.events().publish(("register_feed", &asset), &oracle_address);
    }

    pub fn update_feed(
        env: Env,
        asset: Bytes,
        priority: FeedPriority,
        stale_threshold_seconds: u64,
    ) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();

        let feed_key = DataKey::Feed(asset.clone(), priority as u32);
        let mut feed: PriceFeed = env.storage().instance().get(&feed_key).expect("Feed not found");
        feed.stale_threshold_seconds = if stale_threshold_seconds == 0 {
            DEFAULT_STALE_THRESHOLD_SECONDS
        } else {
            stale_threshold_seconds
        };
        env.storage().instance().set(&feed_key, &feed);
        env.events().publish(("update_feed", &asset), &priority);
    }

    pub fn disable_feed(env: Env, asset: Bytes, priority: FeedPriority) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();

        let feed_key = DataKey::Feed(asset.clone(), priority as u32);
        let mut feed: PriceFeed = env.storage().instance().get(&feed_key).expect("Feed not found");
        feed.enabled = false;
        env.storage().instance().set(&feed_key, &feed);
        env.events().publish(("disable_feed",), &asset);
    }

    pub fn enable_feed(env: Env, asset: Bytes, priority: FeedPriority) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();

        let feed_key = DataKey::Feed(asset.clone(), priority as u32);
        let mut feed: PriceFeed = env.storage().instance().get(&feed_key).expect("Feed not found");
        feed.enabled = true;
        env.storage().instance().set(&feed_key, &feed);
        env.events().publish(("enable_feed",), &asset);
    }

    // ── Emergency freeze ───────────────────────────────────────────────────

    pub fn freeze(env: Env) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();
        env.storage().instance().set(&DataKey::Frozen, &true);
        env.events().publish(("freeze",), &());
    }

    pub fn unfreeze(env: Env) {
        let governance: Address = env.storage().instance().get(&DataKey::Governance).unwrap();
        governance.require_auth();
        env.storage().instance().set(&DataKey::Frozen, &false);
        env.events().publish(("unfreeze",), &());
    }

    pub fn is_frozen(env: Env) -> bool {
        env.storage().instance().get(&DataKey::Frozen).unwrap_or(false)
    }

    // ── Price reporting ────────────────────────────────────────────────────

    pub fn report_price(env: Env, asset: Bytes, price: i128, confidence: u32, priority: FeedPriority) {
        let feed_key = DataKey::Feed(asset.clone(), priority as u32);
        let feed: PriceFeed = env.storage().instance().get(&feed_key).expect("Feed not found");
        feed.oracle_address.require_auth();

        assert!(!env.storage().instance().get::<_, bool>(&DataKey::Frozen).unwrap_or(false), "OracleHub is frozen");
        assert!(feed.enabled, "Feed is disabled");
        assert!(price > 0, "Price must be positive");

        let price_point = PricePoint {
            asset: asset.clone(),
            price,
            timestamp: env.ledger().timestamp(),
            confidence,
        };

        let latest_key = DataKey::LatestPrice(asset.clone(), priority as u32);
        env.storage().instance().set(&latest_key, &price_point);
        env.events().publish(("report_price", &asset), (&price, &confidence, &priority));
    }

    // ── Price queries ──────────────────────────────────────────────────────

    pub fn price(env: Env, _asset: Address) -> i128 {
        let asset_bytes = Bytes::from_slice(&env, &[0u8; 32]);
        let result = Self::get_price(env, asset_bytes);
        result.price
    }

    pub fn get_price(env: Env, asset: Bytes) -> AggregatedPrice {
        assert!(
            !env.storage().instance().get::<_, bool>(&DataKey::Frozen).unwrap_or(false),
            "OracleHub is frozen"
        );

        let current_time = env.ledger().timestamp();
        let mut prices: Vec<(i128, u32, u64, u32)> = Vec::new(&env);
        let mut num_active = 0u32;

        for priority in 0u32..=2 {
            let feed_key = DataKey::Feed(asset.clone(), priority);
            if let Some(feed) = env.storage().instance().get::<_, PriceFeed>(&feed_key) {
                if !feed.enabled {
                    continue;
                }

                let latest_key = DataKey::LatestPrice(asset.clone(), priority);
                if let Some(point) = env.storage().instance().get::<_, PricePoint>(&latest_key) {
                    let stale = current_time.saturating_sub(point.timestamp) > feed.stale_threshold_seconds;
                    if stale {
                        Self::auto_disable_feed(&env, asset.clone(), priority);
                        continue;
                    }
                    num_active += 1;
                    prices.push_back((point.price, point.confidence, point.timestamp, priority));
                }
            }
        }

        assert!(prices.len() > 0, "No active price feeds available");

        let feed_count = prices.len();
        let (median_price, median_conf, median_ts) = if prices.len() == 1 {
            let (p, c, t, _) = prices.get(0).unwrap();
            (p, c, t)
        } else {
            Self::aggregate_with_outlier_rejection(&env, prices)
        };

        AggregatedPrice {
            price: median_price,
            timestamp: median_ts,
            confidence: median_conf,
            num_feeds: feed_count,
            num_active_feeds: num_active,
        }
    }

    fn aggregate_with_outlier_rejection(
        _env: &Env,
        prices: Vec<(i128, u32, u64, u32)>,
    ) -> (i128, u32, u64) {
        let count = prices.len();
        let median_idx = count / 2;

        let mut arr: [i128; 5] = [0; 5];
        let c = core::cmp::min(count, 5) as usize;
        for i in 0..c {
            arr[i] = prices.get(i as u32).unwrap().0;
        }

        let mut sorted = [0i128; 5];
        for i in 0..c {
            sorted[i] = arr[i];
        }

        for i in 0..c {
            for j in (i + 1)..c {
                if sorted[j] < sorted[i] {
                    let tmp = sorted[i];
                    sorted[i] = sorted[j];
                    sorted[j] = tmp;
                }
            }
        }

        let median_price = sorted[median_idx as usize];
        let mut total_conf: u64 = 0;
        let mut valid_prices = 0u32;
        let mut latest_ts = 0u64;

        for i in 0..count {
            let (p, c_val, ts, _) = prices.get(i).unwrap();
            if p == 0 {
                continue;
            }
            let deviation_bps = if median_price > 0 {
                let diff = if p > median_price { p - median_price } else { median_price - p };
                diff.saturating_mul(10_000) / median_price
            } else {
                0
            };

            if deviation_bps <= OUTLIER_DEVIATION_BPS {
                total_conf += c_val as u64;
                valid_prices += 1;
                if ts > latest_ts {
                    latest_ts = ts;
                }
            }
        }

        let avg_conf = if valid_prices > 0 {
            (total_conf / valid_prices as u64) as u32
        } else {
            0
        };

        (median_price, avg_conf, latest_ts)
    }

    fn auto_disable_feed(env: &Env, asset: Bytes, priority: u32) {
        let feed_key = DataKey::Feed(asset.clone(), priority);
        let mut feed: PriceFeed = env.storage().instance().get(&feed_key).expect("Feed not found");
        if feed.enabled {
            feed.enabled = false;
            env.storage().instance().set(&feed_key, &feed);
            env.events().publish(("auto_disable_feed", &asset), &priority);
        }
    }

    // ── Health checks ──────────────────────────────────────────────────────

    pub fn check_feed_health(env: Env, asset: Bytes) -> Vec<FeedStatus> {
        let current_time = env.ledger().timestamp();
        let mut statuses: Vec<FeedStatus> = Vec::new(&env);

        for priority in 0u32..=2 {
            let feed_key = DataKey::Feed(asset.clone(), priority);
            if let Some(feed) = env.storage().instance().get::<_, PriceFeed>(&feed_key) {
                let latest_key = DataKey::LatestPrice(asset.clone(), priority);
                let price = env.storage().instance().get::<_, PricePoint>(&latest_key);

                let (status_code, last_update, is_stale) = if !feed.enabled {
                    (FeedStatusCode::Disabled, 0, true)
                } else if env.storage().instance().get::<_, bool>(&DataKey::Frozen).unwrap_or(false) {
                    (FeedStatusCode::Frozen, 0, true)
                } else if let Some(p) = price {
                    let stale = current_time.saturating_sub(p.timestamp) > feed.stale_threshold_seconds;
                    if stale {
                        (FeedStatusCode::Stale, p.timestamp, true)
                    } else {
                        (FeedStatusCode::Active, p.timestamp, false)
                    }
                } else {
                    (FeedStatusCode::Stale, 0, true)
                };

                statuses.push_back(FeedStatus {
                    asset: asset.clone(),
                    status: status_code,
                    last_update,
                    is_stale,
                });
            }
        }

        statuses
    }
}

// ── Storage keys ─────────────────────────────────────────────────────────────

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Governance,
    Admin,
    FeedCount,
    Version,
    Frozen,
    Feed(Bytes, u32),
    LatestPrice(Bytes, u32),
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke};
    use soroban_sdk::{IntoVal, Vec as SdkVec};

    struct TestEnv {
        env: Env,
        contract_id: Address,
        governance: Address,
        #[allow(dead_code)]
        admin: Address,
    }

    fn setup() -> TestEnv {
        let env = Env::default();
        let governance = Address::generate(&env);
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, OracleHubContract);
        let client = OracleHubContractClient::new(&env, &contract_id);
        client.initialize(&governance, &admin);
        TestEnv { env, contract_id, governance, admin }
    }

    fn client(te: &TestEnv) -> OracleHubContractClient<'_> {
        OracleHubContractClient::new(&te.env, &te.contract_id)
    }

    fn mk_asset(env: &Env, name: &str) -> Bytes {
        Bytes::from_slice(env, name.as_bytes())
    }

    fn gov_auth<T>(te: &TestEnv, fn_name: &str, args: impl IntoVal<Env, SdkVec<soroban_sdk::Val>>, f: impl FnOnce() -> T) -> T {
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

    #[test]
    fn test_initialize() {
        let te = setup();
        assert_eq!(client(&te).version(), VERSION);
        assert!(!client(&te).is_frozen());
    }

    #[test]
    fn test_register_feed() {
        let te = setup();
        let asset = mk_asset(&te.env, "XLM");
        let oracle = Address::generate(&te.env);

        gov_auth(&te, "register_feed", (&asset, &oracle, &FeedPriority::Primary, &3600u64), || {
            let o = oracle.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Primary, &3600);
        });

        let statuses = client(&te).check_feed_health(&asset);
        assert_eq!(statuses.len(), 1);
    }

    #[test]
    fn test_report_price_and_get_price() {
        let te = setup();
        let asset = mk_asset(&te.env, "XLM");
        let oracle = Address::generate(&te.env);

        gov_auth(&te, "register_feed", (&asset, &oracle, &FeedPriority::Primary, &3600u64), || {
            let o = oracle.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Primary, &3600);
        });

        te.env.mock_auths(&[MockAuth {
            address: &oracle,
            invoke: &MockAuthInvoke {
                contract: &te.contract_id,
                fn_name: "report_price",
                args: (&asset, &100_000_000i128, &100u32, &FeedPriority::Primary).into_val(&te.env),
                sub_invokes: &[],
            },
        }]);
        client(&te).report_price(&asset, &100_000_000, &100, &FeedPriority::Primary);

        let aggregated = client(&te).get_price(&asset);
        assert_eq!(aggregated.price, 100_000_000);
        assert_eq!(aggregated.num_feeds, 1);
    }

    #[test]
    fn test_aggregation_multiple_feeds() {
        let te = setup();
        let asset = mk_asset(&te.env, "XLM");
        let oracle1 = Address::generate(&te.env);
        let oracle2 = Address::generate(&te.env);

        gov_auth(&te, "register_feed", (&asset, &oracle1, &FeedPriority::Primary, &3600u64), || {
            let o = oracle1.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Primary, &3600);
        });
        gov_auth(&te, "register_feed", (&asset, &oracle2, &FeedPriority::Secondary, &3600u64), || {
            let o = oracle2.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Secondary, &3600);
        });

        for (oracle, price, priority_val) in [
            (&oracle1, &100_000_000i128, &FeedPriority::Primary),
            (&oracle2, &101_000_000i128, &FeedPriority::Secondary),
        ] {
            te.env.mock_auths(&[MockAuth {
                address: oracle,
                invoke: &MockAuthInvoke {
                    contract: &te.contract_id,
                    fn_name: "report_price",
                    args: (&asset, price, &100u32, priority_val).into_val(&te.env),
                    sub_invokes: &[],
                },
            }]);
            client(&te).report_price(&asset, price, &100, priority_val);
        }

        let aggregated = client(&te).get_price(&asset);
        assert_eq!(aggregated.num_feeds, 2);
        assert!(aggregated.price > 0);
    }

    #[test]
    fn test_outlier_rejection() {
        let te = setup();
        let asset = mk_asset(&te.env, "XLM");
        let oracle0 = Address::generate(&te.env);
        let oracle1 = Address::generate(&te.env);
        let oracle2 = Address::generate(&te.env);

        gov_auth(&te, "register_feed", (&asset, &oracle0, &FeedPriority::Primary, &3600u64), || {
            let o = oracle0.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Primary, &3600);
        });
        gov_auth(&te, "register_feed", (&asset, &oracle1, &FeedPriority::Secondary, &3600u64), || {
            let o = oracle1.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Secondary, &3600);
        });
        gov_auth(&te, "register_feed", (&asset, &oracle2, &FeedPriority::Fallback, &3600u64), || {
            let o = oracle2.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Fallback, &3600);
        });

        let price_data = [
            (&oracle0, 100_000_000i128, FeedPriority::Primary),
            (&oracle1, 100_000_000i128, FeedPriority::Secondary),
            (&oracle2, 200_000_000i128, FeedPriority::Fallback),
        ];
        for (oracle, price, priority) in &price_data {
            te.env.mock_auths(&[MockAuth {
                address: oracle,
                invoke: &MockAuthInvoke {
                    contract: &te.contract_id,
                    fn_name: "report_price",
                    args: (&asset, price, &100u32, priority).into_val(&te.env),
                    sub_invokes: &[],
                },
            }]);
            let p = *price;
            client(&te).report_price(&asset, &p, &100, priority);
        }

        let aggregated = client(&te).get_price(&asset);
        assert_eq!(aggregated.price, 100_000_000);
    }

    #[test]
    fn test_stale_feed_detection() {
        let te = setup();
        let asset = mk_asset(&te.env, "XLM");
        let oracle = Address::generate(&te.env);

        gov_auth(&te, "register_feed", (&asset, &oracle, &FeedPriority::Primary, &100u64), || {
            let o = oracle.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Primary, &100);
        });

        te.env.mock_auths(&[MockAuth {
            address: &oracle,
            invoke: &MockAuthInvoke {
                contract: &te.contract_id,
                fn_name: "report_price",
                args: (&asset, &100_000_000i128, &100u32, &FeedPriority::Primary).into_val(&te.env),
                sub_invokes: &[],
            },
        }]);
        client(&te).report_price(&asset, &100_000_000, &100, &FeedPriority::Primary);

        let statuses = client(&te).check_feed_health(&asset);
        assert_eq!(statuses.get(0).unwrap().status, FeedStatusCode::Active);
        assert!(!statuses.get(0).unwrap().is_stale);

        te.env.ledger().set_timestamp(200);

        let statuses = client(&te).check_feed_health(&asset);
        assert_eq!(statuses.get(0).unwrap().status, FeedStatusCode::Stale);
        assert!(statuses.get(0).unwrap().is_stale);
    }

    #[test]
    fn test_fallback_on_stale_primary() {
        let te = setup();
        let asset = mk_asset(&te.env, "XLM");
        let primary = Address::generate(&te.env);
        let secondary = Address::generate(&te.env);

        gov_auth(&te, "register_feed", (&asset, &primary, &FeedPriority::Primary, &100u64), || {
            let o = primary.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Primary, &100);
        });
        gov_auth(&te, "register_feed", (&asset, &secondary, &FeedPriority::Secondary, &1000u64), || {
            let o = secondary.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Secondary, &1000);
        });

        for (oracle, price, priority_val) in [
            (&primary, &100_000_000i128, &FeedPriority::Primary),
            (&secondary, &101_000_000i128, &FeedPriority::Secondary),
        ] {
            te.env.mock_auths(&[MockAuth {
                address: oracle,
                invoke: &MockAuthInvoke {
                    contract: &te.contract_id,
                    fn_name: "report_price",
                    args: (&asset, price, &100u32, priority_val).into_val(&te.env),
                    sub_invokes: &[],
                },
            }]);
            client(&te).report_price(&asset, price, &100, priority_val);
        }

        te.env.ledger().set_timestamp(500);

        let statuses = client(&te).check_feed_health(&asset);
        let primary_stale = statuses.get(0).unwrap();
        assert_eq!(primary_stale.status, FeedStatusCode::Stale);

        let aggregated = client(&te).get_price(&asset);
        assert_eq!(aggregated.price, 101_000_000);
        assert_eq!(aggregated.num_feeds, 1);
    }

    #[test]
    fn test_emergency_freeze_toggle() {
        let te = setup();

        gov_auth(&te, "freeze", (), || {
            client(&te).freeze();
        });
        assert!(client(&te).is_frozen());

        gov_auth(&te, "unfreeze", (), || {
            client(&te).unfreeze();
        });
        assert!(!client(&te).is_frozen());
    }

    #[test]
    #[should_panic(expected = "HostError")]
    fn test_get_price_reverts_when_frozen() {
        let te = setup();
        let asset = mk_asset(&te.env, "XLM");

        gov_auth(&te, "freeze", (), || {
            client(&te).freeze();
        });

        client(&te).get_price(&asset);
    }

    #[test]
    fn test_disable_and_enable_feed() {
        let te = setup();
        let asset = mk_asset(&te.env, "ETH");
        let oracle = Address::generate(&te.env);

        gov_auth(&te, "register_feed", (&asset, &oracle, &FeedPriority::Primary, &3600u64), || {
            let o = oracle.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Primary, &3600);
        });

        gov_auth(&te, "disable_feed", (&asset, &FeedPriority::Primary), || {
            client(&te).disable_feed(&asset, &FeedPriority::Primary);
        });

        let statuses = client(&te).check_feed_health(&asset);
        assert_eq!(statuses.get(0).unwrap().status, FeedStatusCode::Disabled);

        gov_auth(&te, "enable_feed", (&asset, &FeedPriority::Primary), || {
            client(&te).enable_feed(&asset, &FeedPriority::Primary);
        });

        let statuses = client(&te).check_feed_health(&asset);
        assert_eq!(statuses.get(0).unwrap().status, FeedStatusCode::Stale);
    }

    #[test]
    fn test_upgrade() {
        let te = setup();

        gov_auth(&te, "upgrade", (&2u32,), || {
            client(&te).upgrade(&2);
        });
        assert_eq!(client(&te).version(), 2);
    }

    #[test]
    #[should_panic(expected = "HostError")]
    fn test_upgrade_downgrade_reverts() {
        let te = setup();

        client(&te).upgrade(&1);
    }

    #[test]
    #[should_panic(expected = "HostError")]
    fn test_no_feeds_returns_error() {
        let te = setup();
        let asset = mk_asset(&te.env, "UNKNOWN");

        client(&te).get_price(&asset);
    }

    #[test]
    fn test_update_feed() {
        let te = setup();
        let asset = mk_asset(&te.env, "BTC");
        let oracle = Address::generate(&te.env);

        gov_auth(&te, "register_feed", (&asset, &oracle, &FeedPriority::Primary, &3600u64), || {
            let o = oracle.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Primary, &3600);
        });

        gov_auth(&te, "update_feed", (&asset, &FeedPriority::Primary, &7200u64), || {
            client(&te).update_feed(&asset, &FeedPriority::Primary, &7200);
        });

        let statuses = client(&te).check_feed_health(&asset);
        assert_eq!(statuses.len(), 1);
    }

    #[test]
    #[should_panic(expected = "HostError")]
    fn test_report_price_reverts_when_frozen() {
        let te = setup();
        let asset = mk_asset(&te.env, "XLM");
        let oracle = Address::generate(&te.env);

        gov_auth(&te, "register_feed", (&asset, &oracle, &FeedPriority::Primary, &3600u64), || {
            let o = oracle.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Primary, &3600);
        });
        gov_auth(&te, "freeze", (), || {
            client(&te).freeze();
        });

        te.env.mock_auths(&[MockAuth {
            address: &oracle,
            invoke: &MockAuthInvoke {
                contract: &te.contract_id,
                fn_name: "report_price",
                args: (&asset, &100_000_000i128, &100u32, &FeedPriority::Primary).into_val(&te.env),
                sub_invokes: &[],
            },
        }]);
        client(&te).report_price(&asset, &100_000_000, &100, &FeedPriority::Primary);
    }

    #[test]
    fn test_confidence_tracking() {
        let te = setup();
        let asset = mk_asset(&te.env, "XLM");
        let oracle = Address::generate(&te.env);

        gov_auth(&te, "register_feed", (&asset, &oracle, &FeedPriority::Primary, &3600u64), || {
            let o = oracle.clone();
            client(&te).register_feed(&asset, &o, &FeedPriority::Primary, &3600);
        });

        te.env.mock_auths(&[MockAuth {
            address: &oracle,
            invoke: &MockAuthInvoke {
                contract: &te.contract_id,
                fn_name: "report_price",
                args: (&asset, &100_000_000i128, &95u32, &FeedPriority::Primary).into_val(&te.env),
                sub_invokes: &[],
            },
        }]);
        client(&te).report_price(&asset, &100_000_000, &95, &FeedPriority::Primary);

        let aggregated = client(&te).get_price(&asset);
        assert_eq!(aggregated.confidence, 95);
    }

    #[test]
    fn test_multiple_assets_independent() {
        let te = setup();
        let asset1 = mk_asset(&te.env, "XLM");
        let asset2 = mk_asset(&te.env, "BTC");
        let oracle = Address::generate(&te.env);

        gov_auth(&te, "register_feed", (&asset1, &oracle, &FeedPriority::Primary, &3600u64), || {
            let o = oracle.clone();
            client(&te).register_feed(&asset1, &o, &FeedPriority::Primary, &3600);
        });
        gov_auth(&te, "register_feed", (&asset2, &oracle, &FeedPriority::Primary, &3600u64), || {
            let o = oracle.clone();
            client(&te).register_feed(&asset2, &o, &FeedPriority::Primary, &3600);
        });

        te.env.mock_auths(&[MockAuth {
            address: &oracle,
            invoke: &MockAuthInvoke {
                contract: &te.contract_id,
                fn_name: "report_price",
                args: (&asset1, &50_000_000i128, &100u32, &FeedPriority::Primary).into_val(&te.env),
                sub_invokes: &[],
            },
        }]);
        client(&te).report_price(&asset1, &50_000_000, &100, &FeedPriority::Primary);

        te.env.mock_auths(&[MockAuth {
            address: &oracle,
            invoke: &MockAuthInvoke {
                contract: &te.contract_id,
                fn_name: "report_price",
                args: (&asset2, &1_000_000_000i128, &100u32, &FeedPriority::Primary).into_val(&te.env),
                sub_invokes: &[],
            },
        }]);
        client(&te).report_price(&asset2, &1_000_000_000, &100, &FeedPriority::Primary);

        let agg1 = client(&te).get_price(&asset1);
        let agg2 = client(&te).get_price(&asset2);
        assert_eq!(agg1.price, 50_000_000);
        assert_eq!(agg2.price, 1_000_000_000);
    }
}
