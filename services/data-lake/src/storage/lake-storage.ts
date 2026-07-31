import { promises as fs } from 'node:fs';
import path from 'node:path';
import { groupByPartition } from '../etl/partition.js';
import { encodeParquet, toParquetRows } from './parquet.js';
import type { DataLakeConfig, RawLakeRecord } from '../types.js';
import type { Logger } from '../utils/logger.js';

export interface WriteResult {
  partitions: number;
  files: string[];
  rows: number;
}

/**
 * Persist raw records as Parquet files partitioned by date and event_type.
 */
export class LakeStorage {
  constructor(
    private readonly config: DataLakeConfig,
    private readonly logger: Logger
  ) {}

  async writeRaw(records: RawLakeRecord[]): Promise<WriteResult> {
    const groups = groupByPartition(records);
    const files: string[] = [];

    for (const [partition, group] of groups) {
      const dir = path.join(this.config.root, 'raw', partition);
      await fs.mkdir(dir, { recursive: true });
      const fileName = `part-${Date.now()}-${group.length}.parquet`;
      const filePath = path.join(dir, fileName);
      const buffer = encodeParquet(toParquetRows(group));
      await fs.writeFile(filePath, buffer);
      files.push(filePath);
      this.logger.info('Wrote parquet partition', {
        partition,
        rows: group.length,
        filePath,
        backend: this.config.storageBackend,
      });
    }

    return { partitions: groups.size, files, rows: records.length };
  }

  /**
   * Apply raw retention: delete partition directories older than rawRetentionDays.
   * Aggregated datasets under /agg are retained indefinitely.
   */
  async applyRawRetention(now: Date = new Date()): Promise<string[]> {
    const rawRoot = path.join(this.config.root, 'raw');
    const deleted: string[] = [];
    let entries: string[] = [];
    try {
      entries = await fs.readdir(rawRoot);
    } catch {
      return deleted;
    }

    const cutoff = new Date(now);
    cutoff.setUTCDate(cutoff.getUTCDate() - this.config.rawRetentionDays);
    const cutoffDate = cutoff.toISOString().slice(0, 10);

    for (const entry of entries) {
      if (!entry.startsWith('date=')) continue;
      const date = entry.slice('date='.length);
      if (date < cutoffDate) {
        const full = path.join(rawRoot, entry);
        await fs.rm(full, { recursive: true, force: true });
        deleted.push(full);
      }
    }
    return deleted;
  }
}
