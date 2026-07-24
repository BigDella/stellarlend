export interface HistoricalRatePoint {
  timestamp: string;
  depositApy: number;
  borrowApy: number;
  utilizationRate: number;
  poolAddress?: string;
}

export interface PoolUtilizationPoint {
  timestamp: string;
  utilizationRate: number;
  totalDeposits: string;
  totalBorrows: string;
  poolAddress: string;
}

export interface RateComparison {
  poolAddress: string;
  poolName?: string;
  depositApy: number;
  borrowApy: number;
  utilizationRate: number;
  tvl: string;
}

export interface ProtocolRevenuePoint {
  timestamp: string;
  cumulativeRevenue: string;
  periodRevenue: string;
  revenueSource: 'interest' | 'fees' | 'liquidation';
}

export interface AnalyticsSummary {
  totalPools: number;
  averageDepositApy: number;
  averageBorrowApy: number;
  averageUtilizationRate: number;
  totalValueLocked: string;
  cumulativeRevenue: string;
  activeUsers: number;
  snapshotTimestamp: string;
}

export interface AnalyticsQuery {
  timeRange: '1d' | '7d' | '30d' | '1y';
  poolAddress?: string;
  limit?: number;
  cursor?: string;
}

export interface AnalyticsExportData {
  exportedAt: string;
  timeRange: string;
  historicalRates: HistoricalRatePoint[];
  poolUtilization: PoolUtilizationPoint[];
  rateComparison: RateComparison[];
  revenue: ProtocolRevenuePoint[];
  summary: AnalyticsSummary;
}

// WebSocket analytics message types
export interface WsAnalyticsMessage {
  type: 'analytics_update';
  channel: 'apy' | 'utilization' | 'revenue';
  data: HistoricalRatePoint | PoolUtilizationPoint | ProtocolRevenuePoint;
  timestamp: number;
}

export type RateGranularity = 'hourly' | 'daily' | 'weekly' | 'monthly';

export interface RateVolatilityPoint {
  timestamp: string;
  depositApyStdDev: number;
  borrowApyStdDev: number;
  windowSize: number;
  poolAddress?: string;
}

export interface WeightedAverageRatePoint {
  periodStart: string;
  periodEnd: string;
  granularity: RateGranularity;
  weightedAvgDepositApy: number;
  weightedAvgBorrowApy: number;
  sampleCount: number;
  poolAddress?: string;
}

export interface RateChangeEvent {
  timestamp: string;
  poolAddress?: string;
  previousBorrowApy: number;
  newBorrowApy: number;
  deltaBps: number;
  changeType: 'increase' | 'decrease';
  /**
   * Correlates the rate change with the governance action that caused it
   * (e.g. a parameter-store update). `undefined` when the change was
   * derived purely from recorded rate snapshots without a matching
   * governance event in range — see `analytics.service.ts`.
   */
  governanceActionId?: string;
}

export interface RateHistoryQuery {
  asset?: string;
  from?: string;
  to?: string;
  granularity?: RateGranularity;
}
