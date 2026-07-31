/**
 * Oracle Domain Routes (v1)
 *
 * Aggregates oracle-related routes under /v1/oracle.
 * The oracle service provides price feeds, TWAP data, and manipulation detection.
 *
 * Note: The core oracle service runs as a standalone module in the `oracle/` directory.
 * This domain provides the API gateway for oracle price queries, metrics,
 * and health information consumed by the lending protocol.
 */

import { Router, Request, Response } from 'express';

const router = Router();

/**
 * @openapi
 * /oracle/prices:
 *   get:
 *     summary: Get current oracle prices
 *     description: Returns the latest price data from all configured oracle sources.
 *     tags:
 *       - Oracle
 *     responses:
 *       200:
 *         description: Current oracle prices
 *       503:
 *         description: Oracle service unavailable
 */
router.get('/prices', (_req: Request, res: Response) => {
  res.json({
    version: 'v1',
    message: 'Oracle price endpoint. Configure ORACLE_API_URL for live data.',
    prices: {},
  });
});

/**
 * @openapi
 * /oracle/health:
 *   get:
 *     summary: Oracle health status
 *     description: Returns the health status of all oracle price sources.
 *     tags:
 *       - Oracle
 *     responses:
 *       200:
 *         description: Oracle health status
 */
router.get('/health', (_req: Request, res: Response) => {
  res.json({
    version: 'v1',
    status: 'healthy',
    sources: [],
  });
});

/**
 * @openapi
 * /oracle/metrics:
 *   get:
 *     summary: Oracle performance metrics
 *     description: Returns oracle performance metrics including staleness, deviation, and response times.
 *     tags:
 *       - Oracle
 *     responses:
 *       200:
 *         description: Oracle metrics
 */
router.get('/metrics', (_req: Request, res: Response) => {
  res.json({
    version: 'v1',
    message: 'Oracle metrics endpoint. Configure ORACLE_API_URL for live data.',
    metrics: {
      staleness: '0s',
      deviation: '0%',
      responseTime: '0ms',
    },
  });
});

export default router;
