import pg from 'pg';
import type {
  AssetMetricSample,
  MetricsGap,
  ProtocolMetricSample,
  TimeSeriesPoint,
  TimeSeriesQuery,
} from '../types.js';
import type { MetricsRepository } from './repository.js';
import { resolveBucket, resolveMetricColumn } from './repository.js';

/**
 * TimescaleDB-backed metrics repository.
 */
export class PostgresMetricsRepository implements MetricsRepository {
  private readonly pool: pg.Pool;

  constructor(databaseUrl: string) {
    this.pool = new pg.Pool({ connectionString: databaseUrl });
  }

  async writeProtocolSample(sample: ProtocolMetricSample): Promise<void> {
    await this.pool.query(
      `INSERT INTO protocol_metrics
        (time, tvl, total_borrows, utilization_rate, liquidations, total_deposits, active_users)
       VALUES ($1,$2,$3,$4,$5,$6,$7)
       ON CONFLICT (time) DO UPDATE SET
         tvl = EXCLUDED.tvl,
         total_borrows = EXCLUDED.total_borrows,
         utilization_rate = EXCLUDED.utilization_rate,
         liquidations = EXCLUDED.liquidations,
         total_deposits = EXCLUDED.total_deposits,
         active_users = EXCLUDED.active_users`,
      [
        sample.time.toISOString(),
        sample.tvl,
        sample.totalBorrows,
        sample.utilizationRate,
        sample.liquidations,
        sample.totalDeposits,
        sample.activeUsers,
      ]
    );
    await this.pool.query(
      `UPDATE metrics_collector_state
       SET last_collected_at = $1, samples_written = samples_written + 1, updated_at = NOW()
       WHERE id = 1`,
      [sample.time.toISOString()]
    );
  }

  async writeAssetSamples(samples: AssetMetricSample[]): Promise<void> {
    for (const sample of samples) {
      await this.pool.query(
        `INSERT INTO asset_metrics
          (time, asset, supply, borrow, available_liquidity, price, volatility, apy)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
         ON CONFLICT (time, asset) DO UPDATE SET
           supply = EXCLUDED.supply,
           borrow = EXCLUDED.borrow,
           available_liquidity = EXCLUDED.available_liquidity,
           price = EXCLUDED.price,
           volatility = EXCLUDED.volatility,
           apy = EXCLUDED.apy`,
        [
          sample.time.toISOString(),
          sample.asset,
          sample.supply,
          sample.borrow,
          sample.availableLiquidity,
          sample.price,
          sample.volatility,
          sample.apy,
        ]
      );
    }
  }

  async listProtocolTimestamps(from: Date, to: Date): Promise<Date[]> {
    const result = await this.pool.query<{ time: Date }>(
      `SELECT time FROM protocol_metrics WHERE time >= $1 AND time <= $2 ORDER BY time ASC`,
      [from.toISOString(), to.toISOString()]
    );
    return result.rows.map((r) => r.time);
  }

  async recordGaps(gaps: MetricsGap[]): Promise<void> {
    for (const gap of gaps) {
      await this.pool.query(
        `INSERT INTO metrics_gaps (metric_family, gap_start, gap_end)
         VALUES ($1, $2, $3)`,
        [gap.metricFamily, gap.gapStart.toISOString(), gap.gapEnd.toISOString()]
      );
    }
  }

  async markGapsBackfilled(gaps: MetricsGap[]): Promise<void> {
    for (const gap of gaps) {
      await this.pool.query(
        `UPDATE metrics_gaps
         SET backfilled = TRUE, backfilled_at = NOW()
         WHERE gap_start = $1 AND gap_end = $2 AND backfilled = FALSE`,
        [gap.gapStart.toISOString(), gap.gapEnd.toISOString()]
      );
    }
  }

  async queryTimeSeries(query: TimeSeriesQuery): Promise<TimeSeriesPoint[]> {
    const column = resolveMetricColumn(query.metric);
    const bucket = resolveBucket(query.interval);
    // column is from a fixed allow-list; safe to interpolate
    const sql = `
      SELECT time_bucket($1::interval, time) AS bucket,
             AVG(${column})::float8 AS value
      FROM protocol_metrics
      WHERE time >= $2 AND time <= $3
      GROUP BY bucket
      ORDER BY bucket ASC`;
    const result = await this.pool.query<{ bucket: Date; value: number }>(sql, [
      bucket,
      query.from.toISOString(),
      query.to.toISOString(),
    ]);
    return result.rows.map((row) => ({
      time: row.bucket.toISOString(),
      value: Number(row.value),
    }));
  }

  async close(): Promise<void> {
    await this.pool.end();
  }
}
