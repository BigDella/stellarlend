import type { LiquidationReport } from '../types.js';

/**
 * Dashboard-oriented API payload derived from a liquidation report.
 * Mount under /api/analytics/liquidations/dashboard in the main API if desired.
 */
export function toDashboardCharts(report: LiquidationReport) {
  return {
    summary: {
      totalLiquidations: report.totalLiquidations,
      meanNetProfit: report.profitability.meanProfit,
      profitableShare: report.profitability.profitableShare,
      anomalyCount: report.anomalies.length,
    },
    charts: {
      profitabilityHistogram: {
        p25: report.profitability.p25,
        median: report.profitability.medianProfit,
        p75: report.profitability.p75,
        p95: report.profitability.p95,
      },
      hourOfDay: report.hourOfDay.map((b) => ({ label: b.key, value: b.count })),
      dayOfWeek: report.dayOfWeek.map((b) => ({ label: b.key, value: b.count })),
      collateralFrequency: report.collateralFrequency.map((c) => ({
        label: c.asset,
        value: c.count,
      })),
    },
    anomalies: report.anomalies.slice(0, 50),
    generatedAt: report.generatedAt,
    period: report.period,
  };
}
