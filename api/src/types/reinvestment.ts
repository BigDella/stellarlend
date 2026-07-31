export type ReinvestStrategy = 'same_pool' | 'best_apy' | 'weighted';

export type ReinvestSchedule = 'real_time' | 'daily' | 'weekly' | 'threshold';

export interface WeightedTarget {
  pool: string;
  weightBps: number;
}

export interface ReinvestmentPlan {
  id: string;
  userAddress: string;
  sourcePool: string;
  strategy: ReinvestStrategy;
  schedule: ReinvestSchedule;
  /** Minimum earned amount (in the pool's asset base units) required before a sweep executes. */
  thresholdAmount: string;
  /** Only populated when strategy === 'weighted'; weightBps values must sum to 10000. */
  weightedTargets: WeightedTarget[];
  paused: boolean;
  totalReinvested: string;
  totalSweeps: number;
  createdAt: string;
  updatedAt: string;
  lastSweptAt?: string;
}

export interface CreateReinvestmentPlanRequest {
  userAddress: string;
  sourcePool: string;
  strategy: ReinvestStrategy;
  schedule: ReinvestSchedule;
  thresholdAmount: string;
  weightedTargets?: WeightedTarget[];
}

export interface RecordSweepRequest {
  /** Resolved target pool for 'same_pool' / 'best_apy' strategies; ignored for 'weighted'. */
  targetPool?: string;
  earnedAmount: string;
  poolPaused: boolean;
  estimatedGasCost: string;
  txHash?: string;
}

export interface ReinvestmentEvent {
  planId: string;
  pool: string;
  amount: string;
  /** Cost basis recorded for this reinvestment lot, for external tax-lot accounting. */
  costBasis: string;
  sweptAt: string;
  txHash?: string;
}

export interface ReinvestmentAnalytics {
  planId: string;
  totalReinvested: string;
  totalSweeps: number;
  /** Projected value of reinvested amounts compounding at assumedApyBps since each sweep. */
  compoundedValue: string;
  /** Same amounts with no reinvestment (manual sweep, held idle). */
  manualValue: string;
  additionalYieldFromCompounding: string;
  assumedApyBps: number;
  byPool: Record<string, string>;
}
