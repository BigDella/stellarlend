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
    let seconds_per_year: i128 = 31_536_000;
    let rate_scaled = fp_mul(env, annual_rate_bps, elapsed_secs as i128)?;
    let rate_scaled = fp_div(env, rate_scaled, seconds_per_year)?;
    let factor = crate::exponential::wad_exp(env, rate_scaled)?;
    fp_mul(env, principal, factor)
}
