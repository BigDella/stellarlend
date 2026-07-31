#![no_std]

pub mod integrations;

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SwapRouterError {
    NoSwapRoutes = 1,
    SlippageToleranceExceeded = 2,
    SwapFailed = 3,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum AMMProtocol {
    Phoenix = 0,
    Aquarius = 1,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwapRoute {
    pub protocol: AMMProtocol,
    pub pool_address: Address,
    pub asset_in: Address,
    pub asset_out: Address,
}

#[contract]
pub struct SwapRouterContract;

#[contractimpl]
impl SwapRouterContract {
    pub fn swap_exact_in(
        env: Env,
        caller: Address,
        amount_in: i128,
        min_amount_out: i128,
        routes: Vec<SwapRoute>,
    ) -> Result<i128, SwapRouterError> {
        caller.require_auth();

        if routes.is_empty() {
            return Err(SwapRouterError::NoSwapRoutes);
        }

        let mut current_amount = amount_in;

        for route in routes.iter() {
            let swap_result = match route.protocol {
                AMMProtocol::Phoenix => integrations::phoenix::swap(
                    &env,
                    &route.pool_address,
                    &route.asset_in,
                    &route.asset_out,
                    current_amount,
                ),
                AMMProtocol::Aquarius => integrations::aquarius::swap(
                    &env,
                    &route.pool_address,
                    &route.asset_in,
                    &route.asset_out,
                    current_amount,
                ),
            };

            current_amount = swap_result.map_err(|_| SwapRouterError::SwapFailed)?;
        }

        if current_amount < min_amount_out {
            return Err(SwapRouterError::SlippageToleranceExceeded);
        }

        Ok(current_amount)
    }
}
