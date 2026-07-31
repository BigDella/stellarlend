#[cfg(test)]
mod tests {
    use crate::compound::{compound_interest, compound_interest_continuous};
    use soroban_sdk::Env;

    fn env() -> Env {
        Env::default()
    }

    #[test]
    fn test_compound_interest_zero_periods() {
        let e = env();
        let result = compound_interest(&e, 1_000_000, 500, 0, 10_000).unwrap();
        assert_eq!(result, 1_000_000);
    }

    #[test]
    fn test_compound_interest_one_period() {
        let e = env();
        let result = compound_interest(&e, 1_000_000, 500, 1, 10_000).unwrap();
        assert!(result > 1_000_000);
    }

    #[test]
    fn test_compound_interest_multiple_periods() {
        let e = env();
        let result = compound_interest(&e, 1_000_000, 500, 12, 10_000).unwrap();
        assert!(result > 1_050_000);
    }

    #[test]
    fn test_continuous_compound() {
        let e = env();
        let result = compound_interest_continuous(&e, 1_000_000, 500, 31_536_000).unwrap();
        assert!(result > 1_050_000);
    }

    #[test]
    fn test_compound_interest_zero_principal() {
        let e = env();
        let result = compound_interest(&e, 0, 500, 12, 10_000).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_compound_interest_zero_rate() {
        let e = env();
        let result = compound_interest(&e, 1_000_000, 0, 12, 10_000).unwrap();
        assert_eq!(result, 1_000_000);
    }
}
