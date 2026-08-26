import { StellarService } from './stellar.service';
import { redisCacheService } from './redisCache.service';
import * as analytics from './analytics/pool-performance';
import type {
  BenchmarkComparison,
  ChartSeries,
  PoolComparison,
  PoolPerformanceEvent,
  PoolPerformanceMetrics,
  PoolSnapshot,
  UtilizationHeatmapCell,
} from './analytics/pool-performance';

const CACHE_TTL_S = 60;

export type { PoolSnapshot, PoolPerformanceMetrics, PoolComparison };

export function resetForTests(): void {
  analytics.resetPoolPerformanceStore();
}

function cacheKey(kind: string, id: string): string {
  return redisCacheService.buildKey('pool', `${kind}:${id}`);
}

export async function capturePoolSnapshot(poolAddress: string): Promise<PoolSnapshot> {
  const stellarService = new StellarService();
  const now = Math.floor(Date.now() / 1000);
  const poolData = await stellarService.getPoolStateAt(poolAddress, now);
  const snapshot = analytics.snapshotFromPoolState(poolAddress, poolData as Record<string, number | string>);
  analytics.recordSnapshot(snapshot);
  await redisCacheService.set(cacheKey('snapshot', poolAddress), snapshot, CACHE_TTL_S);
  return snapshot;
}

export async function captureAllPoolSnapshots(): Promise<PoolSnapshot[]> {
  const stellarService = new StellarService();
  const pools = await stellarService.getAllPools();
  const snapshots: PoolSnapshot[] = [];
  for (const pool of pools) {
    snapshots.push(await capturePoolSnapshot(pool.address));
  }
  return snapshots;
}

export async function getPoolSnapshots(poolAddress: string, timeRange: string): Promise<PoolSnapshot[]> {
  const key = cacheKey('snapshots', `${poolAddress}:${timeRange}`);
  const cached = await redisCacheService.get<PoolSnapshot[]>(key);
  if (cached) return cached;

  let snapshots = analytics.snapshotsInPeriod(poolAddress, analyticsToPeriod(timeRange));
  if (snapshots.length === 0) {
    await capturePoolSnapshot(poolAddress);
    snapshots = analytics.snapshotsInPeriod(poolAddress, analyticsToPeriod(timeRange));
  }
  const { filled } = analytics.fillSnapshotGaps(snapshots);
  await redisCacheService.set(key, filled, CACHE_TTL_S);
  return filled;
}

function analyticsToPeriod(timeRange: string): '7d' | '30d' | '90d' | '1y' {
  if (timeRange === '7d' || timeRange === '30d' || timeRange === '90d' || timeRange === '1y') {
    return timeRange;
  }
  return '30d';
}

export async function getPoolMetrics(poolAddress: string, period: string): Promise<PoolPerformanceMetrics> {
  const key = cacheKey('metrics', `${poolAddress}:${period}`);
  const cached = await redisCacheService.get<PoolPerformanceMetrics>(key);
  if (cached) return cached;

  const existing = analytics.snapshotsInPeriod(poolAddress, analyticsToPeriod(period));
  if (existing.length === 0) {
    await capturePoolSnapshot(poolAddress);
  }
  const metrics = analytics.computeMetrics(poolAddress, period);
  await redisCacheService.set(key, metrics, CACHE_TTL_S);
  return metrics;
}

export async function comparePools(timeRange: string): Promise<PoolComparison[]> {
  const key = cacheKey('compare', timeRange);
  const cached = await redisCacheService.get<PoolComparison[]>(key);
  if (cached) return cached;

  const stellarService = new StellarService();
  const pools = await stellarService.getAllPools();
  const comparisons: PoolComparison[] = [];
  for (const pool of pools) {
    const snapshots = await getPoolSnapshots(pool.address, timeRange);
    comparisons.push({
      poolAddress: pool.address,
      poolName: pool.name ?? pool.address,
      currentApy: pool.depositApy,
      tvl: Number(pool.tvl),
      utilization: pool.utilizationRate,
      riskScore: analytics.computeRiskScore(snapshots),
      rank: 0,
    });
  }
  const ranked = analytics.rankPools(comparisons);
  await redisCacheService.set(key, ranked, CACHE_TTL_S);
  return ranked;
}

export async function exportPerformanceData(
  poolAddress: string,
  format: 'csv' | 'json'
): Promise<string | Record<string, unknown>> {
  const [snapshots, metrics, comparison] = await Promise.all([
    getPoolSnapshots(poolAddress, '30d'),
    getPoolMetrics(poolAddress, '30d'),
    comparePools('30d'),
  ]);
  if (format === 'csv') {
    return analytics.snapshotsToCsv(snapshots);
  }
  return {
    exportedAt: new Date().toISOString(),
    snapshots,
    metrics: [metrics],
    comparison,
  };
}

export async function getPerformanceSummary(): Promise<{
  totalPoolsTracked: number;
  avgGlobalApy: number;
  totalTvl: number;
}> {
  const stellarService = new StellarService();
  const [protocolStats, pools] = await Promise.all([
    stellarService.getProtocolStats(),
    stellarService.getAllPools(),
  ]);
  const avgGlobalApy =
    pools.length > 0 ? pools.reduce((sum, p) => sum + p.depositApy, 0) / pools.length : 0;
  return {
    totalPoolsTracked: pools.length,
    avgGlobalApy: Math.round(avgGlobalApy * 1_000_000) / 1_000_000,
    totalTvl: Number(protocolStats.tvl ?? 0),
  };
}

export async function getChartSeries(poolAddress: string, period: string): Promise<ChartSeries[]> {
  if (analytics.snapshotsInPeriod(poolAddress, analyticsToPeriod(period)).length === 0) {
    await capturePoolSnapshot(poolAddress);
  }
  return analytics.buildChartSeries(poolAddress, period);
}

export async function getUtilizationHeatmap(
  poolAddress: string,
  period: string
): Promise<UtilizationHeatmapCell[]> {
  if (analytics.snapshotsInPeriod(poolAddress, analyticsToPeriod(period)).length === 0) {
    await capturePoolSnapshot(poolAddress);
  }
  return analytics.buildUtilizationHeatmap(poolAddress, period);
}

export async function getBenchmarks(poolAddress: string, period: string): Promise<BenchmarkComparison> {
  if (analytics.snapshotsInPeriod(poolAddress, analyticsToPeriod(period)).length === 0) {
    await capturePoolSnapshot(poolAddress);
  }
  return analytics.benchmarkPool(poolAddress, period);
}

export function recordPerformanceEvent(
  poolAddress: string,
  type: 'liquidation' | 'bad_debt' | 'parameter_change',
  payload: Record<string, number | string> = {}
): PoolPerformanceEvent {
  return analytics.recordEvent(poolAddress, type, payload);
}

export function getPerformanceEvents(poolAddress?: string): PoolPerformanceEvent[] {
  return analytics.listEvents(poolAddress);
}
