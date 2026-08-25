import { Request, Response, NextFunction } from 'express';
import { stakingService } from '../services/staking.service';
import type {
  StakeRequest,
  UnstakeRequest,
  DelegateRequest,
  RevokeDelegationRequest,
  CreateYieldStrategyRequest,
  ActivateYieldStrategyRequest,
  CompoundStrategyRequest,
} from '../types/staking';

export const stake = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as StakeRequest;
    const position = stakingService.stake(body);
    return res.status(200).json({ success: true, position });
  } catch (err) {
    next(err);
    return;
  }
};

export const unstake = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as UnstakeRequest;
    const position = stakingService.unstake(body);
    return res.status(200).json({ success: true, position });
  } catch (err) {
    next(err);
    return;
  }
};

export const delegate = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as DelegateRequest;
    const result = stakingService.delegate(body);
    return res.status(200).json({ success: true, ...result });
  } catch (err) {
    next(err);
    return;
  }
};

export const revokeDelegation = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as RevokeDelegationRequest;
    const position = stakingService.revokeDelegation(body);
    return res.status(200).json({ success: true, position });
  } catch (err) {
    next(err);
    return;
  }
};

export const claimRewards = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params;
    const result = stakingService.claimRewards(userAddress!);
    return res.status(200).json({ success: true, ...result });
  } catch (err) {
    next(err);
    return;
  }
};

export const getPosition = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params;
    const position = stakingService.getPosition(userAddress!);
    return res.status(200).json({ success: true, position });
  } catch (err) {
    next(err);
    return;
  }
};

export const getAllPositions = async (_req: Request, res: Response, next: NextFunction) => {
  try {
    const positions = stakingService.getAllPositions();
    return res.status(200).json({ success: true, positions, total: positions.length });
  } catch (err) {
    next(err);
    return;
  }
};

// ─── Yield Farming Strategy Optimizer Controllers ─────────────────────────────

/**
 * POST /api/staking/yield-strategies
 * Create a new yield farming strategy for the authenticated user.
 */
export const createYieldStrategy = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as CreateYieldStrategyRequest;
    const strategy = stakingService.createYieldStrategy(body);
    return res.status(201).json({ success: true, strategy });
  } catch (err) {
    next(err);
    return;
  }
};

/**
 * GET /api/staking/yield-strategies
 * List all available strategies (admin / discovery view).
 */
export const getAllYieldStrategies = async (_req: Request, res: Response, next: NextFunction) => {
  try {
    const strategies = stakingService.getAllYieldStrategies();
    return res.status(200).json({ success: true, strategies, total: strategies.length });
  } catch (err) {
    next(err);
    return;
  }
};

/**
 * GET /api/staking/yield-strategies/:userAddress
 * List all strategies belonging to a specific user.
 */
export const getUserYieldStrategies = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params;
    const strategies = stakingService.getUserYieldStrategies(userAddress!);
    return res.status(200).json({ success: true, strategies, total: strategies.length });
  } catch (err) {
    next(err);
    return;
  }
};

/**
 * POST /api/staking/yield-strategies/:userAddress/activate
 * Activate a strategy for the given user.
 * Body: { strategyId }
 */
export const activateYieldStrategy = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params;
    const body = req.body as Omit<ActivateYieldStrategyRequest, 'userAddress'>;
    const strategy = stakingService.activateYieldStrategy({
      userAddress: userAddress!,
      strategyId: body.strategyId,
    });
    return res.status(200).json({ success: true, strategy });
  } catch (err) {
    next(err);
    return;
  }
};

/**
 * POST /api/staking/yield-strategies/:userAddress/compound
 * Manually trigger auto-compounding for the user's active strategy.
 * Body: { strategyId }
 */
export const compoundStrategy = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params;
    const body = req.body as Omit<CompoundStrategyRequest, 'userAddress'>;
    const result = stakingService.compoundStrategy({
      userAddress: userAddress!,
      strategyId: body.strategyId,
    });
    return res.status(200).json({ success: true, ...result });
  } catch (err) {
    next(err);
    return;
  }
};

/**
 * GET /api/staking/yield-strategies/:userAddress/performance/:strategyId
 * Fetch aggregated performance history for a specific strategy.
 */
export const getStrategyPerformance = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress, strategyId } = req.params;
    const performance = stakingService.getStrategyPerformance(strategyId!, userAddress!);
    return res.status(200).json({ success: true, performance });
  } catch (err) {
    next(err);
    return;
  }
};

/**
 * GET /api/staking/yield-optimization
 * Run the pool allocation optimizer and return rebalancing recommendations.
 * Query params:
 *   pools  – comma-separated list of pool addresses (required)
 *   util   – optional JSON object of { poolAddress: utilizationBps } overrides
 */
export const getYieldOptimization = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const poolsParam = req.query['pools'] as string | undefined;
    if (!poolsParam) {
      return res
        .status(400)
        .json({ success: false, error: 'Query param "pools" is required (comma-separated addresses)' });
    }

    const poolAddresses = poolsParam
      .split(',')
      .map((p) => p.trim())
      .filter(Boolean);

    if (poolAddresses.length === 0) {
      return res
        .status(400)
        .json({ success: false, error: 'At least one pool address is required' });
    }

    // Optional utilization overrides passed as a JSON string in the `util` query param
    let utilizationMap: Record<string, number> = {};
    const utilParam = req.query['util'] as string | undefined;
    if (utilParam) {
      try {
        utilizationMap = JSON.parse(utilParam) as Record<string, number>;
      } catch {
        return res
          .status(400)
          .json({ success: false, error: 'Query param "util" must be valid JSON' });
      }
    }

    const recommendation = stakingService.getYieldOptimizationRecommendation(
      poolAddresses,
      utilizationMap
    );
    return res.status(200).json({ success: true, recommendation });
  } catch (err) {
    next(err);
    return;
  }
};
