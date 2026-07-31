import { loadConfig } from './config.js';
import { createLogger } from './utils/logger.js';
import { SorobanEventFetcher } from './rpc/event-fetcher.js';
import { PostgresEventRepository, InMemoryEventRepository } from './db/repository.js';
import { EventArchiver } from './archive/archiver.js';

export { loadConfig } from './config.js';
export { EventArchiver } from './archive/archiver.js';
export { normalizeEvent } from './archive/normalize.js';
export { verifyLedgerIntegrity, summarizeIntegrity } from './archive/integrity.js';
export { PROTOCOL_EVENT_TOPICS, EVENT_TYPE_IDS } from './types.js';
export type * from './types.js';

async function main(): Promise<void> {
  const config = loadConfig();
  const logger = createLogger(config.logLevel);
  const fetcher = new SorobanEventFetcher(config.stellarRpcUrl, config.contractId, logger);

  const useMemory = process.env.ARCHIVER_IN_MEMORY === 'true';
  const repo = useMemory
    ? new InMemoryEventRepository()
    : new PostgresEventRepository(config.databaseUrl);

  const archiver = new EventArchiver(config, fetcher, repo, logger);

  const shutdown = async () => {
    archiver.stop();
    await repo.close();
    process.exit(0);
  };
  process.on('SIGINT', () => void shutdown());
  process.on('SIGTERM', () => void shutdown());

  if (process.env.ARCHIVER_BACKFILL === 'true') {
    const result = await archiver.backfill();
    logger.info('Backfill finished', result);
  }

  archiver.start();

  // Daily retention job
  const dayMs = 24 * 60 * 60 * 1000;
  setInterval(() => {
    void archiver.runRetention().catch((err) => {
      logger.error('Retention job failed', {
        error: err instanceof Error ? err.message : String(err),
      });
    });
  }, dayMs);
}

const isDirectRun =
  process.argv[1]?.includes('event-archiver') ||
  process.argv[1]?.endsWith('index.ts') ||
  process.argv[1]?.endsWith('index.js');

if (isDirectRun) {
  main().catch((err) => {
    console.error(err);
    process.exit(1);
  });
}
