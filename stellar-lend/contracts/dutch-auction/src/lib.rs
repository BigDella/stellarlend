#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

const BPS_BASE: i128 = 10_000;

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum AuctionStatus {
    Active,
    Settled,
    Expired,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct AuctionConfig {
    pub pool: Address,
    pub collateral_asset: Address,
    pub debt_asset: Address,
    pub collateral_amount: i128,
    pub debt_amount: i128,
    pub oracle_price: i128,
    pub duration_secs: u64,
    pub min_price_bps: i128,
    pub discount_floor_bps: i128,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct Auction {
    pub id: u64,
    pub config: AuctionConfig,
    pub start_price: i128,
    pub current_price: i128,
    pub start_time: u64,
    pub end_time: u64,
    pub status: AuctionStatus,
    pub borrower: Address,
    pub highest_bidder: Option<Address>,
    pub highest_bid_amount: Option<i128>,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct AuctionBid {
    pub auction_id: u64,
    pub bidder: Address,
    pub amount: i128,
    pub collateral_received: i128,
    pub timestamp: u64,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct AuctionAnalytics {
    pub total_auctions: u64,
    pub settled_auctions: u64,
    pub avg_premium_bps: i128,
    pub avg_time_to_fill_secs: u64,
    pub total_collateral_liquidated: i128,
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Admin,
    AuctionCount,
    Auction(u64),
    AuctionBid(u64),
    TotalPremiumBps,
    TotalTimeToFill,
    TotalCollateralLiquidated,
}

#[contract]
pub struct DutchAuctionContract;

#[contractimpl]
impl DutchAuctionContract {
    pub fn initialize(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::AuctionCount, &0u64);
        env.storage()
            .instance()
            .set(&DataKey::TotalPremiumBps, &0i128);
        env.storage()
            .instance()
            .set(&DataKey::TotalTimeToFill, &0u64);
        env.storage()
            .instance()
            .set(&DataKey::TotalCollateralLiquidated, &0i128);
    }

    pub fn create_auction(env: Env, borrower: Address, config: AuctionConfig) -> u64 {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();

        assert!(config.duration_secs > 0, "duration must be positive");
        assert!(
            config.min_price_bps > 0 && config.min_price_bps <= BPS_BASE,
            "invalid min_price_bps"
        );
        assert!(
            config.discount_floor_bps >= 0 && config.discount_floor_bps < BPS_BASE,
            "invalid discount_floor_bps"
        );
        assert!(config.collateral_amount > 0, "collateral must be positive");
        assert!(config.debt_amount > 0, "debt must be positive");

        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AuctionCount)
            .unwrap_or(0);
        let auction_id = count + 1;

        let start_time = env.ledger().timestamp();
        let end_time = start_time + config.duration_secs;

        let auction = Auction {
            id: auction_id,
            config: config.clone(),
            start_price: config.oracle_price,
            current_price: config.oracle_price,
            start_time,
            end_time,
            status: AuctionStatus::Active,
            borrower: borrower.clone(),
            highest_bidder: None,
            highest_bid_amount: None,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Auction(auction_id), &auction);
        env.storage()
            .instance()
            .set(&DataKey::AuctionCount, &auction_id);

        env.events().publish(
            (soroban_sdk::symbol_short!("AucCreate"), auction_id),
            (
                &config.pool,
                &config.collateral_asset,
                &config.debt_asset,
                config.oracle_price,
                config.duration_secs,
            ),
        );

        auction_id
    }

    pub fn get_current_price(env: Env, auction_id: u64) -> i128 {
        let auction: Auction = env
            .storage()
            .persistent()
            .get(&DataKey::Auction(auction_id))
            .expect("auction not found");

        if auction.status != AuctionStatus::Active {
            return auction.current_price;
        }

        let now = env.ledger().timestamp();
        if now >= auction.end_time {
            let min_price =
                (auction.config.oracle_price * auction.config.min_price_bps) / BPS_BASE;
            return min_price;
        }

        let elapsed = now - auction.start_time;
        let min_price =
            (auction.config.oracle_price * auction.config.min_price_bps) / BPS_BASE;
        let total_discount = auction.config.oracle_price - min_price;
        let price_reduction =
            (total_discount * (elapsed as i128)) / (auction.config.duration_secs as i128);

        auction.config.oracle_price - price_reduction
    }

    pub fn place_bid(
        env: Env,
        auction_id: u64,
        bidder: Address,
        debt_repay_amount: i128,
    ) -> AuctionBid {
        bidder.require_auth();

        let mut auction: Auction = env
            .storage()
            .persistent()
            .get(&DataKey::Auction(auction_id))
            .expect("auction not found");

        assert!(auction.status == AuctionStatus::Active, "auction not active");
        let now = env.ledger().timestamp();
        assert!(now < auction.end_time, "auction ended");
        assert!(debt_repay_amount > 0, "repay amount must be positive");

        let current_price = Self::get_current_price(env.clone(), auction_id);
        let collateral_received = (debt_repay_amount * BPS_BASE) / current_price;

        let debt_client = token::Client::new(&env, &auction.config.debt_asset);
        debt_client.transfer_from(
            &env.current_contract_address(),
            &bidder,
            &env.current_contract_address(),
            &debt_repay_amount,
        );

        let collateral_client = token::Client::new(&env, &auction.config.collateral_asset);
        collateral_client.transfer(
            &env.current_contract_address(),
            &bidder,
            &collateral_received,
        );

        auction.highest_bidder = Some(bidder.clone());
        auction.highest_bid_amount = Some(debt_repay_amount);
        auction.current_price = current_price;
        auction.status = AuctionStatus::Settled;

        env.storage()
            .persistent()
            .set(&DataKey::Auction(auction_id), &auction);

        let bid = AuctionBid {
            auction_id,
            bidder: bidder.clone(),
            amount: debt_repay_amount,
            collateral_received,
            timestamp: now,
        };

        env.storage()
            .persistent()
            .set(&DataKey::AuctionBid(auction_id), &bid);

        let premium_bps =
            ((auction.config.oracle_price - current_price) * BPS_BASE) / auction.config.oracle_price;

        let mut total_premium: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalPremiumBps)
            .unwrap_or(0);
        total_premium += premium_bps;
        env.storage()
            .instance()
            .set(&DataKey::TotalPremiumBps, &total_premium);

        let mut total_time: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TotalTimeToFill)
            .unwrap_or(0);
        total_time += now - auction.start_time;
        env.storage()
            .instance()
            .set(&DataKey::TotalTimeToFill, &total_time);

        let mut total_collateral: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalCollateralLiquidated)
            .unwrap_or(0);
        total_collateral += collateral_received;
        env.storage()
            .instance()
            .set(&DataKey::TotalCollateralLiquidated, &total_collateral);

        env.events().publish(
            (soroban_sdk::symbol_short!("BidPlaced"), auction_id),
            (&bidder, debt_repay_amount, collateral_received, current_price),
        );

        bid
    }

    pub fn settle_auction(env: Env, auction_id: u64) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();

        let auction: Auction = env
            .storage()
            .persistent()
            .get(&DataKey::Auction(auction_id))
            .expect("auction not found");

        assert!(
            auction.status == AuctionStatus::Settled,
            "auction not settled"
        );

        env.events().publish(
            (soroban_sdk::symbol_short!("AucSettle"), auction_id),
            (
                &auction.highest_bidder,
                auction.highest_bid_amount,
                auction.current_price,
            ),
        );
    }

    pub fn expire_auction(env: Env, auction_id: u64) {
        let mut auction: Auction = env
            .storage()
            .persistent()
            .get(&DataKey::Auction(auction_id))
            .expect("auction not found");

        assert!(auction.status == AuctionStatus::Active, "auction not active");
        let now = env.ledger().timestamp();
        assert!(now >= auction.end_time, "auction not yet ended");

        auction.status = AuctionStatus::Expired;
        let min_price =
            (auction.config.oracle_price * auction.config.min_price_bps) / BPS_BASE;
        auction.current_price = min_price;

        env.storage()
            .persistent()
            .set(&DataKey::Auction(auction_id), &auction);

        let new_duration = auction.config.duration_secs * 2;
        let new_end_time = auction.start_time + new_duration;
        let mut extended_auction = auction.clone();
        extended_auction.end_time = new_end_time;
        extended_auction.status = AuctionStatus::Active;

        env.storage()
            .persistent()
            .set(&DataKey::Auction(auction_id), &extended_auction);

        env.events().publish(
            (soroban_sdk::symbol_short!("AucExpire"), auction_id),
            (
                min_price,
                new_duration,
                auction.config.discount_floor_bps,
            ),
        );
    }

    pub fn get_auction(env: Env, auction_id: u64) -> Auction {
        env.storage()
            .persistent()
            .get(&DataKey::Auction(auction_id))
            .expect("auction not found")
    }

    pub fn get_active_auctions(env: Env) -> soroban_sdk::Vec<Auction> {
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AuctionCount)
            .unwrap_or(0);

        let mut active = soroban_sdk::Vec::new(&env);
        let mut i = 1u64;
        while i <= count {
            let auction: Auction = env
                .storage()
                .persistent()
                .get(&DataKey::Auction(i))
                .unwrap();
            if auction.status == AuctionStatus::Active {
                active.push_back(auction);
            }
            i += 1;
        }

        active
    }

    pub fn get_analytics(env: Env) -> AuctionAnalytics {
        let total: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AuctionCount)
            .unwrap_or(0);

        let total_premium_bps: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalPremiumBps)
            .unwrap_or(0);
        let total_time: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TotalTimeToFill)
            .unwrap_or(0);
        let total_collateral: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalCollateralLiquidated)
            .unwrap_or(0);

        let mut settled: u64 = 0;
        let mut i = 1u64;
        while i <= total {
            let auction: Auction = env
                .storage()
                .persistent()
                .get(&DataKey::Auction(i))
                .unwrap();
            if auction.status == AuctionStatus::Settled {
                settled += 1;
            }
            i += 1;
        }

        let avg_premium = if settled > 0 {
            total_premium_bps / (settled as i128)
        } else {
            0
        };

        let avg_time = if settled > 0 {
            total_time / settled
        } else {
            0
        };

        AuctionAnalytics {
            total_auctions: total,
            settled_auctions: settled,
            avg_premium_bps: avg_premium,
            avg_time_to_fill_secs: avg_time,
            total_collateral_liquidated: total_collateral,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger as _, MockAuth, MockAuthInvoke};
    use soroban_sdk::IntoVal;

    struct TestEnv {
        env: Env,
        contract_id: Address,
        admin: Address,
        borrower: Address,
        config: AuctionConfig,
    }

    impl TestEnv {
        fn new() -> Self {
            let env = Env::default();
            let admin = Address::generate(&env);
            let borrower = Address::generate(&env);
            let contract_id = env.register(DutchAuctionContract, ());
            let client = DutchAuctionContractClient::new(&env, &contract_id);
            client.initialize(&admin);

            let config = AuctionConfig {
                pool: Address::generate(&env),
                collateral_asset: Address::generate(&env),
                debt_asset: Address::generate(&env),
                collateral_amount: 1_000_000_000,
                debt_amount: 500_000_000,
                oracle_price: 20_000,
                duration_secs: 3600,
                min_price_bps: 7000,
                discount_floor_bps: 3000,
            };

            TestEnv { env, contract_id, admin, borrower, config }
        }

        fn client(&self) -> DutchAuctionContractClient<'_> {
            DutchAuctionContractClient::new(&self.env, &self.contract_id)
        }

        fn mock_admin_auth(&self) {
            self.env.mock_auths(&[MockAuth {
                address: &self.admin,
                invoke: &MockAuthInvoke {
                    contract: &self.contract_id,
                    fn_name: "create_auction",
                    args: (&self.borrower, &self.config).into_val(&self.env),
                    sub_invokes: &[],
                },
            }]);
        }
    }

    #[test]
    fn test_initialize() {
        let t = TestEnv::new();
        let stored: Address = t.env
            .as_contract(&t.contract_id, || t.env.storage().instance().get(&DataKey::Admin))
            .unwrap();
        assert_eq!(stored, t.admin);

        let count: u64 = t.env
            .as_contract(&t.contract_id, || t.env.storage().instance().get(&DataKey::AuctionCount))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_create_auction() {
        let t = TestEnv::new();
        t.mock_admin_auth();

        let id = t.client().create_auction(&t.borrower, &t.config);
        assert_eq!(id, 1);

        let auction = t.client().get_auction(&id);
        assert_eq!(auction.status, AuctionStatus::Active);
        assert_eq!(auction.start_price, 20_000);
        assert_eq!(auction.borrower, t.borrower);
    }

    #[test]
    fn test_price_decay() {
        let t = TestEnv::new();
        t.mock_admin_auth();

        let id = t.client().create_auction(&t.borrower, &t.config);

        let p0 = t.client().get_current_price(&id);
        assert_eq!(p0, 20_000);

        t.env.ledger().set_timestamp(t.env.ledger().timestamp() + 1800);
        let p_mid = t.client().get_current_price(&id);
        assert!(p_mid < 20_000 && p_mid > 14_000);

        t.env.ledger().set_timestamp(t.env.ledger().timestamp() + 1800);
        let p_end = t.client().get_current_price(&id);
        assert_eq!(p_end, 14_000);
    }

    #[test]
    fn test_get_active_auctions() {
        let t = TestEnv::new();
        t.mock_admin_auth();

        t.client().create_auction(&t.borrower, &t.config);
        assert_eq!(t.client().get_active_auctions().len(), 1);
    }

    #[test]
    fn test_analytics_empty() {
        let t = TestEnv::new();
        let a = t.client().get_analytics();
        assert_eq!(a.total_auctions, 0);
        assert_eq!(a.settled_auctions, 0);
        assert_eq!(a.avg_premium_bps, 0);
        assert_eq!(a.avg_time_to_fill_secs, 0);
        assert_eq!(a.total_collateral_liquidated, 0);
    }

    #[test]
    fn test_create_multiple_auctions() {
        let t = TestEnv::new();
        t.env.mock_all_auths();

        let id1 = t.client().create_auction(&t.borrower, &t.config);
        let id2 = t.client().create_auction(&t.borrower, &t.config);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(t.client().get_active_auctions().len(), 2);
    }

    #[test]
    #[should_panic(expected = "auction not found")]
    fn test_auction_not_found() {
        let t = TestEnv::new();
        t.client().get_auction(&999);
    }
}
