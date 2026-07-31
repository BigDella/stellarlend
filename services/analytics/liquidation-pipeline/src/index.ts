import { fileURLToPath } from 'node:url';
import winston from 'winston';
import { pipelineFromRawEvents, writeReportFiles } from './reports/generate.js';
import { toDashboardCharts } from './reports/dashboard.js';

export { extractLiquidations, computeLiquidationMetrics } from './extract/extract.js';
export {
  profitabilityDistribution,
  clusterByHour,
  clusterByDayOfWeek,
  collateralFrequency,
} from './metrics/analytics.js';
export { detectAnomalies } from './anomaly/detect.js';
export { buildReport, writeReportFiles, pipelineFromRawEvents } from './reports/generate.js';
export { toDashboardCharts } from './reports/dashboard.js';
export type * from './types.js';

const logger = winston.createLogger({
  level: process.env.LOG_LEVEL || 'info',
  format: winston.format.combine(winston.format.timestamp(), winston.format.json()),
  defaultMeta: { service: 'liquidation-pipeline' },
  transports: [new winston.transports.Console()],
});

/**
 * Scheduled worker: daily/weekly/monthly report generation.
 */
export class LiquidationPipeline {
  constructor(private readonly reportsDir: string) {}

  async runScheduled(
    period: 'daily' | 'weekly' | 'monthly',
    rawEvents: Array<{
      topic?: string;
      eventName?: string;
      txHash: string;
      ledger: number;
      timestamp: Date | string;
      payload: Record<string, unknown>;
    }>
  ) {
    const to = new Date();
    const from = new Date(to);
    if (period === 'daily') from.setUTCDate(from.getUTCDate() - 1);
    if (period === 'weekly') from.setUTCDate(from.getUTCDate() - 7);
    if (period === 'monthly') from.setUTCDate(from.getUTCDate() - 30);

    const report = pipelineFromRawEvents(rawEvents, period, from, to);
    const paths = await writeReportFiles(report, this.reportsDir);
    const dashboard = toDashboardCharts(report);
    logger.info('Liquidation report generated', {
      period,
      total: report.totalLiquidations,
      anomalies: report.anomalies.length,
      ...paths,
    });
    return { report, dashboard, paths };
  }
}

async function main(): Promise<void> {
  const reportsDir = fileURLToPath(new URL('../reports/', import.meta.url));
  const pipeline = new LiquidationPipeline(reportsDir);
  // CronJob should invoke runScheduled with archiver-fed liquidation events.
  logger.info('Liquidation analysis pipeline ready', { reportsDir });
  void pipeline;
}

const isDirectRun =
  process.argv[1]?.includes('liquidation-pipeline') ||
  process.argv[1]?.endsWith('index.ts') ||
  process.argv[1]?.endsWith('index.js');

if (isDirectRun) {
  main().catch((err) => {
    console.error(err);
    process.exit(1);
  });
}
