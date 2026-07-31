import {
  refreshCrossProtocolData,
  getCrossProtocolComparison,
  computeMarketShare,
  getLeaderboard,
  getAssetComparison,
  getFeeComparison,
  getLiquidationParamsComparison,
  getMarketShareHistory,
  getPositioningReport,
  getBenchmarkScore,
  getWeeklyDigest,
} from '../services/cross-protocol-etl/etl.service';
import { ProtocolAdapter, StandardizedProtocolMetrics } from '../services/cross-protocol-etl/types';
import { redisCacheService } from '../services/redisCache.service';

function metric(overrides: Partial<StandardizedProtocolMetrics> = {}): StandardizedProtocolMetrics {
  return {
    protocol: 'test-protocol',
    displayName: 'Test Protocol',
    chain: 'test-chain',
    asset: 'USDC',
    supplyApy: 0.03,
    borrowApy: 0.05,
    tvlUsd: 1_000_000,
    utilizationRate: 0.6,
    fetchedAt: new Date().toISOString(),
    source: 'test',
    ...overrides,
  };
}

function fakeAdapter(protocolId: string, metrics: StandardizedProtocolMetrics[]): ProtocolAdapter {
  return {
    protocolId,
    displayName: protocolId,
    fetchMetrics: async () => metrics,
  };
}

function failingAdapter(protocolId: string, error: Error): ProtocolAdapter {
  return {
    protocolId,
    displayName: protocolId,
    fetchMetrics: async () => {
      throw error;
    },
  };
}

describe('cross-protocol-etl.service', () => {
  beforeEach(() => {
    redisCacheService.clearAllForTests();
  });

  describe('refreshCrossProtocolData', () => {
    it('merges metrics from all adapters', async () => {
      const adapters = [
        fakeAdapter('a', [metric({ protocol: 'a', asset: 'USDC' })]),
        fakeAdapter('b', [metric({ protocol: 'b', asset: 'USDT' })]),
      ];
      const result = await refreshCrossProtocolData(adapters);
      expect(result.metrics).toHaveLength(2);
      expect(result.failedSources).toEqual([]);
      expect(result.metrics.map((m) => m.protocol).sort()).toEqual(['a', 'b']);
    });

    it('isolates a failing adapter without blocking the others', async () => {
      const adapters = [
        fakeAdapter('good', [metric({ protocol: 'good' })]),
        failingAdapter('bad', new Error('upstream unreachable')),
      ];
      const result = await refreshCrossProtocolData(adapters);
      expect(result.metrics).toHaveLength(1);
      expect(result.metrics[0]!.protocol).toBe('good');
      expect(result.failedSources).toEqual(['bad']);
    });

    it('filters out metrics that fail data quality checks', async () => {
      const adapters = [
        fakeAdapter('mixed', [
          metric({ protocol: 'mixed', asset: 'GOOD' }),
          metric({ protocol: 'mixed', asset: 'BAD_APY', supplyApy: 50 }), // 5000% APY — implausible
          metric({ protocol: 'mixed', asset: 'BAD_UTIL', utilizationRate: 1.5 }),
          metric({ protocol: 'mixed', asset: 'BAD_TVL', tvlUsd: -100 }),
        ]),
      ];
      const result = await refreshCrossProtocolData(adapters);
      expect(result.metrics).toHaveLength(1);
      expect(result.metrics[0]!.asset).toBe('GOOD');
      expect(result.qualityIssues).toHaveLength(3);
      expect(result.qualityIssues.map((i) => i.asset).sort()).toEqual([
        'BAD_APY',
        'BAD_TVL',
        'BAD_UTIL',
      ]);
    });
  });

  describe('getCrossProtocolComparison caching', () => {
    it('caches the result so a second call does not re-invoke adapters', async () => {
      let calls = 0;
      const adapters = [
        {
          protocolId: 'counted',
          displayName: 'counted',
          fetchMetrics: async () => {
            calls += 1;
            return [metric({ protocol: 'counted' })];
          },
        },
      ];
      await getCrossProtocolComparison(adapters);
      await getCrossProtocolComparison(adapters);
      expect(calls).toBe(1);
    });
  });

  describe('computeMarketShare', () => {
    it('computes TVL-weighted share per protocol, sorted descending', () => {
      const metrics = [
        metric({ protocol: 'big', tvlUsd: 750 }),
        metric({ protocol: 'small', tvlUsd: 250 }),
      ];
      const shares = computeMarketShare(metrics);
      expect(shares).toEqual([
        { protocol: 'big', tvlUsd: 750, marketSharePct: 75 },
        { protocol: 'small', tvlUsd: 250, marketSharePct: 25 },
      ]);
    });

    it('sums TVL across multiple pools of the same protocol', () => {
      const metrics = [
        metric({ protocol: 'multi', asset: 'USDC', tvlUsd: 400 }),
        metric({ protocol: 'multi', asset: 'USDT', tvlUsd: 600 }),
      ];
      const shares = computeMarketShare(metrics);
      expect(shares).toEqual([{ protocol: 'multi', tvlUsd: 1000, marketSharePct: 100 }]);
    });

    it('returns 0% shares (not NaN) when total TVL is zero', () => {
      const shares = computeMarketShare([metric({ tvlUsd: 0 })]);
      expect(shares[0]!.marketSharePct).toBe(0);
    });
  });

  describe('getLeaderboard', () => {
    it('ranks protocols by the requested metric, descending', async () => {
      const adapters = [
        fakeAdapter('x', [
          metric({ protocol: 'low-apy', supplyApy: 0.01, tvlUsd: 100 }),
          metric({ protocol: 'high-apy', supplyApy: 0.09, tvlUsd: 50 }),
        ]),
      ];
      const board = await getLeaderboard('supplyApy', 10, adapters);
      expect(board.map((e) => e.protocol)).toEqual(['high-apy', 'low-apy']);
      expect(board[0]!.rank).toBe(1);
      expect(board[0]!.metricValue).toBe(0.09);
    });

    it('respects the limit', async () => {
      const adapters = [
        fakeAdapter(
          'many',
          Array.from({ length: 5 }, (_, i) => metric({ protocol: `p${i}`, tvlUsd: i }))
        ),
      ];
      const board = await getLeaderboard('tvlUsd', 2, adapters);
      expect(board).toHaveLength(2);
    });
  });

  describe('getAssetComparison', () => {
    it('returns only entries for the requested asset, sorted by TVL descending', async () => {
      const adapters = [
        fakeAdapter('a', [
          metric({ protocol: 'a', asset: 'USDC', tvlUsd: 100 }),
          metric({ protocol: 'a', asset: 'ETH', tvlUsd: 999 }),
        ]),
        fakeAdapter('b', [metric({ protocol: 'b', asset: 'USDC', tvlUsd: 500 })]),
      ];
      const result = await getAssetComparison('usdc', adapters);
      expect(result.asset).toBe('USDC');
      expect(result.entries.map((e) => e.protocol)).toEqual(['b', 'a']);
    });

    it('computes spreadBps as (borrowApy - supplyApy) in basis points', async () => {
      const adapters = [
        fakeAdapter('a', [metric({ protocol: 'a', asset: 'USDC', supplyApy: 0.03, borrowApy: 0.05 })]),
      ];
      const result = await getAssetComparison('USDC', adapters);
      expect(result.entries[0]!.spreadBps).toBe(200);
    });
  });

  describe('getFeeComparison', () => {
    it('includes every curated protocol with a live-computed spread', async () => {
      const adapters = [
        fakeAdapter('stellarlend', [
          metric({ protocol: 'stellarlend', asset: 'USDC', supplyApy: 0.03, borrowApy: 0.06 }),
        ]),
      ];
      const result = await getFeeComparison(adapters);
      const stellarLend = result.find((r) => r.protocol === 'stellarlend');
      expect(stellarLend).toBeDefined();
      expect(stellarLend!.reserveFactorBps).toBe(1000);
      expect(stellarLend!.avgSpreadBps).toBe(300);
      // Peers with no live data this refresh still appear, with a 0 live spread.
      expect(result.find((r) => r.protocol === 'aave-v3')).toBeDefined();
    });
  });

  describe('getLiquidationParamsComparison', () => {
    it('returns StellarLend contract defaults alongside curated peer references', () => {
      const params = getLiquidationParamsComparison();
      const stellarLend = params.find((p) => p.protocol === 'stellarlend');
      expect(stellarLend).toMatchObject({
        liquidationThresholdBps: 10500,
        liquidationBonusBps: 1000,
        closeFactorBps: 5000,
        thresholdConvention: 'min-collateral-ratio',
      });
      expect(params.length).toBeGreaterThanOrEqual(5);
    });
  });

  describe('getMarketShareHistory', () => {
    it('appends a snapshot on every fresh (non-cached) refresh', async () => {
      const before = getMarketShareHistory().length;
      await getCrossProtocolComparison([fakeAdapter('h1', [metric({ protocol: 'h1' })])]);
      redisCacheService.clearAllForTests();
      await getCrossProtocolComparison([fakeAdapter('h2', [metric({ protocol: 'h2' })])]);
      const after = getMarketShareHistory().length;
      expect(after).toBe(before + 2);
    });
  });

  describe('getPositioningReport', () => {
    it('flags a metric where StellarLend meaningfully beats the peer average as a strength', async () => {
      const adapters = [
        fakeAdapter('stellarlend', [
          metric({ protocol: 'stellarlend', asset: 'USDC', supplyApy: 0.1, borrowApy: 0.05, tvlUsd: 1_000_000 }),
        ]),
        fakeAdapter('peer', [
          metric({ protocol: 'peer', asset: 'USDC', supplyApy: 0.03, borrowApy: 0.05, tvlUsd: 1_000_000 }),
        ]),
      ];
      const report = await getPositioningReport(adapters);
      expect(report.strengths.some((s) => s.includes('supplyApy'))).toBe(true);
    });

    it('returns an empty report when StellarLend or peer data is absent', async () => {
      const report = await getPositioningReport([fakeAdapter('onlypeer', [metric({ protocol: 'onlypeer' })])]);
      expect(report.strengths).toEqual([]);
      expect(report.weaknesses).toEqual([]);
      expect(report.metrics).toEqual([]);
    });
  });

  describe('getBenchmarkScore', () => {
    it('ranks the protocol with the best combined metrics first', async () => {
      const adapters = [
        fakeAdapter('best', [
          metric({ protocol: 'best', supplyApy: 0.1, borrowApy: 0.02, tvlUsd: 10_000_000 }),
        ]),
        fakeAdapter('worst', [
          metric({ protocol: 'worst', supplyApy: 0.01, borrowApy: 0.2, tvlUsd: 1 }),
        ]),
      ];
      const result = await getBenchmarkScore(adapters);
      expect(result.entries[0]!.protocol).toBe('best');
      expect(result.entries[0]!.rank).toBe(1);
      expect(result.entries[0]!.score).toBeGreaterThan(result.entries[1]!.score);
    });
  });

  describe('getWeeklyDigest', () => {
    it('summarizes current market share with insufficient-history messaging on first run', async () => {
      const digest = await getWeeklyDigest([
        fakeAdapter('stellarlend', [metric({ protocol: 'stellarlend', tvlUsd: 500 })]),
        fakeAdapter('peer', [metric({ protocol: 'peer', tvlUsd: 500 })]),
      ]);
      expect(digest.stellarLendMarketSharePct).toBeCloseTo(50);
      expect(digest.summary).toContain('Insufficient history');
      expect(digest.topMoversByTvl.length).toBeGreaterThan(0);
    });
  });
});
