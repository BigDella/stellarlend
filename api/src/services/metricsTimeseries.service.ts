/**
 * Time-series metrics query service (issue #455).
 *
 * Production deployments set METRICS_DATABASE_URL and run the
 * `services/metrics-collector` worker against TimescaleDB. This API layer
 * queries either:
 *   1. an injected repository (tests / custom adapters), or
 *   2. an in-process series buffer (local/dev dashboards).
 */

export type MetricName =
  | 'tvl'
  | 'totalBorrows'
  | 'utilizationRate'
  | 'liquidations'
  | 'totalDeposits'
  | 'activeUsers';

export type MetricInterval = '1m' | '5m' | '1h' | '1d';

export interface TimeSeriesQueryParams {
  metric: MetricName;
  from: Date;
  to: Date;
  interval: MetricInterval;
}

export interface TimeSeriesResponse {
  metric: MetricName;
  from: string;
  to: string;
  interval: MetricInterval;
  points: Array<{ time: string; value: number }>;
}

export interface MetricsTimeseriesRepository {
  queryTimeSeries(params: TimeSeriesQueryParams): Promise<TimeSeriesResponse>;
}

const VALID_METRICS = new Set<string>([
  'tvl',
  'totalBorrows',
  'utilizationRate',
  'liquidations',
  'totalDeposits',
  'activeUsers',
]);

const VALID_INTERVALS = new Set<string>(['1m', '5m', '1h', '1d']);

const memorySeries: Array<{ time: Date; values: Record<MetricName, number> }> = [];
let injectedRepo: MetricsTimeseriesRepository | null = null;

export function isValidMetric(value: string): value is MetricName {
  return VALID_METRICS.has(value);
}

export function isValidInterval(value: string): value is MetricInterval {
  return VALID_INTERVALS.has(value);
}

export function setMetricsTimeseriesRepository(repo: MetricsTimeseriesRepository | null): void {
  injectedRepo = repo;
}

export function seedMemoryPoint(time: Date, values: Partial<Record<MetricName, number>>): void {
  memorySeries.push({
    time,
    values: {
      tvl: values.tvl ?? 0,
      totalBorrows: values.totalBorrows ?? 0,
      utilizationRate: values.utilizationRate ?? 0,
      liquidations: values.liquidations ?? 0,
      totalDeposits: values.totalDeposits ?? 0,
      activeUsers: values.activeUsers ?? 0,
    },
  });
}

export function clearMemorySeries(): void {
  memorySeries.length = 0;
}

function intervalMs(interval: MetricInterval): number {
  switch (interval) {
    case '1m':
      return 60_000;
    case '5m':
      return 300_000;
    case '1h':
      return 3_600_000;
    case '1d':
      return 86_400_000;
  }
}

function queryMemory(params: TimeSeriesQueryParams): TimeSeriesResponse {
  const bucketSize = intervalMs(params.interval);
  const buckets = new Map<number, number[]>();

  for (const sample of memorySeries) {
    if (sample.time < params.from || sample.time > params.to) continue;
    const bucket = Math.floor(sample.time.getTime() / bucketSize) * bucketSize;
    const arr = buckets.get(bucket) ?? [];
    arr.push(sample.values[params.metric]);
    buckets.set(bucket, arr);
  }

  const points = [...buckets.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([time, values]) => ({
      time: new Date(time).toISOString(),
      value: values.reduce((a, b) => a + b, 0) / values.length,
    }));

  return {
    metric: params.metric,
    from: params.from.toISOString(),
    to: params.to.toISOString(),
    interval: params.interval,
    points,
  };
}

export async function queryTimeSeries(params: TimeSeriesQueryParams): Promise<TimeSeriesResponse> {
  if (injectedRepo) {
    return injectedRepo.queryTimeSeries(params);
  }
  return queryMemory(params);
}
