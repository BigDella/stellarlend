/**
 * Gas Usage Analytics Types — Issue #483
 *
 * Tracks observed (post-execution) gas usage per contract function, distinct
 * from the pre-transaction cost estimation in `types/gas.ts`.
 */

export interface GasUsageSample {
  functionName: string;
  /** Observed gas usage (resource units / stroops) for this call. */
  gasUsed: number;
  /** Transaction calldata size in bytes, when known. */
  calldataSize?: number;
  /** Epoch milliseconds. */
  timestamp: number;
  txHash?: string;
}

export interface FunctionGasStats {
  functionName: string;
  sampleCount: number;
  average: number;
  median: number;
  p95: number;
  p99: number;
  min: number;
  max: number;
  stdDeviation: number;
  period: string;
}

export interface GasAnomaly {
  functionName: string;
  gasUsed: number;
  timestamp: number;
  txHash?: string;
  deviations: number;
  mean: number;
  stdDeviation: number;
}

export interface GasTrendPoint {
  timestamp: string;
  average: number;
  sampleCount: number;
}

export interface GasUsageTrend {
  functionName: string;
  granularity: 'daily' | 'weekly';
  points: GasTrendPoint[];
}

export interface FunctionComparison {
  functionA: FunctionGasStats;
  functionB: FunctionGasStats;
  /** (functionB.average - functionA.average) / functionA.average * 100 */
  averageDeltaPct: number;
}

export interface CalldataCorrelation {
  functionName: string;
  sampleCount: number;
  /** Pearson correlation coefficient between calldata size and gas used, in [-1, 1]. 0 when insufficient data. */
  correlationCoefficient: number;
}

export interface FunctionGasReport {
  functionName: string;
  stats: FunctionGasStats;
  trend: GasUsageTrend;
  anomalies: GasAnomaly[];
  calldataCorrelation: CalldataCorrelation;
  recommendation: string;
}
