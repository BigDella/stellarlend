import pg from 'pg';
import type { ArchivedEvent, SyncState } from '../types.js';

export interface EventRepository {
  ensureContract(contractId: string, network: string): Promise<number>;
  insertEvents(contractSk: number, events: ArchivedEvent[]): Promise<number>;
  getSyncState(): Promise<SyncState>;
  updateSyncState(lastLedger: number, eventsAdded: number): Promise<void>;
  recordLedgerIntegrity(
    ledger: number,
    archivedCount: number,
    expectedCount: number | null,
    integrityOk: boolean
  ): Promise<void>;
  applyRetention(retentionDays: number): Promise<number>;
  close(): Promise<void>;
}

/**
 * In-memory repository used for unit tests and dry-run mode.
 */
export class InMemoryEventRepository implements EventRepository {
  private events: ArchivedEvent[] = [];
  private contracts = new Map<string, number>();
  private nextContractSk = 1;
  private sync: SyncState = { lastLedger: 0, eventsArchived: 0, lastSyncedAt: null };
  readonly integrity: Array<{
    ledger: number;
    archivedCount: number;
    expectedCount: number | null;
    integrityOk: boolean;
  }> = [];

  async ensureContract(contractId: string): Promise<number> {
    const existing = this.contracts.get(contractId);
    if (existing) return existing;
    const sk = this.nextContractSk++;
    this.contracts.set(contractId, sk);
    return sk;
  }

  async insertEvents(_contractSk: number, events: ArchivedEvent[]): Promise<number> {
    const before = this.events.length;
    for (const event of events) {
      const dup = this.events.some(
        (e) =>
          e.ledger === event.ledger &&
          e.txHash === event.txHash &&
          e.eventIndex === event.eventIndex
      );
      if (!dup) this.events.push(event);
    }
    return this.events.length - before;
  }

  async getSyncState(): Promise<SyncState> {
    return { ...this.sync };
  }

  async updateSyncState(lastLedger: number, eventsAdded: number): Promise<void> {
    this.sync = {
      lastLedger: Math.max(this.sync.lastLedger, lastLedger),
      eventsArchived: this.sync.eventsArchived + eventsAdded,
      lastSyncedAt: new Date(),
    };
  }

  async recordLedgerIntegrity(
    ledger: number,
    archivedCount: number,
    expectedCount: number | null,
    integrityOk: boolean
  ): Promise<void> {
    this.integrity.push({ ledger, archivedCount, expectedCount, integrityOk });
  }

  async applyRetention(retentionDays: number): Promise<number> {
    const cutoff = Date.now() - retentionDays * 24 * 60 * 60 * 1000;
    const before = this.events.length;
    this.events = this.events.filter((e) => e.blockTimestamp.getTime() >= cutoff);
    return before - this.events.length;
  }

  async close(): Promise<void> {
    /* no-op */
  }

  getAllEvents(): ArchivedEvent[] {
    return [...this.events];
  }
}

/**
 * PostgreSQL / TimescaleDB repository backing the star schema.
 */
export class PostgresEventRepository implements EventRepository {
  private readonly pool: pg.Pool;

  constructor(databaseUrl: string) {
    this.pool = new pg.Pool({ connectionString: databaseUrl });
  }

  async ensureContract(contractId: string, network: string): Promise<number> {
    const result = await this.pool.query<{ contract_sk: number }>(
      `INSERT INTO dim_contract (contract_id, network)
       VALUES ($1, $2)
       ON CONFLICT (contract_id) DO UPDATE SET contract_id = EXCLUDED.contract_id
       RETURNING contract_sk`,
      [contractId, network]
    );
    const row = result.rows[0];
    if (!row) throw new Error('Failed to upsert dim_contract');
    return row.contract_sk;
  }

  async insertEvents(contractSk: number, events: ArchivedEvent[]): Promise<number> {
    if (events.length === 0) return 0;

    let inserted = 0;
    const client = await this.pool.connect();
    try {
      await client.query('BEGIN');
      for (const event of events) {
        const result = await client.query(
          `INSERT INTO fact_events (
              ledger, tx_hash, event_index, contract_sk, event_type_id, event_name,
              block_timestamp, topics, payload, user_address, asset_address, amount
           ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
           ON CONFLICT (ledger, tx_hash, event_index, block_timestamp) DO NOTHING`,
          [
            event.ledger,
            event.txHash,
            event.eventIndex,
            contractSk,
            event.eventTypeId,
            event.eventName,
            event.blockTimestamp.toISOString(),
            JSON.stringify(event.topics),
            JSON.stringify(event.payload),
            event.userAddress,
            event.assetAddress,
            event.amount,
          ]
        );
        inserted += result.rowCount ?? 0;
      }
      await client.query('COMMIT');
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
    return inserted;
  }

  async getSyncState(): Promise<SyncState> {
    const result = await this.pool.query<{
      last_ledger: string;
      events_archived: string;
      last_synced_at: Date | null;
    }>('SELECT last_ledger, events_archived, last_synced_at FROM archive_sync_state WHERE id = 1');
    const row = result.rows[0];
    if (!row) {
      return { lastLedger: 0, eventsArchived: 0, lastSyncedAt: null };
    }
    return {
      lastLedger: Number(row.last_ledger),
      eventsArchived: Number(row.events_archived),
      lastSyncedAt: row.last_synced_at,
    };
  }

  async updateSyncState(lastLedger: number, eventsAdded: number): Promise<void> {
    await this.pool.query(
      `UPDATE archive_sync_state
       SET last_ledger = GREATEST(last_ledger, $1),
           events_archived = events_archived + $2,
           last_synced_at = NOW(),
           updated_at = NOW()
       WHERE id = 1`,
      [lastLedger, eventsAdded]
    );
  }

  async recordLedgerIntegrity(
    ledger: number,
    archivedCount: number,
    expectedCount: number | null,
    integrityOk: boolean
  ): Promise<void> {
    await this.pool.query(
      `INSERT INTO ledger_event_counts (ledger, expected_count, archived_count, integrity_ok)
       VALUES ($1, $2, $3, $4)
       ON CONFLICT (ledger) DO UPDATE
       SET expected_count = EXCLUDED.expected_count,
           archived_count = EXCLUDED.archived_count,
           integrity_ok = EXCLUDED.integrity_ok,
           verified_at = NOW()`,
      [ledger, expectedCount, archivedCount, integrityOk]
    );
  }

  async applyRetention(retentionDays: number): Promise<number> {
    const result = await this.pool.query<{ archive_apply_retention: string }>(
      'SELECT archive_apply_retention($1)',
      [retentionDays]
    );
    return Number(result.rows[0]?.archive_apply_retention ?? 0);
  }

  async close(): Promise<void> {
    await this.pool.end();
  }
}
