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

export default router;
