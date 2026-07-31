#![no_std]

pub mod error;
pub mod fixed_point;
pub mod int128;
pub mod compound;
pub mod exponential;
pub mod mul_div;
pub mod precision;
pub mod rounding;

pub use error::MathError;

pub use int128::{
    bps_mul, bps_mul_u128, safe_add, safe_add_u128, safe_div, safe_div_u128, safe_mul,
    safe_mul_u128, safe_pow, safe_sqrt, safe_sqrt_u128, safe_sub, safe_sub_u128,
    unchecked_add, unchecked_add_u128, unchecked_div, unchecked_mul, unchecked_mul_u128,
    unchecked_sub, unchecked_sub_u128,
};

pub use fixed_point::{
    bps_ratio, fp_add, fp_div, fp_mul, fp_pow, fp_sqrt, fp_sub, ray_div, ray_mul,
    ray_to_wad, simple_interest, wad_to_ray, HALF_WAD, RAY, SECONDS_PER_YEAR, WAD,
};

pub use compound::{compound_interest, compound_interest_continuous};
pub use exponential::{wad_exp, wad_ln};
pub use mul_div::{mul_div, mul_div_ceil, mul_div_floor, mul_div_round_up, mul_div_with_safety, mul_div_with_rounding};
pub use precision::{PrecisionLoss, PrecisionTracker};
pub use rounding::{round_down, round_nearest, round_up, RoundingMode};
