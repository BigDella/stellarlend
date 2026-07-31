//! # StellarLend Safe-Math Library
//!
//! Overflow-safe arithmetic for Soroban smart contracts, with formal
//! verification proofs via Kani / SMT-LIB.
//!
//! ## Module overview
//!
//! | Module         | Contents                                                     |
//! |----------------|--------------------------------------------------------------|
//! | `error`        | [`MathError`] — unified error enum                          |
//! | `int128`       | `safe_add/sub/mul/div/pow/sqrt` for `i128` (no `Env` needed)|
//! | `fixed_point`  | WAD-scaled 18-decimal ops using `I256` intermediates        |
//!
//! ## Usage in contracts
//!
//! ```ignore
//! use stellarlend_safe_math::{safe_mul, safe_div, fp_mul, WAD, MathError};
//!
//! fn utilization(debt: i128, supply: i128) -> Result<i128, MathError> {
//!     safe_mul(debt, 10_000).and_then(|v| safe_div(v, supply))
//! }
//! ```
//!
//! ## Formal verification
//!
//! SMT-LIB specifications and Kani proof harnesses live in
//! `formal-verification/safe-math-proofs/`.  Run proofs with:
//!
//! ```sh
//! cargo kani --manifest-path formal-verification/safe-math-proofs/Cargo.toml
//! ```

#![no_std]

pub mod error;
pub mod fixed_point;
pub mod int128;

// Flat re-exports for ergonomic use in contracts.
pub use error::MathError;

pub use int128::{
    bps_mul, bps_mul_u128, safe_add, safe_add_u128, safe_div, safe_div_u128, safe_mul,
    safe_mul_u128, safe_pow, safe_sqrt, safe_sqrt_u128, safe_sub, safe_sub_u128,
};

pub use fixed_point::{
    bps_ratio, fp_add, fp_div, fp_mul, fp_pow, fp_sqrt, fp_sub, simple_interest, SECONDS_PER_YEAR,
    WAD,
};

pub mod compound;
pub mod exponential;
pub mod mul_div;
pub mod precision;
pub mod rounding;

pub use compound::{compound_interest, compound_interest_continuous};
pub use exponential::{wad_exp, wad_ln};
pub use mul_div::{mul_div, mul_div_ceil, mul_div_floor, mul_div_round_up};
pub use precision::{PrecisionLoss, PrecisionTracker};
pub use rounding::{round_down, round_nearest, round_up, RoundingMode};
