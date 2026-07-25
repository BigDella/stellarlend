/**
 * Shared types for the StellarLend on-chain event archiver.
 */

export const PROTOCOL_EVENT_TOPICS = [
  'deposit_event',
  'withdrawal_event',
  'borrow_event',
  'repay_event',
  'liquidation_event',
] as const;

export type ProtocolEventTopic = (typeof PROTOCOL_EVENT_TOPICS)[number];

export const EVENT_TYPE_IDS: Record<ProtocolEventTopic, number> = {
  deposit_event: 1,
  withdrawal_event: 2,
  borrow_event: 3,
  repay_event: 4,
  liquidation_event: 5,
};

export interface RawSorobanEvent {
  ledger: number;
  txHash: string;
  eventIndex: number;
  contractId: string;
  topic: string;
  topics: unknown[];
  value: unknown;
  timestamp: Date;
}

export interface ArchivedEvent {
  ledger: number;
  txHash: string;
  eventIndex: number;
  contractId: string;
  eventName: ProtocolEventTopic;
  eventTypeId: number;
  blockTimestamp: Date;
  topics: unknown[];
  payload: Record<string, unknown>;
  userAddress: string | null;
  assetAddress: string | null;
  amount: string | null;
}

export interface SyncState {
  lastLedger: number;
  eventsArchived: number;
  lastSyncedAt: Date | null;
}

export interface LedgerIntegrityResult {
  ledger: number;
  expectedCount: number | null;
  archivedCount: number;
  integrityOk: boolean;
}

export interface EventArchiverConfig {
  stellarNetwork: 'testnet' | 'mainnet';
  stellarRpcUrl: string;
  contractId: string;
  databaseUrl: string;
  startLedger: number;
  pollIntervalMs: number;
  batchSize: number;
  detailRetentionDays: number;
  logLevel: 'debug' | 'info' | 'warn' | 'error';
}
