import { Router } from 'express';
import { body, param, query } from 'express-validator';
import * as stakingController from '../controllers/staking.controller';
import { validateRequest } from '../middleware/validation';

const router: Router = Router();

// ─── Existing staking / governance routes ────────────────────────────────────
router.post('/stake', stakingController.stake);
router.post('/unstake', stakingController.unstake);
router.post('/delegate', stakingController.delegate);
router.post('/revoke-delegation', stakingController.revokeDelegation);
router.post('/claim-rewards/:userAddress', stakingController.claimRewards);
router.get('/positions', stakingController.getAllPositions);
router.get('/positions/:userAddress', stakingController.getPosition);

// ─── Yield Farming Strategy Optimizer routes (#789) ──────────────────────────

/**
 * POST /api/staking/yield-strategies
 * Create a new yield farming strategy.
 *
 * Body:
 *   userAddress         string   – Stellar address of the strategy owner
 *   name                string   – Human-readable strategy name
 *   objective           string   – 'maximize_apy' | 'minimize_il' | 'balanced'
 *   riskTier            string   – 'conservative' | 'balanced' | 'aggressive'
 *   compoundingInterval string   – 'hourly' | 'daily' | 'weekly' | 'manual'
 *   pools               array    – [{ poolAddress, allocationBps, estimatedApy }]
 *   autoCompoundEnabled boolean  – (optional, default true)
 */
router.post(
  '/yield-strategies',
  [
    body('userAddress').isString().notEmpty().withMessage('userAddress is required'),
    body('name').isString().notEmpty().withMessage('name is required'),
    body('objective')
      .isIn(['maximize_apy', 'minimize_il', 'balanced'])
      .withMessage('objective must be maximize_apy | minimize_il | balanced'),
    body('riskTier')
      .isIn(['conservative', 'balanced', 'aggressive'])
      .withMessage('riskTier must be conservative | balanced | aggressive'),
    body('compoundingInterval')
      .isIn(['hourly', 'daily', 'weekly', 'manual'])
      .withMessage('compoundingInterval must be hourly | daily | weekly | manual'),
    body('pools').isArray({ min: 1 }).withMessage('pools must be a non-empty array'),
    body('pools.*.poolAddress')
      .isString()
      .notEmpty()
      .withMessage('Each pool must have a poolAddress'),
    body('pools.*.allocationBps')
      .isInt({ min: 1 })
      .withMessage('Each pool allocationBps must be a positive integer'),
    body('pools.*.estimatedApy')
      .isFloat({ min: 0 })
      .withMessage('Each pool estimatedApy must be a non-negative number'),
    body('autoCompoundEnabled').optional().isBoolean(),
  ],
  validateRequest,
  stakingController.createYieldStrategy
);

/**
 * GET /api/staking/yield-strategies
 * List all yield farming strategies (discovery / admin view).
 */
router.get('/yield-strategies', stakingController.getAllYieldStrategies);

/**
 * GET /api/staking/yield-strategies/:userAddress
 * List all strategies owned by a specific user.
 */
router.get(
  '/yield-strategies/:userAddress',
  [
    param('userAddress').isString().notEmpty().withMessage('userAddress param is required'),
  ],
  validateRequest,
  stakingController.getUserYieldStrategies
);

/**
 * POST /api/staking/yield-strategies/:userAddress/activate
 * Activate a strategy for the given user.
 *
 * Body:
 *   strategyId  string  – UUID of the strategy to activate
 */
router.post(
  '/yield-strategies/:userAddress/activate',
  [
    param('userAddress').isString().notEmpty().withMessage('userAddress param is required'),
    body('strategyId').isString().notEmpty().withMessage('strategyId is required'),
  ],
  validateRequest,
  stakingController.activateYieldStrategy
);

/**
 * POST /api/staking/yield-strategies/:userAddress/compound
 * Manually trigger auto-compounding for a strategy.
 *
 * Body:
 *   strategyId  string  – UUID of the strategy to compound
 */
router.post(
  '/yield-strategies/:userAddress/compound',
  [
    param('userAddress').isString().notEmpty().withMessage('userAddress param is required'),
    body('strategyId').isString().notEmpty().withMessage('strategyId is required'),
  ],
  validateRequest,
  stakingController.compoundStrategy
);

/**
 * GET /api/staking/yield-strategies/:userAddress/performance/:strategyId
 * Fetch aggregated performance history for a strategy.
 */
router.get(
  '/yield-strategies/:userAddress/performance/:strategyId',
  [
    param('userAddress').isString().notEmpty().withMessage('userAddress param is required'),
    param('strategyId').isString().notEmpty().withMessage('strategyId param is required'),
  ],
  validateRequest,
  stakingController.getStrategyPerformance
);

/**
 * GET /api/staking/yield-optimization
 * Run the pool allocation optimizer and return rebalancing recommendations.
 *
 * Query params:
 *   pools  – comma-separated pool addresses (required)
 *   util   – optional JSON string { poolAddress: utilizationBps } overrides
 */
router.get(
  '/yield-optimization',
  [
    query('pools')
      .isString()
      .notEmpty()
      .withMessage('Query param "pools" is required (comma-separated addresses)'),
  ],
  validateRequest,
  stakingController.getYieldOptimization
);

export default router;
