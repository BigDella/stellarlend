import { Router } from 'express';
import * as lendingController from '../controllers/lending.controller';
import { redisCacheService } from '../services/redisCache.service';

const router: Router = Router();

/**
 * @openapi
 * /health/live:
 *   get:
 *     summary: Fast liveness probe
 *     tags:
 *       - Health
 *     responses:
 *       200:
 *         description: API process is alive
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 status:
 *                   type: string
 *                   example: ok
 */
router.get('/live', lendingController.livenessCheck);

/**
 * @openapi
 * /health/ready:
 *   get:
 *     summary: Readiness probe with upstream dependency checks
 *     tags:
 *       - Health
 *     responses:
 *       200:
 *         description: Dependencies are ready
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 status:
 *                   type: string
 *                   example: ok
 *                 horizon:
 *                   type: string
 *                   example: up
 *                 soroban:
 *                   type: string
 *                   example: up
 *       503:
 *         description: One or more dependencies are unavailable
 */
router.get('/ready', lendingController.readinessCheck);

/**
 * @openapi
 * /health:
 *   get:
 *     summary: Backward-compatible readiness summary
 *     tags:
 *       - Health
 *     responses:
 *       200:
 *         description: All services are healthy
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/HealthCheckResponse'
 *       503:
 *         description: One or more services are unhealthy
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/HealthCheckResponse'
 */
router.get('/', lendingController.healthCheck);

/**
 * @openapi
 * /health/coalescing:
 *   get:
 *     summary: Request coalescing metrics
 *     tags:
 *       - Health
 *     responses:
 *       200:
 *         description: Coalescing metrics retrieved successfully
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 timestamp:
 *                   type: string
 *                   format: date-time
 *                 metrics:
 *                   type: object
 *                   properties:
 *                     totalRequests:
 *                       type: integer
 *                       description: Total number of requests processed
 *                     coalescedRequests:
 *                       type: integer
 *                       description: Number of requests that were coalesced
 *                     activeOperations:
 *                       type: integer
 *                       description: Number of currently active coalescing operations
 *                     averageWaitTime:
 *                       type: number
 *                       description: Average wait time for coalesced requests in milliseconds
 *                     timeoutCount:
 *                       type: integer
 *                       description: Number of requests that timed out waiting for coalescing
 */
router.get('/coalescing', lendingController.coalescingMetrics);

/**
 * @openapi
 * /health/cache-metrics:
 *   get:
 *     summary: Redis cache performance metrics
 *     tags:
 *       - Health
 *     responses:
 *       200:
 *         description: Cache hit/miss/error counters
 */
router.get('/cache-metrics', (_req, res) => {
  res.json(redisCacheService.getMetrics());
});

export default router;
