import type { MetricsCollector } from '../collector/collector.js';

export async function runProtocolMetricsPipeline(collector: MetricsCollector): Promise<void> {
  await collector.collectOnce();
}

export async function runGapBackfillPipeline(collector: MetricsCollector): Promise<number> {
  return collector.detectAndBackfillGaps();
}
