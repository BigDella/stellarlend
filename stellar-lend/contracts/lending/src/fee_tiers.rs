#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tier {
    pub min_deposits: i128,
    pub min_borrow_volume: i128,
    pub min_account_ledgers: u32,
    pub min_loyal_ledgers: u32,
    pub discount_bps: u32,
    pub loyalty_bonus_bps: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Metrics {
    pub deposits: i128,
    pub borrow_volume: i128,
    pub account_ledgers: u32,
    pub loyal_ledgers: u32,
}
pub fn eligible(metrics: Metrics, tier: Tier) -> bool {
    metrics.deposits >= tier.min_deposits
        && metrics.borrow_volume >= tier.min_borrow_volume
        && metrics.account_ledgers >= tier.min_account_ledgers
        && metrics.loyal_ledgers >= tier.min_loyal_ledgers
}
pub fn apply_discount(base_fee: i128, minimum_fee: i128, discount_bps: u32) -> i128 {
    let capped = discount_bps.min(5_000) as i128;
    let discounted = base_fee.saturating_mul(10_000 - capped) / 10_000;
    discounted.max(minimum_fee)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discount_is_capped_and_minimum_enforced() {
        assert_eq!(apply_discount(100, 60, 9_000), 60);
    }
    #[test]
    fn checks_all_loyalty_inputs() {
        let tier = Tier {
            min_deposits: 100,
            min_borrow_volume: 50,
            min_account_ledgers: 10,
            min_loyal_ledgers: 5,
            discount_bps: 1000,
            loyalty_bonus_bps: 100,
        };
        assert!(eligible(
            Metrics {
                deposits: 100,
                borrow_volume: 50,
                account_ledgers: 10,
                loyal_ledgers: 5
            },
            tier
        ));
    }
}
