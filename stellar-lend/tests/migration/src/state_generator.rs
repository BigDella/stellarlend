use crate::storage::StorageContext;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use soroban_sdk::{Bytes, String as SorobanString};

pub struct StateGenerator {
    pub rng: StdRng,
}

impl StateGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    pub fn generate_positions(
        &mut self,
        ctx: &StorageContext,
        keys: &mut Vec<SorobanString>,
        values: &mut Vec<Bytes>,
        count: usize,
    ) {
        for i in 0..count {
            let collateral: i128 = self.rng.gen_range(100_000_000..10_000_000_000_000);
            let debt: i128 = self.rng.gen_range(0..collateral / 2);

            let env = ctx.env.clone();
            let key = SorobanString::from_str(&env, &format!("user_{}_collateral", i));
            let val = Bytes::from_slice(&env, &collateral.to_be_bytes());
            keys.push(key);
            values.push(val);

            let key = SorobanString::from_str(&env, &format!("user_{}_debt", i));
            let val = Bytes::from_slice(&env, &debt.to_be_bytes());
            keys.push(key);
            values.push(val);
        }
    }

    pub fn generate_prices(
        &mut self,
        ctx: &StorageContext,
        keys: &mut Vec<SorobanString>,
        values: &mut Vec<Bytes>,
        count: usize,
    ) {
        let assets = ["XLM", "USDC", "BTC", "ETH", "SOL"];
        for i in 0..count.min(assets.len()) {
            let price: i128 = self.rng.gen_range(1_000_000..100_000_000_000);
            let env = ctx.env.clone();
            let key = SorobanString::from_str(&env, &format!("price_{}", assets[i]));
            let val = Bytes::from_slice(&env, &price.to_be_bytes());
            keys.push(key);
            values.push(val);
        }
    }

    pub fn generate_configs(
        &mut self,
        ctx: &StorageContext,
        keys: &mut Vec<SorobanString>,
        values: &mut Vec<Bytes>,
    ) {
        let env = ctx.env.clone();
        let configs: Vec<(&str, u32)> = vec![
            ("config_ltv", self.rng.gen_range(5000..8000)),
            ("config_liquidation_threshold", self.rng.gen_range(8000..9500)),
            ("config_interest_rate", self.rng.gen_range(100..2000)),
            ("config_reserve_factor", self.rng.gen_range(500..2000)),
        ];
        for (name, val) in configs {
            let key = SorobanString::from_str(&env, name);
            let value = Bytes::from_slice(&env, &val.to_be_bytes());
            keys.push(key);
            values.push(value);
        }
    }

    pub fn generate_state(
        &mut self,
        ctx: &StorageContext,
        keys: &mut Vec<SorobanString>,
        values: &mut Vec<Bytes>,
        num_users: usize,
        num_prices: usize,
    ) {
        self.generate_positions(ctx, keys, values, num_users);
        self.generate_prices(ctx, keys, values, num_prices);
        self.generate_configs(ctx, keys, values);
    }
}
