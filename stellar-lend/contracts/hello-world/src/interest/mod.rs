use soroban_sdk::Env;

use crate::interest_rate::InterestRateError;
use crate::traits::InterestModel;

pub struct DefaultInterestModel;

impl InterestModel for DefaultInterestModel {
    type Error = InterestRateError;

    fn utilization(&self, env: &Env) -> Result<i128, Self::Error> {
        crate::interest_rate::calculate_utilization(env)
    }

    fn borrow_rate(&self, env: &Env) -> Result<i128, Self::Error> {
        crate::interest_rate::calculate_borrow_rate(env)
    }
}
