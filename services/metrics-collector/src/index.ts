import { loadConfig } from './config.js';
import { createLogger } from './utils/logger.js';
import { HttpProtocolStatsSource } from './collector/stats-source.js';
import { MetricsCollector } from './collector/collector.js';
import { InMemoryMetricsRepository } from './db/repository.js';
import { PostgresMetricsRepository } from './db/postgres-repository.js';

export { loadConfig } from './config.js';
export { MetricsCollector } from './collector/collector.js';
export { detectGaps, backfillTimestamps } from './collector/gap-detection.js';
export { InMemoryMetricsRepository, resolveMetricColumn, resolveBucket } from './db/repository.js';
export { PostgresMetricsRepository } from './db/postgres-repository.js';
export { toProtocolSample, toAssetSamples } from './collector/stats-source.js';
export type * from './types.js';

async function main(): Promise<void> {
  const config = loadConfig();
  const logger = createLogger(config.logLevel);
  const source = new HttpProtocolStatsSource(config.protocolStatsUrl);
  const repo =
    process.env.METRICS_IN_MEMORY === 'true'
      ? new InMemoryMetricsRepository()
      : new PostgresMetricsRepository(config.databaseUrl);

  const collector = new MetricsCollector(config, source, repo, logger);

  const shutdown = async () => {
    collector.stop();
    await repo.close();
    process.exit(0);
  };
  process.on('SIGINT', () => void shutdown());
  process.on('SIGTERM', () => void shutdown());

  collector.start();
}

const isDirectRun =
  process.argv[1]?.includes('metrics-collector') ||
  process.argv[1]?.endsWith('index.ts') ||
  process.argv[1]?.endsWith('index.js');

if (isDirectRun) {
  main().catch((err) => {
    console.error(err);
    process.exit(1);
  });
}
