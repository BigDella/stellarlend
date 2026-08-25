import { randomUUID } from 'crypto';
import type {
  StakingPosition,
  StakeRequest,
  UnstakeRequest,
  DelegateRequest,
  RevokeDelegationRequest,
  StakingRewardConfig,
  YieldFarmingStrategy,
  YieldStrategyPerformance,
  YieldOptimizationRecommendation,
  CreateYieldStrategyRequest,
  ActivateYieldStrategyRequest,
  CompoundStrategyRequest,
  PoolAllocation,
  StrategyPerformanceSnapshot,
} from '../types/staking';
import logger from '../utils/logger';

const REWARD_CONFIG: StakingRewardConfig = {
  baseAprBps: 500, // 5% base APR
  lockupBonusBps: 200, // +2% per 30-day lockup tier
  epochDurationSeconds: 86_400, // daily epochs
};

const MIN_STAKE_AMOUNT = BigInt(1_000_000); // 1 token in stroops
const LOCKUP_OPTIONS = [0, 30, 90, 180, 365];

// In-memory store. Replace with DB/Redis in production.
const positions = new Map<string, StakingPosition>();

// ─── Yield Farming in-memory stores ──────────────────────────────────────────

/** All persisted strategies keyed by strategyId */
const yieldStrategies = new Map<string, YieldFarmingStrategy>();

/** Performance snapshots keyed by strategyId */
const yieldPerformance = new Map<string, StrategyPerformanceSnapshot[]>();

/**
 * APY lookup table used by the optimizer.
 * In production these would come from an on-chain oracle or price feed.
 */
const POOL_APY_ESTIMATES: Record<string, number> = {};

/** Minimum total allocation BPS required when creating a strategy */
const TOTAL_ALLOCATION_BPS = 10_000;

/** Base APY boost granted by auto-compounding per interval (mirrors vault service) */
const COMPOUND_INTERVAL_MULTIPLIERS: Record<string, number> = {
  hourly: 1.35,
  daily: 1.28,
  weekly: 1.15,
  manual: 1.0,
};

/** Risk-tier APY caps (bps) – conservative strategies get capped yield */
const RISK_APY_CAP: Record<string, number> = {
  conservative: 8,
  balanced: 18,
  aggressive: 40,
};

function now(): string {
  return new Date().toISOString();
}

function computeVotingPower(stakedAmount: bigint, lockupDays: number): string {
  // Voting power = staked * (1 + lockup_multiplier)
  // Each 30-day tier adds 0.25x
  const tierMultiplierBps = Math.floor(lockupDays / 30) * 25; // bps
  const base = stakedAmount * BigInt(10_000 + tierMultiplierBps);
  return (base / BigInt(10_000)).toString();
}

function computeLockupEnd(lockupDays: number): string {
  const ms = lockupDays * 24 * 60 * 60 * 1000;
  return new Date(Date.now() + ms).toISOString();
}

function accrueRewards(position: StakingPosition): string {
  const staked = BigInt(position.stakedAmount);
  if (staked === BigInt(0)) return position.earnedRewards;

  const lockupBonus = Math.floor(position.lockupDays / 30) * REWARD_CONFIG.lockupBonusBps;
  const totalAprBps = REWARD_CONFIG.baseAprBps + lockupBonus;

  const updatedAt = new Date(position.updatedAt).getTime();
  const elapsed = (Date.now() - updatedAt) / 1000; // seconds
  const epochsElapsed = elapsed / REWARD_CONFIG.epochDurationSeconds;

  // rewards = staked * apr * elapsed_epochs / epochs_per_year
  const epochsPerYear = 365;
  const reward =
    (staked * BigInt(Math.round(epochsElapsed * totalAprBps * 100))) /
    BigInt(epochsPerYear * 10_000 * 100);

  const current = BigInt(position.earnedRewards);
  return (current + reward).toString();
}

// ─── Yield farming pure helpers ───────────────────────────────────────────────

/**
 * Compute the weighted-average APY across pools, capped by the risk tier.
 */
function computeBlendedApy(pools: PoolAllocation[], riskTier: string): number {
  if (pools.length === 0) return 0;
  const weighted = pools.reduce(
    (sum, p) => sum + p.estimatedApy * p.allocationBps,
    0
  );
  const raw = weighted / TOTAL_ALLOCATION_BPS;
  const cap = RISK_APY_CAP[riskTier] ?? 40;
  return Math.round(Math.min(raw, cap) * 100) / 100;
}

/**
 * Compute the ISO timestamp for the next scheduled compound based on the
 * chosen compounding interval.
 */
function computeNextCompoundAt(interval: string): string {
  const intervalMs: Record<string, number> = {
    hourly: 60 * 60 * 1000,
    daily: 24 * 60 * 60 * 1000,
    weekly: 7 * 24 * 60 * 60 * 1000,
    manual: 0,
  };
  const ms = intervalMs[interval] ?? intervalMs['daily']!;
  if (ms === 0) return '';
  return new Date(Date.now() + ms).toISOString();
}

export class StakingService {
  stake(req: StakeRequest): StakingPosition {
    const amount = BigInt(req.amount);
    if (amount < MIN_STAKE_AMOUNT) {
      throw Object.assign(new Error('Stake amount below minimum'), { status: 400 });
    }

    const lockupDays = req.lockupDays ?? 0;
    if (!LOCKUP_OPTIONS.includes(lockupDays)) {
      throw Object.assign(
        new Error(`Invalid lockup period. Allowed: ${LOCKUP_OPTIONS.join(', ')} days`),
        { status: 400 }
      );
    }

    const existing = positions.get(req.userAddress);

    if (existing) {
      // Merge into existing position, accruing rewards first
      const accrued = accrueRewards(existing);
      const newStaked = BigInt(existing.stakedAmount) + amount;
      const updated: StakingPosition = {
        ...existing,
        stakedAmount: newStaked.toString(),
        lockupDays: Math.max(existing.lockupDays, lockupDays),
        lockupEndTime: computeLockupEnd(Math.max(existing.lockupDays, lockupDays)),
        votingPower: computeVotingPower(newStaked, Math.max(existing.lockupDays, lockupDays)),
        earnedRewards: accrued,
        updatedAt: now(),
      };
      positions.set(req.userAddress, updated);
      logger.info('Staking position increased', {
        userAddress: req.userAddress,
        amount: req.amount,
      });
      return updated;
    }

    const position: StakingPosition = {
      userAddress: req.userAddress,
      stakedAmount: amount.toString(),
      lockupDays,
      lockupEndTime: computeLockupEnd(lockupDays),
      votingPower: computeVotingPower(amount, lockupDays),
      delegatedFrom: [],
      earnedRewards: '0',
      createdAt: now(),
      updatedAt: now(),
    };
    positions.set(req.userAddress, position);
    logger.info('New staking position created', { userAddress: req.userAddress });
    return position;
  }

  unstake(req: UnstakeRequest): StakingPosition {
    const position = positions.get(req.userAddress);
    if (!position) {
      throw Object.assign(new Error('No staking position found'), { status: 404 });
    }

    const lockupEnd = new Date(position.lockupEndTime).getTime();
    const isEarlyUnstake = position.lockupDays > 0 && Date.now() < lockupEnd;

    let amount = BigInt(req.amount);
    const staked = BigInt(position.stakedAmount);
    if (amount > staked) {
      throw Object.assign(new Error('Unstake amount exceeds staked balance'), { status: 400 });
    }

    // Early unstake penalty: 10% of unstaked amount
    if (isEarlyUnstake) {
      const penalty = amount / BigInt(10);
      amount -= penalty;
      logger.info('Early unstake penalty applied', {
        userAddress: req.userAddress,
        penalty: penalty.toString(),
      });
    }

    const accrued = accrueRewards(position);
    const newStaked = staked - BigInt(req.amount);
    const updated: StakingPosition = {
      ...position,
      stakedAmount: newStaked.toString(),
      votingPower: computeVotingPower(newStaked, position.lockupDays),
      earnedRewards: accrued,
      updatedAt: now(),
    };
    positions.set(req.userAddress, updated);
    logger.info('Unstaked tokens', { userAddress: req.userAddress, amount: req.amount });
    return updated;
  }

  delegate(req: DelegateRequest): { delegator: StakingPosition; delegate: StakingPosition } {
    const delegatorPos = positions.get(req.userAddress);
    if (!delegatorPos) {
      throw Object.assign(new Error('No staking position for delegator'), { status: 404 });
    }
    if (req.userAddress === req.delegateTo) {
      throw Object.assign(new Error('Cannot delegate to self'), { status: 400 });
    }

    const delegatePos = positions.get(req.delegateTo);
    if (!delegatePos) {
      throw Object.assign(new Error('Delegate address has no staking position'), { status: 404 });
    }

    // Remove from previous delegate if any
    if (delegatorPos.delegatedTo) {
      const prevDelegate = positions.get(delegatorPos.delegatedTo);
      if (prevDelegate) {
        const filtered = prevDelegate.delegatedFrom.filter((a) => a !== req.userAddress);
        const prevDelegateVP = computeVotingPower(
          BigInt(prevDelegate.stakedAmount),
          prevDelegate.lockupDays
        );
        positions.set(delegatorPos.delegatedTo, {
          ...prevDelegate,
          delegatedFrom: filtered,
          votingPower: prevDelegateVP,
          updatedAt: now(),
        });
      }
    }

    const updatedDelegator: StakingPosition = {
      ...delegatorPos,
      delegatedTo: req.delegateTo,
      votingPower: '0', // delegator gives up their voting power
      updatedAt: now(),
    };

    const delegatedPower = BigInt(delegatorPos.votingPower || delegatorPos.stakedAmount);
    const delegateNewVP = BigInt(delegatePos.votingPower) + delegatedPower;

    const updatedDelegate: StakingPosition = {
      ...delegatePos,
      delegatedFrom: [...new Set([...delegatePos.delegatedFrom, req.userAddress])],
      votingPower: delegateNewVP.toString(),
      updatedAt: now(),
    };

    positions.set(req.userAddress, updatedDelegator);
    positions.set(req.delegateTo, updatedDelegate);

    logger.info('Vote delegation set', { from: req.userAddress, to: req.delegateTo });
    return { delegator: updatedDelegator, delegate: updatedDelegate };
  }

  revokeDelegation(req: RevokeDelegationRequest): StakingPosition {
    const position = positions.get(req.userAddress);
    if (!position) {
      throw Object.assign(new Error('No staking position found'), { status: 404 });
    }
    if (!position.delegatedTo) {
      throw Object.assign(new Error('No active delegation to revoke'), { status: 400 });
    }

    const delegatePos = positions.get(position.delegatedTo);
    if (delegatePos) {
      const reclaimed = BigInt(position.stakedAmount);
      const delegateNewVP = BigInt(delegatePos.votingPower) - reclaimed;
      positions.set(position.delegatedTo, {
        ...delegatePos,
        delegatedFrom: delegatePos.delegatedFrom.filter((a) => a !== req.userAddress),
        votingPower: (delegateNewVP > BigInt(0) ? delegateNewVP : BigInt(0)).toString(),
        updatedAt: now(),
      });
    }

    const restored: StakingPosition = {
      ...position,
      delegatedTo: undefined,
      votingPower: computeVotingPower(BigInt(position.stakedAmount), position.lockupDays),
      updatedAt: now(),
    };
    positions.set(req.userAddress, restored);
    logger.info('Delegation revoked', { userAddress: req.userAddress });
    return restored;
  }

  claimRewards(userAddress: string): { position: StakingPosition; claimed: string } {
    const position = positions.get(userAddress);
    if (!position) {
      throw Object.assign(new Error('No staking position found'), { status: 404 });
    }

    const totalRewards = accrueRewards(position);
    const updated: StakingPosition = {
      ...position,
      earnedRewards: '0',
      updatedAt: now(),
    };
    positions.set(userAddress, updated);
    logger.info('Rewards claimed', { userAddress, amount: totalRewards });
    return { position: updated, claimed: totalRewards };
  }

  getPosition(userAddress: string): StakingPosition {
    const position = positions.get(userAddress);
    if (!position) {
      throw Object.assign(new Error('No staking position found'), { status: 404 });
    }
    return { ...position, earnedRewards: accrueRewards(position) };
  }

  getAllPositions(): StakingPosition[] {
    return Array.from(positions.values()).map((p) => ({
      ...p,
      earnedRewards: accrueRewards(p),
    }));
  }

  // ─── Yield Farming Strategy Optimizer ──────────────────────────────────────

  /**
   * Create a new yield farming strategy for a user.
   * Validates that pool allocations sum to 10 000 bps and that each pool
   * address is non-empty, then persists the strategy in the in-memory store.
   */
  createYieldStrategy(req: CreateYieldStrategyRequest): YieldFarmingStrategy {
    if (!req.userAddress) {
      throw Object.assign(new Error('userAddress is required'), { status: 400 });
    }
    if (!req.pools || req.pools.length === 0) {
      throw Object.assign(new Error('At least one pool allocation is required'), { status: 400 });
    }

    const totalBps = req.pools.reduce((sum, p) => sum + p.allocationBps, 0);
    if (totalBps !== TOTAL_ALLOCATION_BPS) {
      throw Object.assign(
        new Error(`Pool allocations must sum to ${TOTAL_ALLOCATION_BPS} bps, got ${totalBps}`),
        { status: 400 }
      );
    }

    for (const p of req.pools) {
      if (!p.poolAddress || p.poolAddress.trim() === '') {
        throw Object.assign(new Error('Each pool must have a non-empty poolAddress'), {
          status: 400,
        });
      }
      if (p.allocationBps <= 0) {
        throw Object.assign(new Error('Each pool allocationBps must be positive'), { status: 400 });
      }
    }

    // Deactivate any currently active strategy for this user
    for (const [id, strat] of yieldStrategies) {
      if (strat.userAddress === req.userAddress && strat.active) {
        yieldStrategies.set(id, { ...strat, active: false, updatedAt: now() });
      }
    }

    // Enrich pools with utilization snapshot (defaults to 0 if unknown)
    const enrichedPools: PoolAllocation[] = req.pools.map((p) => ({
      ...p,
      currentUtilizationBps: 0,
      estimatedApy: POOL_APY_ESTIMATES[p.poolAddress] ?? p.estimatedApy,
    }));

    const blendedApy = computeBlendedApy(enrichedPools, req.riskTier);
    const compoundMultiplier = COMPOUND_INTERVAL_MULTIPLIERS[req.compoundingInterval] ?? 1.28;
    const effectiveApy = req.autoCompoundEnabled !== false
      ? Math.round(blendedApy * compoundMultiplier * 100) / 100
      : blendedApy;

    const strategyId = randomUUID();
    const ts = now();

    const strategy: YieldFarmingStrategy = {
      strategyId,
      userAddress: req.userAddress,
      name: req.name,
      objective: req.objective,
      riskTier: req.riskTier,
      compoundingInterval: req.compoundingInterval,
      pools: enrichedPools,
      totalAllocatedBps: TOTAL_ALLOCATION_BPS,
      estimatedBlendedApy: effectiveApy,
      autoCompoundEnabled: req.autoCompoundEnabled !== false,
      active: true,
      createdAt: ts,
      updatedAt: ts,
      nextCompoundAt: computeNextCompoundAt(req.compoundingInterval),
    };

    yieldStrategies.set(strategyId, strategy);
    yieldPerformance.set(strategyId, []);

    logger.info('Yield farming strategy created', {
      strategyId,
      userAddress: req.userAddress,
      objective: req.objective,
    });

    return strategy;
  }

  /**
   * Return all strategies for a specific user.
   */
  getUserYieldStrategies(userAddress: string): YieldFarmingStrategy[] {
    return Array.from(yieldStrategies.values()).filter(
      (s) => s.userAddress === userAddress
    );
  }

  /**
   * Return all available (active) strategies across all users.
   * Used by the strategy listing endpoint.
   */
  getAllYieldStrategies(): YieldFarmingStrategy[] {
    return Array.from(yieldStrategies.values());
  }

  /**
   * Activate a previously created (or deactivated) strategy.
   * Automatically deactivates any other active strategy for the same user.
   */
  activateYieldStrategy(req: ActivateYieldStrategyRequest): YieldFarmingStrategy {
    const strategy = yieldStrategies.get(req.strategyId);
    if (!strategy) {
      throw Object.assign(new Error('Strategy not found'), { status: 404 });
    }
    if (strategy.userAddress !== req.userAddress) {
      throw Object.assign(new Error('Strategy does not belong to this user'), { status: 403 });
    }
    if (strategy.active) {
      throw Object.assign(new Error('Strategy is already active'), { status: 400 });
    }

    // Deactivate any other active strategy for this user
    for (const [id, strat] of yieldStrategies) {
      if (id !== req.strategyId && strat.userAddress === req.userAddress && strat.active) {
        yieldStrategies.set(id, { ...strat, active: false, updatedAt: now() });
      }
    }

    const updated: YieldFarmingStrategy = {
      ...strategy,
      active: true,
      updatedAt: now(),
      nextCompoundAt: computeNextCompoundAt(strategy.compoundingInterval),
    };
    yieldStrategies.set(req.strategyId, updated);

    logger.info('Yield farming strategy activated', {
      strategyId: req.strategyId,
      userAddress: req.userAddress,
    });
    return updated;
  }

  /**
   * Manually trigger auto-compounding for a strategy.
   * Records a performance snapshot, updates LP fee totals, and resets the
   * next-compound timestamp.
   */
  compoundStrategy(req: CompoundStrategyRequest): {
    strategy: YieldFarmingStrategy;
    compoundedAmount: string;
    newApy: number;
  } {
    const strategy = yieldStrategies.get(req.strategyId);
    if (!strategy) {
      throw Object.assign(new Error('Strategy not found'), { status: 404 });
    }
    if (strategy.userAddress !== req.userAddress) {
      throw Object.assign(new Error('Strategy does not belong to this user'), { status: 403 });
    }
    if (!strategy.active) {
      throw Object.assign(new Error('Cannot compound an inactive strategy'), { status: 400 });
    }

    // Simulate compound: estimate yield earned since last compound
    const lastTs = strategy.lastCompoundedAt
      ? new Date(strategy.lastCompoundedAt).getTime()
      : new Date(strategy.createdAt).getTime();
    const elapsedMs = Date.now() - lastTs;
    const elapsedYears = elapsedMs / (365 * 24 * 60 * 60 * 1000);

    // Simplified: compound on a nominal TVL of 1 000 000 stroops per active pool
    const nominalTvl = strategy.pools.length * 1_000_000;
    const compoundedRaw = Math.floor(nominalTvl * (strategy.estimatedBlendedApy / 100) * elapsedYears);
    const compoundedAmount = Math.max(compoundedRaw, 0).toString();

    // Re-score APY with compounding boost
    const multiplier = COMPOUND_INTERVAL_MULTIPLIERS[strategy.compoundingInterval] ?? 1.28;
    const newBlended = computeBlendedApy(strategy.pools, strategy.riskTier);
    const newApy = Math.round(newBlended * multiplier * 100) / 100;

    const ts = now();
    const snapshot: StrategyPerformanceSnapshot = {
      timestamp: ts,
      totalValueLocked: String(nominalTvl),
      yieldEarned: compoundedAmount,
      compoundedAmount,
      apy: newApy,
      ilImpactBps: 0, // IL impact tracked on-chain; 0 in API layer simulation
    };

    const snapshots = yieldPerformance.get(req.strategyId) ?? [];
    snapshots.push(snapshot);
    yieldPerformance.set(req.strategyId, snapshots);

    const updated: YieldFarmingStrategy = {
      ...strategy,
      estimatedBlendedApy: newApy,
      lastCompoundedAt: ts,
      nextCompoundAt: computeNextCompoundAt(strategy.compoundingInterval),
      updatedAt: ts,
    };
    yieldStrategies.set(req.strategyId, updated);

    logger.info('Strategy compounded', {
      strategyId: req.strategyId,
      compoundedAmount,
      newApy,
    });

    return { strategy: updated, compoundedAmount, newApy };
  }

  /**
   * Return aggregated performance data for a strategy.
   */
  getStrategyPerformance(strategyId: string, userAddress: string): YieldStrategyPerformance {
    const strategy = yieldStrategies.get(strategyId);
    if (!strategy) {
      throw Object.assign(new Error('Strategy not found'), { status: 404 });
    }
    if (strategy.userAddress !== userAddress) {
      throw Object.assign(new Error('Strategy does not belong to this user'), { status: 403 });
    }

    const snapshots = yieldPerformance.get(strategyId) ?? [];

    const totalYieldEarned = snapshots
      .reduce((sum, s) => sum + BigInt(s.yieldEarned), BigInt(0))
      .toString();

    const totalCompounded = snapshots
      .reduce((sum, s) => sum + BigInt(s.compoundedAmount), BigInt(0))
      .toString();

    const averageIlImpactBps =
      snapshots.length > 0
        ? Math.round(snapshots.reduce((sum, s) => sum + s.ilImpactBps, 0) / snapshots.length)
        : 0;

    const realizedApy =
      snapshots.length > 0
        ? Math.round((snapshots.reduce((sum, s) => sum + s.apy, 0) / snapshots.length) * 100) / 100
        : strategy.estimatedBlendedApy;

    return {
      strategyId,
      userAddress,
      totalYieldEarned,
      totalCompounded,
      compoundCount: snapshots.length,
      realizedApy,
      averageIlImpactBps,
      snapshots,
    };
  }

  /**
   * Run the pool allocation optimizer over a list of pool addresses.
   * Mirrors the on-chain `optimize_allocation` logic in amm.rs but operates
   * on API-layer utilization data so callers can preview recommendations
   * without submitting an on-chain transaction.
   *
   * Optimal utilization target: 80 % (8 000 bps).
   * Rebalance threshold:          5 % (  500 bps).
   */
  getYieldOptimizationRecommendation(
    poolAddresses: string[],
    utilizationMap: Record<string, number> = {}
  ): YieldOptimizationRecommendation {
    if (!poolAddresses || poolAddresses.length === 0) {
      throw Object.assign(new Error('At least one pool address is required'), { status: 400 });
    }

    const OPTIMAL_BPS = 8_000;
    const REBALANCE_THRESHOLD_BPS = 500;
    const DEFAULT_BUFFER_BPS = 8_000;

    let totalUtilization = 0;

    const pools = poolAddresses.map((addr) => {
      const currentUtilizationBps = utilizationMap[addr] ?? 0;
      totalUtilization += currentUtilizationBps;

      const deviation = Math.abs(currentUtilizationBps - OPTIMAL_BPS);
      let action: 'increase' | 'decrease' | 'no_change';
      let adjustmentAmount: number;

      if (deviation < REBALANCE_THRESHOLD_BPS) {
        action = 'no_change';
        adjustmentAmount = 0;
      } else if (currentUtilizationBps < OPTIMAL_BPS) {
        action = 'increase';
        const available = 10_000 - DEFAULT_BUFFER_BPS;
        adjustmentAmount = Math.floor(
          (available * (OPTIMAL_BPS - currentUtilizationBps)) / 10_000
        );
      } else {
        action = 'decrease';
        const excess = currentUtilizationBps - OPTIMAL_BPS;
        adjustmentAmount = Math.floor((excess * currentUtilizationBps) / 10_000);
      }

      return {
        poolAddress: addr,
        currentUtilizationBps,
        recommendedAllocationBps: OPTIMAL_BPS,
        action,
        adjustmentAmount,
      };
    });

    const avgUtilization =
      poolAddresses.length > 0 ? Math.floor(totalUtilization / poolAddresses.length) : 0;

    const efficiency = Math.min(avgUtilization, OPTIMAL_BPS);

    const estimatedYieldImprovementBps =
      avgUtilization < OPTIMAL_BPS
        ? Math.floor((OPTIMAL_BPS - avgUtilization) / 100)
        : 0;

    return {
      pools,
      totalCapitalEfficiencyBps: efficiency,
      estimatedYieldImprovementBps,
      generatedAt: now(),
    };
  }
}

export const stakingService = new StakingService();
