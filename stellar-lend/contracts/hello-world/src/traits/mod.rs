use soroban_sdk::{Address, Env};

pub trait InterestModel {
    type Error;

    fn utilization(&self, env: &Env) -> Result<i128, Self::Error>;
    fn borrow_rate(&self, env: &Env) -> Result<i128, Self::Error>;
}

pub trait LiquidationStrategy {
    type Error;

    fn dynamic_penalty(
        &self,
        env: &Env,
        collateral_value: i128,
        total_debt: i128,
    ) -> Result<i128, Self::Error>;
}

pub trait FeeCalculator {
    type Error;

    fn reserve_factor(&self, env: &Env, asset: Option<Address>) -> Result<i128, Self::Error>;
}

pub trait RiskParameters {
    type Error;
    type Params;

    fn params(&self, env: &Env) -> Result<Self::Params, Self::Error>;
}

pub fn registered_modules() -> (&'static str, &'static str, &'static str, &'static str) {
    (
        "default-interest-model/v1",
        "default-liquidation-strategy/v1",
        "dynamic-reserve-factor/v1",
        "default-risk-parameters/v1",
    )
}
