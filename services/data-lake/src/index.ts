import { loadConfig } from './config.js';
import { createLogger } from './utils/logger.js';
import { LakeStorage } from './storage/lake-storage.js';
import { DailyEtlJob, StaticEventSource } from './etl/daily-job.js';

export { loadConfig } from './config.js';
export { encodeParquet, decodeParquet, toParquetRows, evolveSchema } from './storage/parquet.js';
export { buildPartition, groupByPartition } from './etl/partition.js';
export { DailyEtlJob, StaticEventSource } from './etl/daily-job.js';
export { LakeStorage } from './storage/lake-storage.js';
export {
  buildAthenaCreateTableSql,
  buildRepairPartitionsSql,
  RAW_TRANSACTIONS_COLUMNS,
} from './catalog/athena.js';
export type * from './types.js';

async function main(): Promise<void> {
  const config = loadConfig();
  const logger = createLogger(config.logLevel);
  logger.info('Data lake worker ready', {
    root: config.root,
    backend: config.storageBackend,
    retentionDays: config.rawRetentionDays,
  });

  // Keep process alive for scheduled daily ETL via external CronJob calling etl:daily
  const storage = new LakeStorage(config, logger);
  const source = new StaticEventSource([]);
  const job = new DailyEtlJob(config, source, storage, logger);
  void job;
}

const isDirectRun =
  process.argv[1]?.includes('data-lake') ||
  process.argv[1]?.endsWith('index.ts') ||
  process.argv[1]?.endsWith('index.js');

if (isDirectRun) {
  main().catch((err) => {
    console.error(err);
    process.exit(1);
  });
}
