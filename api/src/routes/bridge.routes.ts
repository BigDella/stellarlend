/**
 * Bridge Routes  — Issue #799
 *
 * Full REST API for the cross-chain bridge integration including:
 *   Base bridge management: register, configure, deposit, withdraw
 *   Cross-chain messaging: submit, attest, execute
 *   Multi-chain lending: open/repaid/cancel positions, lock/release collateral
 *   Liquidity routing: register, update, list, best-route query
 *   Health factor oracle: submit and query remote health reports
 *   Validators: register, list
 *   Analytics: bridge stats, lending stats, security stats, channel state
 */

import { Router } from 'express';
import * as bridgeController from '../controllers/bridge.controller';

const router: Router = Router();

// ─── Bridge configuration (admin) ─────────────────────────────────────────────

/**
 * @openapi
 * /bridge/register:
 *   post:
 *     summary: Register a new bridge (admin)
 *     tags: [Bridge]
 */
router.post('/register', bridgeController.registerBridge);

/**
 * @openapi
 * /bridge/config/{bridgeId}:
 *   get:
 *     summary: Get bridge configuration
 *     tags: [Bridge]
 */
router.get('/config/:bridgeId', bridgeController.getBridgeConfig);

/**
 * @openapi
 * /bridge/list:
 *   get:
 *     summary: List all registered bridge IDs
 *     tags: [Bridge]
 */
router.get('/list', bridgeController.listBridges);

/**
 * @openapi
 * /bridge/fee:
 *   put:
 *     summary: Update bridge fee (admin)
 *     tags: [Bridge]
 */
router.put('/fee', bridgeController.setBridgeFee);

/**
 * @openapi
 * /bridge/active:
 *   put:
 *     summary: Enable or disable a bridge (admin)
 *     tags: [Bridge]
 */
router.put('/active', bridgeController.setBridgeActive);

/**
 * @openapi
 * /bridge/pause:
 *   post:
 *     summary: Pause or resume global bridge acceptance (admin)
 *     tags: [Bridge]
 */
router.post('/pause', bridgeController.setBridgePaused);

// ─── Base bridge operations ────────────────────────────────────────────────────

/**
 * @openapi
 * /bridge/deposit:
 *   post:
 *     summary: Initiate a bridge deposit
 *     tags: [Bridge]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [senderAddress, bridgeId, amount]
 *             properties:
 *               senderAddress: { type: string }
 *               bridgeId: { type: string }
 *               amount: { type: string }
 */
router.post('/deposit', bridgeController.bridgeDeposit);

/**
 * @openapi
 * /bridge/withdraw:
 *   post:
 *     summary: Execute a bridge withdrawal (admin / verified)
 *     tags: [Bridge]
 */
router.post('/withdraw', bridgeController.bridgeWithdraw);

// ─── Cross-chain messaging ─────────────────────────────────────────────────────

/**
 * @openapi
 * /bridge/messages/submit:
 *   post:
 *     summary: Submit a cross-chain message
 *     tags: [Bridge]
 */
router.post('/messages/submit', bridgeController.submitCrossChainMessage);

/**
 * @openapi
 * /bridge/messages/attest:
 *   post:
 *     summary: Submit validator attestation for a message
 *     tags: [Bridge]
 */
router.post('/messages/attest', bridgeController.attestMessage);

/**
 * @openapi
 * /bridge/messages/execute:
 *   post:
 *     summary: Execute a verified cross-chain withdrawal
 *     tags: [Bridge]
 */
router.post('/messages/execute', bridgeController.executeVerifiedWithdrawal);

/**
 * @openapi
 * /bridge/messages/{messageId}:
 *   get:
 *     summary: Get a cross-chain message by ID
 *     tags: [Bridge]
 */
router.get('/messages/:messageId', bridgeController.getCrossChainMessage);

// ─── Multi-chain lending: positions ───────────────────────────────────────────

/**
 * @openapi
 * /bridge/lending/positions/open:
 *   post:
 *     summary: Open a cross-chain lending position
 *     description: >
 *       Lock Stellar-side collateral and initiate a borrow on a remote chain.
 *       Requires a pre-submitted bridge message ID.
 *     tags: [Bridge, MultiChainLending]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [userAddress, remoteChain, remotePool, bridgeId, collateralAsset, collateralAmount, borrowAsset, borrowAmount]
 *             properties:
 *               userAddress: { type: string }
 *               remoteChain: { type: string, example: "ethereum" }
 *               remotePool: { type: string, example: "aave-v3" }
 *               bridgeId: { type: string }
 *               collateralAsset: { type: string }
 *               collateralAmount: { type: string }
 *               borrowAsset: { type: string }
 *               borrowAmount: { type: string }
 *               interestRateBps: { type: integer }
 *               bridgeMessageId: { type: integer }
 */
router.post('/lending/positions/open', bridgeController.openLendingPosition);

/**
 * @openapi
 * /bridge/lending/positions/repaid:
 *   post:
 *     summary: Mark a cross-chain position as repaid (relayer/validator)
 *     tags: [Bridge, MultiChainLending]
 */
router.post('/lending/positions/repaid', bridgeController.markPositionRepaid);

/**
 * @openapi
 * /bridge/lending/positions/cancel:
 *   post:
 *     summary: Cancel a stuck cross-chain position (admin)
 *     tags: [Bridge, MultiChainLending]
 */
router.post('/lending/positions/cancel', bridgeController.cancelPosition);

/**
 * @openapi
 * /bridge/lending/positions/{userAddress}/{remoteChain}/{remotePool}:
 *   get:
 *     summary: Get a cross-chain lending position
 *     tags: [Bridge, MultiChainLending]
 */
router.get('/lending/positions/:userAddress/:remoteChain/:remotePool', bridgeController.getLendingPosition);

// ─── Multi-chain lending: collateral ──────────────────────────────────────────

/**
 * @openapi
 * /bridge/lending/collateral/lock:
 *   post:
 *     summary: Record a collateral lock for a cross-chain borrow
 *     tags: [Bridge, MultiChainLending]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [callerAddress, userAddress, asset, vaultContract, amount, remoteChain, remotePool]
 *             properties:
 *               callerAddress: { type: string }
 *               userAddress: { type: string }
 *               asset: { type: string }
 *               vaultContract: { type: string }
 *               amount: { type: string }
 *               remoteChain: { type: string }
 *               remotePool: { type: string }
 */
router.post('/lending/collateral/lock', bridgeController.lockCollateral);

/**
 * @openapi
 * /bridge/lending/collateral/release:
 *   post:
 *     summary: Release collateral after position repayment
 *     tags: [Bridge, MultiChainLending]
 */
router.post('/lending/collateral/release', bridgeController.releaseCollateral);

/**
 * @openapi
 * /bridge/lending/collateral/{userAddress}/{remoteChain}/{remotePool}:
 *   get:
 *     summary: Get collateral lock details
 *     tags: [Bridge, MultiChainLending]
 */
router.get('/lending/collateral/:userAddress/:remoteChain/:remotePool', bridgeController.getCollateralLock);

// ─── Liquidity routing ─────────────────────────────────────────────────────────

/**
 * @openapi
 * /bridge/routes/register:
 *   post:
 *     summary: Register a liquidity route (admin)
 *     tags: [Bridge, LiquidityRouting]
 */
router.post('/routes/register', bridgeController.registerRoute);

/**
 * @openapi
 * /bridge/routes/update:
 *   put:
 *     summary: Update a liquidity route (admin)
 *     tags: [Bridge, LiquidityRouting]
 */
router.put('/routes/update', bridgeController.updateRoute);

/**
 * @openapi
 * /bridge/routes/list:
 *   get:
 *     summary: List all registered liquidity route IDs
 *     tags: [Bridge, LiquidityRouting]
 */
router.get('/routes/list', bridgeController.listRoutes);

/**
 * @openapi
 * /bridge/routes/best:
 *   get:
 *     summary: Find the cheapest active route for a remote chain/pool pair
 *     tags: [Bridge, LiquidityRouting]
 *     parameters:
 *       - in: query
 *         name: remoteChain
 *         required: true
 *         schema: { type: string }
 *       - in: query
 *         name: remotePool
 *         required: true
 *         schema: { type: string }
 *       - in: query
 *         name: requiredCapacity
 *         schema: { type: string }
 */
router.get('/routes/best', bridgeController.getBestRoute);

/**
 * @openapi
 * /bridge/routes/{bridgeId}:
 *   get:
 *     summary: Get a specific liquidity route
 *     tags: [Bridge, LiquidityRouting]
 */
router.get('/routes/:bridgeId', bridgeController.getRoute);

// ─── Remote health-factor oracle ───────────────────────────────────────────────

/**
 * @openapi
 * /bridge/health/submit:
 *   post:
 *     summary: Submit a remote health-factor report (validator)
 *     description: >
 *       Validators call this to push the user's health factor on the remote
 *       chain into the on-chain oracle. Reports expire after 100 ledgers.
 *     tags: [Bridge, HealthOracle]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [validatorAddress, userAddress, remoteChain, healthFactorBps]
 *             properties:
 *               validatorAddress: { type: string }
 *               userAddress: { type: string }
 *               remoteChain: { type: string }
 *               healthFactorBps: { type: integer, example: 15000 }
 *               remoteCollateralValue: { type: string }
 *               remoteDebtValue: { type: string }
 */
router.post('/health/submit', bridgeController.submitHealthReport);

/**
 * @openapi
 * /bridge/health/{userAddress}/{remoteChain}:
 *   get:
 *     summary: Get latest remote health-factor report for a user
 *     tags: [Bridge, HealthOracle]
 */
router.get('/health/:userAddress/:remoteChain', bridgeController.getHealthReport);

// ─── Validators ────────────────────────────────────────────────────────────────

/**
 * @openapi
 * /bridge/validators/register:
 *   post:
 *     summary: Register a bridge validator (admin)
 *     tags: [Bridge]
 */
router.post('/validators/register', bridgeController.registerValidator);

/**
 * @openapi
 * /bridge/validators:
 *   get:
 *     summary: List all registered validators
 *     tags: [Bridge]
 */
router.get('/validators', bridgeController.listValidators);

// ─── Analytics ─────────────────────────────────────────────────────────────────

/**
 * @openapi
 * /bridge/analytics:
 *   get:
 *     summary: Analytics for all bridges
 *     tags: [Bridge]
 */
router.get('/analytics', bridgeController.getAllBridgeAnalytics);

/**
 * @openapi
 * /bridge/analytics/{bridgeId}:
 *   get:
 *     summary: Analytics for a specific bridge
 *     tags: [Bridge]
 */
router.get('/analytics/:bridgeId', bridgeController.getBridgeAnalytics);

/**
 * @openapi
 * /bridge/lending/stats:
 *   get:
 *     summary: Global lending bridge statistics
 *     tags: [Bridge, MultiChainLending]
 */
router.get('/lending/stats', bridgeController.getLendingBridgeStats);

/**
 * @openapi
 * /bridge/security/stats:
 *   get:
 *     summary: Bridge security statistics
 *     tags: [Bridge]
 */
router.get('/security/stats', bridgeController.getSecurityStats);

// ─── Channel management ────────────────────────────────────────────────────────

/**
 * @openapi
 * /bridge/channels/{channelId}:
 *   get:
 *     summary: Get channel state
 *     tags: [Bridge]
 */
router.get('/channels/:channelId', bridgeController.getChannelState);

/**
 * @openapi
 * /bridge/channels/close:
 *   post:
 *     summary: Emergency close a channel (admin)
 *     tags: [Bridge]
 */
router.post('/channels/close', bridgeController.closeChannelEmergency);

/**
 * @openapi
 * /bridge/channels/reopen:
 *   post:
 *     summary: Reopen a closed channel (admin)
 *     tags: [Bridge]
 */
router.post('/channels/reopen', bridgeController.reopenChannel);

export default router;
