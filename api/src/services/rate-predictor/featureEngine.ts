import { HistoricalRateSnapshot } from './historicalRateStore';

export interface EngineeredFeatures {
  asset: string;
  currentUtilizationBps: number;
  currentBorrowRateBps: number;
  currentSupplyRateBps: number;
  utilization7dMeanBps: number;
  utilization30dMeanBps: number;
  rate7dVelocityBps: number; // Rate change per day over 7d
  volatility30d: number;
  tvlGrowthRate30d: number;
}

export class FeatureEngine {
  public extractFeatures(history: HistoricalRateSnapshot[]): EngineeredFeatures {
    if (!history || history.length === 0) {
      return {
        asset: 'UNKNOWN',
        currentUtilizationBps: 5000,
        currentBorrowRateBps: 500,
        currentSupplyRateBps: 350,
        utilization7dMeanBps: 5000,
        utilization30dMeanBps: 5000,
        rate7dVelocityBps: 0,
        volatility30d: 0.03,
        tvlGrowthRate30d: 0,
      };
    }

    const latest = history[history.length - 1]!;
    const n = history.length;

    // 7-day subset
    const last7d = history.slice(-Math.min(7, n));
    const util7dSum = last7d.reduce((acc, s) => acc + s.utilizationBps, 0);
    const util7dMean = Math.round(util7dSum / last7d.length);

    // 30-day subset
    const last30d = history.slice(-Math.min(30, n));
    const util30dSum = last30d.reduce((acc, s) => acc + s.utilizationBps, 0);
    const util30dMean = Math.round(util30dSum / last30d.length);

    // Rate velocity over 7d
    const oldest7d = last7d[0]!;
    const rateDiff = latest.borrowRateBps - oldest7d.borrowRateBps;
    const daysDiff = Math.max(1, (latest.timestamp - oldest7d.timestamp) / (86400 * 1000));
    const rate7dVelocity = Math.round((rateDiff / daysDiff) * 100) / 100;

    // Volatility 30d (std dev of rates)
    const rates30d = last30d.map((s) => s.borrowRateBps);
    const meanRate = rates30d.reduce((a, b) => a + b, 0) / rates30d.length;
    const variance = rates30d.reduce((acc, r) => acc + Math.pow(r - meanRate, 2), 0) / rates30d.length;
    const volatility30d = Math.round((Math.sqrt(variance) / meanRate) * 10000) / 10000;

    // TVL growth 30d
    const oldest30d = last30d[0]!;
    const tvlGrowthRate30d = oldest30d.totalValueLocked > 0
      ? Math.round(((latest.totalValueLocked - oldest30d.totalValueLocked) / oldest30d.totalValueLocked) * 10000) / 100
      : 0;

    return {
      asset: latest.asset,
      currentUtilizationBps: latest.utilizationBps,
      currentBorrowRateBps: latest.borrowRateBps,
      currentSupplyRateBps: latest.supplyRateBps,
      utilization7dMeanBps: util7dMean,
      utilization30dMeanBps: util30dMean,
      rate7dVelocityBps: rate7dVelocity,
      volatility30d,
      tvlGrowthRate30d,
    };
  }
}

export const featureEngine = new FeatureEngine();
