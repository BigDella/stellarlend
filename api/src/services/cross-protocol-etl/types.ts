/**
 * Standardized schema every protocol adapter normalizes into, so metrics
 * from different sources (StellarLend's own pools, DefiLlama-tracked peer
 * protocols, ...) can be compared apples-to-apples.
 */
export interface StandardizedProtocolMetrics {
  /** Stable machine-readable identifier, e.g. 'aave-v3', 'stellarlend'. */
  protocol: string;
  displayName: string;
  chain: string;
  /** Asset symbol, e.g. 'USDC'. */
  asset: string;
  /** Fraction, not a percentage: 0.032 == 3.2% APY. */
  supplyApy: number;
  /** Fraction, not a percentage. 0 when the source doesn't expose it. */
  borrowApy: number;
  tvlUsd: number;
  /** 0..1. 0 when the source doesn't expose it. */
  utilizationRate: number;
  fetchedAt: string;
  source: string;
}

/**
 * A single ingestion source in the ETL pipeline. Each adapter is
 * responsible for fetching from its upstream and normalizing into
 * `StandardizedProtocolMetrics` — the orchestrator (`etl.service.ts`)
 * treats every adapter identically and isolates failures per-adapter.
 */
export interface ProtocolAdapter {
  readonly protocolId: string;
  readonly displayName: string;
  fetchMetrics(): Promise<StandardizedProtocolMetrics[]>;
}

export interface DataQualityIssue {
  protocol: string;
  asset: string;
  reason: string;
}

export interface CrossProtocolComparisonResult {
  metrics: StandardizedProtocolMetrics[];
  qualityIssues: DataQualityIssue[];
  /** protocolIds of adapters that errored during this refresh. */
  failedSources: string[];
  refreshedAt: string;
}

export interface ProtocolMarketShare {
  protocol: string;
  tvlUsd: number;
  marketSharePct: number;
}

export type LeaderboardMetric = 'supplyApy' | 'borrowApy' | 'tvlUsd';

export interface LeaderboardEntry {
  rank: number;
  protocol: string;
  asset: string;
  metric: LeaderboardMetric;
  metricValue: number;
  tvlUsd: number;
}

/** Per-protocol metrics for a single asset, for side-by-side comparison. */
export interface AssetComparisonEntry {
  protocol: string;
  displayName: string;
  supplyApy: number;
  borrowApy: number;
  /** borrowApy - supplyApy, in basis points — the protocol's effective spread. */
  spreadBps: number;
  tvlUsd: number;
  utilizationRate: number;
}

export interface AssetComparisonResult {
  asset: string;
  entries: AssetComparisonEntry[];
  refreshedAt: string;
}

export interface MarketShareHistoryPoint {
  timestamp: string;
  shares: ProtocolMarketShare[];
}

export interface PositioningMetricComparison {
  metric: string;
  stellarLendValue: number;
  peerAverage: number;
  /** Positive = StellarLend ahead of the peer average for this metric. */
  deltaVsPeerAverage: number;
  favorable: boolean;
}

export interface PositioningReport {
  asOf: string;
  strengths: string[];
  weaknesses: string[];
  metrics: PositioningMetricComparison[];
}

export interface BenchmarkScoreEntry {
  protocol: string;
  displayName: string;
  /** 0-100, higher is better; percentile rank averaged across tracked metrics. */
  score: number;
  rank: number;
}

export interface BenchmarkScoreResult {
  entries: BenchmarkScoreEntry[];
  asOf: string;
}

export interface WeeklyDigest {
  generatedAt: string;
  stellarLendMarketSharePct: number;
  stellarLendMarketShareDeltaPct: number;
  stellarLendRank: number;
  topMoversByTvl: Array<{ protocol: string; tvlUsd: number }>;
  summary: string;
}
