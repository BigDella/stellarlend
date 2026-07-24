import {
  StandardizedProtocolMetrics,
  ProtocolAdapter,
  DataQualityIssue,
  CrossProtocolComparisonResult,
  ProtocolMarketShare,
  LeaderboardEntry,
  LeaderboardMetric,
} from './types';
import { StellarLendAdapter } from './adapters/stellarLendAdapter';
import { DefiLlamaAdapter } from './adapters/defiLlamaAdapter';
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

export async function getCrossProtocolComparison(
  adapters?: ProtocolAdapter[]
): Promise<CrossProtocolComparisonResult> {
  const cacheKey = redisCacheService.buildKey('protocol', 'cross-protocol-comparison');
  const cached = await redisCacheService.get<CrossProtocolComparisonResult>(cacheKey);
  if (cached) return cached;

  const result = await refreshCrossProtocolData(adapters);
  await redisCacheService.set(cacheKey, result, ETL_CACHE_TTL_S);
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
