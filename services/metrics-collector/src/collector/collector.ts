import { detectGaps, backfillTimestamps } from './gap-detection.js';
import { toAssetSamples, toProtocolSample, type ProtocolStatsSource } from './stats-source.js';
import type { MetricsRepository } from '../db/repository.js';
import type { MetricsCollectorConfig, ProtocolMetricSample } from '../types.js';
import type { Logger } from '../utils/logger.js';

/**
 * Minute-level collector that writes protocol + per-asset metrics to TimescaleDB.
 */
export class MetricsCollector {
  private timer: ReturnType<typeof setInterval> | undefined;
  private running = false;

  constructor(
    private readonly config: MetricsCollectorConfig,
    private readonly source: ProtocolStatsSource,
    private readonly repo: MetricsRepository,
    private readonly logger: Logger
  ) {}

  async collectOnce(time: Date = new Date()): Promise<{
    protocol: ProtocolMetricSample;
    assets: number;
  }> {
    const stats = await this.source.fetchProtocolStats();
    const protocol = toProtocolSample(stats, time);
    const assets = toAssetSamples(stats, time);

    await this.repo.writeProtocolSample(protocol);
    if (assets.length > 0) {
      await this.repo.writeAssetSamples(assets);
    }

    this.logger.info('Metrics sample written', {
      time: time.toISOString(),
      tvl: protocol.tvl,
      assets: assets.length,
    });

    return { protocol, assets: assets.length };
  }

  /**
   * Scan recent window for missing intervals and backfill by re-sampling current stats
   * at each missing timestamp (best-effort when historical RPC snapshots are unavailable).
   */
  async detectAndBackfillGaps(lookbackMs: number = 6 * 60 * 60 * 1000): Promise<number> {
    const to = new Date();
    const from = new Date(to.getTime() - lookbackMs);
    const timestamps = await this.repo.listProtocolTimestamps(from, to);
    const gaps = detectGaps(timestamps, this.config.collectIntervalMs, 'protocol');
    if (gaps.length === 0) return 0;

    await this.repo.recordGaps(gaps);
    let filled = 0;

    for (const gap of gaps) {
      const points = backfillTimestamps(gap, this.config.collectIntervalMs);
      for (const point of points) {
        await this.collectOnce(point);
        filled += 1;
      }
    }

    await this.repo.markGapsBackfilled(gaps);
    this.logger.warn('Backfilled metric gaps', { gaps: gaps.length, samples: filled });
    return filled;
  }

  start(): void {
    if (this.running) return;
    this.running = true;
    this.logger.info('Metrics collector started', {
      intervalMs: this.config.collectIntervalMs,
    });

    const tick = async () => {
      try {
        await this.collectOnce();
      } catch (err) {
        this.logger.error('Metric collection failed', {
          error: err instanceof Error ? err.message : String(err),
        });
      }
    };

    void tick();
    this.timer = setInterval(() => void tick(), this.config.collectIntervalMs);

    // Hourly gap detection
    setInterval(
      () => {
        void this.detectAndBackfillGaps().catch((err) => {
          this.logger.error('Gap detection failed', {
            error: err instanceof Error ? err.message : String(err),
          });
        });
      },
      60 * 60 * 1000
    );
  }

  stop(): void {
    this.running = false;
    if (this.timer) clearInterval(this.timer);
    this.timer = undefined;
  }
}
