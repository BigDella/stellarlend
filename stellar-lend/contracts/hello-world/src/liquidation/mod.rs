use soroban_sdk::Env;

use crate::liquidate::LiquidationError;
use crate::traits::LiquidationStrategy;

pub struct DefaultLiquidationStrategy;

impl LiquidationStrategy for DefaultLiquidationStrategy {
    type Error = LiquidationError;

    fn dynamic_penalty(
        &self,
        env: &Env,
        collateral_value: i128,
        total_debt: i128,
    ) -> Result<i128, Self::Error> {
        crate::liquidate::calculate_dynamic_penalty(env, collateral_value, total_debt)
    }
}
