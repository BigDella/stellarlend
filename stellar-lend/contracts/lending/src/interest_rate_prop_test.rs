use proptest::prelude::*;

const BASE_RATE: i128 = 100;
const KINK: i128 = 8_000;
const SLOPE: i128 = 2_000;
const JUMP_SLOPE: i128 = 10_000;
const FLOOR: i128 = 0;
const CEILING: i128 = 10_000;

fn model_rate(util: i128) -> i128 {
    let raw = if util <= KINK {
        BASE_RATE + util * SLOPE / KINK
    } else {
        let over = util - KINK;
        let range = 10_000 - KINK;
        BASE_RATE
            + SLOPE
            + if range == 0 {
                0
            } else {
                over * JUMP_SLOPE / range
            }
    };
    raw.max(FLOOR).min(CEILING)
}

proptest! {
    /// PROP-IR-01/02: rate always within [floor, ceiling]
    #[test]
    fn prop_rate_within_bounds(util in 0_i128..=10_000_i128) {
        let rate = model_rate(util);
        prop_assert!(rate >= FLOOR,   "rate {} below floor at util {}", rate, util);
        prop_assert!(rate <= CEILING, "rate {} above ceiling at util {}", rate, util);
    }

    /// PROP-IR-03: rate is monotonically non-decreasing
    #[test]
    fn prop_rate_monotonic(util_a in 0_i128..=9_999_i128, delta in 1_i128..=1_000_i128) {
        let util_b = (util_a + delta).min(10_000);
        prop_assert!(model_rate(util_b) >= model_rate(util_a),
            "rate not monotonic: util_a={} rate_a={}, util_b={} rate_b={}",
            util_a, model_rate(util_a), util_b, model_rate(util_b));
    }

    /// PROP-IR-04: supply rate <= borrow rate
    #[test]
    fn prop_supply_le_borrow(util in 0_i128..=10_000_i128) {
        let borrow_rate = model_rate(util);
        let supply_rate = borrow_rate * util / 10_000;
        prop_assert!(supply_rate <= borrow_rate);
    }

    /// PROP-IR-05: zero utilisation equals base rate
    #[test]
    fn prop_zero_util_equals_base(_dummy in 0_u8..=0_u8) {
        prop_assert_eq!(model_rate(0), BASE_RATE);
    }
}
