use crate::account::{MarginAccount, MarginCallLevel, Position};
use soroban_sdk::Env;

pub fn check_isolated_position_health(
    _env: &Env,
    position: &Position,
    current_price: i128,
) -> MarginCallLevel {
    if position.debt == 0 {
        return MarginCallLevel::Safe;
    }

    let collateral_value = (position.amount * current_price) / 10_000; // Mock normalization

    // Liquidation threshold logic for isolated
    // Let's say if debt > 90% of collateral value -> Force close
    // if debt > 80% -> Liquidation
    // if debt > 70% -> Warning

    let debt_ratio = (position.debt * 100) / collateral_value.max(1);

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

pub fn liquidate_isolated_position(
    env: &Env,
    account: &mut MarginAccount,
    position_index: u32,
    current_price: i128,
) -> Result<(), &'static str> {
    if !account.is_isolated() {
        return Err("Account is not in isolated mode");
    }

    if position_index as usize >= account.positions.len() as usize {
        return Err("Invalid position index");
    }

    let position = account.positions.get(position_index).unwrap();
    let health = check_isolated_position_health(env, &position, current_price);

    if health == MarginCallLevel::Safe || health == MarginCallLevel::Warning {
        return Err("Position is healthy, cannot liquidate");
    }

    // In isolated mode, we just wipe out this single position's debt with its own collateral.
    // We do NOT touch other positions in the account array.

    // Note: In real protocol, you'd swap collateral to debt asset via AMM router here.

    account.positions.remove(position_index);

    Ok(())
}
