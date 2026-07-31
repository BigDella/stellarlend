/**
 * MEV Protection Routes
 *
 * All routes are prefixed with /api/mev in app.ts.
 */

import { Router } from 'express';
import * as mevController from '../controllers/mev.controller';

const router: Router = Router();

/**
 * @openapi
 * /mev/commit:
 *   post:
 *     summary: Build an unsigned commit transaction for MEV-protected execution
 *     description: >
 *       Creates a commit for a sensitive operation (Borrow, Withdraw, Liquidate).
 *       The client signs and submits the returned XDR. After the mandatory delay
 *       (`revealAfter`) the caller must submit a reveal transaction to execute.
 *     tags:
 *       - MEV Protection
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [userAddress, operation, amount, maxFeeBps]
 *             properties:
 *               userAddress:
 *                 type: string
 *                 description: Stellar public key of the committing user
 *               operation:
 *                 type: string
 *                 enum: [Borrow, Withdraw, Liquidate]
 *               assetAddress:
 *                 type: string
 *               secondaryAssetAddress:
 *                 type: string
 *                 description: Collateral asset (Liquidate only)
 *               borrowerAddress:
 *                 type: string
 *                 description: Required for Liquidate
 *               amount:
 *                 type: string
 *               maxFeeBps:
 *                 type: integer
 *                 description: Maximum MEV fee the caller accepts (0–10000 bps)
 *               hint:
 *                 type: string
 *                 enum: [Default, PrivateMempool, BatchAuction, DelayedReveal]
 *               maxSlippageBps:
 *                 type: integer
 *                 description: Maximum slippage tolerance (0–10000 bps). 0 = protocol default.
 *               deadline:
 *                 type: integer
 *                 description: Unix timestamp after which the commit must not execute. 0 = no deadline.
 *     responses:
 *       200:
 *         description: Unsigned commit XDR ready for signing
 *       400:
 *         description: Validation error
 */
router.post('/commit', mevController.buildCommit);

/**
 * @openapi
 * /mev/reveal:
 *   post:
 *     summary: Build an unsigned reveal transaction to execute a committed operation
 *     tags:
 *       - MEV Protection
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [userAddress, commitId, operation]
 *             properties:
 *               userAddress:
 *                 type: string
 *               commitId:
 *                 type: string
 *               operation:
 *                 type: string
 *                 enum: [Borrow, Withdraw, Liquidate]
 *     responses:
 *       200:
 *         description: Unsigned reveal XDR ready for signing
 */
router.post('/reveal', mevController.buildReveal);

/**
 * @openapi
 * /mev/auction/bid:
 *   post:
 *     summary: Place a bid in the current batch liquidation auction
 *     description: >
 *       Bids are collected during the open window and settled atomically when
 *       the window closes. Slippage is enforced via `minCollateralOut`.
 *     tags:
 *       - MEV Protection
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [bidderAddress, borrowerAddress, debtAmount, minCollateralOut, maxFeeBps]
 *             properties:
 *               bidderAddress:
 *                 type: string
 *               borrowerAddress:
 *                 type: string
 *               debtAmount:
 *                 type: string
 *               minCollateralOut:
 *                 type: string
 *                 description: Minimum collateral the bidder expects (slippage guard)
 *               maxFeeBps:
 *                 type: integer
 *               deadline:
 *                 type: integer
 *     responses:
 *       200:
 *         description: Unsigned bid XDR and current slot ID
 */
router.post('/auction/bid', mevController.buildAuctionBid);

/**
 * @openapi
 * /mev/auction/settle:
 *   post:
 *     summary: Build an unsigned transaction to settle a closed auction slot
 *     tags:
 *       - MEV Protection
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [callerAddress, slotId]
 *             properties:
 *               callerAddress:
 *                 type: string
 *               slotId:
 *                 type: string
 *     responses:
 *       200:
 *         description: Unsigned settle XDR
 */
router.post('/auction/settle', mevController.buildSettleAuction);

/**
 * @openapi
 * /mev/auction/current:
 *   get:
 *     summary: Return the current open auction slot ID
 *     tags:
 *       - MEV Protection
 *     responses:
 *       200:
 *         description: Current slot ID
 */
router.get('/auction/current', mevController.getCurrentAuctionSlot);

/**
 * @openapi
 * /mev/auction/{slotId}:
 *   get:
 *     summary: Return the settled result for a given auction slot
 *     tags:
 *       - MEV Protection
 *     parameters:
 *       - in: path
 *         name: slotId
 *         required: true
 *         schema:
 *           type: string
 *     responses:
 *       200:
 *         description: Auction result
 *       404:
 *         description: Slot not found or not yet settled
 */
router.get('/auction/:slotId', mevController.getAuctionResult);

/**
 * @openapi
 * /mev/dashboard:
 *   get:
 *     summary: MEV extraction monitoring dashboard
 *     description: >
 *       Returns ordering statistics, protection configuration, and current auction
 *       slot. Cached for 15 seconds.
 *     tags:
 *       - MEV Protection
 *     responses:
 *       200:
 *         description: Dashboard snapshot
 */
router.get('/dashboard', mevController.getDashboard);

/**
 * @openapi
 * /mev/gas-analysis:
 *   get:
 *     summary: Gas price bidding analysis for a given operation
 *     description: >
 *       Returns smoothed base fee, current surge fee, recommended bid, and
 *       high-congestion bid for the given operation. Cached for 10 seconds.
 *     tags:
 *       - MEV Protection
 *     parameters:
 *       - in: query
 *         name: operation
 *         schema:
 *           type: string
 *           enum: [Borrow, Withdraw, Liquidate]
 *       - in: query
 *         name: assetAddress
 *         schema:
 *           type: string
 *       - in: query
 *         name: amount
 *         schema:
 *           type: string
 *     responses:
 *       200:
 *         description: Gas bidding analysis
 */
router.get('/gas-analysis', mevController.getGasBidAnalysis);

/**
 * @openapi
 * /mev/route:
 *   get:
 *     summary: Private mempool routing hint and guidance
 *     tags:
 *       - MEV Protection
 *     parameters:
 *       - in: query
 *         name: operation
 *         schema:
 *           type: string
 *           enum: [Borrow, Withdraw, Liquidate]
 *       - in: query
 *         name: hint
 *         schema:
 *           type: string
 *           enum: [Default, PrivateMempool, BatchAuction, DelayedReveal]
 *     responses:
 *       200:
 *         description: Routing hint and guidance
 */
router.get('/route', mevController.getPrivateMempoolRoute);

/**
 * @openapi
 * /mev/fee-preview:
 *   get:
 *     summary: Preview the effective MEV protection fee without committing
 *     tags:
 *       - MEV Protection
 *     parameters:
 *       - in: query
 *         name: operation
 *         schema:
 *           type: string
 *           enum: [Borrow, Withdraw, Liquidate]
 *       - in: query
 *         name: assetAddress
 *         schema:
 *           type: string
 *       - in: query
 *         name: amount
 *         schema:
 *           type: string
 *     responses:
 *       200:
 *         description: Effective fee in basis points
 */
router.get('/fee-preview', mevController.getFeePreview);

export default router;
