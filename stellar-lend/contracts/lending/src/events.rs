pub use shared_events::*;
//! Contract events for the lending workspace (`LendingContract`, `DataStore`, `UpgradeManager`).
//!
//! # Indexer / off-chain consumers
//!
//! Each event is emitted via the Soroban `contractevent` macro and the generated `publish` helper
//! on the event value, which routes to `Events::publish_event` (not the deprecated `Events::publish`).
//!
//! ## Topic layout
//!
//! - **Lending (main contract)**  
//!   - `borrow_event`, `repay_event`, `withdraw_event`, `flash_loan_event`: first topic is the
//!     event type name in snake_case (Soroban default).  
//!   - `pause_event`: defined on [`crate::pause::PauseEvent`] in the pause module (shares
//!     [`crate::pause::PauseType`] with storage).  
//!   - **Vault vs borrow collateral adds** both use static topic `deposit_event` (see
//!     [`VaultDepositEvent`] and [`BorrowCollateralDepositEvent`]); payloads differ: vault deposits
//!     include `new_balance`; borrow collateral deposits do not.
//!
//! - **Data store contract** — static prefixes: `ds_init`, `writer`, `ds_save`, `ds_bkup`,
//!   `ds_rest`, `ds_migr`, followed by any `#[topic]` fields in struct order.
//!
//! - **Upgrade manager** — static prefixes: `up_init`, `up_apadd`, `up_prop`, `up_appr`, `up_exec`,
//!   `up_roll`, plus `#[topic]` fields as before.

use soroban_sdk::{contractevent, contracttype, Address, Env, String};

use crate::interest_rate::InterestRateConfig;

// ─── Lending (LendingContract) ─────────────────────────────────────────────



/// Collateral added to a borrow position (same static topic as vault deposits; distinguish by payload).




/// Vault / pool deposit (same static topic as [`BorrowCollateralDepositEvent`]; includes `new_balance`).












#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]










// ─── Data store contract ────────────────────────────────────────────────────













// ─── Upgrade manager contract ──────────────────────────────────────────────

















#[allow(deprecated)]
pub fn emit_bad_debt(env: &Env, user: &Address, amount: i128) {
    env.events().publish(("bad_debt",), (user.clone(), amount));
}

#[allow(deprecated)]
pub fn emit_bad_debt_recovered(env: &Env, amount: i128) {
    env.events().publish(("bad_debt_recovered",), (amount,));
}
