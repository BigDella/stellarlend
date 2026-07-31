import { CURRENT_SCHEMA_VERSION, type RawLakeRecord } from '../types.js';
import { LakeStorage } from '../storage/lake-storage.js';
import type { DataLakeConfig } from '../types.js';
import type { Logger } from '../utils/logger.js';

export interface EventSource {
  fetchDay(day: string): Promise<RawLakeRecord[]>;
}

/**
 * Daily ETL: pull raw chain data for a UTC day and land Parquet partitions.
 */
export class DailyEtlJob {
  constructor(
    private readonly config: DataLakeConfig,
    private readonly source: EventSource,
    private readonly storage: LakeStorage,
    private readonly logger: Logger
  ) {}

  async run(day: string): Promise<{ rows: number; files: string[] }> {
    this.logger.info('Starting daily data-lake ETL', { day });
    const records = await this.source.fetchDay(day);
    const normalized = records.map((r) => ({
      ...r,
      schemaVersion: r.schemaVersion || CURRENT_SCHEMA_VERSION,
      contractId: r.contractId || this.config.contractId,
    }));
    const result = await this.storage.writeRaw(normalized);
    const deleted = await this.storage.applyRawRetention();
    this.logger.info('Daily ETL complete', {
      day,
      rows: result.rows,
      files: result.files.length,
      retentionDeleted: deleted.length,
    });
    return { rows: result.rows, files: result.files };
  }
}

/**
 * In-memory / test event source.
 */
export class StaticEventSource implements EventSource {
  constructor(private readonly records: RawLakeRecord[]) {}

  async fetchDay(day: string): Promise<RawLakeRecord[]> {
    return this.records.filter((r) => r.blockTimestamp.toISOString().slice(0, 10) === day);
  }
}
