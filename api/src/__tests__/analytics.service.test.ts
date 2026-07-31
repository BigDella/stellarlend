import {
  getRateVolatility,
  getWeightedAverageRates,
  getRateChangeEvents,
  getRateHistoryRange,
} from '../services/analytics.service';
import { StellarService } from '../services/stellar.service';
import { redisCacheService } from '../services/redisCache.service';
import { ValidationError } from '../utils/errors';

describe('analytics.service — historical rate analytics', () => {
  let getPoolRateAtSpy: jest.SpyInstance;

  beforeEach(() => {
    redisCacheService.clearAllForTests();
    // Deterministic, monotonically increasing borrow APY per call so
    // volatility/weighted-average/rate-change tests have real variance to
    // observe (the default simulated implementation returns a constant
    // rate per pool address, which would make every metric trivially zero).
    let call = 0;
    getPoolRateAtSpy = jest
      .spyOn(StellarService.prototype, 'getPoolRateAt')
      .mockImplementation(async () => {
        call += 1;
        const borrowApy = 0.05 + (call % 5) * 0.01;
        return {
          depositApy: borrowApy * 0.7,
          borrowApy,
          utilizationRate: 0.5,
        };
      });
  });

  afterEach(() => {
    getPoolRateAtSpy.mockRestore();
  });

  describe('getRateVolatility', () => {
    it('computes a rolling standard deviation with the requested window size', async () => {
      const result = await getRateVolatility({ timeRange: '7d', poolAddress: 'pool_a' }, 5);
      expect(result.length).toBeGreaterThan(0);
      for (const point of result) {
        expect(point.windowSize).toBe(5);
        expect(point.borrowApyStdDev).toBeGreaterThanOrEqual(0);
        expect(point.depositApyStdDev).toBeGreaterThanOrEqual(0);
      }
      // With varying rates, at least one window should show non-zero volatility.
      expect(result.some((p) => p.borrowApyStdDev > 0)).toBe(true);
    });

    it('returns an empty array when there are fewer samples than the window size', async () => {
      const result = await getRateVolatility({ timeRange: '7d', poolAddress: 'pool_b' }, 1000);
      expect(result).toEqual([]);
    });
  });

  describe('getWeightedAverageRates', () => {
    it('buckets rate points by the requested granularity', async () => {
      const result = await getWeightedAverageRates(
        { timeRange: '30d', poolAddress: 'pool_c' },
        'weekly'
      );
      expect(result.length).toBeGreaterThan(0);
      for (const bucket of result) {
        expect(bucket.granularity).toBe('weekly');
        expect(bucket.sampleCount).toBeGreaterThan(0);
        expect(new Date(bucket.periodStart).getTime()).toBeLessThan(
          new Date(bucket.periodEnd).getTime()
        );
      }
    });

    it('sorts buckets chronologically', async () => {
      const result = await getWeightedAverageRates(
        { timeRange: '30d', poolAddress: 'pool_d' },
        'daily'
      );
      const starts = result.map((b) => new Date(b.periodStart).getTime());
      const sorted = [...starts].sort((a, b) => a - b);
      expect(starts).toEqual(sorted);
    });
  });

  describe('getRateChangeEvents', () => {
    it('flags borrow-rate moves at or above the threshold', async () => {
      const events = await getRateChangeEvents({ timeRange: '7d', poolAddress: 'pool_e' }, 10);
      expect(events.length).toBeGreaterThan(0);
      for (const event of events) {
        expect(Math.abs(event.deltaBps)).toBeGreaterThanOrEqual(10);
        expect(['increase', 'decrease']).toContain(event.changeType);
        expect(event.governanceActionId).toBeUndefined();
      }
    });

    it('finds no events when the threshold is set impossibly high', async () => {
      const events = await getRateChangeEvents(
        { timeRange: '7d', poolAddress: 'pool_f' },
        1_000_000
      );
      expect(events).toEqual([]);
    });
  });

  describe('getRateHistoryRange', () => {
    it('returns daily-bucketed points across an explicit date range', async () => {
      const from = '2024-01-01T00:00:00.000Z';
      const to = '2024-01-05T00:00:00.000Z';
      const result = await getRateHistoryRange({ asset: 'pool_g', from, to, granularity: 'daily' });
      expect(result.length).toBe(5); // Jan 1..5 inclusive at daily granularity
      expect(result[0]!.poolAddress).toBe('pool_g');
    });

    it('rejects a `from` after `to`', async () => {
      await expect(
        getRateHistoryRange({
          asset: 'pool_h',
          from: '2024-01-05T00:00:00.000Z',
          to: '2024-01-01T00:00:00.000Z',
        })
      ).rejects.toThrow(ValidationError);
    });

    it('rejects an invalid date string', async () => {
      await expect(
        getRateHistoryRange({ asset: 'pool_i', from: 'not-a-date', to: '2024-01-01T00:00:00.000Z' })
      ).rejects.toThrow(ValidationError);
    });

    it('rejects a range that would exceed the maximum bucket count', async () => {
      await expect(
        getRateHistoryRange({
          asset: 'pool_j',
          from: '2000-01-01T00:00:00.000Z',
          to: '2024-01-01T00:00:00.000Z',
          granularity: 'hourly',
        })
      ).rejects.toThrow(ValidationError);
    });

    it('defaults to the last 7 days at daily granularity when no range is given', async () => {
      const result = await getRateHistoryRange({ asset: 'pool_k' });
      expect(result.length).toBeGreaterThan(0);
      expect(result.length).toBeLessThanOrEqual(8);
    });
  });
});
