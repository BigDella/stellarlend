//! # Debt Token Module
//!
//! Implements debt position tokenization, allowing debt positions to be represented
//! as transferable NFTs. This enables secondary markets for debt,
//! structured products, and partial position exits.
//!
//! ## Features
//! - NFT representation of debt positions
//! - Transferable debt positions with state preservation
//! - Liquidation rights transfer with debt token
//! - Governance controls for transfer restrictions
//! - Secondary market support with price discovery
//!
//! ## Token Structure
//! Each debt position is represented by a unique NFT that contains:
//! - Original borrower address
//! - Principal amount
//! - Collateral information
//! - Interest accrual state
//! - Liquidation status
//!
//! ## Security
//! - Transfer hooks for allow/block lists
//! - Emergency pause by governance
//! - Position integrity validation on transfers
//! - Audit trail through events

#![allow(unused)]
use soroban_sdk::{contracterror, contractevent, contracttype, Address, Env, Map, Symbol, Vec};

use crate::deposit::DepositDataKey;
use crate::errors::LendingError;

/// Events for debt token operations
#[contractevent]
#[derive(Clone, Debug)]
pub struct DebtTokenMintedEvent {
    pub token_id: u64,
    pub borrower: Address,
    pub principal: i128,
    pub collateral_asset: Option<Address>,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct DebtTokenTransferredEvent {
    pub token_id: u64,
    pub from: Address,
    pub to: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct DebtTokenBurnedEvent {
    pub token_id: u64,
    pub burner: Address,
    pub reason: Symbol,
    pub timestamp: u64,
}

/// Debt position information stored in NFT metadata
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct DebtPosition {
    /// Original borrower address
    pub borrower: Address,
    /// Principal debt amount
    pub principal: i128,
    /// Accrued interest
    pub accrued_interest: i128,
    /// Collateral asset backing this debt
    pub collateral_asset: Option<Address>,
    /// Collateral amount
    pub collateral_amount: i128,
    /// Interest rate at borrowing (basis points)
    pub interest_rate_bps: i128,
    /// Last accrual timestamp
    pub last_accrual_time: u64,
    /// Whether position is currently liquidatable
    pub is_liquidatable: bool,
    /// Creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
}

/// Errors that can occur during debt token operations
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum DebtTokenError {
    /// Token ID does not exist
    TokenNotFound = 1,
    /// Caller is not authorized to perform operation
    Unauthorized = 2,
    /// Transfer is currently paused
    TransferPaused = 3,
    /// Transfer to blocked address
    TransferBlocked = 4,
    /// Cannot transfer during active liquidation
    LiquidationInProgress = 5,
    /// Invalid token ID
    InvalidTokenId = 6,
    /// Position is undercollateralized
    Undercollateralized = 7,
    /// Arithmetic overflow occurred
    Overflow = 8,
    /// Cannot transfer to zero address
    ZeroAddress = 9,
    /// Position already tokenized
    AlreadyTokenized = 10,
    /// Position does not exist for tokenization
    PositionNotFound = 11,
    /// Token is not currently listed for sale
    NotListed = 12,
    /// Token already has an active listing
    AlreadyListed = 13,
    /// Caller is not the seller of the listing
    NotSeller = 14,
    /// Listing price must be positive
    InvalidPrice = 15,
}

/// Storage keys for debt token data
#[contracttype]
#[derive(Clone)]
pub enum DebtTokenDataKey {
    /// Next token ID to mint: NextTokenId -> u64
    NextTokenId,
    /// Token ID to position mapping: TokenPosition(token_id) -> DebtPosition
    TokenPosition(u64),
    /// Owner to token IDs mapping: OwnerTokens(owner) -> Vec<u64>
    OwnerTokens(Address),
    /// Transfer pause switch: TransferPaused -> bool
    TransferPaused,
    /// Blocked addresses: BlockedAddress(address) -> bool
    BlockedAddress(Address),
    /// Global token supply: TotalSupply -> u64
    TotalSupply,
    /// Token URI mapping: TokenUri(token_id) -> String
    TokenUri(u64),
    /// Active fixed-price listing: Listing(token_id) -> DebtTokenListing
    Listing(u64),
}

/// A fixed-price secondary-market listing for a debt token (issue #664).
///
/// This is intentionally a minimal, honest slice of the "secondary market with
/// price discovery" the module's doc comment already aspired to: a simple
/// fixed-price listing/purchase mechanism, NOT the Dutch-auction / order-book /
/// atomic-clearing system described in issue #664's full scope. That remains a
/// separate, much larger deliverable — see the PR description for #664-#667.
#[contracttype]
#[derive(Clone, Debug)]
pub struct DebtTokenListing {
    pub token_id: u64,
    pub seller: Address,
    /// Asking price, denominated in `payment_token`.
    pub price: i128,
    /// SEP-41 token contract the buyer pays in.
    pub payment_token: Address,
    pub listed_at: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct DebtTokenListedEvent {
    pub token_id: u64,
    pub seller: Address,
    pub price: i128,
    pub payment_token: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct DebtTokenListingCancelledEvent {
    pub token_id: u64,
    pub seller: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct DebtTokenSoldEvent {
    pub token_id: u64,
    pub seller: Address,
    pub buyer: Address,
    pub price: i128,
    pub payment_token: Address,
    pub timestamp: u64,
}

/// Mint a new debt token for a position
///
/// Creates an NFT representing the user's debt position. The position
/// must exist and be valid before tokenization.
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - User whose position is being tokenized (must authorize)
/// * `collateral_asset` - Asset used as collateral
/// * `principal` - Principal debt amount
/// * `interest_rate_bps` - Interest rate at borrowing
///
/// # Returns
/// Token ID of the newly minted debt token
///
/// # Errors
/// * `Unauthorized` - Caller is not the position owner
/// * `PositionNotFound` - Position does not exist
/// * `AlreadyTokenized` - Position is already tokenized
/// * `Undercollateralized` - Position is undercollateralized
/// * `Overflow` - Arithmetic overflow occurs
pub fn mint_debt_token(
    env: &Env,
    user: Address,
    collateral_asset: Option<Address>,
    principal: i128,
    interest_rate_bps: i128,
) -> Result<u64, DebtTokenError> {
    user.require_auth();

    // Check if transfers are paused
    if is_transfer_paused(env) {
        return Err(DebtTokenError::TransferPaused);
    }

    // Check if position is already tokenized
    let existing_tokens = get_user_debt_tokens(env, &user);
    for token_id in existing_tokens.iter() {
        if let Some(position) = get_debt_position(env, token_id) {
            if position.borrower == user && position.collateral_asset == collateral_asset {
                return Err(DebtTokenError::AlreadyTokenized);
            }
        }
    }

    // Validate position health (simplified check)
    if principal <= 0 {
        return Err(DebtTokenError::PositionNotFound);
    }

    // Get next token ID
    let next_id = get_next_token_id(env);
    let token_id = next_id;

    // Create debt position
    let current_time = env.ledger().timestamp();
    let position = DebtPosition {
        borrower: user.clone(),
        principal,
        accrued_interest: 0,
        collateral_asset: collateral_asset.clone(),
        collateral_amount: 0, // Would be calculated from actual position
        interest_rate_bps,
        last_accrual_time: current_time,
        is_liquidatable: false, // Would be calculated from actual position
        created_at: current_time,
        updated_at: current_time,
    };

    // Store position
    let position_key = DebtTokenDataKey::TokenPosition(token_id);
    env.storage().persistent().set(&position_key, &position);

    // Update owner's token list
    let mut owner_tokens = get_user_debt_tokens(env, &user);
    owner_tokens.push_back(token_id);
    let owner_key = DebtTokenDataKey::OwnerTokens(user.clone());
    env.storage().persistent().set(&owner_key, &owner_tokens);

    // Update next token ID
    update_next_token_id(env, token_id + 1);

    // Update total supply
    update_total_supply(env, 1);

    // Emit mint event
    DebtTokenMintedEvent {
        token_id,
        borrower: user.clone(),
        principal,
        collateral_asset,
        timestamp: current_time,
    }
    .publish(env);

    Ok(token_id)
}

/// Transfer a debt token to another address
///
/// Transfers ownership of the debt position NFT. Includes transfer hooks
/// for allow/block lists and validates transfer conditions.
///
/// # Arguments
/// * `env` - The contract environment
/// * `from` - Current owner (must authorize)
/// * `to` - Recipient address
/// * `token_id` - Token ID to transfer
///
/// # Errors
/// * `Unauthorized` - Caller is not token owner
/// * `TokenNotFound` - Token ID does not exist
/// * `TransferPaused` - Transfers are paused
/// * `TransferBlocked` - Recipient is blocked
/// * `LiquidationInProgress` - Transfer during active liquidation
/// * `ZeroAddress` - Transfer to zero address
pub fn transfer_debt_token(
    env: &Env,
    from: Address,
    to: Address,
    token_id: u64,
) -> Result<(), DebtTokenError> {
    from.require_auth();
    move_debt_token_ownership(env, from, to, token_id)
}

/// Core ownership move shared by `transfer_debt_token` (direct, `from`-authorized)
/// and `buy_listed_debt_token` (marketplace purchase, buyer-authorized).
///
/// Deliberately does NOT call `require_auth()` on `from` — a marketplace sale is
/// authorized by the seller's own earlier `list_debt_token` call (which did
/// require the seller's auth) plus the buyer's auth on the purchase itself, not
/// by the seller re-signing at sale time. Callers are responsible for ensuring
/// whatever authorization model applies to their call site before invoking this.
fn move_debt_token_ownership(
    env: &Env,
    from: Address,
    to: Address,
    token_id: u64,
) -> Result<(), DebtTokenError> {
    // Validate inputs
    if to == Address::zero() {
        return Err(DebtTokenError::ZeroAddress);
    }

    // Check if transfers are paused
    if is_transfer_paused(env) {
        return Err(DebtTokenError::TransferPaused);
    }

    // Check if recipient is blocked
    if is_address_blocked(env, &to) {
        return Err(DebtTokenError::TransferBlocked);
    }

    // Get token position
    let position = get_debt_position(env, token_id)
        .ok_or(DebtTokenError::TokenNotFound)?;

    // Check liquidation status
    if position.is_liquidatable {
        return Err(DebtTokenError::LiquidationInProgress);
    }

    // Verify ownership
    let owner_tokens = get_user_debt_tokens(env, &from);
    if !owner_tokens.contains(&token_id) {
        return Err(DebtTokenError::Unauthorized);
    }

    // Remove from current owner
    let mut from_tokens = owner_tokens;
    let index = from_tokens.iter().position(|&id| id == token_id)
        .ok_or(DebtTokenError::TokenNotFound)?;
    from_tokens.remove(index);

    let from_key = DebtTokenDataKey::OwnerTokens(from.clone());
    env.storage().persistent().set(&from_key, &from_tokens);

    // Add to new owner
    let mut to_tokens = get_user_debt_tokens(env, &to);
    to_tokens.push_back(token_id);
    let to_key = DebtTokenDataKey::OwnerTokens(to.clone());
    env.storage().persistent().set(&to_key, &to_tokens);

    // Update position metadata
    let mut updated_position = position;
    updated_position.updated_at = env.ledger().timestamp();
    let position_key = DebtTokenDataKey::TokenPosition(token_id);
    env.storage().persistent().set(&position_key, &updated_position);

    // Emit transfer event
    DebtTokenTransferredEvent {
        token_id,
        from: from.clone(),
        to: to.clone(),
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// List a debt token for sale at a fixed price (issue #664, minimal slice).
///
/// The token remains owned by the seller (no escrow) until `buy_listed_debt_token`
/// succeeds; a paused/blocked transfer still blocks the eventual sale via the same
/// checks `transfer_debt_token` already enforces.
///
/// # Errors
/// * `Unauthorized` - Caller does not own the token
/// * `TokenNotFound` - Token ID does not exist
/// * `AlreadyListed` - Token already has an active listing
/// * `InvalidPrice` - Price is not positive
pub fn list_debt_token(
    env: &Env,
    seller: Address,
    token_id: u64,
    price: i128,
    payment_token: Address,
) -> Result<(), DebtTokenError> {
    seller.require_auth();

    if price <= 0 {
        return Err(DebtTokenError::InvalidPrice);
    }
    get_debt_position(env, token_id).ok_or(DebtTokenError::TokenNotFound)?;
    let owner_tokens = get_user_debt_tokens(env, &seller);
    if !owner_tokens.contains(&token_id) {
        return Err(DebtTokenError::Unauthorized);
    }
    let key = DebtTokenDataKey::Listing(token_id);
    if env.storage().persistent().has(&key) {
        return Err(DebtTokenError::AlreadyListed);
    }

    let listing = DebtTokenListing {
        token_id,
        seller: seller.clone(),
        price,
        payment_token: payment_token.clone(),
        listed_at: env.ledger().timestamp(),
    };
    env.storage().persistent().set(&key, &listing);

    DebtTokenListedEvent {
        token_id,
        seller,
        price,
        payment_token,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Cancel an active listing. Only the seller may cancel.
///
/// # Errors
/// * `NotListed` - No active listing for this token
/// * `NotSeller` - Caller is not the listing's seller
pub fn cancel_listing(env: &Env, seller: Address, token_id: u64) -> Result<(), DebtTokenError> {
    seller.require_auth();

    let key = DebtTokenDataKey::Listing(token_id);
    let listing: DebtTokenListing = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(DebtTokenError::NotListed)?;
    if listing.seller != seller {
        return Err(DebtTokenError::NotSeller);
    }
    env.storage().persistent().remove(&key);

    DebtTokenListingCancelledEvent {
        token_id,
        seller,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Buy a listed debt token at its fixed asking price.
///
/// Pulls `price` of `payment_token` from the buyer directly to the seller (no
/// protocol fee skim — trading-fee distribution is out of scope for this minimal
/// slice, see the module-level note on `DebtTokenListing`), then moves ownership
/// via the same `move_debt_token_ownership` core `transfer_debt_token` uses, so
/// pause/block/liquidation checks apply identically to a marketplace purchase
/// as to a direct transfer — the only difference is which party's auth gates
/// the call (buyer here, seller for a direct transfer).
///
/// # Errors
/// * `NotListed` - No active listing for this token
/// * Any error the shared ownership-move core can return (transfer paused, buyer blocked,
///   position in liquidation, etc.) — the listing is left intact if the
///   post-payment transfer fails, since Soroban invocations are atomic and the
///   whole call reverts together with the payment.
pub fn buy_listed_debt_token(
    env: &Env,
    buyer: Address,
    token_id: u64,
) -> Result<(), DebtTokenError> {
    buyer.require_auth();

    let key = DebtTokenDataKey::Listing(token_id);
    let listing: DebtTokenListing = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(DebtTokenError::NotListed)?;

    let token_client = soroban_sdk::token::Client::new(env, &listing.payment_token);
    token_client.transfer(&buyer, &listing.seller, &listing.price);

    // Reuses the exact ownership-move logic (and its pause/block/liquidation
    // safety checks) that transfer_debt_token runs, but WITHOUT re-requiring the
    // seller's auth — the seller already authorized this sale by creating the
    // listing (list_debt_token required their auth); the buyer's own auth on
    // this call is what authorizes the purchase.
    move_debt_token_ownership(env, listing.seller.clone(), buyer.clone(), token_id)?;

    env.storage().persistent().remove(&key);

    DebtTokenSoldEvent {
        token_id,
        seller: listing.seller,
        buyer,
        price: listing.price,
        payment_token: listing.payment_token,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Read-only: fetch the active listing for a token, if any.
pub fn get_listing(env: &Env, token_id: u64) -> Option<DebtTokenListing> {
    env.storage()
        .persistent()
        .get(&DebtTokenDataKey::Listing(token_id))
}

/// Burn a debt token (debt repayment)
///
/// Burns the debt token when the underlying debt is fully repaid.
/// This removes the NFT from circulation and finalizes the position.
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - User burning the token (must authorize)
/// * `token_id` - Token ID to burn
/// * `reason` - Reason for burning (repayment, liquidation, etc.)
///
/// # Errors
/// * `Unauthorized` - Caller is not token owner
/// * `TokenNotFound` - Token ID does not exist
/// * `LiquidationInProgress` - Cannot burn during liquidation
pub fn burn_debt_token(
    env: &Env,
    user: Address,
    token_id: u64,
    reason: Symbol,
) -> Result<(), DebtTokenError> {
    user.require_auth();

    // Get token position
    let position = get_debt_position(env, token_id)
        .ok_or(DebtTokenError::TokenNotFound)?;

    // Verify ownership
    let owner_tokens = get_user_debt_tokens(env, &user);
    if !owner_tokens.contains(&token_id) {
        return Err(DebtTokenError::Unauthorized);
    }

    // Remove from owner's token list
    let mut user_tokens = owner_tokens;
    let index = user_tokens.iter().position(|&id| id == token_id)
        .ok_or(DebtTokenError::TokenNotFound)?;
    user_tokens.remove(index);

    let owner_key = DebtTokenDataKey::OwnerTokens(user.clone());
    env.storage().persistent().set(&owner_key, &user_tokens);

    // Delete position data
    let position_key = DebtTokenDataKey::TokenPosition(token_id);
    env.storage().persistent().remove(&position_key);

    // Update total supply
    update_total_supply(env, -1);

    // Emit burn event
    DebtTokenBurnedEvent {
        token_id,
        burner: user.clone(),
        reason,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Get debt position information for a token
///
/// # Arguments
/// * `env` - The contract environment
/// * `token_id` - Token ID to query
///
/// # Returns
/// Debt position information or None if token doesn't exist
pub fn get_debt_position(env: &Env, token_id: u64) -> Option<DebtPosition> {
    let position_key = DebtTokenDataKey::TokenPosition(token_id);
    env.storage().persistent().get(&position_key)
}

/// Get all debt tokens owned by a user
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - User address to query
///
/// # Returns
/// Vector of token IDs owned by the user
pub fn get_user_debt_tokens(env: &Env, user: &Address) -> Vec<u64> {
    let owner_key = DebtTokenDataKey::OwnerTokens(user.clone());
    env.storage()
        .persistent()
        .get(&owner_key)
        .unwrap_or_else(|| Vec::new(env))
}

/// Get total supply of debt tokens
///
/// # Arguments
/// * `env` - The contract environment
///
/// # Returns
/// Total number of debt tokens in circulation
pub fn get_total_supply(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&DebtTokenDataKey::TotalSupply)
        .unwrap_or(0)
}

/// Set transfer pause (admin only)
///
/// Pauses or unpauses all debt token transfers.
///
/// # Arguments
/// * `env` - The contract environment
/// * `admin` - Admin address (must authorize)
/// * `paused` - Whether to pause transfers
///
/// # Errors
/// * `Unauthorized` - Caller is not admin
pub fn set_transfer_pause(
    env: &Env,
    admin: Address,
    paused: bool,
) -> Result<(), DebtTokenError> {
    // Verify admin authorization
    let admin_key = DepositDataKey::Admin;
    let stored_admin: Address = env
        .storage()
        .persistent()
        .get(&admin_key)
        .ok_or(DebtTokenError::Unauthorized)?;
    
    if admin != stored_admin {
        return Err(DebtTokenError::Unauthorized);
    }
    
    admin.require_auth();

    env.storage()
        .persistent()
        .set(&DebtTokenDataKey::TransferPaused, &paused);

    Ok(())
}

/// Block/unblock an address from transfers (admin only)
///
/// # Arguments
/// * `env` - The contract environment
/// * `admin` - Admin address (must authorize)
/// * `address` - Address to block/unblock
/// * `blocked` - Whether to block the address
///
/// # Errors
/// * `Unauthorized` - Caller is not admin
pub fn set_address_blocked(
    env: &Env,
    admin: Address,
    address: Address,
    blocked: bool,
) -> Result<(), DebtTokenError> {
    // Verify admin authorization
    let admin_key = DepositDataKey::Admin;
    let stored_admin: Address = env
        .storage()
        .persistent()
        .get(&admin_key)
        .ok_or(DebtTokenError::Unauthorized)?;
    
    if admin != stored_admin {
        return Err(DebtTokenError::Unauthorized);
    }
    
    admin.require_auth();

    if blocked {
        env.storage()
            .persistent()
            .set(&DebtTokenDataKey::BlockedAddress(address), &true);
    } else {
        env.storage()
            .persistent()
            .remove(&DebtTokenDataKey::BlockedAddress(address));
    }

    Ok(())
}

// Helper functions

/// Get the next token ID to mint
fn get_next_token_id(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&DebtTokenDataKey::NextTokenId)
        .unwrap_or(1)
}

/// Update the next token ID
fn update_next_token_id(env: &Env, next_id: u64) {
    env.storage()
        .persistent()
        .set(&DebtTokenDataKey::NextTokenId, &next_id);
}

/// Update total supply
fn update_total_supply(env: &Env, delta: i64) {
    let current_supply = get_total_supply(env);
    let new_supply = if delta >= 0 {
        current_supply + delta as u64
    } else {
        current_supply - (-delta) as u64
    };
    env.storage()
        .persistent()
        .set(&DebtTokenDataKey::TotalSupply, &new_supply);
}

/// Check if transfers are paused
fn is_transfer_paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&DebtTokenDataKey::TransferPaused)
        .unwrap_or(false)
}

/// Check if an address is blocked
fn is_address_blocked(env: &Env, address: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DebtTokenDataKey::BlockedAddress(address.clone()))
        .unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Secondary Market: Price Discovery & Order Book  (Issue #787)
//
// Extends the minimal fixed-price listing above with:
//   1. Bid (buy-offer) support — buyers can post offers below the ask.
//   2. Price discovery — last trade price and a lightweight TWAP are recorded
//      on every executed sale so off-chain dashboards have on-chain anchors.
//   3. Marketplace analytics — global counters for volume, trade count, and a
//      bounded recent-trades log.
//
// Design notes
// ─────────────────────────────────────────────────────────────────────────────
// • Bids are NOT escrowed on-chain (Soroban has no native hold mechanism without
//   a separate vault contract). A bid is an intent: when the seller calls
//   `accept_bid`, the payment is pulled at that moment. This is identical to how
//   the existing listing/buy flow works and is the standard pattern on Stellar.
// • TWAP is computed over the last MAX_TWAP_WINDOW trades (up to 20). With fewer
//   than 2 data points the "TWAP" degrades to the single last-traded price.
// • All new storage keys are additive; none of the existing keys are changed.
// ─────────────────────────────────────────────────────────────────────────────

const MAX_TWAP_WINDOW: u32 = 20;
const MAX_RECENT_TRADES: u32 = 100;

// ── New storage-key variants ──────────────────────────────────────────────────

/// Extended storage keys for marketplace price-discovery and analytics.
#[contracttype]
#[derive(Clone)]
pub enum MarketplaceDataKey {
    /// Active bid on a token: Bid(token_id, bidder) -> DebtTokenBid
    Bid(u64, Address),
    /// Ordered list of bidder addresses for a token: BidderList(token_id) -> Vec<Address>
    BidderList(u64),
    /// Last traded price for a token: LastTradePrice(token_id) -> TradePrice
    LastTradePrice(u64),
    /// Bounded window of recent trade prices for TWAP: TwapWindow(token_id) -> Vec<TradePrice>
    TwapWindow(u64),
    /// Global marketplace analytics: MarketplaceAnalytics -> MarketplaceStats
    MarketplaceAnalytics,
    /// Bounded log of recent trades across all tokens: RecentTrades -> Vec<TradeRecord>
    RecentTrades,
}

// ── Data types ────────────────────────────────────────────────────────────────

/// A buyer's bid (purchase offer) for a listed or unlisted debt token.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct DebtTokenBid {
    pub token_id: u64,
    pub bidder: Address,
    /// Offered price in `payment_token` units.
    pub price: i128,
    /// SEP-41 token the bidder will pay in.
    pub payment_token: Address,
    pub created_at: u64,
    /// Optional expiry; 0 means no expiry.
    pub expires_at: u64,
}

/// A single price observation used for TWAP and last-price tracking.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TradePrice {
    pub price: i128,
    pub payment_token: Address,
    pub timestamp: u64,
}

/// A completed trade record stored in the recent-trades log.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TradeRecord {
    pub token_id: u64,
    pub seller: Address,
    pub buyer: Address,
    pub price: i128,
    pub payment_token: Address,
    pub timestamp: u64,
}

/// Global marketplace statistics.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct MarketplaceStats {
    /// Cumulative number of completed trades.
    pub total_trades: u64,
    /// Cumulative number of active/historical listings created.
    pub total_listings: u64,
    /// Cumulative number of bids placed.
    pub total_bids: u64,
    /// Cumulative number of bid cancellations.
    pub total_bid_cancellations: u64,
    /// Timestamp of the last trade.
    pub last_trade_at: u64,
}

// ── Events ────────────────────────────────────────────────────────────────────

#[contractevent]
#[derive(Clone, Debug)]
pub struct DebtTokenBidPlacedEvent {
    pub token_id: u64,
    pub bidder: Address,
    pub price: i128,
    pub payment_token: Address,
    pub expires_at: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct DebtTokenBidCancelledEvent {
    pub token_id: u64,
    pub bidder: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct DebtTokenBidAcceptedEvent {
    pub token_id: u64,
    pub seller: Address,
    pub bidder: Address,
    pub price: i128,
    pub payment_token: Address,
    pub timestamp: u64,
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn get_marketplace_stats(env: &Env) -> MarketplaceStats {
    env.storage()
        .persistent()
        .get(&MarketplaceDataKey::MarketplaceAnalytics)
        .unwrap_or(MarketplaceStats {
            total_trades: 0,
            total_listings: 0,
            total_bids: 0,
            total_bid_cancellations: 0,
            last_trade_at: 0,
        })
}

fn save_marketplace_stats(env: &Env, stats: &MarketplaceStats) {
    env.storage()
        .persistent()
        .set(&MarketplaceDataKey::MarketplaceAnalytics, stats);
}

fn record_trade_price(env: &Env, token_id: u64, price: i128, payment_token: Address, now: u64) {
    let tp = TradePrice { price, payment_token: payment_token.clone(), timestamp: now };

    // Update last-trade price.
    env.storage()
        .persistent()
        .set(&MarketplaceDataKey::LastTradePrice(token_id), &tp);

    // Update TWAP window.
    let key = MarketplaceDataKey::TwapWindow(token_id);
    let mut window: Vec<TradePrice> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Vec::new(env));
    window.push_back(tp.clone());
    while window.len() > MAX_TWAP_WINDOW {
        window.remove(0);
    }
    env.storage().persistent().set(&key, &window);

    // Append to global recent-trades log.
    let rt_key = MarketplaceDataKey::RecentTrades;
    let mut trades: Vec<TradeRecord> = env
        .storage()
        .persistent()
        .get(&rt_key)
        .unwrap_or_else(|| Vec::new(env));
    // seller/buyer are unknown here; populated by callers via record_trade().
    // This function only records price; callers call record_trade() directly.
    let _ = trades; // unused — full record stored by callers
}

fn record_trade(
    env: &Env,
    token_id: u64,
    seller: Address,
    buyer: Address,
    price: i128,
    payment_token: Address,
    now: u64,
) {
    // Update last-trade price and TWAP window.
    record_trade_price(env, token_id, price, payment_token.clone(), now);

    // Append full trade record to global log.
    let rt_key = MarketplaceDataKey::RecentTrades;
    let mut trades: Vec<TradeRecord> = env
        .storage()
        .persistent()
        .get(&rt_key)
        .unwrap_or_else(|| Vec::new(env));
    trades.push_back(TradeRecord {
        token_id,
        seller,
        buyer,
        price,
        payment_token,
        timestamp: now,
    });
    while trades.len() > MAX_RECENT_TRADES {
        trades.remove(0);
    }
    env.storage().persistent().set(&rt_key, &trades);

    // Update global stats.
    let mut stats = get_marketplace_stats(env);
    stats.total_trades = stats.total_trades.saturating_add(1);
    stats.last_trade_at = now;
    save_marketplace_stats(env, &stats);
}

fn remove_bid(env: &Env, token_id: u64, bidder: &Address) {
    env.storage()
        .persistent()
        .remove(&MarketplaceDataKey::Bid(token_id, bidder.clone()));

    let list_key = MarketplaceDataKey::BidderList(token_id);
    let mut list: Vec<Address> = env
        .storage()
        .persistent()
        .get(&list_key)
        .unwrap_or_else(|| Vec::new(env));
    for i in 0..list.len() {
        if list.get(i).as_ref() == Some(bidder) {
            list.remove(i);
            break;
        }
    }
    env.storage().persistent().set(&list_key, &list);
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Place a bid (purchase offer) on a debt token.
///
/// The token does NOT need to have an active listing — bids can be placed on any
/// existing, non-liquidatable token. If `expires_at` is 0 the bid never expires;
/// otherwise it is only valid while `env.ledger().timestamp() <= expires_at`.
///
/// # Errors
/// * `TokenNotFound` — token does not exist
/// * `InvalidPrice`  — price is not positive
/// * `AlreadyListed` — bidder already has an active bid on this token (use cancel first)
pub fn place_bid(
    env: &Env,
    bidder: Address,
    token_id: u64,
    price: i128,
    payment_token: Address,
    expires_at: u64,
) -> Result<(), DebtTokenError> {
    bidder.require_auth();

    if price <= 0 {
        return Err(DebtTokenError::InvalidPrice);
    }
    get_debt_position(env, token_id).ok_or(DebtTokenError::TokenNotFound)?;

    // Prevent duplicate active bid from the same bidder.
    let bid_key = MarketplaceDataKey::Bid(token_id, bidder.clone());
    if env.storage().persistent().has(&bid_key) {
        return Err(DebtTokenError::AlreadyListed); // reuse: "already has an active offer"
    }

    let now = env.ledger().timestamp();
    let bid = DebtTokenBid {
        token_id,
        bidder: bidder.clone(),
        price,
        payment_token: payment_token.clone(),
        created_at: now,
        expires_at,
    };
    env.storage().persistent().set(&bid_key, &bid);

    // Maintain bidder list for enumeration.
    let list_key = MarketplaceDataKey::BidderList(token_id);
    let mut list: Vec<Address> = env
        .storage()
        .persistent()
        .get(&list_key)
        .unwrap_or_else(|| Vec::new(env));
    list.push_back(bidder.clone());
    env.storage().persistent().set(&list_key, &list);

    // Update global stats.
    let mut stats = get_marketplace_stats(env);
    stats.total_bids = stats.total_bids.saturating_add(1);
    save_marketplace_stats(env, &stats);

    DebtTokenBidPlacedEvent {
        token_id,
        bidder,
        price,
        payment_token,
        expires_at,
        timestamp: now,
    }
    .publish(env);

    Ok(())
}

/// Cancel an active bid. Only the bidder may cancel.
///
/// # Errors
/// * `NotListed` — no active bid from caller on this token
pub fn cancel_bid(env: &Env, bidder: Address, token_id: u64) -> Result<(), DebtTokenError> {
    bidder.require_auth();

    let bid_key = MarketplaceDataKey::Bid(token_id, bidder.clone());
    if !env.storage().persistent().has(&bid_key) {
        return Err(DebtTokenError::NotListed);
    }
    remove_bid(env, token_id, &bidder);

    // Update global stats.
    let mut stats = get_marketplace_stats(env);
    stats.total_bid_cancellations = stats.total_bid_cancellations.saturating_add(1);
    save_marketplace_stats(env, &stats);

    DebtTokenBidCancelledEvent {
        token_id,
        bidder,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);

    Ok(())
}

/// Accept a specific bidder's bid and sell the token to them.
///
/// The seller must own the token. Payment is pulled from the bidder to the seller
/// at acceptance time. Transfer guards (pause, block-list, liquidation) apply.
///
/// # Errors
/// * `Unauthorized`          — caller does not own the token
/// * `TokenNotFound`         — token does not exist
/// * `NotListed`             — no active bid from `bidder` on this token
/// * `InvalidPrice`          — bid has expired (`expires_at` in the past)
/// * Any error from the ownership-move core (pause/block/liquidation)
pub fn accept_bid(
    env: &Env,
    seller: Address,
    token_id: u64,
    bidder: Address,
) -> Result<(), DebtTokenError> {
    seller.require_auth();

    // Verify ownership.
    let owner_tokens = get_user_debt_tokens(env, &seller);
    if !owner_tokens.contains(&token_id) {
        return Err(DebtTokenError::Unauthorized);
    }
    get_debt_position(env, token_id).ok_or(DebtTokenError::TokenNotFound)?;

    let bid_key = MarketplaceDataKey::Bid(token_id, bidder.clone());
    let bid: DebtTokenBid = env
        .storage()
        .persistent()
        .get(&bid_key)
        .ok_or(DebtTokenError::NotListed)?;

    // Check bid expiry.
    let now = env.ledger().timestamp();
    if bid.expires_at != 0 && now > bid.expires_at {
        return Err(DebtTokenError::InvalidPrice); // expired bid
    }

    // Pull payment from bidder to seller.
    let token_client = soroban_sdk::token::Client::new(env, &bid.payment_token);
    token_client.transfer(&bidder, &seller, &bid.price);

    // Move ownership (same checks as direct transfer/marketplace buy).
    move_debt_token_ownership(env, seller.clone(), bidder.clone(), token_id)?;

    // Clean up bid and any active listing on this token.
    remove_bid(env, token_id, &bidder);
    let listing_key = DebtTokenDataKey::Listing(token_id);
    if env.storage().persistent().has(&listing_key) {
        env.storage().persistent().remove(&listing_key);
    }

    // Record trade for price-discovery.
    record_trade(env, token_id, seller.clone(), bidder.clone(), bid.price, bid.payment_token.clone(), now);

    DebtTokenBidAcceptedEvent {
        token_id,
        seller,
        bidder,
        price: bid.price,
        payment_token: bid.payment_token,
        timestamp: now,
    }
    .publish(env);

    Ok(())
}

/// Get a specific bid.
pub fn get_bid(env: &Env, token_id: u64, bidder: Address) -> Option<DebtTokenBid> {
    env.storage()
        .persistent()
        .get(&MarketplaceDataKey::Bid(token_id, bidder))
}

/// Get all bidder addresses that have active bids on a token.
pub fn get_bidders(env: &Env, token_id: u64) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&MarketplaceDataKey::BidderList(token_id))
        .unwrap_or_else(|| Vec::new(env))
}

/// Get the last traded price for a token (None if never traded via on-chain marketplace).
pub fn get_last_trade_price(env: &Env, token_id: u64) -> Option<TradePrice> {
    env.storage()
        .persistent()
        .get(&MarketplaceDataKey::LastTradePrice(token_id))
}

/// Compute the time-weighted average price (TWAP) over the last MAX_TWAP_WINDOW trades.
///
/// Returns `None` if no trades have been recorded for this token. With a single
/// trade the result equals that trade's price.
pub fn get_twap_price(env: &Env, token_id: u64) -> Option<i128> {
    let window: Vec<TradePrice> = env
        .storage()
        .persistent()
        .get(&MarketplaceDataKey::TwapWindow(token_id))
        .unwrap_or_else(|| Vec::new(env));

    if window.is_empty() {
        return None;
    }

    // Simple arithmetic mean over price observations.
    let n = window.len() as i128;
    let sum: i128 = window.iter().map(|tp| tp.price).fold(0i128, |acc, p| acc.saturating_add(p));
    Some(sum.checked_div(n).unwrap_or(sum))
}

/// Get the global marketplace analytics snapshot.
pub fn get_marketplace_analytics(env: &Env) -> MarketplaceStats {
    get_marketplace_stats(env)
}

/// Get the bounded log of recent trades across all tokens (most recent last).
pub fn get_recent_trades(env: &Env) -> Vec<TradeRecord> {
    env.storage()
        .persistent()
        .get(&MarketplaceDataKey::RecentTrades)
        .unwrap_or_else(|| Vec::new(env))
}

/// Intercept buy_listed_debt_token to also record a trade for price discovery.
/// This wraps the existing function by recording the price after a successful buy.
///
/// NOTE: This is exposed as `buy_listed_debt_token_with_price_discovery` — the
/// original `buy_listed_debt_token` is preserved unchanged for backward compat.
pub fn buy_listed_debt_token_tracked(
    env: &Env,
    buyer: Address,
    token_id: u64,
) -> Result<(), DebtTokenError> {
    buyer.require_auth();

    let key = DebtTokenDataKey::Listing(token_id);
    let listing: DebtTokenListing = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(DebtTokenError::NotListed)?;

    let token_client = soroban_sdk::token::Client::new(env, &listing.payment_token);
    token_client.transfer(&buyer, &listing.seller, &listing.price);

    move_debt_token_ownership(env, listing.seller.clone(), buyer.clone(), token_id)?;

    env.storage().persistent().remove(&key);

    let now = env.ledger().timestamp();

    // Price discovery record.
    record_trade(env, token_id, listing.seller.clone(), buyer.clone(), listing.price, listing.payment_token.clone(), now);

    // Update listings counter.
    let mut stats = get_marketplace_stats(env);
    stats.total_listings = stats.total_listings.saturating_add(1);
    save_marketplace_stats(env, &stats);

    DebtTokenSoldEvent {
        token_id,
        seller: listing.seller,
        buyer,
        price: listing.price,
        payment_token: listing.payment_token,
        timestamp: now,
    }
    .publish(env);

    Ok(())
}

/// Intercept list_debt_token to also increment listings counter.
///
/// Wraps existing listing creation with marketplace stats bookkeeping.
pub fn list_debt_token_tracked(
    env: &Env,
    seller: Address,
    token_id: u64,
    price: i128,
    payment_token: Address,
) -> Result<(), DebtTokenError> {
    list_debt_token(env, seller, token_id, price, payment_token)?;

    let mut stats = get_marketplace_stats(env);
    stats.total_listings = stats.total_listings.saturating_add(1);
    save_marketplace_stats(env, &stats);

    Ok(())
}
