import {
  recordSnapshot,
  fillSnapshotGaps,
  computeMetrics,
  rankPools,
  benchmarkPool,
  snapshotsToCsv,
  resetPoolPerformanceStore,
  recordEvent,
  snapshotFromPoolState,
  type PoolSnapshot,
} from '../services/analytics/pool-performance';

describe('pool-performance analytics (#611)', () => {
  beforeEach(() => {
    resetPoolPerformanceStore();
  });

  function snap(partial: Partial<PoolSnapshot> & { timestamp: string }): PoolSnapshot {
    return {
      poolAddress: 'pool_xlm_001',
      tvl: 1_000_000,
      utilizationRate: 0.5,
      borrowApy: 0.06,
      supplyApy: 0.04,
      badDebt: 0,
      totalDeposits: 1_000_000,
      totalBorrows: 500_000,
      ...partial,
    };
  }

  it('fills hourly gaps from network downtime', () => {
    const a = snap({ timestamp: '2026-01-01T00:00:00.000Z' });
    const b = snap({ timestamp: '2026-01-01T03:00:00.000Z', tvl: 1_300_000 });
    const { filled, gapFilledPoints } = fillSnapshotGaps([a, b]);
    expect(gapFilledPoints).toBe(2);
    expect(filled).toHaveLength(4);
  });

  it('marks new pools with short history', () => {
    recordSnapshot(snap({ timestamp: new Date().toISOString() }));
    const metrics = computeMetrics('pool_xlm_001', '7d');
    expect(metrics.historyTooShort).toBe(true);
    expect(metrics.sampleCount).toBe(1);
  });

  it('computes averages, volatility, and ranks pools', () => {
    const now = Date.now();
    for (let i = 0; i < 30; i++) {
      recordSnapshot(
        snap({
          timestamp: new Date(now - i * 3600_000).toISOString(),
          supplyApy: 0.04 + (i % 3) * 0.001,
        })
      );
    }
    const metrics = computeMetrics('pool_xlm_001', '7d');
    expect(metrics.avgSupplyApy).toBeGreaterThan(0);
    expect(metrics.volatility).toBeGreaterThanOrEqual(0);

    const ranked = rankPools([
      { poolAddress: 'a', poolName: 'A', currentApy: 0.02, tvl: 100, utilization: 0.4, riskScore: 80, rank: 0 },
      { poolAddress: 'b', poolName: 'B', currentApy: 0.08, tvl: 1_000_000, utilization: 0.5, riskScore: 10, rank: 0 },
    ]);
    expect(ranked[0]!.poolAddress).toBe('b');
    expect(ranked[0]!.rank).toBe(1);
  });

  it('benchmarks against Compound and Aave and exports CSV', () => {
    recordSnapshot(snap({ timestamp: new Date().toISOString(), supplyApy: 0.05 }));
    const bench = benchmarkPool('pool_xlm_001', '30d');
    expect(bench.benchmarks.map((b) => b.name)).toEqual(['compound', 'aave']);
    const csv = snapshotsToCsv([snap({ timestamp: '2026-01-01T00:00:00.000Z' })]);
    expect(csv).toContain('poolAddress,timestamp');
    expect(csv).toContain('pool_xlm_001');
  });

  it('records liquidation and bad-debt events', () => {
    const evt = recordEvent('pool_xlm_001', 'liquidation', { amount: 100 });
    expect(evt.type).toBe('liquidation');
    expect(evt.id).toBeTruthy();
  });

  it('normalizes string pool state from the chain adapter', () => {
    const snapshot = snapshotFromPoolState('pool_usdc_001', {
      utilizationRate: 0.6,
      totalDeposits: '1000',
      totalBorrows: '600',
    });
    expect(snapshot.tvl).toBe(1000);
    expect(snapshot.totalBorrows).toBe(600);
  });
});
