import { Router } from 'express';
import * as crossProtocolController from '../controllers/crossProtocol.controller';

const router: Router = Router();

/**
 * @openapi
 * /cross-protocol/compare:
 *   get:
 *     summary: Side-by-side metrics comparison across StellarLend and tracked peer protocols
 *     tags:
 *       - Cross-Protocol
 */
router.get('/compare', crossProtocolController.compare);

/**
 * @openapi
 * /cross-protocol/market-share:
 *   get:
 *     summary: TVL-weighted market share per protocol
 *     tags:
 *       - Cross-Protocol
 */
router.get('/market-share', crossProtocolController.marketShare);

/**
 * @openapi
 * /cross-protocol/leaderboard:
 *   get:
 *     summary: Protocol ranking by a chosen metric
 *     tags:
 *       - Cross-Protocol
 *     parameters:
 *       - in: query
 *         name: metric
 *         schema:
 *           type: string
 *           enum: [supplyApy, borrowApy, tvlUsd]
 *           default: tvlUsd
 *       - in: query
 *         name: limit
 *         schema:
 *           type: integer
 *           default: 20
 */
router.get('/leaderboard', crossProtocolController.leaderboard);

/**
 * @openapi
 * /cross-protocol/asset-comparison:
 *   get:
 *     summary: Per-asset comparison across tracked protocols (e.g. USDC, ETH, BTC)
 *     tags:
 *       - Cross-Protocol
 *     parameters:
 *       - in: query
 *         name: asset
 *         required: true
 *         schema: { type: string }
 */
router.get('/asset-comparison', crossProtocolController.assetComparison);

/**
 * @openapi
 * /cross-protocol/fee-comparison:
 *   get:
 *     summary: Reserve factor and effective spread per protocol
 *     tags:
 *       - Cross-Protocol
 */
router.get('/fee-comparison', crossProtocolController.feeComparison);

/**
 * @openapi
 * /cross-protocol/liquidation-params:
 *   get:
 *     summary: Liquidation threshold, bonus, and close factor per protocol
 *     tags:
 *       - Cross-Protocol
 */
router.get('/liquidation-params', crossProtocolController.liquidationParamsComparison);

/**
 * @openapi
 * /cross-protocol/market-share/history:
 *   get:
 *     summary: Market share snapshots over time
 *     tags:
 *       - Cross-Protocol
 *     parameters:
 *       - in: query
 *         name: limit
 *         schema: { type: integer }
 */
router.get('/market-share/history', crossProtocolController.marketShareHistory);

/**
 * @openapi
 * /cross-protocol/positioning-report:
 *   get:
 *     summary: StellarLend's strengths/weaknesses vs. the peer average
 *     tags:
 *       - Cross-Protocol
 */
router.get('/positioning-report', crossProtocolController.positioningReport);

/**
 * @openapi
 * /cross-protocol/benchmark-score:
 *   get:
 *     summary: 0-100 benchmark score per protocol across tracked metrics
 *     tags:
 *       - Cross-Protocol
 */
router.get('/benchmark-score', crossProtocolController.benchmarkScore);

/**
 * @openapi
 * /cross-protocol/weekly-digest:
 *   get:
 *     summary: Automated weekly comparison digest (cached for 7 days)
 *     tags:
 *       - Cross-Protocol
 */
router.get('/weekly-digest', crossProtocolController.weeklyDigest);

export default router;
