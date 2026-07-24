import {
  HistoricalRatePoint,
  PoolUtilizationPoint,
  RateComparison,
  ProtocolRevenuePoint,
  AnalyticsSummary,
  AnalyticsQuery,
  AnalyticsExportData,
  RateVolatilityPoint,
  WeightedAverageRatePoint,
  RateChangeEvent,
  RateHistoryQuery,
  RateGranularity,
} from '../types/analytics';
import { StellarService } from './stellar.service';
import { redisCacheService } from './redisCache.service';
import { config } from '../config';
import { ValidationError } from '../utils/errors';

const ANALYTICS_CACHE_TTL_S = 60;
const DEFAULT_VOLATILITY_WINDOW = 10;
const DEFAULT_RATE_CHANGE_THRESHOLD_BPS = 10;
const MAX_RATE_HISTORY_BUCKETS = 1000;

const GRANULARITY_MS: Record<RateGranularity, number> = {
  hourly: 3_600_000,
  daily: 86_400_000,
  weekly: 604_800_000,
  monthly: 2_592_000_000,
};

function standardDeviation(values: number[]): number {
  if (values.length === 0) return 0;
  const mean = values.reduce((sum, v) => sum + v, 0) / values.length;
  const variance = values.reduce((sum, v) => sum + (v - mean) ** 2, 0) / values.length;
  return Math.sqrt(variance);
}

function generateTimePoints(timeRange: string, count: number): number[] {
  const now = Date.now();
  const ranges: Record<string, number> = {
    '1d': 86400000,
    '7d': 604800000,
    '30d': 2592000000,
    '1y': 31536000000,
  };
  const range = ranges[timeRange] || 86400000;
  const interval = range / count;
  return Array.from({ length: count }, (_, i) => now - range + interval * i);
}

export async function getHistoricalRates(query: AnalyticsQuery): Promise<HistoricalRatePoint[]> {
  const cacheKey = redisCacheService.buildKey(
    'protocol',
    `historical-rates:${query.timeRange}:${query.poolAddress || 'all'}`
  );

  const cached = await redisCacheService.get<HistoricalRatePoint[]>(cacheKey);
  if (cached) return cached;

  const stellarService = new StellarService();
  const timePoints = generateTimePoints(query.timeRange, 100);

  const rates: HistoricalRatePoint[] = await Promise.all(
    timePoints.map(async (timestamp) => {
      const rateData = await stellarService.getPoolRateAt(
        query.poolAddress || '',
        Math.floor(timestamp / 1000)
      );
      return {
        timestamp: new Date(timestamp).toISOString(),
        depositApy: rateData.depositApy,
        borrowApy: rateData.borrowApy,
        utilizationRate: rateData.utilizationRate,
        poolAddress: query.poolAddress,
      };
    })
  );

  await redisCacheService.set(cacheKey, rates, ANALYTICS_CACHE_TTL_S);
  return rates;
}

export async function getPoolUtilization(query: AnalyticsQuery): Promise<PoolUtilizationPoint[]> {
  const cacheKey = redisCacheService.buildKey(
    'pool',
    `utilization:${query.timeRange}:${query.poolAddress || 'all'}`
  );

  const cached = await redisCacheService.get<PoolUtilizationPoint[]>(cacheKey);
  if (cached) return cached;

  const stellarService = new StellarService();
  const timePoints = generateTimePoints(query.timeRange, 100);

  const utilization: PoolUtilizationPoint[] = await Promise.all(
    timePoints.map(async (timestamp) => {
      const poolData = await stellarService.getPoolStateAt(
        query.poolAddress || '',
        Math.floor(timestamp / 1000)
      );
      return {
        timestamp: new Date(timestamp).toISOString(),
        utilizationRate: poolData.utilizationRate,
        totalDeposits: poolData.totalDeposits,
        totalBorrows: poolData.totalBorrows,
        poolAddress: query.poolAddress || 'all',
      };
    })
  );

  await redisCacheService.set(cacheKey, utilization, ANALYTICS_CACHE_TTL_S);
  return utilization;
}

export async function getRateComparison(): Promise<RateComparison[]> {
  const cacheKey = redisCacheService.buildKey('protocol', 'rate-comparison');
  const cached = await redisCacheService.get<RateComparison[]>(cacheKey);
  if (cached) return cached;

  const stellarService = new StellarService();
  const pools = await stellarService.getAllPools();

  const comparisons: RateComparison[] = pools.map((pool) => ({
    poolAddress: pool.address,
    poolName: pool.name,
    depositApy: pool.depositApy,
    borrowApy: pool.borrowApy,
    utilizationRate: pool.utilizationRate,
    tvl: pool.tvl,
  }));

  await redisCacheService.set(cacheKey, comparisons, ANALYTICS_CACHE_TTL_S);
  return comparisons;
}

export async function getProtocolRevenue(query: AnalyticsQuery): Promise<ProtocolRevenuePoint[]> {
  const cacheKey = redisCacheService.buildKey('protocol', `revenue:${query.timeRange}`);

  const cached = await redisCacheService.get<ProtocolRevenuePoint[]>(cacheKey);
  if (cached) return cached;

  const stellarService = new StellarService();
  const timePoints = generateTimePoints(query.timeRange, 100);

  const revenue: ProtocolRevenuePoint[] = await Promise.all(
    timePoints.map(async (timestamp) => {
      const revData = await stellarService.getProtocolRevenueAt(Math.floor(timestamp / 1000));
      return {
        timestamp: new Date(timestamp).toISOString(),
        cumulativeRevenue: revData.cumulativeRevenue,
        periodRevenue: revData.periodRevenue,
        revenueSource: 'interest',
      };
    })
  );

  await redisCacheService.set(cacheKey, revenue, ANALYTICS_CACHE_TTL_S);
  return revenue;
}

export async function getAnalyticsSummary(): Promise<AnalyticsSummary> {
  const cacheKey = redisCacheService.buildKey('protocol', 'analytics-summary');
  const cached = await redisCacheService.get<AnalyticsSummary>(cacheKey);
  if (cached) return cached;

  const stellarService = new StellarService();
  const [protocolStats, pools] = await Promise.all([
    stellarService.getProtocolStats(),
    stellarService.getAllPools(),
  ]);

  const avgDepositApy =
    pools.length > 0 ? pools.reduce((sum, p) => sum + p.depositApy, 0) / pools.length : 0;
  const avgBorrowApy =
    pools.length > 0 ? pools.reduce((sum, p) => sum + p.borrowApy, 0) / pools.length : 0;
  const avgUtilization =
    pools.length > 0 ? pools.reduce((sum, p) => sum + p.utilizationRate, 0) / pools.length : 0;

  const summary: AnalyticsSummary = {
    totalPools: pools.length,
    averageDepositApy: avgDepositApy,
    averageBorrowApy: avgBorrowApy,
    averageUtilizationRate: avgUtilization,
    totalValueLocked: protocolStats.tvl,
    cumulativeRevenue: protocolStats.totalBorrows,
    activeUsers: protocolStats.numberOfUsers,
    snapshotTimestamp: new Date().toISOString(),
  };

  await redisCacheService.set(cacheKey, summary, ANALYTICS_CACHE_TTL_S);
  return summary;
}

export async function exportAnalytics(
  query: AnalyticsQuery,
  format: 'csv' | 'json'
): Promise<AnalyticsExportData | string> {
  const [historicalRates, poolUtilization, rateComparison, revenue, summary] = await Promise.all([
    getHistoricalRates(query),
    getPoolUtilization(query),
    getRateComparison(),
    getProtocolRevenue(query),
    getAnalyticsSummary(),
  ]);

  const data: AnalyticsExportData = {
    exportedAt: new Date().toISOString(),
    timeRange: query.timeRange,
    historicalRates,
    poolUtilization,
    rateComparison,
    revenue,
    summary,
  };

  if (format === 'csv') {
    return toAnalyticsCSV(data);
  }

  return data;
}

function toAnalyticsCSV(data: AnalyticsExportData): string {
  const header = 'timestamp,depositApy,borrowApy,utilizationRate,poolAddress,cumulativeRevenue';
  const rows = data.historicalRates.map(
    (r) =>
      `${r.timestamp},${r.depositApy},${r.borrowApy},${r.utilizationRate},${r.poolAddress || ''},`
  );
  const revenueRows = data.revenue.map((r) => `${r.timestamp},,,,,"${r.cumulativeRevenue}"`);
  return [header, ...rows, ...revenueRows].join('\n');
}

/**
 * Rate volatility: rolling standard deviation of deposit/borrow APY over a
 * fixed-size sliding window of historical rate snapshots.
 */
export async function getRateVolatility(
  query: AnalyticsQuery,
  windowSize: number = DEFAULT_VOLATILITY_WINDOW
): Promise<RateVolatilityPoint[]> {
  const cacheKey = redisCacheService.buildKey(
    'protocol',
    `rate-volatility:${query.timeRange}:${query.poolAddress || 'all'}:${windowSize}`
  );
  const cached = await redisCacheService.get<RateVolatilityPoint[]>(cacheKey);
  if (cached) return cached;

  const rates = await getHistoricalRates(query);
  const volatility: RateVolatilityPoint[] = [];
  for (let i = windowSize - 1; i < rates.length; i++) {
    const window = rates.slice(i - windowSize + 1, i + 1);
    volatility.push({
      timestamp: rates[i]!.timestamp,
      depositApyStdDev: standardDeviation(window.map((r) => r.depositApy)),
      borrowApyStdDev: standardDeviation(window.map((r) => r.borrowApy)),
      windowSize,
      poolAddress: query.poolAddress,
    });
  }

  await redisCacheService.set(cacheKey, volatility, ANALYTICS_CACHE_TTL_S);
  return volatility;
}

/**
 * Weighted average deposit/borrow APY bucketed by day/week/month.
 *
 * Historical rate snapshots are evenly spaced in time (see
 * `generateTimePoints`), so the time-weighted average within a bucket
 * reduces to the arithmetic mean of the samples that fall in it.
 */
export async function getWeightedAverageRates(
  query: AnalyticsQuery,
  granularity: RateGranularity = 'daily'
): Promise<WeightedAverageRatePoint[]> {
  const cacheKey = redisCacheService.buildKey(
    'protocol',
    `rate-weighted-avg:${query.timeRange}:${query.poolAddress || 'all'}:${granularity}`
  );
  const cached = await redisCacheService.get<WeightedAverageRatePoint[]>(cacheKey);
  if (cached) return cached;

  const rates = await getHistoricalRates(query);
  const bucketMs = GRANULARITY_MS[granularity];
  const buckets = new Map<number, HistoricalRatePoint[]>();

  for (const point of rates) {
    const bucketStart = Math.floor(new Date(point.timestamp).getTime() / bucketMs) * bucketMs;
    const bucket = buckets.get(bucketStart);
    if (bucket) {
      bucket.push(point);
    } else {
      buckets.set(bucketStart, [point]);
    }
  }

  const result: WeightedAverageRatePoint[] = Array.from(buckets.entries())
    .sort(([a], [b]) => a - b)
    .map(([bucketStart, points]) => {
      const n = points.length;
      return {
        periodStart: new Date(bucketStart).toISOString(),
        periodEnd: new Date(bucketStart + bucketMs).toISOString(),
        granularity,
        weightedAvgDepositApy: points.reduce((s, p) => s + p.depositApy, 0) / n,
        weightedAvgBorrowApy: points.reduce((s, p) => s + p.borrowApy, 0) / n,
        sampleCount: n,
        poolAddress: query.poolAddress,
      };
    });

  await redisCacheService.set(cacheKey, result, ANALYTICS_CACHE_TTL_S);
  return result;
}

/**
 * Detects material borrow-rate changes between consecutive historical rate
 * snapshots (>= `thresholdBps` move). Each event carries an optional
 * `governanceActionId` for traceability back to the parameter change that
 * caused it; this simulated analytics layer (see `StellarService`'s
 * `getPoolRateAt`) has no live governance-event feed to correlate against,
 * so the field is left `undefined` here rather than fabricated.
 */
export async function getRateChangeEvents(
  query: AnalyticsQuery,
  thresholdBps: number = DEFAULT_RATE_CHANGE_THRESHOLD_BPS
): Promise<RateChangeEvent[]> {
  const cacheKey = redisCacheService.buildKey(
    'protocol',
    `rate-change-events:${query.timeRange}:${query.poolAddress || 'all'}:${thresholdBps}`
  );
  const cached = await redisCacheService.get<RateChangeEvent[]>(cacheKey);
  if (cached) return cached;

  const rates = await getHistoricalRates(query);
  const events: RateChangeEvent[] = [];

  for (let i = 1; i < rates.length; i++) {
    const previous = rates[i - 1]!.borrowApy;
    const current = rates[i]!.borrowApy;
    const deltaBps = Math.round((current - previous) * 10_000);
    if (Math.abs(deltaBps) >= thresholdBps) {
      events.push({
        timestamp: rates[i]!.timestamp,
        poolAddress: query.poolAddress,
        previousBorrowApy: previous,
        newBorrowApy: current,
        deltaBps,
        changeType: deltaBps > 0 ? 'increase' : 'decrease',
      });
    }
  }

  await redisCacheService.set(cacheKey, events, ANALYTICS_CACHE_TTL_S);
  return events;
}

/**
 * `GET /api/rates/history?asset=&from=&to=&granularity=` — historical rate
 * points over an explicit date range at a given granularity, rather than
 * the preset `timeRange` buckets `getHistoricalRates` uses.
 */
export async function getRateHistoryRange(query: RateHistoryQuery): Promise<HistoricalRatePoint[]> {
  const granularity = query.granularity ?? 'daily';
  const to = query.to ? Date.parse(query.to) : Date.now();
  const from = query.from ? Date.parse(query.from) : to - GRANULARITY_MS.daily * 7;

  if (Number.isNaN(from) || Number.isNaN(to)) {
    throw new ValidationError('Invalid `from`/`to` date for rate history query');
  }
  if (from > to) {
    throw new ValidationError('`from` must not be after `to`');
  }

  const bucketMs = GRANULARITY_MS[granularity];
  const bucketCount = Math.floor((to - from) / bucketMs) + 1;
  if (bucketCount > MAX_RATE_HISTORY_BUCKETS) {
    throw new ValidationError(
      `Requested range produces ${bucketCount} buckets, exceeding the maximum of ${MAX_RATE_HISTORY_BUCKETS}. Widen the granularity or narrow the range.`
    );
  }

  const cacheKey = redisCacheService.buildKey(
    'protocol',
    `rate-history-range:${query.asset || 'all'}:${from}:${to}:${granularity}`
  );
  const cached = await redisCacheService.get<HistoricalRatePoint[]>(cacheKey);
  if (cached) return cached;

  const stellarService = new StellarService();
  const timestamps: number[] = [];
  for (let t = from; t <= to; t += bucketMs) {
    timestamps.push(t);
  }
  if (timestamps.length === 0) timestamps.push(from);

  const rates: HistoricalRatePoint[] = await Promise.all(
    timestamps.map(async (timestamp) => {
      const rateData = await stellarService.getPoolRateAt(
        query.asset || '',
        Math.floor(timestamp / 1000)
      );
      return {
        timestamp: new Date(timestamp).toISOString(),
        depositApy: rateData.depositApy,
        borrowApy: rateData.borrowApy,
        utilizationRate: rateData.utilizationRate,
        poolAddress: query.asset,
      };
    })
  );

  await redisCacheService.set(cacheKey, rates, ANALYTICS_CACHE_TTL_S);
  return rates;
}
