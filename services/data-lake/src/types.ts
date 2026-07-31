export type EventType =
  | 'deposit_event'
  | 'withdrawal_event'
  | 'borrow_event'
  | 'repay_event'
  | 'liquidation_event'
  | 'transaction';

export interface RawLakeRecord {
  ledger: number;
  txHash: string;
  contractId: string;
  eventType: EventType | string;
  eventIndex: number;
  blockTimestamp: Date;
  userAddress: string | null;
  assetAddress: string | null;
  amount: string | null;
  payload: Record<string, unknown>;
  schemaVersion: number;
}

export interface DataLakeConfig {
  root: string;
  storageBackend: 'local' | 's3' | 'gcs';
  s3Bucket: string;
  s3Region: string;
  gcsBucket: string;
  rawRetentionDays: number;
  contractId: string;
  stellarRpcUrl: string;
  logLevel: 'debug' | 'info' | 'warn' | 'error';
}

export interface PartitionPath {
  date: string;
  eventType: string;
  relativePath: string;
}

export const CURRENT_SCHEMA_VERSION = 1;
