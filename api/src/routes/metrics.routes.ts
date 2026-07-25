import { Router } from 'express';
import * as metricsController from '../controllers/metrics.controller';

const router: Router = Router();

/**
 * @openapi
 * /metrics/timeseries:
 *   get:
 *     summary: Query protocol metrics time-series
 *     description: >
 *       Returns downsampled protocol health metrics from the TimescaleDB
 *       metrics collector (issue #455). Supports range and granularity.
 *     tags:
 *       - Metrics
 *     parameters:
 *       - in: query
 *         name: metric
 *         required: true
 *         schema:
 *           type: string
 *           enum: [tvl, totalBorrows, utilizationRate, liquidations, totalDeposits, activeUsers]
 *       - in: query
 *         name: from
 *         schema:
 *           type: string
 *           format: date-time
 *       - in: query
 *         name: to
 *         schema:
 *           type: string
 *           format: date-time
 *       - in: query
 *         name: interval
 *         schema:
 *           type: string
 *           enum: [1m, 5m, 1h, 1d]
 *           default: 1h
 *     responses:
 *       200:
 *         description: Time-series points
 *       400:
 *         description: Invalid query parameters
 */
router.get('/timeseries', metricsController.getTimeSeries);

export default router;
