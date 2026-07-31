import { describe, expect, it } from 'vitest';
import { normalizeEvent, groupCountsByLedger } from '../src/archive/normalize.js';
import { verifyLedgerIntegrity, summarizeIntegrity } from '../src/archive/integrity.js';
import { EventArchiver } from '../src/archive/archiver.js';
import { InMemoryEventRepository } from '../src/db/repository.js';
import { loadConfig } from '../src/config.js';
import type { EventFetcher } from '../src/rpc/event-fetcher.js';
import type { RawSorobanEvent, EventArchiverConfig } from '../src/types.js';
import { createLogger } from '../src/utils/logger.js';

const baseConfig: EventArchiverConfig = {
  stellarNetwork: 'testnet',
  stellarRpcUrl: 'https://soroban-testnet.stellar.org',
  contractId: 'CONTRACT123',
  databaseUrl: 'postgres://localhost/test',
  startLedger: 100,
  pollIntervalMs: 1000,
  batchSize: 50,
  detailRetentionDays: 730,
  logLevel: 'error',
};

function makeRaw(partial: Partial<RawSorobanEvent> & Pick<RawSorobanEvent, 'topic'>): RawSorobanEvent {
  return {
    ledger: partial.ledger ?? 1000,
    txHash: partial.txHash ?? 'abc',
    eventIndex: partial.eventIndex ?? 0,
    contractId: partial.contractId ?? 'CONTRACT123',
    topic: partial.topic,
    topics: partial.topics ?? [partial.topic],
    value: partial.value ?? { user: 'GUSER', asset: null, amount: '1000', timestamp: 1 },
    timestamp: partial.timestamp ?? new Date('2026-01-01T00:00:00Z'),
  };
}

describe('normalizeEvent', () => {
  it('maps deposit events to warehouse rows', () => {
    const archived = normalizeEvent(makeRaw({ topic: 'deposit_event' }));
    expect(archived).not.toBeNull();
    expect(archived!.eventTypeId).toBe(1);
    expect(archived!.userAddress).toBe('GUSER');
    expect(archived!.amount).toBe('1000');
  });

  it('maps liquidation events using borrower / debt fields', () => {
    const archived = normalizeEvent(
      makeRaw({
        topic: 'liquidation_event',
        value: {
          liquidator: 'GLIQ',
          borrower: 'GBOR',
          debt_asset: 'GASSET',
          collateral_asset: null,
          debt_liquidated: '500',
          collateral_seized: '600',
          incentive_amount: '50',
          timestamp: 2,
        },
      })
    );
    expect(archived!.eventTypeId).toBe(5);
    expect(archived!.userAddress).toBe('GBOR');
    expect(archived!.assetAddress).toBe('GASSET');
    expect(archived!.amount).toBe('500');
  });

  it('ignores non-protocol topics', () => {
    const raw = makeRaw({ topic: 'deposit_event' });
    raw.topic = 'admin_action_event';
    expect(normalizeEvent(raw)).toBeNull();
  });
});

describe('integrity checks', () => {
  it('passes when counts match', () => {
    const result = verifyLedgerIntegrity(10, 3, 3);
    expect(result.integrityOk).toBe(true);
  });

  it('fails when counts diverge', () => {
    const result = verifyLedgerIntegrity(10, 2, 3);
    expect(result.integrityOk).toBe(false);
  });

  it('summarizes failures', () => {
    const summary = summarizeIntegrity([
      verifyLedgerIntegrity(1, 1, 1),
      verifyLedgerIntegrity(2, 1, 2),
    ]);
    expect(summary.failed).toBe(1);
    expect(summary.failedLedgers).toEqual([2]);
  });
});

describe('EventArchiver', () => {
  it('archives fetched events and advances sync cursor', async () => {
    const fetcher: EventFetcher = {
      async getLatestLedger() {
        return 2000;
      },
      async fetchEvents() {
        return [
          makeRaw({ ledger: 1500, txHash: 'tx1', topic: 'borrow_event', eventIndex: 0 }),
          makeRaw({ ledger: 1501, txHash: 'tx2', topic: 'repay_event', eventIndex: 0 }),
        ];
      },
    };

    const repo = new InMemoryEventRepository();
    const archiver = new EventArchiver(baseConfig, fetcher, repo, createLogger('error'));
    const result = await archiver.pollOnce();

    expect(result.fetched).toBe(2);
    expect(result.archived).toBe(2);
    expect(result.lastLedger).toBe(1501);
    expect(repo.getAllEvents()).toHaveLength(2);

    const state = await repo.getSyncState();
    expect(state.lastLedger).toBe(1501);
    expect(state.eventsArchived).toBe(2);
  });

  it('deduplicates on re-ingest', async () => {
    const events = [makeRaw({ ledger: 1500, txHash: 'tx1', topic: 'deposit_event' })];
    const fetcher: EventFetcher = {
      async getLatestLedger() {
        return 2000;
      },
      async fetchEvents() {
        return events;
      },
    };
    const repo = new InMemoryEventRepository();
    const archiver = new EventArchiver(baseConfig, fetcher, repo, createLogger('error'));

    await archiver.pollOnce();
    const second = await archiver.pollOnce();
    expect(second.archived).toBe(0);
    expect(repo.getAllEvents()).toHaveLength(1);
  });

  it('groups ledger counts', () => {
    const counts = groupCountsByLedger([
      normalizeEvent(makeRaw({ ledger: 1, topic: 'deposit_event', eventIndex: 0 }))!,
      normalizeEvent(makeRaw({ ledger: 1, topic: 'borrow_event', eventIndex: 1, txHash: 'b' }))!,
      normalizeEvent(makeRaw({ ledger: 2, topic: 'repay_event', eventIndex: 0, txHash: 'c' }))!,
    ]);
    expect(counts.get(1)).toBe(2);
    expect(counts.get(2)).toBe(1);
  });
});

describe('loadConfig', () => {
  it('loads required env', () => {
    const config = loadConfig({
      CONTRACT_ID: 'C123',
      DATABASE_URL: 'postgres://u:p@localhost/db',
      STELLAR_NETWORK: 'testnet',
    });
    expect(config.contractId).toBe('C123');
    expect(config.detailRetentionDays).toBe(730);
    expect(config.stellarRpcUrl).toContain('testnet');
  });

  it('rejects missing contract id', () => {
    expect(() =>
      loadConfig({
        DATABASE_URL: 'postgres://u:p@localhost/db',
      })
    ).toThrow(/CONTRACT_ID/);
  });
});
