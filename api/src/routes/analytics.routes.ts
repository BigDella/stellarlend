/**
 * Analytics Routes  — Issue #795
 *
 * Existing rate/revenue/export routes preserved + new real-time dashboard
 * endpoints added below.
 */

import { Router } from 'express';
import * as analyticsController from '../controllers/analytics.controller';

const router: Router = Router();

// ─── Existing rate / revenue / export endpoints ───────────────────────────────

/**
 * @openapi
 * /analytics/historical-rates:
 *   get:
 *     summary: Historical APY rates
 *     description: Returns deposit and borrow APY over a configurable time range.
 *     tags:
 *       - Analytics
 *     parameters:
 *       - in: query
 *         name: timeRange
 *         schema:
 *           type: string
 *           enum: [1d, 7d, 30d, 1y]
 *           default: 7d
 *       - in: query
 *         name: poolAddress
 *         schema:
 *           type: string
 */
router.get('/historical-rates', analyticsController.historicalRates);

/**
 * @openapi
 * /analytics/pool-utilization:
 *   get:
 *     summary: Pool utilization over time
 *     tags:
 *       - Analytics
 *     parameters:
 *       - in: query
 *         name: timeRange
 *         schema:
 *           type: string
 *           enum: [1d, 7d, 30d, 1y]
 *           default: 7d
 *       - in: query
 *         name: poolAddress
 *         schema:
 *           type: string
 */
router.get('/pool-utilization', analyticsController.poolUtilization);

/**
 * @openapi
 * /analytics/rate-comparison:
 *   get:
 *     summary: Rate comparison across pools
 *     tags:
 *       - Analytics
 */
router.get('/rate-comparison', analyticsController.rateComparison);

/**
 * @openapi
 * /analytics/revenue:
 *   get:
 *     summary: Protocol revenue tracking
 *     tags:
 *       - Analytics
 *     parameters:
 *       - in: query
 *         name: timeRange
 *         schema:
 *           type: string
 *           enum: [1d, 7d, 30d, 1y]
 *           default: 30d
 */
router.get('/revenue', analyticsController.protocolRevenue);

/**
 * @openapi
 * /analytics/summary:
 *   get:
 *     summary: Analytics summary snapshot
 *     tags:
 *       - Analytics
 */
router.get('/summary', analyticsController.analyticsSummary);

/**
 * @openapi
 * /analytics/export:
 *   get:
 *     summary: Export analytics data
 *     tags:
 *       - Analytics
 *     parameters:
 *       - in: query
 *         name: format
 *         schema:
 *           type: string
 *           enum: [json, csv]
 *           default: json
 *       - in: query
 *         name: timeRange
 *         schema:
 *           type: string
 *           enum: [1d, 7d, 30d, 1y]
 *           default: 7d
 */
router.get('/export', analyticsController.analyticsExport);

/**
 * @openapi
 * /analytics/rate-volatility:
 *   get:
 *     summary: Rolling standard deviation of deposit/borrow APY
 *     tags:
 *       - Analytics
 *     parameters:
 *       - in: query
 *         name: timeRange
 *         schema:
 *           type: string
 *           enum: [1d, 7d, 30d, 1y]
 *           default: 7d
 *       - in: query
 *         name: poolAddress
 *         schema:
 *           type: string
 *       - in: query
 *         name: windowSize
 *         schema:
 *           type: integer
 *           default: 10
 */
router.get('/rate-volatility', analyticsController.rateVolatility);

/**
 * @openapi
 * /analytics/weighted-average-rates:
 *   get:
 *     summary: Weighted average APY bucketed by day/week/month
 *     tags:
 *       - Analytics
 *     parameters:
 *       - in: query
 *         name: timeRange
 *         schema:
 *           type: string
 *           enum: [1d, 7d, 30d, 1y]
 *           default: 30d
 *       - in: query
 *         name: poolAddress
 *         schema:
 *           type: string
 *       - in: query
 *         name: granularity
 *         schema:
 *           type: string
 *           enum: [hourly, daily, weekly, monthly]
 *           default: daily
 */
router.get('/weighted-average-rates', analyticsController.weightedAverageRates);

/**
 * @openapi
 * /analytics/rate-change-events:
 *   get:
 *     summary: Detected material borrow-rate change events
 *     tags:
 *       - Analytics
 *     parameters:
 *       - in: query
 *         name: timeRange
 *         schema:
 *           type: string
 *           enum: [1d, 7d, 30d, 1y]
 *           default: 7d
 *       - in: query
 *         name: poolAddress
 *         schema:
 *           type: string
 *       - in: query
 *         name: thresholdBps
 *         schema:
 *           type: integer
 *           default: 10
 */
router.get('/rate-change-events', analyticsController.rateChangeEvents);

// ─── Real-time dashboard endpoints  (Issue #795) ─────────────────────────────

/**
 * @openapi
 * /analytics/dashboard:
 *   get:
 *     summary: Full real-time dashboard snapshot
 *     description: >
 *       Returns all dashboard panels (protocol metrics, activity feed,
 *       collateral ratios, active alerts) in a single aggregated response.
 *       Backed by a 15-second Redis cache for hot-path performance.
 *     tags: [Analytics, Dashboard]
 *     responses:
 *       200:
 *         description: Aggregated dashboard view
 */
router.get('/dashboard', analyticsController.dashboardView);

/**
 * @openapi
 * /analytics/dashboard/protocol:
 *   get:
 *     summary: Real-time protocol metrics panel
 *     description: TVL, utilization, avg borrow rate, total users and transactions.
 *     tags: [Analytics, Dashboard]
 */
router.get('/dashboard/protocol', analyticsController.protocolMetrics);

/**
 * @openapi
 * /analytics/dashboard/user/{userAddress}:
 *   get:
 *     summary: Per-user metrics panel
 *     description: Health factor, collateral, debt, risk level, activity score.
 *     tags: [Analytics, Dashboard]
 *     parameters:
 *       - in: path
 *         name: userAddress
 *         required: true
 *         schema:
 *           type: string
 */
router.get('/dashboard/user/:userAddress', analyticsController.userMetrics);

/**
 * @openapi
 * /analytics/dashboard/activity:
 *   get:
 *     summary: Real-time activity feed
 *     description: Most-recent protocol-wide or per-user activity entries.
 *     tags: [Analytics, Dashboard]
 *     parameters:
 *       - in: query
 *         name: limit
 *         schema:
 *           type: integer
 *           default: 20
 *       - in: query
 *         name: offset
 *         schema:
 *           type: integer
 *           default: 0
 *       - in: query
 *         name: userAddress
 *         schema:
 *           type: string
 *         description: Filter to a specific user (optional)
 */
router.get('/dashboard/activity', analyticsController.activityFeed);

/**
 * @openapi
 * /analytics/dashboard/metrics-history:
 *   get:
 *     summary: Historical metrics snapshots for trend charts
 *     description: Bounded history (up to 90 snapshots) for TVL / utilization / rate charts.
 *     tags: [Analytics, Dashboard]
 */
router.get('/dashboard/metrics-history', analyticsController.metricsHistory);

/**
 * @openapi
 * /analytics/dashboard/tvl-forecast:
 *   get:
 *     summary: Linear TVL forecast
 *     description: Least-squares linear regression over snapshot history, projected forward.
 *     tags: [Analytics, Dashboard]
 *     parameters:
 *       - in: query
 *         name: periodsAhead
 *         schema:
 *           type: integer
 *           default: 5
 *           minimum: 1
 *           maximum: 30
 */
router.get('/dashboard/tvl-forecast', analyticsController.tvlForecast);

/**
 * @openapi
 * /analytics/dashboard/collateral-ratios:
 *   get:
 *     summary: Real-time collateral ratio snapshots
 *     description: Current health-factor and risk-level for each tracked asset.
 *     tags: [Analytics, Dashboard]
 */
router.get('/dashboard/collateral-ratios', analyticsController.collateralRatios);

/**
 * @openapi
 * /analytics/dashboard/alerts:
 *   get:
 *     summary: Triggered metric alerts log
 *     description: Bounded audit trail of threshold-breach events.
 *     tags: [Analytics, Dashboard]
 */
router.get('/dashboard/alerts', analyticsController.triggeredAlerts);

/**
 * @openapi
 * /analytics/dashboard/risk-distribution:
 *   get:
 *     summary: User risk-level distribution
 *     description: >
 *       Counts of sampled users at each risk level (1=safe → 5=critical),
 *       derived from the bounded activity log.
 *     tags: [Analytics, Dashboard]
 */
router.get('/dashboard/risk-distribution', analyticsController.riskDistribution);

/**
 * @openapi
 * /analytics/dashboard/volume:
 *   get:
 *     summary: Protocol volume summary
 *     description: >
 *       Cumulative deposit, borrow, withdrawal, repayment and liquidation
 *       volumes from the on-chain activity log.
 *     tags: [Analytics, Dashboard]
 */
router.get('/dashboard/volume', analyticsController.volumeSummary);

/**
 * @openapi
 * /analytics/dashboard/alerts/config:
 *   post:
 *     summary: Configure a metric alert threshold (admin)
 *     tags: [Analytics, Dashboard]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [adminAddress, metric, threshold]
 *             properties:
 *               adminAddress: { type: string }
 *               metric:
 *                 type: string
 *                 enum: [tvl, utilization, avg_rate]
 *               threshold: { type: string }
 */
router.post('/dashboard/alerts/config', analyticsController.setAlertThreshold);

export default router;
