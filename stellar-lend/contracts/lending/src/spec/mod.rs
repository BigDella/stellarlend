//! # Formal Verification Specification — StellarLend Lending Protocol
//!
//! This module contains the machine-checkable formal specifications for every
//! critical function in the StellarLend lending contract.  All specifications
//! are written as Rust `#[cfg(test)]` functions so that they compile as part of
//! the normal test suite and can also be consumed by external property-based
//! verifiers (e.g. Kani, Prusti, or proptest).
//!
//! ## Organisation
//!
//! | Sub-module           | Target function(s)                          |
//! |----------------------|---------------------------------------------|
//! | `interest`           | `calculate_interest`                        |
//! | `collateral`         | `validate_collateral_ratio`                 |
//! | `accounting`         | `borrow`, `repay` — total-debt invariants   |
//! | `health_factor`      | `compute_health_factor`                     |
//! | `liquidation`        | `get_max_liquidatable_amount`               |
//! | `rewards`            | `update_global_index`, `update_user`        |
//! | `deposit`            | `deposit` — cap / overflow properties       |
//! | `withdraw`           | `withdraw` — collateral-ratio post-cond     |
//!
//! ## How to run
//!
//! ```bash
//! # Property-based tests (fast, in-process)
//! cargo test -p lending --features spec -- spec::
//!
//! # Kani model-checking (exhaustive bounded verification)
//! cargo kani --package lending --harness spec::
//! ```
//!
//! ## Assumptions (global)
//!
//! * All `i128` arithmetic is checked (panics on overflow in debug, uses
//!   `checked_*` in production).  This is enforced by `overflow-checks = true`
//!   in the release profile and by the use of `checked_add` / `checked_mul`
//!   throughout the implementation.
//! * Ledger timestamps are monotonically non-decreasing within a single
//!   ledger cycle.  The Soroban runtime enforces this.
//! * The oracle contract is trusted; its price return is assumed to be ≥ 0.

#[cfg(any(test, feature = "spec"))]
pub mod accounting;
#[cfg(any(test, feature = "spec"))]
pub mod collateral;
#[cfg(any(test, feature = "spec"))]
pub mod deposit_spec;
#[cfg(any(test, feature = "spec"))]
pub mod health_factor;
#[cfg(any(test, feature = "spec"))]
pub mod interest;
#[cfg(any(test, feature = "spec"))]
pub mod liquidation;
#[cfg(any(test, feature = "spec"))]
pub mod protocol_invariants;
#[cfg(any(test, feature = "spec"))]
pub mod rewards;
#[cfg(any(test, feature = "spec"))]
pub mod withdraw_spec;
