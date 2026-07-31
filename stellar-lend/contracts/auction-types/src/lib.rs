#![no_std]

use soroban_sdk::{contracterror, contracttype, Address, BytesN};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum AuctionState {
    Pending,
    Active,
    PartiallyFilled,
    FullyFilled,
    Settled,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct AuctionConfig {
    pub starting_premium_bps: i128,
    pub floor_discount_bps: i128,
    pub auction_duration: u64,
    pub price_decay_function: PriceDecayFunction,
    pub min_bid_size: i128,
    pub commit_phase_duration: u64,
    pub reveal_phase_duration: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum PriceDecayFunction {
    Linear,
    Exponential,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Auction {
    pub id: u64,
    pub pool: Address,
    pub user: Address,
    pub collateral_asset: Address,
    pub debt_asset: Address,
    pub collateral_amount: i128,
    pub debt_amount: i128,
    pub start_time: u64,
    pub end_time: u64,
    pub starting_price: i128,
    pub floor_price: i128,
    pub current_price: i128,
    pub remaining_collateral: i128,
    pub filled_amount: i128,
    pub state: AuctionState,
    pub config: AuctionConfig,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct BidCommitment {
    pub bidder: Address,
    pub commitment_hash: BytesN<32>,
    pub auction_id: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct BidReveal {
    pub bidder: Address,
    pub auction_id: u64,
    pub collateral_amount: i128,
    pub max_price: i128,
    pub nonce: BytesN<32>,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct BidResult {
    pub auction_id: u64,
    pub bidder: Address,
    pub collateral_filled: i128,
    pub price_paid: i128,
    pub debt_repaid: i128,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AuctionError {
    AlreadyInitialized = 1,
    NotAuthorized = 2,
    AuctionNotFound = 3,
    InvalidState = 4,
    AuctionExpired = 5,
    AuctionNotExpired = 6,
    InsufficientCollateral = 7,
    InvalidBidAmount = 8,
    CommitmentAlreadyExists = 9,
    CommitmentNotFound = 10,
    RevealPhaseNotActive = 11,
    InvalidReveal = 12,
    BidTooLow = 13,
    AlreadyFilled = 14,
    InvalidConfig = 15,
    AuctionStillActive = 16,
    PositionHealthRestored = 17,
    NoBidsReceived = 18,
}
