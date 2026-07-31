use crate::error::MathError;
use crate::fixed_point::{fp_div, fp_mul, WAD};
use crate::int128::safe_add;
use soroban_sdk::Env;

pub fn compound_interest(
    env: &Env,
    principal: i128,
    rate_bps: i128,
    periods: u32,
    bps_scale: i128,
) -> Result<i128, MathError> {
    if principal == 0 || rate_bps == 0 || periods == 0 {
        return Ok(principal);
    }
    let mut amount = principal;
    for _ in 0..periods {
        let interest = fp_mul(env, amount, rate_bps)?;
        let interest = fp_div(env, interest, bps_scale)?;
        amount = safe_add(amount, interest)?;
    }
    Ok(amount)
}

pub fn compound_interest_continuous(
    env: &Env,
    principal: i128,
    annual_rate_bps: i128,
    elapsed_secs: u64,
) -> Result<i128, MathError> {
    if principal == 0 || annual_rate_bps == 0 || elapsed_secs == 0 {
        return Ok(principal);
    }
    let secs_per_year: i128 = 31_536_000;
    let rate_wad = crate::int128::safe_mul(annual_rate_bps, WAD / 10_000)?;
    let rate_t = crate::int128::safe_mul(rate_wad, elapsed_secs as i128)?;
    let rate_scaled = crate::int128::safe_div(rate_t, secs_per_year)?;
    let factor = crate::exponential::wad_exp(env, rate_scaled)?;
    fp_mul(env, principal, factor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    fn env() -> Env {
        Env::default()
    }

    #[test]
    fn compound_interest_no_periods() {
        let e = env();
        assert_eq!(
            compound_interest(&e, 1000, 500, 0, crate::fixed_point::WAD).unwrap(),
            1000
        );
    }

    #[test]
    fn compound_interest_zero_principal() {
        let e = env();
        assert_eq!(
            compound_interest(&e, 0, 500, 10, crate::fixed_point::WAD).unwrap(),
            0
        );
    }

    #[test]
    fn compound_interest_zero_rate() {
        let e = env();
        assert_eq!(
            compound_interest(&e, 1000, 0, 10, crate::fixed_point::WAD).unwrap(),
            1000
        );
    }

    #[test]
    fn compound_interest_single_period() {
        let e = env();
        let principal = 10_000 * crate::WAD;
        let rate = 500i128;
        let result = compound_interest(&e, principal, rate, 1, 10_000).unwrap();
        assert!(result > principal, "result {result} <= principal {principal}");
        assert!(result < principal + principal / 10);
    }

    #[test]
    fn compound_interest_multiple_periods() {
        let e = env();
        let principal = 1_000 * crate::WAD;
        let rate = 100i128;
        let result = compound_interest(&e, principal, rate, 12, 10_000).unwrap();
        assert!(result > principal, "result {result} <= principal {principal}");
        assert!(result < principal * 2);
    }

    #[test]
    fn compound_interest_continuous_no_elapsed() {
        let e = env();
        assert_eq!(
            compound_interest_continuous(&e, 1000, 500, 0).unwrap(),
            1000
        );
    }

    #[test]
    fn compound_interest_continuous_basic() {
        let e = env();
        let principal = 1_000_000i128;
        let rate = 500i128;
        let secs = 31_536_000u64;
        let result = compound_interest_continuous(&e, principal, rate, secs).unwrap();
        assert!(result > principal);
        assert!(
            result < principal + principal / 10,
            "result {result} should be < {}+10%",
            principal
        );
    }

    #[test]
    fn compound_interest_continuous_short_period() {
        let e = env();
        let principal = 1_000_000_000i128;
        let rate = 1000i128;
        let secs = 3600u64;
        let result = compound_interest_continuous(&e, principal, rate, secs).unwrap();
        assert!(result > principal);
    }
}
