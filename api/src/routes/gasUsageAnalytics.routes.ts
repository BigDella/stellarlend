/**
 * Gas Usage Analytics Routes — Issue #483
 *
 * Mounted at /api/analytics/gas
 */

import { Router } from 'express';
import { gasUsageAnalyticsController } from '../controllers/gasUsageAnalytics.controller';

const router = Router();

/**
 * @openapi
 * /analytics/gas/by-function:
 *   get:
 *     summary: Gas usage stats (avg/median/p95/p99) for every tracked function
 *     tags:
 *       - Gas Usage Analytics
 *     parameters:
 *       - in: query
 *         name: period
 *         schema:
 *           type: string
 *           enum: [24h, 7d, 30d]
 *           default: 30d
 */
router.get('/by-function', (req, res) => gasUsageAnalyticsController.getAllFunctions(req, res));

/**
 * @openapi
 * /analytics/gas/by-function/{functionName}:
 *   get:
 *     summary: Gas usage stats for a single contract function
 *     tags:
 *       - Gas Usage Analytics
 */
router.get('/by-function/:functionName', (req, res) => gasUsageAnalyticsController.getByFunction(req, res));

/**
 * @openapi
 * /analytics/gas/anomalies:
 *   get:
 *     summary: Gas usage samples deviating more than N standard deviations from the mean
 *     tags:
 *       - Gas Usage Analytics
 *     parameters:
 *       - in: query
 *         name: stdDevThreshold
 *         schema:
 *           type: number
 *           default: 3
 */
router.get('/anomalies', (req, res) => gasUsageAnalyticsController.getAnomalies(req, res));

/**
 * @openapi
 * /analytics/gas/trend/{functionName}:
 *   get:
 *     summary: Daily/weekly gas usage trend for a function
 *     tags:
 *       - Gas Usage Analytics
 *     parameters:
 *       - in: query
 *         name: granularity
 *         schema:
 *           type: string
 *           enum: [daily, weekly]
 *           default: daily
 */
router.get('/trend/:functionName', (req, res) => gasUsageAnalyticsController.getTrend(req, res));

/**
 * @openapi
 * /analytics/gas/compare:
 *   get:
 *     summary: Compare gas usage between two functions
 *     tags:
 *       - Gas Usage Analytics
 *     parameters:
 *       - in: query
 *         name: functionA
 *         required: true
 *         schema: { type: string }
 *       - in: query
 *         name: functionB
 *         required: true
 *         schema: { type: string }
 */
router.get('/compare', (req, res) => gasUsageAnalyticsController.compare(req, res));

/**
 * @openapi
 * /analytics/gas/calldata-correlation/{functionName}:
 *   get:
 *     summary: Pearson correlation between calldata size and gas used
 *     tags:
 *       - Gas Usage Analytics
 */
router.get('/calldata-correlation/:functionName', (req, res) =>
  gasUsageAnalyticsController.getCalldataCorrelation(req, res)
);

/**
 * @openapi
 * /analytics/gas/report/{functionName}:
 *   get:
 *     summary: Consolidated optimization-targeting report for a function
 *     tags:
 *       - Gas Usage Analytics
 */
router.get('/report/:functionName', (req, res) => gasUsageAnalyticsController.getReport(req, res));

/**
 * @openapi
 * /analytics/gas/record:
 *   post:
 *     summary: Record an observed gas usage sample for a contract function
 *     tags:
 *       - Gas Usage Analytics
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [functionName, gasUsed]
 *             properties:
 *               functionName: { type: string }
 *               gasUsed: { type: number }
 *               calldataSize: { type: number }
 *               txHash: { type: string }
 */
router.post('/record', (req, res) => gasUsageAnalyticsController.recordSample(req, res));

export default router;
