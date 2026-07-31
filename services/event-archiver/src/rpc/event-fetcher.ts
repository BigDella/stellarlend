import { rpc, scValToNative, nativeToScVal } from '@stellar/stellar-sdk';
import { PROTOCOL_EVENT_TOPICS, type RawSorobanEvent } from '../types.js';
import type { Logger } from '../utils/logger.js';

export interface EventFetcher {
  fetchEvents(startLedger: number, limit: number): Promise<RawSorobanEvent[]>;
  getLatestLedger(): Promise<number>;
}

function topicToString(topic: unknown): string {
  if (typeof topic === 'string') return topic;
  if (topic && typeof topic === 'object') {
    try {
      const native = scValToNative(topic as Parameters<typeof scValToNative>[0]);
      if (typeof native === 'string') return native;
      return String(native);
    } catch {
      return String(topic);
    }
  }
  return String(topic);
}

/**
 * Fetches Soroban contract events via Stellar RPC getEvents.
 */
export class SorobanEventFetcher implements EventFetcher {
  private readonly server: rpc.Server;
  private readonly contractId: string;
  private readonly logger: Logger;

  constructor(rpcUrl: string, contractId: string, logger: Logger) {
    this.server = new rpc.Server(rpcUrl, { allowHttp: rpcUrl.startsWith('http://') });
    this.contractId = contractId;
    this.logger = logger;
  }

  async getLatestLedger(): Promise<number> {
    const info = await this.server.getLatestLedger();
    return info.sequence;
  }

  async fetchEvents(startLedger: number, limit: number): Promise<RawSorobanEvent[]> {
    const topicFilters = PROTOCOL_EVENT_TOPICS.map((topic) =>
      nativeToScVal(topic, { type: 'symbol' }).toXDR('base64')
    );

    const filters = [
      {
        type: 'contract' as const,
        contractIds: [this.contractId],
        // OR across protocol topics at topic position 0
        topics: [topicFilters],
      },
    ];

    // SDK types require either startLedger or cursor; always pin a concrete startLedger.
    const effectiveStart = startLedger > 0 ? startLedger : 1;
    const page = await this.server.getEvents({
      startLedger: effectiveStart,
      filters,
      limit,
    });

    const events: RawSorobanEvent[] = [];

    for (let i = 0; i < page.events.length; i++) {
      const event = page.events[i];
      if (!event) continue;

      try {
        const topicsNative = (event.topic ?? []).map((t) => {
          try {
            return scValToNative(t);
          } catch {
            return t;
          }
        });
        const primaryTopic = topicToString(topicsNative[0] ?? '');
        let value: unknown;
        try {
          value = event.value ? scValToNative(event.value) : {};
        } catch {
          value = {};
        }

        const ledger = Number(event.ledger);
        const timestamp = event.ledgerClosedAt
          ? new Date(event.ledgerClosedAt)
          : new Date();

        const contractId =
          typeof event.contractId === 'string'
            ? event.contractId
            : this.contractId;

        events.push({
          ledger,
          txHash: event.txHash ?? '',
          eventIndex: typeof event.id === 'string' ? i : Number(event.id ?? i),
          contractId,
          topic: primaryTopic,
          topics: topicsNative,
          value,
          timestamp,
        });
      } catch (err) {
        this.logger.warn('Failed to decode event', {
          error: err instanceof Error ? err.message : String(err),
        });
      }
    }

    return events;
  }
}
