use crate::error::MathError;
use soroban_sdk::{Env, I256};

pub fn mul_div(env: &Env, a: i128, b: i128, denominator: i128) -> Result<i128, MathError> {
    if denominator == 0 {
        return Err(MathError::DivisionByZero);
    }
    let a256 = I256::from_i128(env, a);
    let b256 = I256::from_i128(env, b);
    let d256 = I256::from_i128(env, denominator);
    let product = a256.mul(&b256);
    let result = product.div(&d256);
    result.to_i128().ok_or(MathError::Overflow)
}

pub fn mul_div_round_up(
    env: &Env,
    a: i128,
    b: i128,
    denominator: i128,
) -> Result<i128, MathError> {
    if denominator == 0 {
        return Err(MathError::DivisionByZero);
    }
    let a256 = I256::from_i128(env, a);
    let b256 = I256::from_i128(env, b);
    let d256 = I256::from_i128(env, denominator);
    let product = a256.mul(&b256);
    let remainder = product.rem_euclid(&d256);
    let result = product.div(&d256);
    let zero = I256::from_i128(env, 0);
    if remainder > zero {
        let one = I256::from_i128(env, 1);
        result
            .add(&one)
            .to_i128()
            .ok_or(MathError::Overflow)
    } else {
        result.to_i128().ok_or(MathError::Overflow)
    }
}

pub fn mul_div_floor(env: &Env, a: i128, b: i128, denominator: i128) -> Result<i128, MathError> {
    mul_div(env, a, b, denominator)
}

pub fn mul_div_ceil(env: &Env, a: i128, b: i128, denominator: i128) -> Result<i128, MathError> {
    mul_div_round_up(env, a, b, denominator)
}
