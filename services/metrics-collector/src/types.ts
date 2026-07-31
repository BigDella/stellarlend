export type ProtocolMetricName =
  | 'tvl'
  | 'totalBorrows'
  | 'utilizationRate'
  | 'liquidations'
  | 'totalDeposits'
  | 'activeUsers';

export interface ProtocolMetricSample {
  time: Date;
  tvl: number;
  totalBorrows: number;
  utilizationRate: number;
  liquidations: number;
  totalDeposits: number;
  activeUsers: number;
}

export interface AssetMetricSample {
  time: Date;
  asset: string;
  supply: number;
  borrow: number;
  availableLiquidity: number;
  price: number | null;
  volatility: number | null;
  apy: number | null;
}

export interface MetricsGap {
  metricFamily: 'protocol' | 'asset';
  gapStart: Date;
  gapEnd: Date;
}

export interface TimeSeriesPoint {
  time: string;
  value: number;
}

export interface TimeSeriesQuery {
  metric: ProtocolMetricName;
  from: Date;
  to: Date;
  interval: '1m' | '5m' | '1h' | '1d';
  asset?: string;
}

export interface MetricsCollectorConfig {
  protocolStatsUrl: string;
  databaseUrl: string;
  collectIntervalMs: number;
  rawRetentionDays: number;
  aggregatedRetentionDays: number;
  logLevel: 'debug' | 'info' | 'warn' | 'error';
}

export const PROTOCOL_METRIC_COLUMNS: Record<ProtocolMetricName, string> = {
  tvl: 'tvl',
  totalBorrows: 'total_borrows',
  utilizationRate: 'utilization_rate',
  liquidations: 'liquidations',
  totalDeposits: 'total_deposits',
  activeUsers: 'active_users',
};
