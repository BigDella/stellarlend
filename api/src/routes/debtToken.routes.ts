/**
 * Debt Token Routes  — Issue #787
 *
 * Provides REST endpoints for the debt-token NFT module and its secondary
 * trading marketplace. Covers:
 *   Core NFT operations: mint, transfer, burn
 *   Fixed-price marketplace: list, cancel listing, buy
 *   Order-book / bid system: place bid, cancel bid, accept bid
 *   Price discovery: last trade price, TWAP
 *   Analytics: global marketplace stats, recent trades, all listings/bids
 */

import { Router } from 'express';
import * as debtTokenController from '../controllers/debtToken.controller';

const router: Router = Router();

// ─── Core NFT operations ──────────────────────────────────────────────────────

/**
 * @openapi
 * /debt-token/mint:
 *   post:
 *     summary: Mint a new debt-position NFT
 *     tags: [DebtToken]
 */
router.post('/mint', debtTokenController.mintDebtToken);

/**
 * @openapi
 * /debt-token/transfer:
 *   post:
 *     summary: Direct transfer of a debt token
 *     tags: [DebtToken]
 */
router.post('/transfer', debtTokenController.transferDebtToken);

/**
 * @openapi
 * /debt-token/burn:
 *   post:
 *     summary: Burn a debt token (repayment / liquidation)
 *     tags: [DebtToken]
 */
router.post('/burn', debtTokenController.burnDebtToken);

/**
 * @openapi
 * /debt-token/position/{tokenId}:
 *   get:
 *     summary: Get debt position for a token ID
 *     tags: [DebtToken]
 */
router.get('/position/:tokenId', debtTokenController.getDebtPosition);

/**
 * @openapi
 * /debt-token/tokens/{userAddress}:
 *   get:
 *     summary: Get all debt token IDs owned by a user
 *     tags: [DebtToken]
 */
router.get('/tokens/:userAddress', debtTokenController.getUserDebtTokens);

/**
 * @openapi
 * /debt-token/total-supply:
 *   get:
 *     summary: Get total debt token supply
 *     tags: [DebtToken]
 */
router.get('/total-supply', debtTokenController.getDebtTokenTotalSupply);

// ─── Admin controls ───────────────────────────────────────────────────────────

/**
 * @openapi
 * /debt-token/transfer-pause:
 *   post:
 *     summary: Pause or resume all debt token transfers (admin)
 *     tags: [DebtToken]
 */
router.post('/transfer-pause', debtTokenController.setDebtTokenTransferPause);

/**
 * @openapi
 * /debt-token/address-blocked:
 *   post:
 *     summary: Block or unblock an address from transfers (admin)
 *     tags: [DebtToken]
 */
router.post('/address-blocked', debtTokenController.setDebtTokenAddressBlocked);

// ─── Fixed-price marketplace ──────────────────────────────────────────────────

/**
 * @openapi
 * /debt-token/marketplace/list:
 *   post:
 *     summary: List a debt token at a fixed price
 *     tags: [DebtToken, Marketplace]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [sellerAddress, tokenId, price, paymentToken]
 *             properties:
 *               sellerAddress: { type: string }
 *               tokenId: { type: integer }
 *               price: { type: string, description: "Price in stroops / base units" }
 *               paymentToken: { type: string, description: "SEP-41 token contract address" }
 */
router.post('/marketplace/list', debtTokenController.listDebtToken);

/**
 * @openapi
 * /debt-token/marketplace/cancel-listing:
 *   post:
 *     summary: Cancel an active fixed-price listing
 *     tags: [DebtToken, Marketplace]
 */
router.post('/marketplace/cancel-listing', debtTokenController.cancelListing);

/**
 * @openapi
 * /debt-token/marketplace/buy:
 *   post:
 *     summary: Buy a listed debt token at its asking price
 *     tags: [DebtToken, Marketplace]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [buyerAddress, tokenId]
 *             properties:
 *               buyerAddress: { type: string }
 *               tokenId: { type: integer }
 */
router.post('/marketplace/buy', debtTokenController.buyListedDebtToken);

/**
 * @openapi
 * /debt-token/marketplace/listing/{tokenId}:
 *   get:
 *     summary: Get the active listing for a token
 *     tags: [DebtToken, Marketplace]
 */
router.get('/marketplace/listing/:tokenId', debtTokenController.getListing);

// ─── Order-book / bid system ──────────────────────────────────────────────────

/**
 * @openapi
 * /debt-token/marketplace/bid:
 *   post:
 *     summary: Place a bid (purchase offer) on a debt token
 *     tags: [DebtToken, Marketplace]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [bidderAddress, tokenId, price, paymentToken]
 *             properties:
 *               bidderAddress: { type: string }
 *               tokenId: { type: integer }
 *               price: { type: string }
 *               paymentToken: { type: string }
 *               expiresAt: { type: integer, description: "Unix timestamp; 0 = no expiry" }
 */
router.post('/marketplace/bid', debtTokenController.placeBid);

/**
 * @openapi
 * /debt-token/marketplace/cancel-bid:
 *   post:
 *     summary: Cancel an active bid
 *     tags: [DebtToken, Marketplace]
 */
router.post('/marketplace/cancel-bid', debtTokenController.cancelBid);

/**
 * @openapi
 * /debt-token/marketplace/accept-bid:
 *   post:
 *     summary: Accept a bidder's offer and sell the token
 *     tags: [DebtToken, Marketplace]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [sellerAddress, tokenId, bidderAddress]
 *             properties:
 *               sellerAddress: { type: string }
 *               tokenId: { type: integer }
 *               bidderAddress: { type: string }
 */
router.post('/marketplace/accept-bid', debtTokenController.acceptBid);

/**
 * @openapi
 * /debt-token/marketplace/bids/{tokenId}:
 *   get:
 *     summary: Get all active bids for a token
 *     tags: [DebtToken, Marketplace]
 */
router.get('/marketplace/bids/:tokenId', debtTokenController.getBidsForToken);

/**
 * @openapi
 * /debt-token/marketplace/bid/{tokenId}/{bidderAddress}:
 *   get:
 *     summary: Get a specific bid
 *     tags: [DebtToken, Marketplace]
 */
router.get('/marketplace/bid/:tokenId/:bidderAddress', debtTokenController.getBid);

// ─── Price discovery ──────────────────────────────────────────────────────────

/**
 * @openapi
 * /debt-token/marketplace/last-price/{tokenId}:
 *   get:
 *     summary: Get the last traded price for a token
 *     tags: [DebtToken, Marketplace]
 */
router.get('/marketplace/last-price/:tokenId', debtTokenController.getLastTradePrice);

/**
 * @openapi
 * /debt-token/marketplace/twap/{tokenId}:
 *   get:
 *     summary: Get the TWAP (time-weighted average price) for a token
 *     description: Average over the last 20 on-chain trades.
 *     tags: [DebtToken, Marketplace]
 */
router.get('/marketplace/twap/:tokenId', debtTokenController.getTwapPrice);

// ─── Marketplace analytics ────────────────────────────────────────────────────

/**
 * @openapi
 * /debt-token/marketplace/analytics:
 *   get:
 *     summary: Global marketplace analytics (volume, trade count, bid count)
 *     tags: [DebtToken, Marketplace]
 */
router.get('/marketplace/analytics', debtTokenController.getMarketplaceAnalytics);

/**
 * @openapi
 * /debt-token/marketplace/recent-trades:
 *   get:
 *     summary: Bounded log of recent trades across all tokens
 *     tags: [DebtToken, Marketplace]
 */
router.get('/marketplace/recent-trades', debtTokenController.getRecentTrades);

export default router;
