export interface StakingPosition {
  userAddress: string;
  stakedAmount: string;
  lockupDays: number;
  lockupEndTime: string;
  votingPower: string;
  delegatedTo?: string;
  delegatedFrom: string[];
  earnedRewards: string;
  createdAt: string;
  updatedAt: string;
}

export interface StakeRequest {
  userAddress: string;
  amount: string;
  lockupDays?: number;
}

export interface UnstakeRequest {
  userAddress: string;
  amount: string;
}

export interface DelegateRequest {
  userAddress: string;
  delegateTo: string;
}

export interface RevokeDelegationRequest {
  userAddress: string;
}

export interface ClaimRewardsRequest {
  userAddress: string;
}

export interface StakingRewardConfig {
  baseAprBps: number;
  lockupBonusBps: number;
  epochDurationSeconds: number;
}

// ─── Yield Farming Strategy Optimizer Types ───────────────────────────────────

/** Risk profile for a yield farming strategy */
export type YieldStrategyRisk = 'conservative' | 'balanced' | 'aggressive';

/** Objective the optimizer should prioritise */
export type YieldStrategyObjective = 'maximize_apy' | 'minimize_il' | 'balanced';

/** Compounding cadence */
export type CompoundingInterval = 'hourly' | 'daily' | 'weekly' | 'manual';

/** Per-pool allocation within a strategy */
export interface PoolAllocation {
  poolAddress: string;
  allocationBps: number; // basis points, e.g. 5000 = 50 %
  estimatedApy: number;  // annualised %, e.g. 12.5
  currentUtilizationBps: number;
}

/** A complete yield farming strategy */
export interface YieldFarmingStrategy {
  strategyId: string;
  userAddress: string;
  name: string;
  objective: YieldStrategyObjective;
  riskTier: YieldStrategyRisk;
  compoundingInterval: CompoundingInterval;
  pools: PoolAllocation[];
  totalAllocatedBps: number;    // must sum to 10 000
  estimatedBlendedApy: number;  // weighted average APY
  autoCompoundEnabled: boolean;
  active: boolean;
  createdAt: string;
  updatedAt: string;
  lastCompoundedAt?: string;
  nextCompoundAt?: string;
}

/** Request body for creating / updating a yield farming strategy */
export interface CreateYieldStrategyRequest {
  userAddress: string;
  name: string;
  objective: YieldStrategyObjective;
  riskTier: YieldStrategyRisk;
  compoundingInterval: CompoundingInterval;
  pools: Omit<PoolAllocation, 'currentUtilizationBps'>[];
  autoCompoundEnabled?: boolean;
}

/** Request body for activating / deactivating a strategy */
export interface ActivateYieldStrategyRequest {
  userAddress: string;
  strategyId: string;
}

/** Request body for a manual compound trigger */
export interface CompoundStrategyRequest {
  userAddress: string;
  strategyId: string;
}

/** A single performance snapshot for a strategy */
export interface StrategyPerformanceSnapshot {
  timestamp: string;
  totalValueLocked: string;
  yieldEarned: string;
  compoundedAmount: string;
  apy: number;
  ilImpactBps: number;
}

/** Aggregated performance for a strategy */
export interface YieldStrategyPerformance {
  strategyId: string;
  userAddress: string;
  totalYieldEarned: string;
  totalCompounded: string;
  compoundCount: number;
  realizedApy: number;
  averageIlImpactBps: number;
  snapshots: StrategyPerformanceSnapshot[];
}

/** Optimizer recommendation for a set of pools */
export interface YieldOptimizationRecommendation {
  pools: Array<{
    poolAddress: string;
    currentUtilizationBps: number;
    recommendedAllocationBps: number;
    action: 'increase' | 'decrease' | 'no_change';
    adjustmentAmount: number;
  }>;
  totalCapitalEfficiencyBps: number;
  estimatedYieldImprovementBps: number;
  generatedAt: string;
}
