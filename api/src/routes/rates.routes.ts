import { Router } from 'express';
import * as ratesController from '../controllers/rates.controller';

const router: Router = Router();

/**
 * @openapi
 * /rates/history:
 *   get:
 *     summary: Historical interest rates for an asset over an explicit date range
 *     description: >
 *       Returns deposit/borrow APY snapshots for `asset` between `from` and
 *       `to`, bucketed at `granularity`. Range is capped to 1000 buckets.
 *     tags:
 *       - Rates
 *     parameters:
 *       - in: query
 *         name: asset
 *         schema:
 *           type: string
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
 *         name: granularity
 *         schema:
 *           type: string
 *           enum: [hourly, daily, weekly, monthly]
 *           default: daily
 */
router.get('/history', ratesController.rateHistory);

export default router;
