#!/usr/bin/env node
import { loadConfig } from '../config.js';
import { createLogger } from '../utils/logger.js';
import { LakeStorage } from '../storage/lake-storage.js';
import { DailyEtlJob, StaticEventSource } from './daily-job.js';
import { CURRENT_SCHEMA_VERSION } from '../types.js';

async function main(): Promise<void> {
  const day = process.argv[2] || new Date().toISOString().slice(0, 10);
  const config = loadConfig();
  const logger = createLogger(config.logLevel);
  const storage = new LakeStorage(config, logger);

  // In production, replace StaticEventSource with RPC/archiver-backed source.
  const source = new StaticEventSource([
    {
      ledger: 1,
      txHash: 'bootstrap',
      contractId: config.contractId || 'UNKNOWN',
      eventType: 'deposit_event',
      eventIndex: 0,
      blockTimestamp: new Date(`${day}T12:00:00.000Z`),
      userAddress: null,
      assetAddress: null,
      amount: null,
      payload: { note: 'replace with RPC ingest' },
      schemaVersion: CURRENT_SCHEMA_VERSION,
    },
  ]);

  const job = new DailyEtlJob(config, source, storage, logger);
  const result = await job.run(day);
  logger.info('ETL finished', result);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
