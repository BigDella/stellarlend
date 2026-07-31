use crate::account::{MarginAccount, MarginCallLevel};
use soroban_sdk::Env;

pub fn check_cross_margin_health(_env: &Env, account: &MarginAccount) -> MarginCallLevel {
    if account.total_debt_value == 0 {
        return MarginCallLevel::Safe;
    }

    // In cross margin, health is aggregated across all positions.
    // We use the account's total collateral value vs total debt value.
    let debt_ratio = (account.total_debt_value * 100) / account.total_collateral_value.max(1);

    if debt_ratio >= 90 {
        MarginCallLevel::ForcedClose
    } else if debt_ratio >= 80 {
        MarginCallLevel::Liquidation
    } else if debt_ratio >= 70 {
        MarginCallLevel::Warning
    } else {
        MarginCallLevel::Safe
    }
}

pub fn liquidate_cross_margin_account(
    env: &Env,
    account: &mut MarginAccount,
) -> Result<(), &'static str> {
    if !account.is_cross() {
        return Err("Account is not in cross margin mode");
    }

    let health = check_cross_margin_health(env, account);

    if health == MarginCallLevel::Safe || health == MarginCallLevel::Warning {
        return Err("Account is healthy, cannot liquidate");
    }

    // In cross-margin mode, liquidator can use ANY collateral in the account to cover the debt.
    // For simplicity, we assume the liquidator seizes the entire account positions up to debt coverage.

    // Wipe all positions to simulate forced close (in a real scenario, partial liquidation may occur).
    account.positions = soroban_sdk::Vec::new(env);
    account.total_collateral_value = 0;
    account.total_debt_value = 0;

    Ok(())
}
