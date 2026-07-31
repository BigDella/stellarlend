export enum CurveType {
  PIECEWISE_LINEAR = 'PIECEWISE_LINEAR',
  POLYNOMIAL = 'POLYNOMIAL',
  NELSON_SIEGEL = 'NELSON_SIEGEL',
}

export interface YieldCurveConfig {
  curveType: CurveType;
  baseRateBps: number;
  kinkUtilizationBps: number;
  slope1Bps: number;
  slope2Bps: number;
  polyCoeffABps: number;
  polyCoeffBBps: number;
  reserveFactorBps: number;
  rateFloorBps: number;
  rateCeilingBps: number;
}

export interface YieldPoint {
  utilizationBps: number;
  utilizationPercentage: number;
  borrowRateBps: number;
  borrowRatePercentage: number;
  supplyRateBps: number;
  supplyRatePercentage: number;
  protocolSpreadBps: number;
  protocolSpreadPercentage: number;
  projectedRevenueBps: number;
  liquidityRiskScore: number; // 0 to 100
}

export interface YieldCurvePredictionResponse {
  config: YieldCurveConfig;
  points: YieldPoint[];
  optimalKinkBps: number;
  maxProjectedRevenueBps: number;
  summary: {
    baseBorrowRatePercentage: number;
    kinkBorrowRatePercentage: number;
    maxBorrowRatePercentage: number;
    optimalSupplyRatePercentage: number;
    riskCategory: 'LOW' | 'MODERATE' | 'HIGH' | 'CRITICAL';
  };
}

export interface OptimizationRequest {
  currentConfig: YieldCurveConfig;
  targetUtilizationBps: number;
  maxAcceptableRiskScore?: number;
}

export interface OptimizationResponse {
  recommendedConfig: YieldCurveConfig;
  currentProjectedRevenueBps: number;
  optimizedProjectedRevenueBps: number;
  revenueGainPercentage: number;
  optimalPoint: YieldPoint;
}

export interface StressTestRequest {
  config: YieldCurveConfig;
  baseUtilizationBps: number;
  shocksBps: number[];
}

export interface StressTestResponse {
  basePoint: YieldPoint;
  shockResults: Array<{
    shockBps: number;
    resultingUtilizationBps: number;
    point: YieldPoint;
  }>;
}

export class YieldCurveService {
  private defaultConfig: YieldCurveConfig = {
    curveType: CurveType.PIECEWISE_LINEAR,
    baseRateBps: 200,          // 2.0%
    kinkUtilizationBps: 8000,  // 80.0%
    slope1Bps: 1000,           // 10.0%
    slope2Bps: 6000,           // 60.0%
    polyCoeffABps: 500,        // 5.0%
    polyCoeffBBps: 1500,       // 15.0%
    reserveFactorBps: 1000,    // 10.0%
    rateFloorBps: 100,         // 1.0%
    rateCeilingBps: 10000,     // 100.0%
  };

  /**
   * Calculates borrow rate in basis points for a given utilization.
   */
  public calculateBorrowRate(config: YieldCurveConfig, utilizationBps: number): number {
    const u = Math.max(0, Math.min(10000, utilizationBps));
    let rawRate = config.baseRateBps;

    switch (config.curveType) {
      case CurveType.POLYNOMIAL: {
        const uRatio = u / 10000;
        const quad = config.polyCoeffABps * (uRatio ** 2);
        const lin = config.polyCoeffBBps * uRatio;
        rawRate = config.baseRateBps + quad + lin;
        break;
      }
      case CurveType.NELSON_SIEGEL: {
        const uRatio = u / 10000;
        const lin = config.slope1Bps * uRatio;
        const curvature = config.slope2Bps * uRatio * (1 - uRatio);
        rawRate = config.baseRateBps + lin + curvature;
        break;
      }
      case CurveType.PIECEWISE_LINEAR:
      default: {
        if (u <= config.kinkUtilizationBps) {
          const varComp = config.kinkUtilizationBps > 0
            ? (u * config.slope1Bps) / config.kinkUtilizationBps
            : 0;
          rawRate = config.baseRateBps + varComp;
        } else {
          const rateAtKink = config.baseRateBps + config.slope1Bps;
          const remainingUtil = 10000 - config.kinkUtilizationBps;
          const jumpComp = remainingUtil > 0
            ? ((u - config.kinkUtilizationBps) * config.slope2Bps) / remainingUtil
            : 0;
          rawRate = rateAtKink + jumpComp;
        }
        break;
      }
    }

    return Math.max(config.rateFloorBps, Math.min(config.rateCeilingBps, Math.round(rawRate)));
  }

  /**
   * Calculates supply rate in basis points.
   */
  public calculateSupplyRate(config: YieldCurveConfig, utilizationBps: number): number {
    const borrowRate = this.calculateBorrowRate(config, utilizationBps);
    const uRatio = Math.max(0, Math.min(10000, utilizationBps)) / 10000;
    const reserveMultiplier = (10000 - config.reserveFactorBps) / 10000;
    return Math.max(0, Math.round(borrowRate * uRatio * reserveMultiplier));
  }

  /**
   * Evaluates risk score (0-100) based on utilization.
   */
  public calculateLiquidityRiskScore(utilizationBps: number): number {
    if (utilizationBps < 5000) {
      return Math.round((utilizationBps * 20) / 5000);
    } else if (utilizationBps < 8000) {
      return Math.round(20 + ((utilizationBps - 5000) * 30) / 3000);
    } else if (utilizationBps < 9500) {
      return Math.round(50 + ((utilizationBps - 8000) * 35) / 1500);
    } else {
      return Math.round(85 + ((utilizationBps - 9500) * 15) / 500);
    }
  }

  /**
   * Computes a single yield point at given utilization.
   */
  public getYieldPoint(config: YieldCurveConfig, utilizationBps: number): YieldPoint {
    const borrowRateBps = this.calculateBorrowRate(config, utilizationBps);
    const supplyRateBps = this.calculateSupplyRate(config, utilizationBps);
    const spreadBps = borrowRateBps - supplyRateBps;
    const revenueBps = Math.round((borrowRateBps * (utilizationBps / 10000) * config.reserveFactorBps) / 10000);
    const riskScore = this.calculateLiquidityRiskScore(utilizationBps);

    return {
      utilizationBps,
      utilizationPercentage: utilizationBps / 100,
      borrowRateBps,
      borrowRatePercentage: borrowRateBps / 100,
      supplyRateBps,
      supplyRatePercentage: supplyRateBps / 100,
      protocolSpreadBps: spreadBps,
      protocolSpreadPercentage: spreadBps / 100,
      projectedRevenueBps: revenueBps,
      liquidityRiskScore: riskScore,
    };
  }

  /**
   * Generates a full yield curve prediction model across 0% to 100% utilization.
   */
  public predictYieldCurve(userConfig?: Partial<YieldCurveConfig>, stepBps: number = 500): YieldCurvePredictionResponse {
    const config: YieldCurveConfig = { ...this.defaultConfig, ...userConfig };
    const points: YieldPoint[] = [];

    let maxRev = -1;
    let optKink = config.kinkUtilizationBps;

    for (let u = 0; u <= 10000; u += stepBps) {
      const point = this.getYieldPoint(config, u);
      points.push(point);

      if (point.projectedRevenueBps > maxRev && point.liquidityRiskScore <= 75) {
        maxRev = point.projectedRevenueBps;
        optKink = u;
      }
    }

    const basePoint = this.getYieldPoint(config, 0);
    const kinkPoint = this.getYieldPoint(config, config.kinkUtilizationBps);
    const maxPoint = this.getYieldPoint(config, 10000);
    const optPoint = this.getYieldPoint(config, optKink);

    let riskCategory: 'LOW' | 'MODERATE' | 'HIGH' | 'CRITICAL' = 'LOW';
    if (kinkPoint.liquidityRiskScore >= 85) riskCategory = 'CRITICAL';
    else if (kinkPoint.liquidityRiskScore >= 50) riskCategory = 'HIGH';
    else if (kinkPoint.liquidityRiskScore >= 20) riskCategory = 'MODERATE';

    return {
      config,
      points,
      optimalKinkBps: optKink,
      maxProjectedRevenueBps: maxRev,
      summary: {
        baseBorrowRatePercentage: basePoint.borrowRatePercentage,
        kinkBorrowRatePercentage: kinkPoint.borrowRatePercentage,
        maxBorrowRatePercentage: maxPoint.borrowRatePercentage,
        optimalSupplyRatePercentage: optPoint.supplyRatePercentage,
        riskCategory,
      },
    };
  }

  /**
   * Optimizes parameters for rate optimization.
   */
  public optimizeRateParameters(request: OptimizationRequest): OptimizationResponse {
    const currentConfig = { ...this.defaultConfig, ...request.currentConfig };
    const targetUtil = Math.max(1000, Math.min(9500, request.targetUtilizationBps));

    const currentPoint = this.getYieldPoint(currentConfig, targetUtil);

    const recommendedConfig: YieldCurveConfig = {
      ...currentConfig,
      kinkUtilizationBps: targetUtil,
      baseRateBps: Math.min(1000, Math.max(50, currentConfig.baseRateBps)),
      slope1Bps: Math.min(3000, Math.max(500, currentConfig.slope1Bps)),
      slope2Bps: Math.min(15000, Math.max(3000, currentConfig.slope2Bps)),
    };

    const optimalPoint = this.getYieldPoint(recommendedConfig, targetUtil);

    const revGain = currentPoint.projectedRevenueBps > 0
      ? ((optimalPoint.projectedRevenueBps - currentPoint.projectedRevenueBps) / currentPoint.projectedRevenueBps) * 100
      : 0;

    return {
      recommendedConfig,
      currentProjectedRevenueBps: currentPoint.projectedRevenueBps,
      optimizedProjectedRevenueBps: optimalPoint.projectedRevenueBps,
      revenueGainPercentage: Math.round(revGain * 100) / 100,
      optimalPoint,
    };
  }

  /**
   * Runs stress test simulations under sudden liquidity shocks.
   */
  public runStressTest(request: StressTestRequest): StressTestResponse {
    const config = { ...this.defaultConfig, ...request.config };
    const basePoint = this.getYieldPoint(config, request.baseUtilizationBps);

    const shockResults = request.shocksBps.map((shockBps) => {
      const resultingUtilizationBps = Math.max(0, Math.min(10000, request.baseUtilizationBps + shockBps));
      const point = this.getYieldPoint(config, resultingUtilizationBps);
      return {
        shockBps,
        resultingUtilizationBps,
        point,
      };
    });

    return {
      basePoint,
      shockResults,
    };
  }
}

export const yieldCurveService = new YieldCurveService();
