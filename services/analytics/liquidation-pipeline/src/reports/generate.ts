import { promises as fs } from 'node:fs';
import path from 'node:path';
import { computeLiquidationMetrics, extractLiquidations } from '../extract/extract.js';
import {
  clusterByDayOfWeek,
  clusterByHour,
  collateralFrequency,
  profitabilityDistribution,
} from '../metrics/analytics.js';
import { detectAnomalies } from '../anomaly/detect.js';
import type { LiquidationEvent, LiquidationReport } from '../types.js';

export function buildReport(
  events: LiquidationEvent[],
  period: LiquidationReport['period'],
  from: Date,
  to: Date
): LiquidationReport {
  const metrics = events.map(computeLiquidationMetrics);
  return {
    period,
    from: from.toISOString(),
    to: to.toISOString(),
    generatedAt: new Date().toISOString(),
    totalLiquidations: metrics.length,
    profitability: profitabilityDistribution(metrics),
    hourOfDay: clusterByHour(metrics),
    dayOfWeek: clusterByDayOfWeek(metrics),
    collateralFrequency: collateralFrequency(metrics),
    anomalies: detectAnomalies(metrics),
  };
}

export async function writeReportFiles(
  report: LiquidationReport,
  reportsDir: string
): Promise<{ jsonPath: string; mdPath: string }> {
  await fs.mkdir(reportsDir, { recursive: true });
  const stamp = report.to.slice(0, 10);
  const jsonPath = path.join(reportsDir, `liquidation-${report.period}-${stamp}.json`);
  const mdPath = path.join(reportsDir, `liquidation-${report.period}-${stamp}.md`);

  await fs.writeFile(jsonPath, JSON.stringify(report, null, 2));
  await fs.writeFile(mdPath, renderMarkdown(report));
  return { jsonPath, mdPath };
}

export function renderMarkdown(report: LiquidationReport): string {
  const topCollateral = report.collateralFrequency
    .slice(0, 5)
    .map((c) => `- ${c.asset}: ${c.count} (${(c.share * 100).toFixed(1)}%)`)
    .join('\n');

  return `# Liquidation ${report.period} report

Generated: ${report.generatedAt}
Window: ${report.from} → ${report.to}

## Summary

- Total liquidations: ${report.totalLiquidations}
- Mean net profit: ${report.profitability.meanProfit.toFixed(4)}
- Median net profit: ${report.profitability.medianProfit.toFixed(4)}
- Profitable share: ${(report.profitability.profitableShare * 100).toFixed(1)}%
- Anomalies: ${report.anomalies.length}

## Collateral frequency

${topCollateral || '- none'}

## Anomalies

${
  report.anomalies
    .slice(0, 10)
    .map((a) => `- ${a.txHash}: ${a.reason} (z=${a.score.toFixed(2)})`)
    .join('\n') || '- none'
}
`;
}

export function pipelineFromRawEvents(
  raw: Array<{
    topic?: string;
    eventName?: string;
    txHash: string;
    ledger: number;
    timestamp: Date | string;
    payload: Record<string, unknown>;
  }>,
  period: LiquidationReport['period'],
  from: Date,
  to: Date
): LiquidationReport {
  const events = extractLiquidations(raw).filter((e) => e.timestamp >= from && e.timestamp <= to);
  return buildReport(events, period, from, to);
}
