import { ValidationError } from '../utils/errors';

export type FunnelStage =
  | 'visit'
  | 'wallet_connect'
  | 'first_deposit'
  | 'first_borrow'
  | 'repeat_action';

export interface UserEventInput {
  /** Pseudonymous identifier — callers are responsible for hashing/anonymizing raw user IDs before sending. */
  userId: string;
  stage: FunnelStage;
  sessionId: string;
  timestamp?: number;
  volumeUsd?: number;
  experimentVariant?: string;
  /** GDPR opt-out: when true, the event is discarded rather than stored. */
  optedOut?: boolean;
}

interface StoredEvent {
  userId: string;
  stage: FunnelStage;
  sessionId: string;
  timestamp: number;
  volumeUsd: number;
  experimentVariant?: string;
}

export interface FunnelStageResult {
  stage: FunnelStage;
  uniqueUsers: number;
  conversionFromPreviousPercent: number;
  dropOffPercent: number;
}

export interface CohortRetentionRow {
  cohort: string;
  cohortSize: number;
  retentionByPeriod: number[];
}

export interface PowerUser {
  userId: string;
  volumeUsd: number;
  eventCount: number;
  firstSeenAt: number;
  lastSeenAt: number;
  longevityDays: number;
}

export interface ChurnRiskUser {
  userId: string;
  lastSeenAt: number;
  daysSinceLastActivity: number;
  riskScore: number;
}

export interface ConversionRates {
  visitorToDepositorPercent: number;
  depositorToBorrowerPercent: number;
}

export interface AbTestVariantMetrics {
  variant: string;
  users: number;
  conversions: number;
  conversionRatePercent: number;
}

const FUNNEL_ORDER: FunnelStage[] = [
  'visit',
  'wallet_connect',
  'first_deposit',
  'first_borrow',
  'repeat_action',
];

const CHURN_RISK_THRESHOLD_DAYS = 14;

const store = {
  events: [] as StoredEvent[],
};

export function resetForTests(): void {
  store.events = [];
}

export function recordEvent(input: UserEventInput): void {
  if (input.optedOut) return;

  if (!input.userId || !input.sessionId) {
    throw new ValidationError('userId and sessionId are required');
  }
  if (!FUNNEL_ORDER.includes(input.stage)) {
    throw new ValidationError(`stage must be one of: ${FUNNEL_ORDER.join(', ')}`);
  }

  store.events.push({
    userId: input.userId,
    stage: input.stage,
    sessionId: input.sessionId,
    timestamp: input.timestamp ?? Date.now(),
    volumeUsd: input.volumeUsd ?? 0,
    experimentVariant: input.experimentVariant,
  });
}

function usersAtOrPastStage(stage: FunnelStage): Set<string> {
  const stageIndex = FUNNEL_ORDER.indexOf(stage);
  const users = new Set<string>();
  for (const event of store.events) {
    if (FUNNEL_ORDER.indexOf(event.stage) >= stageIndex) {
      users.add(event.userId);
    }
  }
  return users;
}

export function getFunnel(): FunnelStageResult[] {
  const uniqueUsersByStage = FUNNEL_ORDER.map((stage) => usersAtOrPastStage(stage).size);

  return FUNNEL_ORDER.map((stage, index) => {
    const uniqueUsers = uniqueUsersByStage[index]!;
    const previousUsers = index === 0 ? uniqueUsers : uniqueUsersByStage[index - 1]!;
    const conversionFromPreviousPercent =
      index === 0 ? 100 : previousUsers > 0 ? (uniqueUsers / previousUsers) * 100 : 0;

    return {
      stage,
      uniqueUsers,
      conversionFromPreviousPercent,
      dropOffPercent: index === 0 ? 0 : 100 - conversionFromPreviousPercent,
    };
  });
}

export function getConversionRates(): ConversionRates {
  const visitors = usersAtOrPastStage('visit').size;
  const depositors = usersAtOrPastStage('first_deposit').size;
  const borrowers = usersAtOrPastStage('first_borrow').size;

  return {
    visitorToDepositorPercent: visitors > 0 ? (depositors / visitors) * 100 : 0,
    depositorToBorrowerPercent: depositors > 0 ? (borrowers / depositors) * 100 : 0,
  };
}

function cohortKey(timestamp: number, granularity: 'weekly' | 'monthly'): string {
  const date = new Date(timestamp);
  if (granularity === 'monthly') {
    return `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(2, '0')}`;
  }
  const oneWeekMs = 7 * 24 * 60 * 60 * 1000;
  const weekIndex = Math.floor(date.getTime() / oneWeekMs);
  return `week-${weekIndex}`;
}

export function getCohortRetention(
  granularity: 'weekly' | 'monthly' = 'weekly',
  periods = 6
): CohortRetentionRow[] {
  const firstSeenByUser = new Map<string, number>();
  for (const event of store.events) {
    const existing = firstSeenByUser.get(event.userId);
    if (existing === undefined || event.timestamp < existing) {
      firstSeenByUser.set(event.userId, event.timestamp);
    }
  }

  const cohorts = new Map<string, Set<string>>();
  firstSeenByUser.forEach((timestamp, userId) => {
    const key = cohortKey(timestamp, granularity);
    if (!cohorts.has(key)) cohorts.set(key, new Set());
    cohorts.get(key)!.add(userId);
  });

  const periodMs =
    granularity === 'monthly' ? 30 * 24 * 60 * 60 * 1000 : 7 * 24 * 60 * 60 * 1000;

  const activityByUser = new Map<string, number[]>();
  for (const event of store.events) {
    if (!activityByUser.has(event.userId)) activityByUser.set(event.userId, []);
    activityByUser.get(event.userId)!.push(event.timestamp);
  }

  const rows: CohortRetentionRow[] = [];
  cohorts.forEach((users, cohort) => {
    const cohortStart = Math.min(...Array.from(users).map((u) => firstSeenByUser.get(u)!));
    const retentionByPeriod: number[] = [];

    for (let period = 0; period < periods; period++) {
      const periodStart = cohortStart + period * periodMs;
      const periodEnd = periodStart + periodMs;
      let activeUsers = 0;
      users.forEach((userId) => {
        const timestamps = activityByUser.get(userId) ?? [];
        if (timestamps.some((t) => t >= periodStart && t < periodEnd)) activeUsers++;
      });
      retentionByPeriod.push(users.size > 0 ? (activeUsers / users.size) * 100 : 0);
    }

    rows.push({ cohort, cohortSize: users.size, retentionByPeriod });
  });

  return rows.sort((a, b) => a.cohort.localeCompare(b.cohort));
}

export function getPowerUsers(topPercent = 10): PowerUser[] {
  const byUser = new Map<string, StoredEvent[]>();
  for (const event of store.events) {
    if (!byUser.has(event.userId)) byUser.set(event.userId, []);
    byUser.get(event.userId)!.push(event);
  }

  const users: PowerUser[] = Array.from(byUser.entries()).map(([userId, events]) => {
    const timestamps = events.map((e) => e.timestamp);
    const firstSeenAt = Math.min(...timestamps);
    const lastSeenAt = Math.max(...timestamps);
    return {
      userId,
      volumeUsd: events.reduce((sum, e) => sum + e.volumeUsd, 0),
      eventCount: events.length,
      firstSeenAt,
      lastSeenAt,
      longevityDays: (lastSeenAt - firstSeenAt) / (24 * 60 * 60 * 1000),
    };
  });

  users.sort((a, b) => b.volumeUsd - a.volumeUsd || b.eventCount - a.eventCount);

  const cutoff = Math.max(1, Math.ceil((users.length * topPercent) / 100));
  return users.slice(0, cutoff);
}

export function getChurnRisk(now = Date.now()): ChurnRiskUser[] {
  const lastSeenByUser = new Map<string, number>();
  for (const event of store.events) {
    const existing = lastSeenByUser.get(event.userId);
    if (existing === undefined || event.timestamp > existing) {
      lastSeenByUser.set(event.userId, event.timestamp);
    }
  }

  const results: ChurnRiskUser[] = [];
  lastSeenByUser.forEach((lastSeenAt, userId) => {
    const daysSinceLastActivity = (now - lastSeenAt) / (24 * 60 * 60 * 1000);
    if (daysSinceLastActivity >= CHURN_RISK_THRESHOLD_DAYS) {
      const riskScore = Math.min(100, Math.round((daysSinceLastActivity / 60) * 100));
      results.push({ userId, lastSeenAt, daysSinceLastActivity, riskScore });
    }
  });

  return results.sort((a, b) => b.riskScore - a.riskScore);
}

export function getAbTestMetrics(conversionStage: FunnelStage = 'first_deposit'): AbTestVariantMetrics[] {
  const byVariant = new Map<string, { users: Set<string>; converted: Set<string> }>();

  for (const event of store.events) {
    if (!event.experimentVariant) continue;
    if (!byVariant.has(event.experimentVariant)) {
      byVariant.set(event.experimentVariant, { users: new Set(), converted: new Set() });
    }
    const bucket = byVariant.get(event.experimentVariant)!;
    bucket.users.add(event.userId);
    if (FUNNEL_ORDER.indexOf(event.stage) >= FUNNEL_ORDER.indexOf(conversionStage)) {
      bucket.converted.add(event.userId);
    }
  }

  return Array.from(byVariant.entries()).map(([variant, bucket]) => ({
    variant,
    users: bucket.users.size,
    conversions: bucket.converted.size,
    conversionRatePercent:
      bucket.users.size > 0 ? (bucket.converted.size / bucket.users.size) * 100 : 0,
  }));
}
