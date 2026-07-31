import { parquetWriteBuffer } from 'hyparquet-writer';
import { parquetReadObjects } from 'hyparquet';
import { CURRENT_SCHEMA_VERSION, type RawLakeRecord } from '../types.js';

export interface FlatParquetRow {
  ledger: number;
  tx_hash: string;
  contract_id: string;
  event_type: string;
  event_index: number;
  block_timestamp: number;
  user_address: string | null;
  asset_address: string | null;
  amount: string | null;
  payload_json: string;
  schema_version: number;
}

/**
 * Flatten lake records into the Glue/Athena column layout.
 */
export function toParquetRows(records: RawLakeRecord[]): FlatParquetRow[] {
  return records.map((record) => ({
    ledger: record.ledger,
    tx_hash: record.txHash,
    contract_id: record.contractId,
    event_type: record.eventType,
    event_index: record.eventIndex,
    block_timestamp: record.blockTimestamp.getTime(),
    user_address: record.userAddress,
    asset_address: record.assetAddress,
    amount: record.amount,
    payload_json: JSON.stringify(record.payload),
    schema_version: record.schemaVersion || CURRENT_SCHEMA_VERSION,
  }));
}

function toArrayBuffer(buffer: Buffer): ArrayBuffer {
  return buffer.buffer.slice(buffer.byteOffset, buffer.byteOffset + buffer.byteLength) as ArrayBuffer;
}

/**
 * Encode rows as real Apache Parquet (readable by Athena / Spark / DuckDB / hyparquet).
 */
export function encodeParquet(rows: FlatParquetRow[]): Buffer {
  const arrayBuffer = parquetWriteBuffer({
    columnData: [
      { name: 'ledger', data: rows.map((r) => BigInt(r.ledger)), type: 'INT64' },
      { name: 'tx_hash', data: rows.map((r) => r.tx_hash), type: 'BYTE_ARRAY' },
      { name: 'contract_id', data: rows.map((r) => r.contract_id), type: 'BYTE_ARRAY' },
      { name: 'event_type', data: rows.map((r) => r.event_type), type: 'BYTE_ARRAY' },
      { name: 'event_index', data: rows.map((r) => r.event_index), type: 'INT32' },
      {
        name: 'block_timestamp',
        data: rows.map((r) => BigInt(r.block_timestamp)),
        type: 'INT64',
      },
      { name: 'user_address', data: rows.map((r) => r.user_address), type: 'BYTE_ARRAY' },
      { name: 'asset_address', data: rows.map((r) => r.asset_address), type: 'BYTE_ARRAY' },
      { name: 'amount', data: rows.map((r) => r.amount), type: 'BYTE_ARRAY' },
      { name: 'payload_json', data: rows.map((r) => r.payload_json), type: 'BYTE_ARRAY' },
      { name: 'schema_version', data: rows.map((r) => r.schema_version), type: 'INT32' },
    ],
  });
  return Buffer.from(arrayBuffer);
}

export async function decodeParquet(buffer: Buffer): Promise<FlatParquetRow[]> {
  const objects = await parquetReadObjects({ file: toArrayBuffer(buffer) });
  return objects.map((row) => {
    const r = row as Record<string, unknown>;
    return {
      ledger: Number(r.ledger),
      tx_hash: String(r.tx_hash ?? ''),
      contract_id: String(r.contract_id ?? ''),
      event_type: String(r.event_type ?? ''),
      event_index: Number(r.event_index ?? 0),
      block_timestamp: Number(r.block_timestamp ?? 0),
      user_address: r.user_address == null ? null : String(r.user_address),
      asset_address: r.asset_address == null ? null : String(r.asset_address),
      amount: r.amount == null ? null : String(r.amount),
      payload_json: String(r.payload_json ?? '{}'),
      schema_version: Number(r.schema_version ?? CURRENT_SCHEMA_VERSION),
    };
  });
}

/**
 * Schema evolution: merge reader schema with writer schema using additive rules.
 */
export function evolveSchema(
  existingFields: string[],
  incomingFields: string[]
): { fields: string[]; added: string[]; removed: string[] } {
  const existing = new Set(existingFields);
  const added = incomingFields.filter((f) => !existing.has(f));
  const removed = existingFields.filter((f) => !incomingFields.includes(f));
  if (removed.length > 0) {
    throw new Error(
      `Breaking schema change: removed fields ${removed.join(', ')}. Additive evolution only.`
    );
  }
  return { fields: [...existingFields, ...added], added, removed };
}
