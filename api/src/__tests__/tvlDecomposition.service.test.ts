import {
  getTvlBreakdown,
  getTvlAttribution,
  getHistoricalTvlBreakdown,
  getCompetitorTvlComparison,
} from '../services/tvlDecomposition.service';
import { StellarService } from '../services/stellar.service';
import * as etlService from '../services/cross-protocol-etl/etl.service';

describe('tvlDecomposition.service', () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  describe('getTvlBreakdown', () => {
    it('decomposes TVL by asset, user segment, and strategy, summing to 100%', async () => {
      const result = await getTvlBreakdown();

      expect(result.totalTvlUsd).toBeGreaterThan(0);
      expect(result.byAsset.length).toBeGreaterThan(0);

      const assetPct = result.byAsset.reduce((sum, a) => sum + a.percentOfTotal, 0);
      const segmentPct = result.bySegment.reduce((sum, s) => sum + s.percentOfTotal, 0);
      const strategyPct = result.byStrategy.reduce((sum, s) => sum + s.percentOfTotal, 0);

      expect(assetPct).toBeCloseTo(100, 5);
      expect(segmentPct).toBeCloseTo(100, 5);
      expect(strategyPct).toBeCloseTo(100, 5);

      expect(result.bySegment.map((s) => s.segment).sort()).toEqual(['mid', 'retail', 'whale']);
      expect(result.byStrategy.map((s) => s.strategy).sort()).toEqual([
        'direct_deposit',
        'leveraged',
        'yield_farming',
      ]);
    });
  });

  describe('getTvlAttribution', () => {
    it('decomposes TVL change into price and flow effects that sum to the total change', async () => {
      jest
        .spyOn(StellarService.prototype, 'getPoolStateAt')
        .mockImplementation(async (poolAddress: string, timestamp: number) => {
          const isPrevious = timestamp < Math.floor(Date.now() / 1000) - 1000;
          return {
            utilizationRate: isPrevious ? 0.4 : 0.5,
            totalDeposits: isPrevious ? '1000000000' : '1200000000',
            totalBorrows: '500000000',
          };
        });

      const result = await getTvlAttribution('7d');

      expect(result.byAsset.length).toBeGreaterThan(0);
      const sumOfEffects = result.byAsset.reduce(
        (sum, a) => sum + a.priceEffectUsd + a.flowEffectUsd,
        0
      );
      expect(sumOfEffects).toBeCloseTo(result.totalChangeUsd, 5);
      expect(result.waterfall).toHaveLength(4);
      expect(result.waterfall[0]!.label).toBe('Starting TVL');
      expect(result.waterfall[3]!.label).toBe('Ending TVL');
    });

    it('classifies momentum based on the growth rate', async () => {
      jest.spyOn(StellarService.prototype, 'getPoolStateAt').mockImplementation(async () => ({
        utilizationRate: 0.5,
        totalDeposits: '1000000000',
        totalBorrows: '500000000',
      }));

      const result = await getTvlAttribution('7d');
      expect(result.growthRatePercent).toBeCloseTo(0, 5);
      expect(result.momentum).toBe('stable');
    });
  });

  describe('getHistoricalTvlBreakdown', () => {
    it('returns the requested number of time-travel snapshots', async () => {
      const history = await getHistoricalTvlBreakdown('30d', 4);
      expect(history).toHaveLength(4);
      for (const point of history) {
        expect(point.totalTvlUsd).toBeGreaterThanOrEqual(0);
        expect(new Date(point.timestamp).toString()).not.toBe('Invalid Date');
      }
    });
  });

  describe('getCompetitorTvlComparison', () => {
    it('delegates to the cross-protocol ETL market-share computation', async () => {
      const spy = jest.spyOn(etlService, 'getMarketShare').mockResolvedValue([
        { protocol: 'stellarlend', tvlUsd: 25_000_000, marketSharePct: 40 },
        { protocol: 'aave-v3', tvlUsd: 37_500_000, marketSharePct: 60 },
      ]);

      const result = await getCompetitorTvlComparison();

      expect(spy).toHaveBeenCalled();
      expect(result).toHaveLength(2);
      expect(result[0]!.protocol).toBe('stellarlend');
    });
  });
});
