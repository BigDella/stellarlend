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

// ─── Real-time Dashboard types  (Issue #795) ─────────────────────────────────

/** A single protocol-wide real-time metrics snapshot. */
export interface ProtocolMetricsSnapshot {
  totalValueLocked: string;
  totalDeposits: string;
  totalBorrows: string;
  utilizationRateBps: number;
  averageBorrowRateBps: number;
  totalUsers: number;
  totalTransactions: number;
  timestamp: string;
}

/** Per-user metrics for the dashboard. */
export interface UserMetricsDashboard {
  userAddress: string;
  collateral: string;
  debt: string;
  healthFactor: number;
  totalDeposits: string;
  totalBorrows: string;
  totalWithdrawals: string;
  totalRepayments: string;
  activityScore: number;
  riskLevel: number;
  transactionCount: number;
}

/** A single entry in the real-time activity feed. */
export interface ActivityFeedEntry {
  userAddress: string;
  activityType: string;
  amount: string;
  asset: string | null;
  timestamp: string;
}

/** Historical metrics snapshot for trend charts. */
export interface MetricsHistoryPoint {
  timestamp: string;
  totalValueLocked: string;
  utilizationRateBps: number;
  averageBorrowRateBps: number;
}

/** TVL forecast data point. */
export interface TvlForecastPoint {
  periodIndex: number;
  forecastedTvl: string;
  periodsAhead: number;
}

/** A configured metric alert threshold. */
export interface MetricAlertConfig {
  metric: 'tvl' | 'utilization' | 'avg_rate';
  threshold: string;
}

/** A triggered alert record. */
export interface TriggeredAlertRecord {
  metric: string;
  value: string;
  threshold: string;
  timestamp: string;
}

/** Collateral ratio snapshot for an asset. */
export interface CollateralRatioSnapshot {
  asset: string;
  currentRatioBps: number;
  requiredRatioBps: number;
  healthFactor: number;
  riskLevel: 'safe' | 'warning' | 'danger' | 'critical';
  collateralValue: string;
  debtValue: string;
  timestamp: string;
}

/** Dashboard aggregated view — all panels in one response. */
export interface DashboardView {
  protocol: ProtocolMetricsSnapshot;
  activityFeed: ActivityFeedEntry[];
  collateralRatios: CollateralRatioSnapshot[];
  alerts: TriggeredAlertRecord[];
  generatedAt: string;
}
