import { gasUsageAnalyticsService } from '../services/analytics/gasUsageAnalytics.service';

describe('gasUsageAnalyticsService', () => {
  const FN = `test_fn_${Math.random().toString(36).slice(2)}`;

  describe('recordSample / getStats', () => {
    it('returns zeroed stats for a function with no samples', () => {
      const stats = gasUsageAnalyticsService.getStats('unknown_fn_xyz');
      expect(stats.sampleCount).toBe(0);
      expect(stats.average).toBe(0);
    });

    it('computes average/median/p95/p99 from recorded samples', () => {
      const values = [100, 200, 300, 400, 500, 600, 700, 800, 900, 1000];
      for (const v of values) {
        gasUsageAnalyticsService.recordSample({ functionName: FN, gasUsed: v, timestamp: Date.now() });
      }

      const stats = gasUsageAnalyticsService.getStats(FN, '24h');
      expect(stats.sampleCount).toBe(10);
      expect(stats.average).toBe(550);
      expect(stats.min).toBe(100);
      expect(stats.max).toBe(1000);
      expect(stats.p95).toBeGreaterThanOrEqual(stats.median);
      expect(stats.p99).toBeGreaterThanOrEqual(stats.p95);
    });

    it('includes seeded lending-operation functions in listFunctions', () => {
      const fns = gasUsageAnalyticsService.listFunctions();
      expect(fns).toEqual(expect.arrayContaining(['deposit', 'withdraw', 'borrow', 'repay', 'liquidation', 'flash_loan']));
    });
  });

  describe('getAllStats', () => {
    it('returns stats for every tracked function', () => {
      const all = gasUsageAnalyticsService.getAllStats('30d');
      const names = all.map((s) => s.functionName);
      expect(names).toEqual(expect.arrayContaining(['deposit', 'borrow']));
      for (const stat of all) {
        expect(stat.sampleCount).toBeGreaterThan(0);
      }
    });
  });

  describe('detectAnomalies', () => {
    it('flags a sample far outside the normal distribution', () => {
      const fn = `anomaly_fn_${Math.random().toString(36).slice(2)}`;
      for (let i = 0; i < 50; i++) {
        gasUsageAnalyticsService.recordSample({ functionName: fn, gasUsed: 1000 + (i % 5), timestamp: Date.now() });
      }
      gasUsageAnalyticsService.recordSample({ functionName: fn, gasUsed: 50000, timestamp: Date.now() });

      const anomalies = gasUsageAnalyticsService.detectAnomalies('24h', 3);
      const found = anomalies.find((a) => a.functionName === fn);
      expect(found).toBeDefined();
      expect(found!.gasUsed).toBe(50000);
      expect(found!.deviations).toBeGreaterThan(3);
    });

    it('does not flag anything for a stable, low-variance series', () => {
      const fn = `stable_fn_${Math.random().toString(36).slice(2)}`;
      for (let i = 0; i < 20; i++) {
        gasUsageAnalyticsService.recordSample({ functionName: fn, gasUsed: 1000, timestamp: Date.now() });
      }
      const anomalies = gasUsageAnalyticsService.detectAnomalies('24h', 3);
      expect(anomalies.find((a) => a.functionName === fn)).toBeUndefined();
    });
  });

  describe('compareFunctions', () => {
    it('computes the relative delta between two functions', () => {
      const comparison = gasUsageAnalyticsService.compareFunctions('deposit', 'repay', '30d');
      expect(comparison.functionA.functionName).toBe('deposit');
      expect(comparison.functionB.functionName).toBe('repay');
      expect(typeof comparison.averageDeltaPct).toBe('number');
    });
  });

  describe('getCalldataCorrelation', () => {
    it('reports a positive correlation for seeded lending functions', () => {
      const correlation = gasUsageAnalyticsService.getCalldataCorrelation('deposit', '30d');
      expect(correlation.sampleCount).toBeGreaterThan(0);
      expect(correlation.correlationCoefficient).toBeGreaterThan(0);
      expect(correlation.correlationCoefficient).toBeLessThanOrEqual(1);
    });

    it('returns 0 correlation when no calldata sizes are present', () => {
      const fn = `no_calldata_fn_${Math.random().toString(36).slice(2)}`;
      gasUsageAnalyticsService.recordSample({ functionName: fn, gasUsed: 100, timestamp: Date.now() });
      const correlation = gasUsageAnalyticsService.getCalldataCorrelation(fn, '24h');
      expect(correlation.sampleCount).toBe(0);
      expect(correlation.correlationCoefficient).toBe(0);
    });
  });

  describe('getTrend', () => {
    it('buckets samples into daily points', () => {
      const trend = gasUsageAnalyticsService.getTrend('borrow', 'daily', '30d');
      expect(trend.functionName).toBe('borrow');
      expect(trend.granularity).toBe('daily');
      expect(trend.points.length).toBeGreaterThan(0);
    });
  });

  describe('getFunctionReport', () => {
    it('returns a consolidated report with a recommendation', () => {
      const report = gasUsageAnalyticsService.getFunctionReport('liquidation', '30d');
      expect(report.functionName).toBe('liquidation');
      expect(report.stats.sampleCount).toBeGreaterThan(0);
      expect(typeof report.recommendation).toBe('string');
      expect(report.recommendation.length).toBeGreaterThan(0);
    });
  });
});
