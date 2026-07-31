/**
 * Governance Domain Routes (v1)
 *
 * Aggregates all governance-related routes under /v1/governance:
 * - Staking (stake, unstake, delegate, claim rewards)
 * - Rebalancing (configure, execute, emergency controls)
 * - Risk monitoring (pool health, liquidation heatmap, oracle health, alerts)
 */

import { Router } from 'express';
import stakingRoutes from '../../staking.routes';
import rebalancingRoutes from '../../rebalancing.routes';
import riskRoutes from '../../risk.routes';

const router = Router();

// Staking: /v1/governance/staking/*
router.use('/staking', stakingRoutes);

// Rebalancing: /v1/governance/rebalancing/*
router.use('/rebalancing', rebalancingRoutes);

// Risk monitoring: /v1/governance/risk/*
router.use('/risk', riskRoutes);

export default router;
