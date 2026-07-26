import {
  StandardizedProtocolMetrics,
  ProtocolAdapter,
  DataQualityIssue,
  CrossProtocolComparisonResult,
  ProtocolMarketShare,
  LeaderboardEntry,
  LeaderboardMetric,
  AssetComparisonEntry,
  AssetComparisonResult,
  MarketShareHistoryPoint,
  PositioningMetricComparison,
  PositioningReport,
  BenchmarkScoreEntry,
  BenchmarkScoreResult,
  WeeklyDigest,
} from './types';
import { StellarLendAdapter } from './adapters/stellarLendAdapter';
import { DefiLlamaAdapter } from './adapters/defiLlamaAdapter';
import {
  PROTOCOL_FEE_PARAMETERS,
  PROTOCOL_LIQUIDATION_PARAMETERS,
  ProtocolFeeParameters,
  ProtocolLiquidationParameters,
} from './protocolParameters';
import { redisCacheService } from '../redisCache.service';
import logger from '../../utils/logger';

// Data refreshes on-demand behind a cache TTL rather than a standalone cron
// process (no job-scheduling infra exists elsewhere in this API service to
// hook into); callers wanting a literal daily refresh cadence can invoke
// `refreshCrossProtocolData` directly from an external scheduler.
const ETL_CACHE_TTL_S = 300;

const DEFAULT_ADAPTERS: ProtocolAdapter[] = [new StellarLendAdapter(), new DefiLlamaAdapter()];

// Sanity bounds for the data-quality gate: an APY outside [0, 500%] or a
// utilization outside [0, 1] indicates a malformed/corrupted upstream
// record rather than a real market condition, and is excluded from
// comparison output rather than silently skewing it.
const MAX_PLAUSIBLE_APY = 5;

function runDataQualityChecks(metrics: StandardizedProtocolMetrics[]): {
  clean: StandardizedProtocolMetrics[];
  issues: DataQualityIssue[];
} {
  const clean: StandardizedProtocolMetrics[] = [];
  const issues: DataQualityIssue[] = [];

  for (const metric of metrics) {
    const reasons: string[] = [];
    if (metric.supplyApy < 0 || metric.supplyApy > MAX_PLAUSIBLE_APY) {
      reasons.push(
        `supplyApy ${metric.supplyApy} outside plausible range [0, ${MAX_PLAUSIBLE_APY}]`
      );
    }
    if (metric.borrowApy < 0 || metric.borrowApy > MAX_PLAUSIBLE_APY) {
      reasons.push(
        `borrowApy ${metric.borrowApy} outside plausible range [0, ${MAX_PLAUSIBLE_APY}]`
      );
    }
    if (metric.tvlUsd < 0) {
      reasons.push(`negative tvlUsd ${metric.tvlUsd}`);
    }
    if (metric.utilizationRate < 0 || metric.utilizationRate > 1) {
      reasons.push(`utilizationRate ${metric.utilizationRate} outside [0, 1]`);
    }

    if (reasons.length > 0) {
      issues.push({ protocol: metric.protocol, asset: metric.asset, reason: reasons.join('; ') });
    } else {
      clean.push(metric);
    }
  }

  return { clean, issues };
}

/**
 * Runs every adapter, isolating failures so one broken/unreachable source
 * never blocks metrics from the rest, then applies the data-quality gate.
 */
export async function refreshCrossProtocolData(
  adapters: ProtocolAdapter[] = DEFAULT_ADAPTERS
): Promise<CrossProtocolComparisonResult> {
  const settled = await Promise.allSettled(adapters.map((adapter) => adapter.fetchMetrics()));

  const rawMetrics: StandardizedProtocolMetrics[] = [];
  const failedSources: string[] = [];

  settled.forEach((result, index) => {
    const adapter = adapters[index]!;
    if (result.status === 'fulfilled') {
      rawMetrics.push(...result.value);
    } else {
      failedSources.push(adapter.protocolId);
      logger.warn('Cross-protocol ETL adapter failed', {
        protocolId: adapter.protocolId,
        error: result.reason instanceof Error ? result.reason.message : result.reason,
      });
    }
  });

  const { clean, issues } = runDataQualityChecks(rawMetrics);

  return {
    metrics: clean,
    qualityIssues: issues,
    failedSources,
    refreshedAt: new Date().toISOString(),
  };
}

// In-memory market-share history — one point per *fresh* refresh (i.e. not
// served from cache), so the cadence tracks ETL_CACHE_TTL_S rather than
// every request. Bounded like `concentrationMonitor.service.ts`'s history.
const MAX_MARKET_SHARE_HISTORY_POINTS = 500;
const marketShareHistory: MarketShareHistoryPoint[] = [];

function appendMarketShareHistory(shares: ProtocolMarketShare[]): void {
  marketShareHistory.push({ timestamp: new Date().toISOString(), shares });
  if (marketShareHistory.length > MAX_MARKET_SHARE_HISTORY_POINTS) {
    marketShareHistory.splice(0, marketShareHistory.length - MAX_MARKET_SHARE_HISTORY_POINTS);
  }
}

export async function getCrossProtocolComparison(
  adapters?: ProtocolAdapter[]
): Promise<CrossProtocolComparisonResult> {
  const cacheKey = redisCacheService.buildKey('protocol', 'cross-protocol-comparison');
  const cached = await redisCacheService.get<CrossProtocolComparisonResult>(cacheKey);
  if (cached) return cached;

  const result = await refreshCrossProtocolData(adapters);
  await redisCacheService.set(cacheKey, result, ETL_CACHE_TTL_S);
  appendMarketShareHistory(computeMarketShare(result.metrics));
  return result;
}

/** TVL-weighted market share per protocol, sorted descending by TVL. */
export function computeMarketShare(metrics: StandardizedProtocolMetrics[]): ProtocolMarketShare[] {
  const totalsByProtocol = new Map<string, number>();
  for (const metric of metrics) {
    totalsByProtocol.set(
      metric.protocol,
      (totalsByProtocol.get(metric.protocol) ?? 0) + metric.tvlUsd
    );
  }

  const grandTotal = Array.from(totalsByProtocol.values()).reduce((sum, v) => sum + v, 0);

  return Array.from(totalsByProtocol.entries())
    .map(([protocol, tvlUsd]) => ({
      protocol,
      tvlUsd,
      marketSharePct: grandTotal > 0 ? (tvlUsd / grandTotal) * 100 : 0,
    }))
    .sort((a, b) => b.tvlUsd - a.tvlUsd);
}

export async function getMarketShare(adapters?: ProtocolAdapter[]): Promise<ProtocolMarketShare[]> {
  const { metrics } = await getCrossProtocolComparison(adapters);
  return computeMarketShare(metrics);
}

export async function getLeaderboard(
  metric: LeaderboardMetric = 'tvlUsd',
  limit = 20,
  adapters?: ProtocolAdapter[]
): Promise<LeaderboardEntry[]> {
  const { metrics } = await getCrossProtocolComparison(adapters);

  return [...metrics]
    .sort((a, b) => b[metric] - a[metric])
    .slice(0, limit)
    .map((m, index) => ({
      rank: index + 1,
      protocol: m.protocol,
      asset: m.asset,
      metric,
      metricValue: m[metric],
      tvlUsd: m.tvlUsd,
    }));
}

// ─── #482 extensions: per-asset comparison, fees/liquidation params, ──────────
// ─── market-share history, positioning report, benchmark score, digest ───────

interface ProtocolAggregate {
  protocol: string;
  displayName: string;
  avgSupplyApy: number;
  avgBorrowApy: number;
  totalTvlUsd: number;
  avgUtilizationRate: number;
  avgSpreadBps: number;
}

function aggregateByProtocol(metrics: StandardizedProtocolMetrics[]): ProtocolAggregate[] {
  const byProtocol = new Map<string, StandardizedProtocolMetrics[]>();
  for (const m of metrics) {
    const list = byProtocol.get(m.protocol) ?? [];
    list.push(m);
    byProtocol.set(m.protocol, list);
  }

  return Array.from(byProtocol.entries()).map(([protocol, rows]) => {
    const avg = (values: number[]) => values.reduce((s, v) => s + v, 0) / values.length;
    return {
      protocol,
      displayName: rows[0]!.displayName,
      avgSupplyApy: avg(rows.map((r) => r.supplyApy)),
      avgBorrowApy: avg(rows.map((r) => r.borrowApy)),
      totalTvlUsd: rows.reduce((s, r) => s + r.tvlUsd, 0),
      avgUtilizationRate: avg(rows.map((r) => r.utilizationRate)),
      avgSpreadBps: avg(rows.map((r) => (r.borrowApy - r.supplyApy) * 10000)),
    };
  });
}

/** Side-by-side comparison of every tracked protocol for a single asset (e.g. USDC, ETH, BTC). */
export async function getAssetComparison(
  asset: string,
  adapters?: ProtocolAdapter[]
): Promise<AssetComparisonResult> {
  const { metrics, refreshedAt } = await getCrossProtocolComparison(adapters);
  const normalizedAsset = asset.toUpperCase();

  const entries: AssetComparisonEntry[] = metrics
    .filter((m) => m.asset.toUpperCase() === normalizedAsset)
    .map((m) => ({
      protocol: m.protocol,
      displayName: m.displayName,
      supplyApy: m.supplyApy,
      borrowApy: m.borrowApy,
      spreadBps: Math.round((m.borrowApy - m.supplyApy) * 10000),
      tvlUsd: m.tvlUsd,
      utilizationRate: m.utilizationRate,
    }))
    .sort((a, b) => b.tvlUsd - a.tvlUsd);

  return { asset: normalizedAsset, entries, refreshedAt };
}

/** Reserve factor (config) + live-computed spread per protocol. */
export async function getFeeComparison(
  adapters?: ProtocolAdapter[]
): Promise<Array<ProtocolFeeParameters & { avgSpreadBps: number }>> {
  const { metrics } = await getCrossProtocolComparison(adapters);
  const aggregates = new Map(aggregateByProtocol(metrics).map((a) => [a.protocol, a]));

  return PROTOCOL_FEE_PARAMETERS.map((params) => ({
    ...params,
    avgSpreadBps: Math.round(aggregates.get(params.protocol)?.avgSpreadBps ?? 0),
  }));
}

/** Curated liquidation parameters (LT/bonus/close factor) per protocol. */
export function getLiquidationParamsComparison(): ProtocolLiquidationParameters[] {
  return PROTOCOL_LIQUIDATION_PARAMETERS;
}

export function getMarketShareHistory(limit?: number): MarketShareHistoryPoint[] {
  return limit ? marketShareHistory.slice(-limit) : [...marketShareHistory];
}

const POSITIONING_FAVORABLE_THRESHOLD_PCT = 5; // min relative delta to call out as a strength/weakness

/**
 * Compares StellarLend's own aggregate metrics against the average of every
 * other tracked protocol, surfacing strengths/weaknesses beyond a minimum
 * relative-difference threshold (differences smaller than that are noise,
 * not a real competitive signal).
 */
export async function getPositioningReport(adapters?: ProtocolAdapter[]): Promise<PositioningReport> {
  const { metrics, refreshedAt } = await getCrossProtocolComparison(adapters);
  const aggregates = aggregateByProtocol(metrics);
  const stellarLend = aggregates.find((a) => a.protocol === 'stellarlend');
  const peers = aggregates.filter((a) => a.protocol !== 'stellarlend');

  if (!stellarLend || peers.length === 0) {
    return { asOf: refreshedAt, strengths: [], weaknesses: [], metrics: [] };
  }

  const avg = (values: number[]) => values.reduce((s, v) => s + v, 0) / values.length;

  function compare(
    metric: string,
    stellarLendValue: number,
    peerValues: number[],
    higherIsFavorable: boolean
  ): PositioningMetricComparison {
    const peerAverage = avg(peerValues);
    const deltaVsPeerAverage = stellarLendValue - peerAverage;
    const relativeDeltaPct = peerAverage !== 0 ? (deltaVsPeerAverage / Math.abs(peerAverage)) * 100 : 0;
    const favorable = higherIsFavorable ? relativeDeltaPct > 0 : relativeDeltaPct < 0;
    return {
      metric,
      stellarLendValue,
      peerAverage,
      deltaVsPeerAverage,
      favorable: Math.abs(relativeDeltaPct) >= POSITIONING_FAVORABLE_THRESHOLD_PCT ? favorable : true,
    };
  }

  const comparisons: PositioningMetricComparison[] = [
    compare('supplyApy', stellarLend.avgSupplyApy, peers.map((p) => p.avgSupplyApy), true),
    compare('borrowApy', stellarLend.avgBorrowApy, peers.map((p) => p.avgBorrowApy), false),
    compare('tvlUsd', stellarLend.totalTvlUsd, peers.map((p) => p.totalTvlUsd), true),
  ];

  const strengths: string[] = [];
  const weaknesses: string[] = [];
  for (const c of comparisons) {
    const relativeDeltaPct =
      c.peerAverage !== 0 ? (c.deltaVsPeerAverage / Math.abs(c.peerAverage)) * 100 : 0;
    if (Math.abs(relativeDeltaPct) < POSITIONING_FAVORABLE_THRESHOLD_PCT) continue;
    const direction = c.deltaVsPeerAverage >= 0 ? 'above' : 'below';
    const message = `${c.metric} is ${Math.abs(relativeDeltaPct).toFixed(1)}% ${direction} the peer average`;
    if (c.favorable) strengths.push(message);
    else weaknesses.push(message);
  }

  return { asOf: refreshedAt, strengths, weaknesses, metrics: comparisons };
}

/**
 * 0-100 benchmark score per protocol, averaging its percentile rank across
 * supplyApy (higher is better), borrowApy (lower is better), and TVL
 * (higher is better). Requires at least 2 protocols to be meaningful.
 */
export async function getBenchmarkScore(adapters?: ProtocolAdapter[]): Promise<BenchmarkScoreResult> {
  const { metrics, refreshedAt } = await getCrossProtocolComparison(adapters);
  const aggregates = aggregateByProtocol(metrics);

  if (aggregates.length === 0) {
    return { entries: [], asOf: refreshedAt };
  }

  function percentileRank(values: number[], value: number, higherIsBetter: boolean): number {
    if (values.length <= 1) return 100;
    const better = values.filter((v) => (higherIsBetter ? v < value : v > value)).length;
    return (better / (values.length - 1)) * 100;
  }

  const supplyApys = aggregates.map((a) => a.avgSupplyApy);
  const borrowApys = aggregates.map((a) => a.avgBorrowApy);
  const tvls = aggregates.map((a) => a.totalTvlUsd);

  const entries: BenchmarkScoreEntry[] = aggregates
    .map((a) => {
      const supplyRank = percentileRank(supplyApys, a.avgSupplyApy, true);
      const borrowRank = percentileRank(borrowApys, a.avgBorrowApy, false);
      const tvlRank = percentileRank(tvls, a.totalTvlUsd, true);
      const score = Math.round((supplyRank + borrowRank + tvlRank) / 3);
      return { protocol: a.protocol, displayName: a.displayName, score, rank: 0 };
    })
    .sort((a, b) => b.score - a.score)
    .map((entry, index) => ({ ...entry, rank: index + 1 }));

  return { entries, asOf: refreshedAt };
}

const WEEKLY_DIGEST_CACHE_TTL_S = 7 * 24 * 60 * 60;
const SEVEN_DAYS_MS = 7 * 24 * 60 * 60 * 1000;

/**
 * On-demand weekly comparison digest, cached for 7 days. As noted in
 * `refreshCrossProtocolData`'s comment above, this API service has no
 * standalone job-scheduling infra to hook a literal weekly cron into —
 * computed lazily behind a week-long cache TTL instead, refreshing itself
 * the first time it's requested after the previous digest expires.
 */
export async function getWeeklyDigest(adapters?: ProtocolAdapter[]): Promise<WeeklyDigest> {
  const cacheKey = redisCacheService.buildKey('protocol', 'cross-protocol-weekly-digest');
  const cached = await redisCacheService.get<WeeklyDigest>(cacheKey);
  if (cached) return cached;

  const { metrics } = await getCrossProtocolComparison(adapters);
  const currentShares = computeMarketShare(metrics);
  const stellarLendShare = currentShares.find((s) => s.protocol === 'stellarlend');

  const cutoff = Date.now() - SEVEN_DAYS_MS;
  const weekAgoPoint = [...marketShareHistory].reverse().find((p) => new Date(p.timestamp).getTime() <= cutoff);
  const weekAgoStellarLendPct =
    weekAgoPoint?.shares.find((s) => s.protocol === 'stellarlend')?.marketSharePct ?? null;

  const currentPct = stellarLendShare?.marketSharePct ?? 0;
  const deltaPct = weekAgoStellarLendPct !== null ? currentPct - weekAgoStellarLendPct : 0;
  const rank = currentShares.findIndex((s) => s.protocol === 'stellarlend') + 1;

  const summary =
    weekAgoStellarLendPct !== null
      ? `StellarLend holds ${currentPct.toFixed(2)}% tracked market share (rank #${rank || currentShares.length}), ${deltaPct >= 0 ? 'up' : 'down'} ${Math.abs(deltaPct).toFixed(2)}pp vs. 7 days ago.`
      : `StellarLend holds ${currentPct.toFixed(2)}% tracked market share (rank #${rank || currentShares.length}). Insufficient history yet for a week-over-week comparison.`;

  const digest: WeeklyDigest = {
    generatedAt: new Date().toISOString(),
    stellarLendMarketSharePct: currentPct,
    stellarLendMarketShareDeltaPct: deltaPct,
    stellarLendRank: rank || currentShares.length,
    topMoversByTvl: currentShares.slice(0, 3).map((s) => ({ protocol: s.protocol, tvlUsd: s.tvlUsd })),
    summary,
  };

  await redisCacheService.set(cacheKey, digest, WEEKLY_DIGEST_CACHE_TTL_S);
  return digest;
}
