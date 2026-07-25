import type {
  AssetMetricSample,
  MetricsGap,
  ProtocolMetricName,
  ProtocolMetricSample,
  TimeSeriesPoint,
  TimeSeriesQuery,
} from '../types.js';
import { PROTOCOL_METRIC_COLUMNS } from '../types.js';

export interface MetricsRepository {
  writeProtocolSample(sample: ProtocolMetricSample): Promise<void>;
  writeAssetSamples(samples: AssetMetricSample[]): Promise<void>;
  listProtocolTimestamps(from: Date, to: Date): Promise<Date[]>;
  recordGaps(gaps: MetricsGap[]): Promise<void>;
  markGapsBackfilled(gaps: MetricsGap[]): Promise<void>;
  queryTimeSeries(query: TimeSeriesQuery): Promise<TimeSeriesPoint[]>;
  close(): Promise<void>;
}

const INTERVAL_TO_BUCKET: Record<TimeSeriesQuery['interval'], string> = {
  '1m': '1 minute',
  '5m': '5 minutes',
  '1h': '1 hour',
  '1d': '1 day',
};

/**
 * In-memory TSDB stand-in for tests and local dry-runs.
 */
export class InMemoryMetricsRepository implements MetricsRepository {
  protocol: ProtocolMetricSample[] = [];
  assets: AssetMetricSample[] = [];
  gaps: MetricsGap[] = [];

  async writeProtocolSample(sample: ProtocolMetricSample): Promise<void> {
    this.protocol.push(sample);
  }

  async writeAssetSamples(samples: AssetMetricSample[]): Promise<void> {
    this.assets.push(...samples);
  }

  async listProtocolTimestamps(from: Date, to: Date): Promise<Date[]> {
    return this.protocol
      .filter((s) => s.time >= from && s.time <= to)
      .map((s) => s.time)
      .sort((a, b) => a.getTime() - b.getTime());
  }

  async recordGaps(gaps: MetricsGap[]): Promise<void> {
    this.gaps.push(...gaps);
  }

  async markGapsBackfilled(gaps: MetricsGap[]): Promise<void> {
    for (const gap of gaps) {
      const match = this.gaps.find(
        (g) =>
          g.gapStart.getTime() === gap.gapStart.getTime() &&
          g.gapEnd.getTime() === gap.gapEnd.getTime()
      );
      if (match) {
        (match as MetricsGap & { backfilled?: boolean }).backfilled = true;
      }
    }
  }

  async queryTimeSeries(query: TimeSeriesQuery): Promise<TimeSeriesPoint[]> {
    const key = query.metric;
    const intervalMs =
      query.interval === '1m'
        ? 60_000
        : query.interval === '5m'
          ? 300_000
          : query.interval === '1h'
            ? 3_600_000
            : 86_400_000;

    const samples = this.protocol.filter((s) => s.time >= query.from && s.time <= query.to);
    const buckets = new Map<number, number[]>();

    for (const sample of samples) {
      const bucket = Math.floor(sample.time.getTime() / intervalMs) * intervalMs;
      const values = buckets.get(bucket) ?? [];
      const value =
        key === 'tvl'
          ? sample.tvl
          : key === 'totalBorrows'
            ? sample.totalBorrows
            : key === 'utilizationRate'
              ? sample.utilizationRate
              : key === 'liquidations'
                ? sample.liquidations
                : key === 'totalDeposits'
                  ? sample.totalDeposits
                  : sample.activeUsers;
      values.push(value);
      buckets.set(bucket, values);
    }

    return [...buckets.entries()]
      .sort((a, b) => a[0] - b[0])
      .map(([time, values]) => ({
        time: new Date(time).toISOString(),
        value: values.reduce((a, b) => a + b, 0) / values.length,
      }));
  }

  async close(): Promise<void> {
    /* no-op */
  }
}

/**
 * Query helper shared by the collector service and API adapter.
 */
export function resolveMetricColumn(metric: ProtocolMetricName): string {
  const column = PROTOCOL_METRIC_COLUMNS[metric];
  if (!column) throw new Error(`Unsupported metric: ${metric}`);
  return column;
}

export function resolveBucket(interval: TimeSeriesQuery['interval']): string {
  return INTERVAL_TO_BUCKET[interval];
}
