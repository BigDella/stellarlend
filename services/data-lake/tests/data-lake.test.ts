import { describe, expect, it } from 'vitest';
import { promises as fs } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { buildPartition, groupByPartition } from '../src/etl/partition.js';
import {
  decodeParquet,
  encodeParquet,
  evolveSchema,
  toParquetRows,
} from '../src/storage/parquet.js';
import { LakeStorage } from '../src/storage/lake-storage.js';
import { DailyEtlJob, StaticEventSource } from '../src/etl/daily-job.js';
import { buildAthenaCreateTableSql } from '../src/catalog/athena.js';
import { createLogger } from '../src/utils/logger.js';
import { CURRENT_SCHEMA_VERSION, type RawLakeRecord } from '../src/types.js';
import { loadConfig } from '../src/config.js';

function sample(partial: Partial<RawLakeRecord> = {}): RawLakeRecord {
  return {
    ledger: 100,
    txHash: 'tx1',
    contractId: 'C123',
    eventType: 'deposit_event',
    eventIndex: 0,
    blockTimestamp: new Date('2026-07-01T10:00:00Z'),
    userAddress: 'GUSER',
    assetAddress: null,
    amount: '1000',
    payload: { user: 'GUSER', amount: '1000' },
    schemaVersion: CURRENT_SCHEMA_VERSION,
    ...partial,
  };
}

describe('partitioning', () => {
  it('builds date and event_type partitions', () => {
    const part = buildPartition(sample());
    expect(part.relativePath).toBe('date=2026-07-01/event_type=deposit_event');
  });

  it('groups mixed event types', () => {
    const groups = groupByPartition([
      sample({ eventType: 'deposit_event' }),
      sample({ eventType: 'liquidation_event', txHash: 'tx2' }),
    ]);
    expect(groups.size).toBe(2);
  });
});

describe('parquet encode/decode', () => {
  it('round-trips columnar parquet buffers', async () => {
    const rows = toParquetRows([sample(), sample({ txHash: 'tx2', eventIndex: 1 })]);
    const buffer = encodeParquet(rows);
    expect(buffer.subarray(0, 4).toString()).toBe('PAR1');
    const decoded = await decodeParquet(buffer);
    expect(decoded).toHaveLength(2);
    expect(decoded[0]!.tx_hash).toBe('tx1');
    expect(decoded[1]!.event_index).toBe(1);
  });
});

describe('schema evolution', () => {
  it('allows additive fields', () => {
    const result = evolveSchema(['a', 'b'], ['a', 'b', 'c']);
    expect(result.added).toEqual(['c']);
  });

  it('rejects removed fields', () => {
    expect(() => evolveSchema(['a', 'b'], ['a'])).toThrow(/Breaking schema change/);
  });
});

describe('daily ETL + retention', () => {
  it('writes partitioned parquet and enforces 90-day raw retention', async () => {
    const root = await fs.mkdtemp(path.join(os.tmpdir(), 'lake-'));
    const config = {
      ...loadConfig({ DATA_LAKE_ROOT: root, RAW_RETENTION_DAYS: '90' }),
      root,
      rawRetentionDays: 90,
    };
    const logger = createLogger('error');
    const storage = new LakeStorage(config, logger);
    const source = new StaticEventSource([
      sample({ blockTimestamp: new Date('2026-07-01T10:00:00Z') }),
      sample({
        eventType: 'borrow_event',
        txHash: 'txb',
        blockTimestamp: new Date('2026-07-01T11:00:00Z'),
      }),
      sample({
        txHash: 'old',
        blockTimestamp: new Date('2026-01-01T00:00:00Z'),
      }),
    ]);

    const job = new DailyEtlJob(config, source, storage, logger);
    const result = await job.run('2026-07-01');
    expect(result.rows).toBe(2);
    expect(result.files.length).toBe(2);

    // Seed an expired partition and ensure retention removes it
    const expired = path.join(root, 'raw', 'date=2025-01-01', 'event_type=deposit_event');
    await fs.mkdir(expired, { recursive: true });
    await fs.writeFile(path.join(expired, 'part-old.parquet'), encodeParquet([]));
    const deleted = await storage.applyRawRetention(new Date('2026-07-25T00:00:00Z'));
    expect(deleted.some((p) => p.includes('date=2025-01-01'))).toBe(true);
  });
});

describe('athena catalog', () => {
  it('emits external table DDL', () => {
    const sql = buildAthenaCreateTableSql({
      database: 'stellarlend_dev_lake',
      table: 'raw_transactions',
      location: 's3://bucket/raw/',
    });
    expect(sql).toContain('STORED AS PARQUET');
    expect(sql).toContain('PARTITIONED BY');
  });
});
