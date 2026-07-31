import { yieldCurveService, CurveType } from '../services/yieldCurve.service';

describe('YieldCurveService', () => {
  describe('calculateBorrowRate', () => {
    it('should calculate piecewise linear rate correctly', () => {
      const config = {
        curveType: CurveType.PIECEWISE_LINEAR,
        baseRateBps: 200,
        kinkUtilizationBps: 8000,
        slope1Bps: 1000,
        slope2Bps: 6000,
        polyCoeffABps: 500,
        polyCoeffBBps: 1500,
        reserveFactorBps: 1000,
        rateFloorBps: 100,
        rateCeilingBps: 10000,
      };

      expect(yieldCurveService.calculateBorrowRate(config, 0)).toBe(200);
      expect(yieldCurveService.calculateBorrowRate(config, 8000)).toBe(1200);
      expect(yieldCurveService.calculateBorrowRate(config, 10000)).toBe(7200);
    });

    it('should calculate polynomial rate correctly', () => {
      const config = {
        curveType: CurveType.POLYNOMIAL,
        baseRateBps: 200,
        kinkUtilizationBps: 8000,
        slope1Bps: 1000,
        slope2Bps: 6000,
        polyCoeffABps: 1000,
        polyCoeffBBps: 2000,
        reserveFactorBps: 1000,
        rateFloorBps: 100,
        rateCeilingBps: 10000,
      };

      const rate = yieldCurveService.calculateBorrowRate(config, 5000);
      expect(rate).toBe(1450);
    });

    it('should respect rate ceiling and floor', () => {
      const config = {
        curveType: CurveType.PIECEWISE_LINEAR,
        baseRateBps: 200,
        kinkUtilizationBps: 8000,
        slope1Bps: 1000,
        slope2Bps: 20000,
        polyCoeffABps: 500,
        polyCoeffBBps: 1500,
        reserveFactorBps: 1000,
        rateFloorBps: 500,
        rateCeilingBps: 5000,
      };

      expect(yieldCurveService.calculateBorrowRate(config, 0)).toBe(500); // Floor enforced
      expect(yieldCurveService.calculateBorrowRate(config, 10000)).toBe(5000); // Ceiling enforced
    });
  });

  describe('predictYieldCurve', () => {
    it('should generate yield curve points across utilization levels', () => {
      const prediction = yieldCurveService.predictYieldCurve({}, 1000);
      expect(prediction.points).toHaveLength(11); // 0, 10, 20, ..., 100%
      expect(prediction.optimalKinkBps).toBeGreaterThan(0);
      expect(prediction.summary.riskCategory).toBeDefined();
    });
  });

  describe('optimizeRateParameters', () => {
    it('should calculate optimized configuration and revenue gain', () => {
      const currentConfig = {
        curveType: CurveType.PIECEWISE_LINEAR,
        baseRateBps: 200,
        kinkUtilizationBps: 8000,
        slope1Bps: 1000,
        slope2Bps: 6000,
        polyCoeffABps: 500,
        polyCoeffBBps: 1500,
        reserveFactorBps: 1000,
        rateFloorBps: 100,
        rateCeilingBps: 10000,
      };

      const opt = yieldCurveService.optimizeRateParameters({
        currentConfig,
        targetUtilizationBps: 7500,
      });

      expect(opt.recommendedConfig.kinkUtilizationBps).toBe(7500);
      expect(opt.optimalPoint.utilizationBps).toBe(7500);
    });
  });

  describe('runStressTest', () => {
    it('should evaluate liquidity shock scenarios', () => {
      const res = yieldCurveService.runStressTest({
        config: {} as any,
        baseUtilizationBps: 7000,
        shocksBps: [-1000, 1000, 2000],
      });

      expect(res.basePoint.utilizationBps).toBe(7000);
      expect(res.shockResults).toHaveLength(3);
      expect(res.shockResults[1].resultingUtilizationBps).toBe(8000);
    });
  });
});
