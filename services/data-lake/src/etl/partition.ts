import type { PartitionPath, RawLakeRecord } from '../types.js';

/**
 * Build Hive-style partition path: date=YYYY-MM-DD/event_type=<topic>/
 */
export function buildPartition(record: RawLakeRecord): PartitionPath {
  const date = record.blockTimestamp.toISOString().slice(0, 10);
  const eventType = record.eventType || 'unknown';
  return {
    date,
    eventType,
    relativePath: `date=${date}/event_type=${eventType}`,
  };
}

export function groupByPartition(records: RawLakeRecord[]): Map<string, RawLakeRecord[]> {
  const groups = new Map<string, RawLakeRecord[]>();
  for (const record of records) {
    const { relativePath } = buildPartition(record);
    const list = groups.get(relativePath) ?? [];
    list.push(record);
    groups.set(relativePath, list);
  }
  return groups;
}
