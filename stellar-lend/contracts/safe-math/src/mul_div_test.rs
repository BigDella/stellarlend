#[cfg(test)]
mod tests {
    use crate::mul_div::{mul_div, mul_div_ceil, mul_div_floor, mul_div_round_up};
    use soroban_sdk::Env;

    fn env() -> Env {
        Env::default()
    }

    #[test]
    fn test_mul_div_basic() {
        let e = env();
        let result = mul_div(&e, 100, 200, 10).unwrap();
        assert_eq!(result, 2_000);
    }

    #[test]
    fn test_mul_div_round_up() {
        let e = env();
        let result = mul_div_round_up(&e, 10, 3, 4).unwrap();
        assert_eq!(result, 8);
    }

    #[test]
    fn test_mul_div_by_zero() {
        let e = env();
        assert!(mul_div(&e, 100, 200, 0).is_err());
    }

    #[test]
    fn test_mul_div_floor_equals_trunc() {
        let e = env();
        let a = 7;
        let b = 5;
        let d = 3;
        assert_eq!(mul_div_floor(&e, a, b, d), mul_div(&e, a, b, d));
    }

    #[test]
    fn test_mul_div_ceil_equals_round_up() {
        let e = env();
        let a = 7;
        let b = 5;
        let d = 3;
        assert_eq!(mul_div_ceil(&e, a, b, d), mul_div_round_up(&e, a, b, d));
    }

    #[test]
    fn test_mul_div_exact() {
        let e = env();
        let result = mul_div(&e, 10, 10, 5).unwrap();
        assert_eq!(result, 20);
    }

    #[test]
    fn test_mul_div_large_numbers() {
        let e = env();
        let a = 1_000_000_000_000_000_000i128;
        let b = 2_000_000_000_000_000_000i128;
        let d = 1_000_000_000_000_000_000i128;
        let result = mul_div(&e, a, b, d).unwrap();
        assert_eq!(result, b);
    }
}
