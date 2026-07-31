use liquidation_auction::{LiquidationAuctionContract, LiquidationAuctionContractClient};
use soroban_sdk::{Address, Env};

// Example: deploy the contract
pub fn deploy(env: &Env) {
    let contract_id = env
        .deployer()
        .with_current_contract("")
        .deploy(LiquidationAuctionContract);
    let _client = LiquidationAuctionContractClient::new(env, &contract_id);
}
