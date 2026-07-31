/**
 * Gas Cost Estimation Types
 * 
 * Provides comprehensive gas cost estimation for all lending operations
 * with historical data, optimization suggestions, and accuracy tracking.
 */

export type GasOperation = 
  | 'deposit' 
  | 'withdraw' 
  | 'borrow' 
  | 'repay' 
  | 'liquidation' 
  | 'flash_loan';

export interface GasCostBreakdown {
  /** Base transaction cost in stroops */
  baseCost: string;
  /** Storage write operations cost */
  storageCost: string;
  /** Cross-contract call costs */
  crossContractCost: string;
  /** Total estimated cost */
  totalCost: string;
  /** CPU instructions consumed */
  cpuInstructions: string;
  /** Memory bytes consumed */
  memoryBytes: string;
  /** Minimum resource fee in stroops */
  minResourceFee: string;
}

export interface GasOptimizationSuggestion {
  type: 'timing' | 'batching' | 'method';
  title: string;
  description: string;
  /** Potential savings in stroops */
  potentialSavings: string;
  /** Priority level: high, medium, low */
  priority: 'high' | 'medium' | 'low';
}

export interface HistoricalGasData {
  operation: GasOperation;
  /** Average cost in stroops over time period */
  averageCost: string;
  /** Minimum cost observed */
  minCost: string;
  /** Maximum cost observed */
  maxCost: string;
  /** Standard deviation */
  stdDeviation: string;
  /** Number of samples */
  sampleCount: number;
  /** Time period covered (e.g., "24h", "7d", "30d") */
  period: string;
}

export interface GasCostEstimate {
  operation: GasOperation;
  /** Estimated cost breakdown */
  breakdown: GasCostBreakdown;
  /** Optimization suggestions */
  optimizations: GasOptimizationSuggestion[];
  /** Historical data for this operation */
  historical?: HistoricalGasData;
  /** Estimated USD cost (if price data available) */
  estimatedUsdCost?: string;
  /** Timestamp of estimation */
  timestamp: string;
  /** Confidence level: high, medium, low */
  confidence: 'high' | 'medium' | 'low';
}

export interface GasEstimateRequest {
  operation: GasOperation;
  userAddress: string;
  assetAddress?: string;
  amount: string;
  /** Include optimization suggestions */
  includeOptimizations?: boolean;
  /** Include historical data */
  includeHistorical?: boolean;
}

export interface GasComparisonItem {
  operation: GasOperation;
  averageCost: string;
  /** Relative cost: 1 = baseline */
  relativeCost: number;
  /** Rank from cheapest to most expensive */
  rank: number;
}

export interface GasComparisonResponse {
  /** Operations sorted by cost (cheapest first) */
  operations: GasComparisonItem[];
  /** Reference operation for relative cost */
  referenceOperation: GasOperation;
  /** Timestamp of comparison */
  timestamp: string;
}

export interface GasCostAlert {
  id: string;
  operation: GasOperation;
  /** Threshold in stroops */
  threshold: string;
  /** Actual cost */
  actualCost: string;
  /** Percentage over threshold */
  overagePercent: number;
  timestamp: string;
  userAddress?: string;
}

export interface GasAlertConfig {
  /** User address (optional, for user-specific alerts) */
  userAddress?: string;
  /** Operation to monitor */
  operation: GasOperation;
  /** Threshold in stroops */
  threshold: string;
  /** Enable/disable alert */
  enabled: boolean;
}

export interface GasAccuracyMetric {
  operation: GasOperation;
  /** Estimated cost */
  estimated: string;
  /** Actual cost from transaction */
  actual: string;
  /** Difference in stroops */
  difference: string;
  /** Percentage error */
  errorPercent: number;
  /** Transaction hash for reference */
  txHash: string;
  timestamp: string;
}

export interface GasAccuracyReport {
  /** Overall accuracy metrics */
  overall: {
    meanAbsoluteError: string;
    meanPercentageError: number;
    /** Percentage of estimates within 10% of actual */
    accuracyWithin10Percent: number;
  };
  /** Per-operation accuracy */
  byOperation: Record<GasOperation, {
    meanAbsoluteError: string;
    meanPercentageError: number;
    sampleCount: number;
  }>;
  /** Time period covered */
  period: string;
  timestamp: string;
}

export interface HistoricalGasChartData {
  operation: GasOperation;
  dataPoints: {
    timestamp: string;
    averageCost: string;
    minCost: string;
    maxCost: string;
    sampleCount: number;
  }[];
  /** Chart period (e.g., "24h", "7d", "30d") */
  period: string;
}

export interface GasTimingRecommendation {
  /** Recommended execution time (ISO string) */
  recommendedTime: string;
  /** Expected cost at recommended time */
  expectedCost: string;
  /** Current cost */
  currentCost: string;
  /** Potential savings */
  savings: string;
  /** Reason for recommendation */
  reason: string;
}

export interface BatchGasEstimate {
  /** Individual operations in batch */
  operations: GasEstimateRequest[];
  /** Total estimated cost if executed individually */
  individualTotalCost: string;
  /** Total estimated cost if batched */
  batchedTotalCost: string;
  /** Savings from batching */
  batchSavings: string;
  /** Savings percentage */
  batchSavingsPercent: number;
  /** Recommended batch strategy */
  recommendation: string;
}
