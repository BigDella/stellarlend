pub const MIN_AMOUNT: i128 = 100;
pub const MAX_AMOUNT: i128 = 1_000_000_000;
pub const LARGE_CEILING: i128 = 100_000_000_000;
pub const DEFAULT_PRICE: i128 = 1_000_000;

pub const DEFAULT_COLLATERAL_FACTOR: i128 = 7500;
pub const DEFAULT_LIQUIDATION_THRESHOLD: i128 = 8000;
pub const DEFAULT_RESERVE_FACTOR: i128 = 1000;

pub const MAX_BPS: u64 = 10_000;

pub struct AmountFixtures;

impl AmountFixtures {
    pub const ZERO: i128 = 0;
    pub const SMALL: i128 = 1_000;
    pub const MEDIUM: i128 = 100_000;
    pub const LARGE: i128 = 10_000_000;
    pub const HUGE: i128 = 1_000_000_000;
}

pub struct TimeFixtures;

impl TimeFixtures {
    pub const MINUTE: u64 = 60;
    pub const HOUR: u64 = 3_600;
    pub const DAY: u64 = 86_400;
    pub const WEEK: u64 = 604_800;
    pub const MONTH: u64 = 2_592_000;
    pub const YEAR: u64 = 31_536_000;
}

pub struct RateFixtures;

impl RateFixtures {
    pub const ZERO_PERCENT: i128 = 0;
    pub const ONE_PERCENT: i128 = 100;
    pub const FIVE_PERCENT: i128 = 500;
    pub const TEN_PERCENT: i128 = 1_000;
    pub const FIFTY_PERCENT: i128 = 5_000;
    pub const HUNDRED_PERCENT: i128 = 10_000;
}

pub struct AssetConfigFixture {
    pub collateral_factor: i128,
    pub liquidation_threshold: i128,
    pub reserve_factor: i128,
    pub max_supply: i128,
    pub max_borrow: i128,
    pub price: i128,
}

impl Default for AssetConfigFixture {
    fn default() -> Self {
        Self {
            collateral_factor: DEFAULT_COLLATERAL_FACTOR,
            liquidation_threshold: DEFAULT_LIQUIDATION_THRESHOLD,
            reserve_factor: DEFAULT_RESERVE_FACTOR,
            max_supply: MAX_AMOUNT * 10,
            max_borrow: MAX_AMOUNT * 5,
            price: DEFAULT_PRICE,
        }
    }
}

impl AssetConfigFixture {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_collateral_factor(mut self, factor: i128) -> Self {
        self.collateral_factor = factor;
        self
    }

    pub fn with_liquidation_threshold(mut self, threshold: i128) -> Self {
        self.liquidation_threshold = threshold;
        self
    }

    pub fn with_reserve_factor(mut self, factor: i128) -> Self {
        self.reserve_factor = factor;
        self
    }

    pub fn with_price(mut self, price: i128) -> Self {
        self.price = price;
        self
    }

    pub fn conservative() -> Self {
        Self {
            collateral_factor: 5000,
            liquidation_threshold: 6000,
            reserve_factor: 2000,
            max_supply: MAX_AMOUNT,
            max_borrow: MAX_AMOUNT / 2,
            price: DEFAULT_PRICE,
        }
    }

    pub fn aggressive() -> Self {
        Self {
            collateral_factor: 9000,
            liquidation_threshold: 9500,
            reserve_factor: 500,
            max_supply: MAX_AMOUNT * 100,
            max_borrow: MAX_AMOUNT * 90,
            price: DEFAULT_PRICE,
        }
    }
}
