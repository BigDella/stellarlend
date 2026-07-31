import { createHash, randomUUID } from 'crypto';
import logger from '../utils/logger';

export type AuditAction =
  | 'DEPOSIT'
  | 'BORROW'
  | 'REPAY'
  | 'WITHDRAW'
  | 'TRANSACTION_EXECUTED'
  | 'ROLE_ASSIGNED'
  | 'ROLE_REVOKED'
  | 'PROTOCOL_PAUSED'
  | 'PROTOCOL_RESUMED'
  | 'WITHDRAWAL_QUEUED';

export interface AuditLogEntry {
  id: string;
  sequence: number;
  timestamp: string;
  action: AuditAction | string;
  actor: string;
  status: 'success' | 'failed' | 'pending' | 'queued';
  txHash?: string;
  ledger?: number;
  amount?: string;
  assetAddress?: string;
  ip?: string;
  beforeState?: Record<string, unknown>;
  afterState?: Record<string, unknown>;
  /** SHA-256 hash of the previous entry (genesis entry uses '0'). Forms an integrity chain. */
  prevHash: string;
  /** SHA-256(prevHash + canonical JSON of all fields except hash itself). */
  hash: string;
}

export interface AuditLogFilter {
  action?: string;
  actor?: string;
  status?: string;
  from?: string;
  to?: string;
  limit?: number;
  offset?: number;
}

export interface AuditLogVerifyResult {
  valid: boolean;
  checkedEntries: number;
  firstBadSequence?: number;
}

const MAX_ENTRIES = 10_000;

function hashEntry(prevHash: string, fields: Omit<AuditLogEntry, 'hash'>): string {
  const payload = prevHash + JSON.stringify(fields, Object.keys(fields).sort());
  return createHash('sha256').update(payload).digest('hex');
}

class AuditLogService {
  private entries: AuditLogEntry[] = [];
  private sequence = 0;

  record(params: {
    action: AuditLogEntry['action'];
    actor: string;
    status: AuditLogEntry['status'];
    txHash?: string;
    ledger?: number;
    amount?: string;
    assetAddress?: string;
    ip?: string;
    beforeState?: Record<string, unknown>;
    afterState?: Record<string, unknown>;
  }): AuditLogEntry {
    const prevHash = this.entries.length > 0 ? (this.entries[this.entries.length - 1]?.hash ?? '0') : '0';

    const seq = ++this.sequence;
    const entry: Omit<AuditLogEntry, 'hash'> = {
      id: randomUUID(),
      sequence: seq,
      timestamp: new Date().toISOString(),
      action: params.action,
      actor: params.actor,
      status: params.status,
      txHash: params.txHash,
      ledger: params.ledger,
      amount: params.amount,
      assetAddress: params.assetAddress,
      ip: params.ip,
      beforeState: params.beforeState,
      afterState: params.afterState,
      prevHash,
    };

    const full: AuditLogEntry = { ...entry, hash: hashEntry(prevHash, entry) };

    this.entries.push(full);

    // Enforce retention: drop oldest entries when over capacity
    if (this.entries.length > MAX_ENTRIES) {
      this.entries.splice(0, this.entries.length - MAX_ENTRIES);
    }

    logger.info('AUDIT', {
      id: full.id,
      sequence: full.sequence,
      action: full.action,
      actor: full.actor,
      status: full.status,
      txHash: full.txHash,
      ledger: full.ledger,
      timestamp: full.timestamp,
      hash: full.hash,
    });

    return full;
  }

  search(filter: AuditLogFilter = {}): AuditLogEntry[] {
    let results = [...this.entries];

    if (filter.action) {
      results = results.filter((e) => e.action === filter.action);
    }
    if (filter.actor) {
      results = results.filter((e) => e.actor === filter.actor);
    }
    if (filter.status) {
      results = results.filter((e) => e.status === filter.status);
    }
    if (filter.from) {
      const from = new Date(filter.from).getTime();
      results = results.filter((e) => new Date(e.timestamp).getTime() >= from);
    }
    if (filter.to) {
      const to = new Date(filter.to).getTime();
      results = results.filter((e) => new Date(e.timestamp).getTime() <= to);
    }

    const offset = filter.offset ?? 0;
    const limit = filter.limit ?? 100;
    return results.slice(offset, offset + limit);
  }

  export(filter: AuditLogFilter = {}): string {
    return JSON.stringify(this.search(filter), null, 2);
  }

  verify(): AuditLogVerifyResult {
    let prevHash = '0';
    for (let i = 0; i < this.entries.length; i++) {
      const entry = this.entries[i]!;
      const { hash, ...fields } = entry;
      const expected = hashEntry(prevHash, fields);
      if (expected !== hash) {
        return { valid: false, checkedEntries: i + 1, firstBadSequence: entry.sequence };
      }
      prevHash = hash;
    }
    return { valid: true, checkedEntries: this.entries.length };
  }

  count(): number {
    return this.entries.length;
  }

  /** Exposed for testing only — resets in-memory state. */
  _reset(): void {
    this.entries = [];
    this.sequence = 0;
  }
}

export const auditLogService = new AuditLogService();
