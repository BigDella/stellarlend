#![no_std]
#![allow(deprecated)]
mod bridge;
pub mod lending_bridge;

#[cfg(any(test, feature = "testutils"))]
pub use bridge::BridgeContractClient;
pub use bridge::{BridgeContract, ContractError};
pub use lending_bridge::{
    CrossChainLendingPosition, CollateralLock, LiquidityRoute,
    RemoteHealthReport, LendingBridgeStats, LendingBridgeError,
    PositionStatus,
};

#[cfg(test)]
mod math_safety_test;
#[cfg(test)]
mod test;
