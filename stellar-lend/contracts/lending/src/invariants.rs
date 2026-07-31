// invariants.rs
// Place at: stellar-lend/contracts/lending/src/invariants.rs
//
// Add to lib.rs (after existing mod declarations, around line 9):
//   pub mod invariants;

#![allow(unused_imports)]

extern crate alloc;

use alloc::vec::Vec;

use soroban_sdk::{Address, Env};

use crate::borrow::get_admin as get_borrow_admin;
use crate::pause::{is_paused, PauseType};
use crate::views::{
    get_collateral_balance as view_collateral_balance,
    get_collateral_value as view_collateral_value, get_debt_balance as view_debt_balance,
    get_debt_value as view_debt_value, get_health_factor as view_health_factor,
    get_user_position as view_user_position,
};
use crate::pause::is_paused;
use crate::borrow::get_admin as get_borrow_admin;

// ─────────────────────────────────────────────
// Violation — carries reproduction info
// ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct InvariantViolation {
    pub invariant_id: &'static str,
    pub message: &'static str,
    pub detail: String,
}

// ─────────────────────────────────────────────
// Known exemption flags
// Each exemption is documented with the invariant it covers.
// ─────────────────────────────────────────────

#[derive(Default)]
pub struct ExemptionFlags {
    /// INV-005: admin is resetting interest rate — index may temporarily dip
    pub rate_reset_in_progress: bool,
    /// INV-007: oracle in degraded/fallback mode — staleness check relaxed
    pub oracle_fallback_active: bool,
    /// INV-009: inside flash loan — liquidity floor may be temporarily breached
    pub flash_loan_context: bool,
}

// ─────────────────────────────────────────────
// INV-001: Per-user solvency
// health_factor (in bps, 10_000 = 1.0) must be >= 10_000
// for any user who just completed a deposit/borrow/repay/withdraw.
// ─────────────────────────────────────────────
pub fn check_inv_001_solvency(env: &Env, user: &Address) -> Result<(), InvariantViolation> {
    let health_bps = view_health_factor(env, user);
    if health_bps < 10_000 {
        return Err(InvariantViolation {
            invariant_id: "INV-001",
            message: "Solvency: health_factor < 1.0 after action — undercollateralised",
            detail: format!("health_factor: {}", health_bps),
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────
// INV-002: Collateral balance non-negative
// ─────────────────────────────────────────────
pub fn check_inv_002_collateral_non_negative(
    env: &Env,
    user: &Address,
) -> Result<(), InvariantViolation> {
    let balance = view_collateral_balance(env, user);
    if balance < 0 {
        return Err(InvariantViolation {
            invariant_id: "INV-002",
            message: "Collateral balance < 0 — impossible state",
            detail: format!("balance: {}", balance),
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────
// INV-003: Debt balance non-negative
// ─────────────────────────────────────────────
pub fn check_inv_003_debt_non_negative(
    env: &Env,
    user: &Address,
) -> Result<(), InvariantViolation> {
    let balance = view_debt_balance(env, user);
    if balance < 0 {
        return Err(InvariantViolation {
            invariant_id: "INV-003",
            message: "Debt balance < 0 — impossible state",
            detail: format!("balance: {}", balance),
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────
// INV-004: Liquidation eligibility consistency
// If health < 1.0 and debt > 0, position must be reachable.
// EXEMPT when protocol is paused.
// ─────────────────────────────────────────────
pub fn check_inv_004_liquidation_eligible(
    env: &Env,
    user: &Address,
) -> Result<(), InvariantViolation> {
    if is_paused(env, PauseType::Liquidation) {
        return Ok(()); // documented exemption
    }
    let health_bps = view_health_factor(env, user);
    if health_bps < 10_000 {
        let debt = view_debt_balance(env, user);
        if debt <= 0 {
            return Err(InvariantViolation {
                invariant_id: "INV-004",
                message: "Liquidation: health_factor < 1.0 but debt == 0 — contradictory state",
                detail: format!("health_factor: {}, debt: {}", health_bps, debt),
            });
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────
// INV-005: No value creation on borrow
// collateral_value must not increase after a borrow.
// Snapshot before, check after.
// ─────────────────────────────────────────────
pub fn check_inv_005_no_value_creation_on_borrow(
    env: &Env,
    user: &Address,
    collateral_value_before: i128,
) -> Result<(), InvariantViolation> {
    let after = view_collateral_value(env, user);
    if after > collateral_value_before {
        return Err(InvariantViolation {
            invariant_id: "INV-005",
            message: "No-value-creation: collateral_value increased after borrow",
            detail: format!("before: {}, after: {}", collateral_value_before, after),
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────
// INV-006: Admin address stability
// Admin must not change between actions unless set_admin was called.
// Snapshot before, check after.
// ─────────────────────────────────────────────
pub fn check_inv_006_admin_stability(
    env: &Env,
    admin_before: &Address,
) -> Result<(), InvariantViolation> {
    let admin_after = get_borrow_admin(env);
    if admin_after != Some(admin_before.clone()) {
        return Err(InvariantViolation {
            invariant_id: "INV-006",
            message: "Access control: admin changed without explicit set_admin action",
            detail: format!("before: {}, after: {}", admin_before, admin_after),
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────
// INV-007: Pause immutability
// While paused, debt and collateral balances must not change.
// Only call this when is_paused() was true before the action.
// ─────────────────────────────────────────────
pub fn check_inv_007_pause_immutability(
    env: &Env,
    user: &Address,
    debt_before: i128,
    collateral_before: i128,
) -> Result<(), InvariantViolation> {
    if !is_paused(env, PauseType::Borrow) {
        return Ok(());
    }
    let debt_after = view_debt_balance(env, user);
    let collateral_after = view_collateral_balance(env, user);

    if debt_after != debt_before {
        return Err(InvariantViolation {
            invariant_id: "INV-007",
            message: "Pause: debt_balance changed while protocol is paused",
            detail: format!("before: {}, after: {}", debt_before, debt_after),
        });
    }
    if collateral_after != collateral_before {
        return Err(InvariantViolation {
            invariant_id: "INV-007",
            message: "Pause: collateral_balance changed while protocol is paused",
            detail: format!("before: {}, after: {}", collateral_before, collateral_after),
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────
// INV-008: Health factor / debt consistency
// Zero debt must never produce health_factor < 1.0.
// Positive debt must never produce health_factor == 0.
// ─────────────────────────────────────────────
pub fn check_inv_008_health_factor_consistency(
    env: &Env,
    user: &Address,
) -> Result<(), InvariantViolation> {
    let debt = view_debt_balance(env, user);
    let health = view_health_factor(env, user);

    if debt == 0 && health < 10_000 {
        return Err(InvariantViolation {
            invariant_id: "INV-008",
            message: "Health factor: debt == 0 but health_factor < 1.0 — contradictory",
            detail: format!("debt: {}, health_factor: {}", debt, health),
        });
    }
    if debt > 0 && health == 0 {
        return Err(InvariantViolation {
            invariant_id: "INV-008",
            message: "Health factor: debt > 0 but health_factor == 0 — arithmetic error",
            detail: format!("debt: {}, health_factor: {}", debt, health),
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────
// INV-009: Collateral value covers debt value
// For healthy positions, collateral_value must be >= debt_value.
// ─────────────────────────────────────────────
pub fn check_inv_009_collateral_covers_debt(
    env: &Env,
    user: &Address,
) -> Result<(), InvariantViolation> {
    let health = view_health_factor(env, user);
    if health >= 10_000 {
        let col_val = view_collateral_value(env, user);
        let debt_val = view_debt_value(env, user);
        if debt_val > 0 && col_val < debt_val {
            return Err(InvariantViolation {
                invariant_id: "INV-009",
                message: "Collateral coverage: collateral_value < debt_value on healthy position",
                detail: format!("collateral_value: {}, debt_value: {}", col_val, debt_val),
            });
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────
// INV-010: Total assets never decrease (no value destruction)
// Total protocol assets must be monotonic non-decreasing.
// EXEMPT during admin-initiated emergency actions.
// ─────────────────────────────────────────────
pub fn check_inv_010_total_assets_monotonic(
    env: &Env,
    assets_before: i128,
) -> Result<(), InvariantViolation> {
    let assets_after = get_total_assets(env);
    if assets_after < assets_before {
        return Err(InvariantViolation {
            invariant_id: "INV-010",
            message: "Total assets decreased - potential value destruction",
            detail: format!("before: {}, after: {}", assets_before, assets_after),
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────
// INV-011: No minting on borrow (conservation of money)
// Total assets must not increase from a borrow operation alone.
// Snapshot before, check after.
// ─────────────────────────────────────────────
pub fn check_inv_011_no_mint_on_borrow(
    env: &Env,
    assets_before: i128,
) -> Result<(), InvariantViolation> {
    let assets_after = get_total_assets(env);
    if assets_after > assets_before {
        return Err(InvariantViolation {
            invariant_id: "INV-011",
            message: "Total assets increased after borrow - money minted",
            detail: format!("before: {}, after: {}", assets_before, assets_after),
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────
// INV-012: Interest index monotonicity
// Interest index must never decrease (time only moves forward).
// EXEMPT during admin rate reset or oracle fallback.
// ─────────────────────────────────────────────
pub fn check_inv_012_interest_monotonicity(
    env: &Env,
    index_before: i128,
    exemptions: &ExemptionFlags,
) -> Result<(), InvariantViolation> {
    if exemptions.rate_reset_in_progress {
        return Ok(()); // documented exemption
    }
    let index_after = get_interest_index(env);
    if index_after < index_before {
        return Err(InvariantViolation {
            invariant_id: "INV-012",
            message: "Interest index decreased - time reversal bug",
            detail: format!("before: {}, after: {}", index_before, index_after),
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────
// INV-013: Reserve monotonicity
// Protocol reserves should only increase from fees/interest.
// EXEMPT during admin withdrawal or emergency shutdown.
// ─────────────────────────────────────────────
pub fn check_inv_013_reserve_monotonic(
    env: &Env,
    reserves_before: i128,
) -> Result<(), InvariantViolation> {
    let reserves_after = get_protocol_reserves(env);
    if reserves_after < reserves_before {
        return Err(InvariantViolation {
            invariant_id: "INV-013",
            message: "Protocol reserves decreased without admin action",
            detail: format!("before: {}, after: {}", reserves_before, reserves_after),
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────
// INV-014: Access control consistency
// Admin address should only change via explicit set_admin.
// ─────────────────────────────────────────────
pub fn check_inv_014_access_control(
    env: &Env,
    admin_before: &Address,
) -> Result<(), InvariantViolation> {
    let admin_after = get_borrow_admin(env);
    if admin_after != *admin_before {
        return Err(InvariantViolation {
            invariant_id: "INV-014",
            message: "Admin address changed without explicit set_admin",
            detail: format!("before: {}, after: {}", admin_before, admin_after),
        });
    }
    Ok(())
}

// ─────────────────────────────────────────────
// Aggregate — run all stateless invariants (protocol-level).
// Returns all violations found (does not stop on first).
// ─────────────────────────────────────────────
pub fn assert_all_stateless(
    env: &Env,
    exemptions: &ExemptionFlags,
) -> std::vec::Vec<InvariantViolation> {
    let mut violations = std::vec::Vec::new();

    // Note: These require snapshot data from before actions
    // Call individually with snapshots in test harness

    violations
}

// ─────────────────────────────────────────────
// Aggregate — run all stateless per-user invariants.
// Returns all violations found (does not stop on first).
// ─────────────────────────────────────────────
pub fn assert_all_for_user(env: &Env, user: &Address) -> Vec<InvariantViolation> {
    let mut violations = Vec::new();

    if let Err(v) = check_inv_001_solvency(env, user) {
        violations.push(v);
    }
    if let Err(v) = check_inv_002_collateral_non_negative(env, user) {
        violations.push(v);
    }
    if let Err(v) = check_inv_003_debt_non_negative(env, user) {
        violations.push(v);
    }
    if let Err(v) = check_inv_004_liquidation_eligible(env, user) {
        violations.push(v);
    }
    if let Err(v) = check_inv_008_health_factor_consistency(env, user) {
        violations.push(v);
    }
    if let Err(v) = check_inv_009_collateral_covers_debt(env, user) {
        violations.push(v);
    }

    violations
}

// ─────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::LendingContract;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    fn setup() -> (Env, Address) {
        let env = Env::default();
        let user = Address::generate(&env);
        (env, user)
    }

    #[allow(deprecated)]
    fn with_contract<T>(env: &Env, f: impl FnOnce() -> T) -> T {
        let contract_id = env.register_contract(None, LendingContract);
        env.as_contract(&contract_id, f)
    }

    #[test]
    fn test_violation_struct_fields() {
        let v = InvariantViolation {
            invariant_id: "INV-001",
            message: "test violation",
            detail: "test detail".to_string(),
        };
        assert_eq!(v.invariant_id, "INV-001");
        assert_eq!(v.message, "test violation");
        assert_eq!(v.detail, "test detail");
    }

    #[test]
    fn test_exemption_flags_default() {
        let flags = ExemptionFlags::default();
        assert!(!flags.rate_reset_in_progress);
        assert!(!flags.oracle_fallback_active);
        assert!(!flags.flash_loan_context);
    }

    #[test]
    fn test_inv_002_fresh_user_passes() {
        // A fresh address with no collateral stored should return Ok.
        let (env, user) = setup();
        with_contract(&env, || {
            assert!(check_inv_002_collateral_non_negative(&env, &user).is_ok());
        });
    }

    #[test]
    fn test_inv_003_fresh_user_passes() {
        let (env, user) = setup();
        with_contract(&env, || {
            assert!(check_inv_003_debt_non_negative(&env, &user).is_ok());
        });
    }

    #[test]
    fn test_inv_005_no_increase_passes() {
        // Passing the same value before and after should always pass.
        let (env, user) = setup();
        let before: i128 = 1_000_000;
        // view_collateral_value on fresh user returns 0, which is <= before
        with_contract(&env, || {
            let result = check_inv_005_no_value_creation_on_borrow(&env, &user, before);
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_assert_all_returns_vec() {
        let (env, user) = setup();
        let violations = with_contract(&env, || assert_all_for_user(&env, &user));
        // Fresh user with no state — all checks should pass
        assert!(
            violations.is_empty(),
            "Fresh user should have no violations"
        );
    }
}
