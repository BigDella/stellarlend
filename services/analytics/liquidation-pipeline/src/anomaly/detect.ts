import type { Anomaly, LiquidationMetrics } from '../types.js';

/**
 * Flag unusual liquidations using robust z-scores on discount and net profit.
 */
export function detectAnomalies(
  metrics: LiquidationMetrics[],
  zThreshold: number = 3
): Anomaly[] {
  if (metrics.length < 5) return [];

  const discounts = metrics.map((m) => m.discount);
  const profits = metrics.map((m) => m.netProfit);
  const dMean = mean(discounts);
  const dStd = std(discounts, dMean) || 1;
  const pMean = mean(profits);
  const pStd = std(profits, pMean) || 1;

  const anomalies: Anomaly[] = [];
  for (const m of metrics) {
    const dZ = Math.abs((m.discount - dMean) / dStd);
    const pZ = Math.abs((m.netProfit - pMean) / pStd);
    const score = Math.max(dZ, pZ);
    if (score >= zThreshold) {
      anomalies.push({
        txHash: m.txHash,
        reason: dZ >= pZ ? 'unusual_discount' : 'unusual_profit',
        score,
        metrics: m,
      });
    }
  }
  return anomalies.sort((a, b) => b.score - a.score);
}

function mean(values: number[]): number {
  return values.length === 0 ? 0 : values.reduce((a, b) => a + b, 0) / values.length;
}

function std(values: number[], avg: number): number {
  if (values.length === 0) return 0;
  const variance = values.reduce((acc, v) => acc + (v - avg) ** 2, 0) / values.length;
  return Math.sqrt(variance);
}
