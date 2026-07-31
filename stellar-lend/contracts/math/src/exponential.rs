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

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    fn env() -> Env {
        Env::default()
    }

    #[test]
    fn wad_exp_zero() {
        let e = env();
        assert_eq!(wad_exp(&e, 0).unwrap(), WAD);
    }

    #[test]
    fn wad_exp_small_positive() {
        let e = env();
        let result = wad_exp(&e, WAD / 100).unwrap();
        assert!(result > WAD);
    }

    #[test]
    fn wad_exp_small_negative() {
        let e = env();
        let result = wad_exp(&e, -WAD / 100).unwrap();
        assert!(result < WAD);
        assert!(result > 0);
    }

    #[test]
    fn wad_exp_large_negative() {
        let e = env();
        let result = wad_exp(&e, -50_000_000_000_000_000_000i128).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn wad_exp_large_positive_overflow() {
        let e = env();
        assert_eq!(
            wad_exp(&e, 200_000_000_000_000_000_000i128),
            Err(MathError::Overflow)
        );
    }

    #[test]
    fn wad_exp_negative() {
        let e = env();
        let result = wad_exp(&e, -WAD).unwrap();
        assert!(result < WAD);
        assert!(result > 0);
        assert!(result < WAD / 2);
    }

    #[test]
    fn wad_ln_one() {
        let e = env();
        assert_eq!(wad_ln(&e, WAD).unwrap(), 0);
    }

    #[test]
    fn wad_ln_of_e_is_positive() {
        let e = env();
        let e_approx = 2_718281828459045235i128;
        let result = wad_ln(&e, e_approx).unwrap();
        assert!(result > 0, "wad_ln(e) should be positive, got {result}");
    }

    #[test]
    fn wad_ln_greater_than_one() {
        let e = env();
        let result = wad_ln(&e, 2 * WAD).unwrap();
        assert!(result > 0);
    }

    #[test]
    fn wad_ln_less_than_one() {
        let e = env();
        let result = wad_ln(&e, WAD / 2).unwrap();
        assert!(result < 0);
    }

    #[test]
    fn wad_ln_zero_or_negative() {
        let e = env();
        assert_eq!(wad_ln(&e, 0), Err(MathError::DivisionByZero));
        assert_eq!(wad_ln(&e, -WAD), Err(MathError::DivisionByZero));
    }

    #[test]
    fn wad_ln_large_values() {
        let e = env();
        let result = wad_ln(&e, 100 * WAD).unwrap();
        assert!(result > WAD);
    }

    #[test]
    fn wad_ln_small_values() {
        let e = env();
        let result = wad_ln(&e, WAD / 100).unwrap();
        assert!(result < 0);
    }
}
