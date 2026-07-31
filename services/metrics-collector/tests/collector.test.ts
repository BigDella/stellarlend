import { describe, expect, it, beforeEach } from 'vitest';
import { detectGaps, backfillTimestamps } from '../src/collector/gap-detection.js';
import { MetricsCollector } from '../src/collector/collector.js';
import { toProtocolSample, toAssetSamples } from '../src/collector/stats-source.js';
import { InMemoryMetricsRepository } from '../src/db/repository.js';
import { loadConfig } from '../src/config.js';
import { createLogger } from '../src/utils/logger.js';
import type { ProtocolStatsSource } from '../src/collector/stats-source.js';

describe('gap detection', () => {
  it('detects missing intervals', () => {
    const stamps = [
      new Date('2026-01-01T00:00:00Z'),
      new Date('2026-01-01T00:01:00Z'),
      new Date('2026-01-01T00:05:00Z'),
    ];
    const gaps = detectGaps(stamps, 60_000);
    expect(gaps).toHaveLength(1);
    expect(gaps[0]!.gapStart.toISOString()).toBe('2026-01-01T00:02:00.000Z');
  });

  it('generates backfill timestamps', () => {
    const points = backfillTimestamps(
      {
        metricFamily: 'protocol',
        gapStart: new Date('2026-01-01T00:02:00Z'),
        gapEnd: new Date('2026-01-01T00:04:00Z'),
      },
      60_000
    );
    expect(points.map((p) => p.toISOString())).toEqual([
      '2026-01-01T00:02:00.000Z',
      '2026-01-01T00:03:00.000Z',
      '2026-01-01T00:04:00.000Z',
    ]);
  });
});

describe('MetricsCollector', () => {
  const source: ProtocolStatsSource = {
    async fetchProtocolStats() {
      return {
        tvl: 1_000_000,
        totalBorrows: 250_000,
        utilizationRate: 0.25,
        liquidations: 2,
        totalDeposits: 1_000_000,
        activeUsers: 42,
        assets: [
          {
            asset: 'XLM',
            supply: 800_000,
            borrow: 200_000,
            availableLiquidity: 600_000,
            price: 0.12,
            volatility: 0.04,
            apy: 0.03,
          },
        ],
      };
    },
  };

  beforeEach(() => {
    /* isolated per test via new repo */
  });

  it('writes protocol and per-asset samples', async () => {
    const repo = new InMemoryMetricsRepository();
    const collector = new MetricsCollector(
      {
        protocolStatsUrl: 'http://localhost/stats',
        databaseUrl: 'postgres://localhost/db',
        collectIntervalMs: 60_000,
        rawRetentionDays: 30,
        aggregatedRetentionDays: 365,
        logLevel: 'error',
      },
      source,
      repo,
      createLogger('error')
    );

    const result = await collector.collectOnce(new Date('2026-01-01T00:00:00Z'));
    expect(result.protocol.tvl).toBe(1_000_000);
    expect(result.assets).toBe(1);
    expect(repo.protocol).toHaveLength(1);
    expect(repo.assets[0]!.asset).toBe('XLM');
  });

  it('queries downsampled time-series', async () => {
    const repo = new InMemoryMetricsRepository();
    const t0 = new Date('2026-01-01T00:00:00Z');
    const t1 = new Date('2026-01-01T00:01:00Z');
    await repo.writeProtocolSample(toProtocolSample(await source.fetchProtocolStats(), t0));
    await repo.writeProtocolSample({
      ...toProtocolSample(await source.fetchProtocolStats(), t1),
      tvl: 1_100_000,
    });

    const series = await repo.queryTimeSeries({
      metric: 'tvl',
      from: t0,
      to: t1,
      interval: '1m',
    });
    expect(series.length).toBeGreaterThanOrEqual(1);
    expect(series[0]!.value).toBeGreaterThan(0);
  });

  it('backfills detected gaps', async () => {
    const repo = new InMemoryMetricsRepository();
    const collector = new MetricsCollector(
      {
        protocolStatsUrl: 'http://localhost/stats',
        databaseUrl: 'postgres://localhost/db',
        collectIntervalMs: 60_000,
        rawRetentionDays: 30,
        aggregatedRetentionDays: 365,
        logLevel: 'error',
      },
      source,
      repo,
      createLogger('error')
    );

    const now = Date.now();
    await repo.writeProtocolSample(
      toProtocolSample(await source.fetchProtocolStats(), new Date(now - 5 * 60_000))
    );
    await repo.writeProtocolSample(
      toProtocolSample(await source.fetchProtocolStats(), new Date(now))
    );

    const filled = await collector.detectAndBackfillGaps(10 * 60 * 1000);
    expect(filled).toBeGreaterThan(0);
    expect(repo.gaps.length).toBeGreaterThan(0);
  });
});

describe('stats mapping', () => {
  it('maps asset samples', () => {
    const samples = toAssetSamples({
      tvl: 1,
      totalBorrows: 1,
      utilizationRate: 1,
      totalDeposits: 1,
      assets: [{ asset: 'USDC', supply: 10, borrow: 2, availableLiquidity: 8, apy: 0.05 }],
    });
    expect(samples).toHaveLength(1);
    expect(samples[0]!.apy).toBe(0.05);
  });
});

describe('loadConfig', () => {
  it('requires database url', () => {
    expect(() => loadConfig({})).toThrow(/DATABASE_URL/);
  });

  it('loads defaults', () => {
    const config = loadConfig({ DATABASE_URL: 'postgres://localhost/db' });
    expect(config.collectIntervalMs).toBe(60_000);
    expect(config.rawRetentionDays).toBe(30);
  });
});
