/**
 * Athena / Glue / Presto compatible catalog definitions for the data lake.
 */

export interface GlueColumn {
  name: string;
  type: string;
  comment?: string;
}

export const RAW_TRANSACTIONS_COLUMNS: GlueColumn[] = [
  { name: 'ledger', type: 'bigint' },
  { name: 'tx_hash', type: 'string' },
  { name: 'contract_id', type: 'string' },
  { name: 'event_type', type: 'string' },
  { name: 'event_index', type: 'int' },
  { name: 'block_timestamp', type: 'bigint', comment: 'epoch millis' },
  { name: 'user_address', type: 'string' },
  { name: 'asset_address', type: 'string' },
  { name: 'amount', type: 'string' },
  { name: 'payload_json', type: 'string' },
  { name: 'schema_version', type: 'int' },
];

export function buildAthenaCreateTableSql(opts: {
  database: string;
  table: string;
  location: string;
}): string {
  const columns = RAW_TRANSACTIONS_COLUMNS.map(
    (c) => `  ${c.name} ${c.type}${c.comment ? ` COMMENT '${c.comment}'` : ''}`
  ).join(',\n');

  return `CREATE EXTERNAL TABLE IF NOT EXISTS ${opts.database}.${opts.table} (
${columns}
)
PARTITIONED BY (date string, event_type string)
STORED AS PARQUET
LOCATION '${opts.location}'
TBLPROPERTIES ('parquet.compress'='UNCOMPRESSED', 'classification'='parquet');`;
}

export function buildRepairPartitionsSql(database: string, table: string): string {
  return `MSCK REPAIR TABLE ${database}.${table};`;
}
