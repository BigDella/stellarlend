//! Debt Token Tests
//!
//! Comprehensive test suite for debt tokenization functionality.
//! Tests minting, transferring, burning, and access controls.

use soroban_sdk::{Address, Env, Symbol};

use crate::debt_token::{
    burn_debt_token, get_debt_position, get_debt_token_total_supply,
    get_user_debt_tokens, mint_debt_token, set_address_blocked,
    set_transfer_pause, transfer_debt_token, DebtPosition, DebtTokenError,
};
use crate::deposit::DepositDataKey;
use crate::test_utils::*;

#[test]
fn test_mint_debt_token_success() {
    let env = Env::default();
    let user = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    // Setup admin
    setup_admin(&env);

    // Test successful minting
    let principal = 1000000;
    let interest_rate = 500; // 5% interest rate
    let result = mint_debt_token(&env, user.clone(), collateral_asset, principal, interest_rate);
    
    assert!(result.is_ok());
    let token_id = result.unwrap();
    assert!(token_id > 0);
    
    // Verify position data
    let position = get_debt_position(&env, token_id).unwrap();
    assert_eq!(position.borrower, user);
    assert_eq!(position.principal, principal);
    assert_eq!(position.interest_rate_bps, interest_rate);
    assert_eq!(position.collateral_asset, collateral_asset);
    assert!(!position.is_liquidatable);
    
    // Verify user owns the token
    let user_tokens = get_user_debt_tokens(&env, &user);
    assert!(user_tokens.contains(&token_id));
    
    // Verify total supply
    let total_supply = get_debt_token_total_supply(&env);
    assert_eq!(total_supply, 1);
}

#[test]
fn test_mint_debt_token_unauthorized() {
    let env = Env::default();
    let user = Address::generate(&env);
    let other_user = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    // Setup admin
    setup_admin(&env);

    // Test unauthorized minting (different user)
    let principal = 1000000;
    let result = mint_debt_token(&env, other_user, collateral_asset, principal, 500);
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DebtTokenError::Unauthorized);
}

#[test]
fn test_mint_debt_token_already_tokenized() {
    let env = Env::default();
    let user = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    // Setup admin
    setup_admin(&env);

    // Mint first token
    let token_id = mint_debt_token(&env, user.clone(), collateral_asset.clone(), 1000000, 500).unwrap();
    
    // Try to mint second token for same position
    let result = mint_debt_token(&env, user.clone(), collateral_asset, 2000000, 500);
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DebtTokenError::AlreadyTokenized);
    
    // Verify only one token exists
    let total_supply = get_debt_token_total_supply(&env);
    assert_eq!(total_supply, 1);
}

#[test]
fn test_transfer_debt_token_success() {
    let env = Env::default();
    let user = Address::generate(&env);
    let recipient = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    // Setup admin and mint token
    setup_admin(&env);
    let token_id = mint_debt_token(&env, user.clone(), collateral_asset, 1000000, 500).unwrap();
    
    // Test successful transfer
    let result = transfer_debt_token(&env, user.clone(), recipient.clone(), token_id);
    
    assert!(result.is_ok());
    
    // Verify ownership transfer
    let user_tokens = get_user_debt_tokens(&env, &user);
    assert!(!user_tokens.contains(&token_id));
    
    let recipient_tokens = get_user_debt_tokens(&env, &recipient);
    assert!(recipient_tokens.contains(&token_id));
    
    // Verify position was updated
    let position = get_debt_position(&env, token_id).unwrap();
    assert!(position.updated_at > 0);
}

#[test]
fn test_transfer_debt_token_unauthorized() {
    let env = Env::default();
    let user = Address::generate(&env);
    let other_user = Address::generate(&env);
    let recipient = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    // Setup admin and mint token
    setup_admin(&env);
    let token_id = mint_debt_token(&env, user, collateral_asset, 1000000, 500).unwrap();
    
    // Test unauthorized transfer
    let result = transfer_debt_token(&env, other_user, recipient, token_id);
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DebtTokenError::Unauthorized);
}

#[test]
fn test_transfer_debt_token_to_zero_address() {
    let env = Env::default();
    let user = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    // Setup admin and mint token
    setup_admin(&env);
    let token_id = mint_debt_token(&env, user.clone(), collateral_asset, 1000000, 500).unwrap();
    
    // Test transfer to zero address
    let result = transfer_debt_token(&env, user, Address::zero(), token_id);
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DebtTokenError::ZeroAddress);
}

#[test]
fn test_transfer_debt_token_paused() {
    let env = Env::default();
    let user = Address::generate(&env);
    let recipient = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    // Setup admin and mint token
    setup_admin(&env);
    let token_id = mint_debt_token(&env, user.clone(), collateral_asset, 1000000, 500).unwrap();
    
    // Pause transfers
    set_transfer_pause(&env, user.clone(), true).unwrap();
    
    // Test transfer while paused
    let result = transfer_debt_token(&env, user, recipient, token_id);
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DebtTokenError::TransferPaused);
}

#[test]
fn test_burn_debt_token_success() {
    let env = Env::default();
    let user = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    // Setup admin and mint token
    setup_admin(&env);
    let token_id = mint_debt_token(&env, user.clone(), collateral_asset, 1000000, 500).unwrap();
    
    // Test successful burn
    let result = burn_debt_token(&env, user.clone(), token_id, Symbol::new(&env, "repayment"));
    
    assert!(result.is_ok());
    
    // Verify token is burned
    let position = get_debt_position(&env, token_id);
    assert!(position.is_none());
    
    // Verify user no longer owns token
    let user_tokens = get_user_debt_tokens(&env, &user);
    assert!(!user_tokens.contains(&token_id));
    
    // Verify total supply decreased
    let total_supply = get_debt_token_total_supply(&env);
    assert_eq!(total_supply, 0);
}

#[test]
fn test_burn_debt_token_unauthorized() {
    let env = Env::default();
    let user = Address::generate(&env);
    let other_user = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    // Setup admin and mint token
    setup_admin(&env);
    let token_id = mint_debt_token(&env, user, collateral_asset, 1000000, 500).unwrap();
    
    // Test unauthorized burn
    let result = burn_debt_token(&env, other_user, token_id, Symbol::new(&env, "repayment"));
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DebtTokenError::Unauthorized);
}

#[test]
fn test_set_transfer_pause_success() {
    let env = Env::default();
    let admin = Address::generate(&env);

    // Setup admin
    setup_admin(&env);

    // Test successful pause
    let result = set_transfer_pause(&env, admin.clone(), true);
    
    assert!(result.is_ok());
    
    // Verify pause is active
    assert!(is_transfer_paused(&env));
}

#[test]
fn test_set_transfer_pause_unauthorized() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);

    // Setup admin
    setup_admin(&env);

    // Test unauthorized pause
    let result = set_transfer_pause(&env, unauthorized_user, true);
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DebtTokenError::Unauthorized);
}

#[test]
fn test_set_address_blocked_success() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let blocked_address = Address::generate(&env);

    // Setup admin
    setup_admin(&env);

    // Test successful address blocking
    let result = set_address_blocked(&env, admin.clone(), blocked_address.clone(), true);
    
    assert!(result.is_ok());
    
    // Verify address is blocked
    assert!(is_address_blocked(&env, &blocked_address));
}

#[test]
fn test_set_address_blocked_unauthorized() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let address = Address::generate(&env);

    // Setup admin
    setup_admin(&env);

    // Test unauthorized address blocking
    let result = set_address_blocked(&env, unauthorized_user, address, true);
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), DebtTokenError::Unauthorized);
}

#[test]
fn test_debt_token_position_metadata() {
    let env = Env::default();
    let user = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    // Setup admin and mint token
    setup_admin(&env);
    let token_id = mint_debt_token(&env, user.clone(), collateral_asset.clone(), 1000000, 500).unwrap();
    
    // Test position metadata retrieval
    let position = get_debt_position(&env, token_id).unwrap();
    
    assert_eq!(position.borrower, user);
    assert_eq!(position.principal, 1000000);
    assert_eq!(position.interest_rate_bps, 500);
    assert_eq!(position.collateral_asset, collateral_asset);
    assert_eq!(position.accrued_interest, 0);
    assert_eq!(position.last_accrual_time, 0); // Set to current time
    assert!(!position.is_liquidatable);
    assert!(position.created_at > 0);
    assert!(position.updated_at > 0);
}

#[test]
fn test_get_user_debt_tokens_empty() {
    let env = Env::default();
    let user = Address::generate(&env);

    // Test getting tokens for user with no tokens
    let tokens = get_user_debt_tokens(&env, &user);
    
    assert_eq!(tokens.len(), 0);
}

#[test]
fn test_get_debt_token_total_supply() {
    let env = Env::default();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    // Setup admin and mint multiple tokens
    setup_admin(&env);
    let token1 = mint_debt_token(&env, user1.clone(), collateral_asset.clone(), 1000000, 500).unwrap();
    let token2 = mint_debt_token(&env, user2.clone(), collateral_asset, 2000000, 500).unwrap();
    
    // Test total supply
    let total_supply = get_debt_token_total_supply(&env);
    assert_eq!(total_supply, 2);
    
    // Burn one token
    burn_debt_token(&env, user1, token1, Symbol::new(&env, "repayment")).unwrap();
    
    // Verify total supply decreased
    let new_total_supply = get_debt_token_total_supply(&env);
    assert_eq!(new_total_supply, 1);
}

// Helper functions

fn setup_admin(env: &Env) {
    let admin = Address::generate(&env);
    env.storage()
        .persistent()
        .set(&DepositDataKey::Admin, &admin);
}

fn is_transfer_paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&crate::debt_token::DebtTokenDataKey::TransferPaused)
        .unwrap_or(false)
}

fn is_address_blocked(env: &Env, address: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&crate::debt_token::DebtTokenDataKey::BlockedAddress(address.clone()))
        .unwrap_or(false)
}

// ── Issue #664: minimal fixed-price secondary-market listing ──────────────────

use crate::debt_token::{buy_listed_debt_token, cancel_listing, get_listing, list_debt_token};

fn setup_payment_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract_v2(admin.clone())
        .address()
}

#[test]
fn test_list_and_buy_debt_token() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    setup_admin(&env);
    let token_id = mint_debt_token(&env, seller.clone(), collateral_asset, 1_000_000, 500).unwrap();

    let payment_token = setup_payment_token(&env, &admin);
    let payment_client = soroban_sdk::token::StellarAssetClient::new(&env, &payment_token);
    payment_client.mint(&buyer, &500_000i128);

    list_debt_token(&env, seller.clone(), token_id, 500_000i128, payment_token.clone()).unwrap();
    let listing = get_listing(&env, token_id).unwrap();
    assert_eq!(listing.seller, seller);
    assert_eq!(listing.price, 500_000i128);

    buy_listed_debt_token(&env, buyer.clone(), token_id).unwrap();

    // Ownership moved to the buyer, listing is cleared, and payment settled.
    let buyer_tokens = get_user_debt_tokens(&env, &buyer);
    assert!(buyer_tokens.contains(&token_id));
    let seller_tokens = get_user_debt_tokens(&env, &seller);
    assert!(!seller_tokens.contains(&token_id));
    assert!(get_listing(&env, token_id).is_none());

    let payment_view = soroban_sdk::token::TokenClient::new(&env, &payment_token);
    assert_eq!(payment_view.balance(&buyer), 0);
    assert_eq!(payment_view.balance(&seller), 500_000i128);
}

#[test]
fn test_cannot_list_token_you_do_not_own() {
    let env = Env::default();
    env.mock_all_auths();

    let seller = Address::generate(&env);
    let impostor = Address::generate(&env);
    let admin = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    setup_admin(&env);
    let token_id = mint_debt_token(&env, seller, collateral_asset, 1_000_000, 500).unwrap();
    let payment_token = setup_payment_token(&env, &admin);

    let result = list_debt_token(&env, impostor, token_id, 100i128, payment_token);
    assert_eq!(result.unwrap_err(), DebtTokenError::Unauthorized);
}

#[test]
fn test_cannot_list_at_zero_or_negative_price() {
    let env = Env::default();
    env.mock_all_auths();

    let seller = Address::generate(&env);
    let admin = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    setup_admin(&env);
    let token_id = mint_debt_token(&env, seller.clone(), collateral_asset, 1_000_000, 500).unwrap();
    let payment_token = setup_payment_token(&env, &admin);

    let result = list_debt_token(&env, seller, token_id, 0i128, payment_token);
    assert_eq!(result.unwrap_err(), DebtTokenError::InvalidPrice);
}

#[test]
fn test_cannot_double_list_same_token() {
    let env = Env::default();
    env.mock_all_auths();

    let seller = Address::generate(&env);
    let admin = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    setup_admin(&env);
    let token_id = mint_debt_token(&env, seller.clone(), collateral_asset, 1_000_000, 500).unwrap();
    let payment_token = setup_payment_token(&env, &admin);

    list_debt_token(&env, seller.clone(), token_id, 100i128, payment_token.clone()).unwrap();
    let result = list_debt_token(&env, seller, token_id, 200i128, payment_token);
    assert_eq!(result.unwrap_err(), DebtTokenError::AlreadyListed);
}

#[test]
fn test_seller_can_cancel_listing() {
    let env = Env::default();
    env.mock_all_auths();

    let seller = Address::generate(&env);
    let admin = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    setup_admin(&env);
    let token_id = mint_debt_token(&env, seller.clone(), collateral_asset, 1_000_000, 500).unwrap();
    let payment_token = setup_payment_token(&env, &admin);

    list_debt_token(&env, seller.clone(), token_id, 100i128, payment_token).unwrap();
    cancel_listing(&env, seller.clone(), token_id).unwrap();
    assert!(get_listing(&env, token_id).is_none());
}

#[test]
fn test_non_seller_cannot_cancel_listing() {
    let env = Env::default();
    env.mock_all_auths();

    let seller = Address::generate(&env);
    let impostor = Address::generate(&env);
    let admin = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    setup_admin(&env);
    let token_id = mint_debt_token(&env, seller.clone(), collateral_asset, 1_000_000, 500).unwrap();
    let payment_token = setup_payment_token(&env, &admin);

    list_debt_token(&env, seller, token_id, 100i128, payment_token).unwrap();
    let result = cancel_listing(&env, impostor, token_id);
    assert_eq!(result.unwrap_err(), DebtTokenError::NotSeller);
}

#[test]
fn test_buying_unlisted_token_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    setup_admin(&env);
    let token_id = mint_debt_token(&env, seller, collateral_asset, 1_000_000, 500).unwrap();

    let result = buy_listed_debt_token(&env, buyer, token_id);
    assert_eq!(result.unwrap_err(), DebtTokenError::NotListed);
}

#[test]
fn test_buying_respects_transfer_pause() {
    let env = Env::default();
    env.mock_all_auths();

    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let token_issuer = Address::generate(&env);
    let collateral_asset = Some(Address::generate(&env));

    setup_admin(&env);
    let real_admin: Address = env
        .storage()
        .persistent()
        .get(&DepositDataKey::Admin)
        .unwrap();
    let token_id = mint_debt_token(&env, seller.clone(), collateral_asset, 1_000_000, 500).unwrap();

    let payment_token = setup_payment_token(&env, &token_issuer);
    let payment_client = soroban_sdk::token::StellarAssetClient::new(&env, &payment_token);
    payment_client.mint(&buyer, &500_000i128);

    list_debt_token(&env, seller.clone(), token_id, 500_000i128, payment_token).unwrap();

    // Pausing transfers after listing must still block the eventual sale — the
    // listing mechanism doesn't bypass the same safety switch a direct transfer
    // respects.
    set_transfer_pause(&env, real_admin, true).unwrap();

    let result = buy_listed_debt_token(&env, buyer, token_id);
    assert_eq!(result.unwrap_err(), DebtTokenError::TransferPaused);
}
