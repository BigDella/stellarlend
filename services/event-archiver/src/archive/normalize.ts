import {
  EVENT_TYPE_IDS,
  PROTOCOL_EVENT_TOPICS,
  type ArchivedEvent,
  type ProtocolEventTopic,
  type RawSorobanEvent,
} from '../types.js';

function isProtocolTopic(topic: string): topic is ProtocolEventTopic {
  return (PROTOCOL_EVENT_TOPICS as readonly string[]).includes(topic);
}

function asRecord(value: unknown): Record<string, unknown> {
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return { value };
}

function asNullableString(value: unknown): string | null {
  if (value === null || value === undefined) return null;
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'bigint') return String(value);
  return null;
}

function asNullableAmount(value: unknown): string | null {
  if (value === null || value === undefined) return null;
  if (typeof value === 'bigint') return value.toString();
  if (typeof value === 'number') return Math.trunc(value).toString();
  if (typeof value === 'string' && /^-?\d+$/.test(value)) return value;
  return null;
}

/**
 * Normalize a decoded Soroban event into the warehouse fact row shape.
 */
export function normalizeEvent(raw: RawSorobanEvent): ArchivedEvent | null {
  if (!isProtocolTopic(raw.topic)) {
    return null;
  }

  const payload = asRecord(raw.value);
  const userAddress =
    asNullableString(payload.user) ??
    asNullableString(payload.borrower) ??
    asNullableString(payload.liquidator);

  const assetAddress =
    asNullableString(payload.asset) ??
    asNullableString(payload.debt_asset) ??
    asNullableString(payload.collateral_asset);

  const amount =
    asNullableAmount(payload.amount) ??
    asNullableAmount(payload.debt_liquidated) ??
    asNullableAmount(payload.collateral_seized);

  return {
    ledger: raw.ledger,
    txHash: raw.txHash,
    eventIndex: raw.eventIndex,
    contractId: raw.contractId,
    eventName: raw.topic,
    eventTypeId: EVENT_TYPE_IDS[raw.topic],
    blockTimestamp: raw.timestamp,
    topics: raw.topics,
    payload,
    userAddress,
    assetAddress,
    amount,
  };
}

export function groupCountsByLedger(events: ArchivedEvent[]): Map<number, number> {
  const counts = new Map<number, number>();
  for (const event of events) {
    counts.set(event.ledger, (counts.get(event.ledger) ?? 0) + 1);
  }
  return counts;
}
