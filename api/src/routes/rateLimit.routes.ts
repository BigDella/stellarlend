/**
 * Rate Limiter Routes  — Issue #790
 *
 * Provides REST endpoints for monitoring and administering the adaptive
 * sensitive-operation rate limiter. Exposes:
 *  - GET  /api/rate-limit/analytics        — per-user counters + adaptive state
 *  - GET  /api/rate-limit/config           — static operation limits
 *  - POST /api/rate-limit/report-congestion — push congestion signal (reporter role)
 *  - POST /api/rate-limit/reset-congestion  — clear congestion back to normal (admin)
 *  - POST /api/rate-limit/reset             — flush all in-memory counters (admin)
 */

import { Router, Request, Response, NextFunction } from 'express';
import {
  getSensitiveRateLimitAnalytics,
  getSensitiveRateLimitConfig,
  resetSensitiveRateLimits,
  reportAdaptiveCongestion,
  resetAdaptiveCongestion,
  getAdaptiveState,
} from '../middleware/rate-limit';
import logger from '../utils/logger';
import { auditLogService } from '../services/auditLog.service';

const router: Router = Router();

/**
 * @openapi
 * /rate-limit/analytics:
 *   get:
 *     summary: Rate-limiter analytics dashboard
 *     description: >
 *       Returns per-key counter rows (one per (operation, userId) pair that has
 *       been seen since the last reset) together with the current adaptive
 *       throttling state. This is the primary data source for the protocol
 *       analytics dashboard's rate-limiter panel (Issue #790).
 *     tags: [RateLimit]
 *     responses:
 *       200:
 *         description: Analytics data including counters and adaptive state
 */
router.get('/analytics', (_req: Request, res: Response, next: NextFunction) => {
  try {
    const rows = getSensitiveRateLimitAnalytics();
    const adaptive = getAdaptiveState();

    const summary = {
      totalKeys: rows.length,
      blockedKeys: rows.filter((r) => r.penalty === 'block').length,
      throttledKeys: rows.filter((r) => r.penalty === 'throttle').length,
      warningKeys: rows.filter((r) => r.penalty === 'warning').length,
      violationsByOperation: rows.reduce<Record<string, number>>((acc, r) => {
        acc[r.operation] = (acc[r.operation] ?? 0) + r.violations;
        return acc;
      }, {}),
    };

    res.status(200).json({
      success: true,
      adaptive,
      summary,
      rows,
      generatedAt: new Date().toISOString(),
    });
  } catch (error) {
    next(error);
  }
});

/**
 * @openapi
 * /rate-limit/config:
 *   get:
 *     summary: Current rate-limit configuration
 *     description: Returns the static per-operation limit configuration (before adaptive scaling).
 *     tags: [RateLimit]
 *     responses:
 *       200:
 *         description: Operation limit configuration
 */
router.get('/config', (_req: Request, res: Response, next: NextFunction) => {
  try {
    const config = getSensitiveRateLimitConfig();
    const adaptive = getAdaptiveState();
    res.status(200).json({
      success: true,
      config,
      adaptive,
    });
  } catch (error) {
    next(error);
  }
});

/**
 * @openapi
 * /rate-limit/adaptive-state:
 *   get:
 *     summary: Current adaptive throttling state
 *     description: Returns the current congestion signal and derived scale factor.
 *     tags: [RateLimit]
 *     responses:
 *       200:
 *         description: Adaptive state
 */
router.get('/adaptive-state', (_req: Request, res: Response, next: NextFunction) => {
  try {
    const adaptive = getAdaptiveState();
    res.status(200).json({ success: true, adaptive });
  } catch (error) {
    next(error);
  }
});

/**
 * @openapi
 * /rate-limit/report-congestion:
 *   post:
 *     summary: Report network congestion index
 *     description: >
 *       Pushes a congestion signal into the adaptive throttler. Intended for
 *       off-chain network-monitoring services. congestionBps=10000 is normal;
 *       higher values throttle requests proportionally (down to 25% throughput
 *       at 40000 bps). Reports expire after the configured TTL.
 *     tags: [RateLimit]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [congestionBps]
 *             properties:
 *               congestionBps:
 *                 type: integer
 *                 example: 15000
 *               reportedBy:
 *                 type: string
 *                 example: "network-monitor-v1"
 *               ttlMs:
 *                 type: integer
 *                 description: Optional TTL override in milliseconds
 *     responses:
 *       200:
 *         description: Congestion reported successfully
 *       400:
 *         description: Invalid congestion value
 */
router.post('/report-congestion', (req: Request, res: Response, next: NextFunction) => {
  try {
    const { congestionBps, reportedBy, ttlMs } = req.body as {
      congestionBps?: unknown;
      reportedBy?: unknown;
      ttlMs?: unknown;
    };

    if (
      typeof congestionBps !== 'number' ||
      !Number.isFinite(congestionBps) ||
      congestionBps <= 0
    ) {
      res.status(400).json({
        success: false,
        error: 'congestionBps must be a positive finite number',
      });
      return;
    }

    const reporter = typeof reportedBy === 'string' ? reportedBy : 'api';
    const ttl = typeof ttlMs === 'number' && Number.isFinite(ttlMs) && ttlMs > 0 ? ttlMs : undefined;

    reportAdaptiveCongestion(congestionBps, reporter, ttl);
    const newState = getAdaptiveState();

    logger.info('Rate-limit congestion reported', { congestionBps, reporter, scaleFactor: newState.scaleFactor });
    auditLogService.record({
      action: 'RATE_LIMIT_CONGESTION_REPORT',
      actor: reporter,
      status: 'success',
      ip: req.ip,
    });

    res.status(200).json({
      success: true,
      message: 'Congestion reported',
      adaptive: newState,
    });
  } catch (error) {
    next(error);
  }
});

/**
 * @openapi
 * /rate-limit/reset-congestion:
 *   post:
 *     summary: Reset adaptive congestion state to normal
 *     description: >
 *       Clears any active congestion signal and restores limits to their
 *       configured values (scale factor = 1.0). Admin-only operation.
 *     tags: [RateLimit]
 *     responses:
 *       200:
 *         description: Congestion state reset
 */
router.post('/reset-congestion', (req: Request, res: Response, next: NextFunction) => {
  try {
    resetAdaptiveCongestion();
    const newState = getAdaptiveState();

    logger.info('Rate-limit congestion state reset to normal');
    auditLogService.record({
      action: 'RATE_LIMIT_CONGESTION_RESET',
      actor: req.ip ?? 'SYSTEM',
      status: 'success',
      ip: req.ip,
    });

    res.status(200).json({
      success: true,
      message: 'Congestion state reset to normal',
      adaptive: newState,
    });
  } catch (error) {
    next(error);
  }
});

/**
 * @openapi
 * /rate-limit/reset:
 *   post:
 *     summary: Reset all in-memory rate-limit counters
 *     description: >
 *       Flushes all per-user, per-operation in-memory counters. Use with care
 *       — this effectively grants every user a fresh window. Intended for
 *       emergency use or test resets only. Admin-only.
 *     tags: [RateLimit]
 *     responses:
 *       200:
 *         description: Counters reset successfully
 */
router.post('/reset', (req: Request, res: Response, next: NextFunction) => {
  try {
    resetSensitiveRateLimits();

    logger.warn('All sensitive-operation rate-limit counters flushed', { ip: req.ip });
    auditLogService.record({
      action: 'RATE_LIMIT_COUNTERS_RESET',
      actor: req.ip ?? 'SYSTEM',
      status: 'success',
      ip: req.ip,
    });

    res.status(200).json({
      success: true,
      message: 'All rate-limit counters reset',
    });
  } catch (error) {
    next(error);
  }
});

export default router;
