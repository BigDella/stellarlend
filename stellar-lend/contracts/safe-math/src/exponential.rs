use crate::error::MathError;
use crate::fixed_point::{fp_mul, WAD};
use soroban_sdk::Env;

pub fn wad_exp(env: &Env, x: i128) -> Result<i128, MathError> {
    if x == 0 {
        return Ok(WAD);
    }
    if x < -42_139_600_000_000_000_000 {
        return Ok(0);
    }
    if x > 130_000_000_000_000_000_000 {
        return Err(MathError::Overflow);
    }
    let mut result = WAD;
    let mut term = WAD;
    let mut k: i128 = 1;
    loop {
        term = fp_mul(env, term, x)?;
        term = term / k;
        let next = result + term;
        if next == result || term == 0 {
            break;
        }
        result = next;
        k += 1;
        if k > 20 {
            break;
        }
    }
    Ok(result)
}

pub fn wad_ln(env: &Env, x: i128) -> Result<i128, MathError> {
    if x <= 0 {
        return Err(MathError::DivisionByZero);
    }
    if x == WAD {
        return Ok(0);
    }
    let mut result = 0i128;
    let mut val = x;
    while val >= 2 * WAD {
        result += WAD;
        val = fp_mul(env, val, WAD / 2)?;
    }
    while val < WAD / 2 {
        result -= WAD;
        val = fp_mul(env, val, 2 * WAD)?;
    }
    let z = fp_mul(env, val - WAD, WAD + WAD)?;
    let y = val - WAD;
    let mut ln = z;
    let mut power = z;
    for i in 2..=10 {
        power = fp_mul(env, power, y)?;
        let term = power / (i as i128);
        if term == 0 {
            break;
        }
        if i % 2 == 0 {
            ln -= term;
        } else {
            ln += term;
        }
    }
    Ok(result + ln)
}
