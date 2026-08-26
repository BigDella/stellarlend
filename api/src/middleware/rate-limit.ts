import { NextFunction, Request, Response } from 'express';

type Penalty = 'none' | 'warning' | 'throttle' | 'block';
type Operation = 'borrow' | 'withdraw' | 'liquidate';

interface OperationLimit {
  windowMs: number;
  max: number;
  throttleAfter: number;
  blockAfter: number;
  /** Adaptive throttle: slowdown factor applied when congestion is detected (0–1). */
  adaptiveDecayFactor?: number;
}

interface Counter {
  timestamps: number[];
  violations: number;
  blockedUntil?: number;
  lastViolationAt?: string;
  /** Running average inter-request interval (ms) for adaptive detection. */
  avgIntervalMs?: number;
  /** Total accepted requests — used for throughput metrics. */
  totalAccepted?: number;
}

interface AnalyticsRow {
  key: string;
  userId: string;
  operation: Operation;
  count: number;
  remaining: number;
  resetAt: string;
  violations: number;
  penalty: Penalty;
  blockedUntil?: string;
  trusted: boolean;
  /** Effective window (ms) after adaptive scaling. */
  effectiveWindowMs: number;
  /** Effective max requests after adaptive scaling. */
  effectiveMax: number;
  /** Average inter-request interval observed for this key. */
  avgIntervalMs: number;
  /** Total accepted requests recorded for this key. */
  totalAccepted: number;
}

/** Adaptive throttling state — global, updated by congestion reports. */
interface AdaptiveState {
  /** Congestion index in basis points (10_000 = normal). */
  congestionBps: number;
  /** Scaling factor derived from congestion (0–1); 1 = no throttle. */
  scaleFactor: number;
  /** Timestamp of last congestion update. */
  updatedAt: number;
  /** Who last reported congestion. */
  reportedBy: string;
  /** TTL for congestion reports in ms (default: 5 min). */
  reportTtlMs: number;
}

const DEFAULT_LIMITS: Record<Operation, OperationLimit> = {
  borrow: { windowMs: 60_000, max: 8, throttleAfter: 1, blockAfter: 3, adaptiveDecayFactor: 0.5 },
  withdraw: { windowMs: 60_000, max: 10, throttleAfter: 1, blockAfter: 3, adaptiveDecayFactor: 0.5 },
  liquidate: { windowMs: 60_000, max: 5, throttleAfter: 1, blockAfter: 2, adaptiveDecayFactor: 0.5 },
};

const counters = new Map<string, Counter>();

/** Module-level adaptive state shared across all operations. */
let adaptiveState: AdaptiveState = {
  congestionBps: 10_000,
  scaleFactor: 1.0,
  updatedAt: 0,
  reportedBy: 'none',
  reportTtlMs: 300_000, // 5 minutes
};

/**
 * Derive a scaling factor from a congestion index (inverse relationship).
 * congestion_bps=10_000 → scale=1.0 (no throttle)
 * congestion_bps=20_000 → scale=0.5 (half throughput)
 * Clamped to [0.25, 1.0].
 */
function scaleFactorFromCongestion(congestionBps: number): number {
  if (congestionBps <= 0) return 1.0;
  const raw = 10_000 / congestionBps;
  return Math.max(0.25, Math.min(1.0, raw));
}

/**
 * Apply the current adaptive scale factor to an operation limit.
 * Returns a new limit with window and max adjusted proportionally.
 */
function applyAdaptiveScaling(
  limit: OperationLimit,
  scaleFactor: number,
  now: number
): OperationLimit {
  if (scaleFactor >= 1.0) return limit;
  // Only apply if we have a recent report (not stale).
  if (adaptiveState.updatedAt > 0 && now - adaptiveState.updatedAt > adaptiveState.reportTtlMs) {
    return limit;
  }
  return {
    ...limit,
    max: Math.max(1, Math.floor(limit.max * scaleFactor)),
  };
}

function parseLimit(operation: Operation): OperationLimit {
  const envName = `SENSITIVE_RATE_LIMIT_${operation.toUpperCase()}`;
  const raw = process.env[envName];
  if (!raw) {
    return DEFAULT_LIMITS[operation];
  }

  const parts = raw.split(':').map((part) => Number(part));
  const m = parts[0];
  const w = parts[1];
  const t = parts[2];
  const b = parts[3];

  return {
    max: m !== undefined && Number.isFinite(m) && m > 0 ? m : DEFAULT_LIMITS[operation].max,
    windowMs:
      w !== undefined && Number.isFinite(w) && w > 0 ? w : DEFAULT_LIMITS[operation].windowMs,
    throttleAfter:
      t !== undefined && Number.isFinite(t) && t >= 0
        ? t
        : DEFAULT_LIMITS[operation].throttleAfter,
    blockAfter:
      b !== undefined && Number.isFinite(b) && b > 0
        ? b
        : DEFAULT_LIMITS[operation].blockAfter,
    adaptiveDecayFactor: DEFAULT_LIMITS[operation].adaptiveDecayFactor,
  };
}

const operationLimits: Record<Operation, OperationLimit> = {
  borrow: parseLimit('borrow'),
  withdraw: parseLimit('withdraw'),
  liquidate: parseLimit('liquidate'),
};

function trustedUsers(): Set<string> {
  return new Set(
    (process.env.RATE_LIMIT_TRUSTED_USERS || '')
      .split(',')
      .map((value) => value.trim())
      .filter(Boolean)
  );
}

function userIdFromRequest(req: Request): string {
  const bodyUser = typeof req.body?.userAddress === 'string' ? req.body.userAddress : undefined;
  const queryUser = typeof req.query?.userAddress === 'string' ? req.query.userAddress : undefined;
  const headerUser =
    typeof req.headers['x-user-address'] === 'string' ? req.headers['x-user-address'] : undefined;
  return bodyUser || queryUser || headerUser || req.ip || 'anonymous';
}

function operationFromRequest(req: Request): Operation | undefined {
  const bodyOperation =
    typeof req.body?.operation === 'string' ? req.body.operation.toLowerCase() : undefined;
  const pathOperation =
    typeof req.params?.operation === 'string'
      ? req.params.operation.toLowerCase()
      : req.path.split('/').find((part) => ['borrow', 'withdraw', 'liquidate'].includes(part));
  const operation = bodyOperation || pathOperation;

  if (operation === 'borrow' || operation === 'withdraw' || operation === 'liquidate') {
    return operation;
  }
  return undefined;
}

function counterKey(userId: string, operation: Operation): string {
  return `${operation}:${userId}`;
}

function currentPenalty(counter: Counter, limit: OperationLimit, now: number): Penalty {
  if (counter.blockedUntil && counter.blockedUntil > now) {
    return 'block';
  }
  if (counter.violations >= limit.blockAfter) {
    return 'block';
  }
  if (counter.violations >= limit.throttleAfter) {
    return 'throttle';
  }
  if (counter.violations > 0) {
    return 'warning';
  }
  return 'none';
}

function setHeaders(
  res: Response,
  limit: OperationLimit,
  count: number,
  resetAt: number,
  penalty: Penalty,
  effectiveMax: number
): void {
  res.setHeader('RateLimit-Limit', String(effectiveMax));
  res.setHeader('RateLimit-Remaining', String(Math.max(effectiveMax - count, 0)));
  res.setHeader('RateLimit-Reset', String(Math.ceil(resetAt / 1000)));
  res.setHeader('X-RateLimit-Penalty', penalty);
  if (adaptiveState.scaleFactor < 1.0) {
    res.setHeader('X-RateLimit-Adaptive', 'throttled');
    res.setHeader('X-RateLimit-CongestionBps', String(adaptiveState.congestionBps));
  }
}

/** Update the rolling average inter-request interval for a counter. */
function updateAvgInterval(counter: Counter, now: number): void {
  const ts = counter.timestamps;
  if (ts.length < 2) {
    counter.avgIntervalMs = 0;
    return;
  }
  const last = ts[ts.length - 1] ?? now;
  const prev = ts[ts.length - 2] ?? last;
  const interval = last - prev;
  const alpha = 0.2; // EWA smoothing
  counter.avgIntervalMs =
    counter.avgIntervalMs !== undefined
      ? counter.avgIntervalMs * (1 - alpha) + interval * alpha
      : interval;
}

export function sensitiveOperationRateLimiter(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  const operation = operationFromRequest(req);
  if (!operation) {
    next();
    return;
  }

  const userId = userIdFromRequest(req);
  if (trustedUsers().has(userId)) {
    res.setHeader('X-RateLimit-Bypass', 'trusted-user');
    next();
    return;
  }

  const now = Date.now();
  const baseLimit = operationLimits[operation];
  const limit = applyAdaptiveScaling(baseLimit, adaptiveState.scaleFactor, now);

  const key = counterKey(userId, operation);
  const counter = counters.get(key) || { timestamps: [], violations: 0, totalAccepted: 0 };

  counter.timestamps = counter.timestamps.filter((stamp) => now - stamp < limit.windowMs);
  const resetAt = (counter.timestamps[0] || now) + limit.windowMs;

  if (counter.blockedUntil && counter.blockedUntil > now) {
    setHeaders(res, limit, counter.timestamps.length, resetAt, 'block', limit.max);
    res.setHeader('Retry-After', String(Math.ceil((counter.blockedUntil - now) / 1000)));
    res.status(429).json({
      success: false,
      error: 'Sensitive operation temporarily blocked after repeated rate limit violations',
      operation,
      penalty: 'block',
    });
    return;
  }

  counter.timestamps.push(now);
  updateAvgInterval(counter, now);

  if (counter.timestamps.length > limit.max) {
    counter.violations += 1;
    counter.lastViolationAt = new Date(now).toISOString();

    const penalty = currentPenalty(counter, limit, now);
    if (penalty === 'block') {
      counter.blockedUntil = now + limit.windowMs * 2;
    }

    counters.set(key, counter);
    setHeaders(res, limit, counter.timestamps.length, resetAt, penalty, limit.max);
    res.setHeader('Retry-After', String(Math.ceil(limit.windowMs / 1000)));
    res.status(429).json({
      success: false,
      error:
        penalty === 'warning'
          ? 'Sensitive operation rate limit warning'
          : 'Sensitive operation rate limit exceeded',
      operation,
      penalty,
      adaptive: adaptiveState.scaleFactor < 1.0
        ? { active: true, congestionBps: adaptiveState.congestionBps, scaleFactor: adaptiveState.scaleFactor }
        : { active: false },
    });
    return;
  }

  counter.totalAccepted = (counter.totalAccepted ?? 0) + 1;
  counters.set(key, counter);
  setHeaders(res, limit, counter.timestamps.length, resetAt, currentPenalty(counter, limit, now), limit.max);
  next();
}

export function getSensitiveRateLimitAnalytics(): AnalyticsRow[] {
  const now = Date.now();
  const trusted = trustedUsers();

  return [...counters.entries()].map(([key, counter]) => {
    const [operation, ...userParts] = key.split(':');
    const op = operation as Operation;
    const userId = userParts.join(':');
    const baseLimit = operationLimits[op];
    const limit = applyAdaptiveScaling(baseLimit, adaptiveState.scaleFactor, now);
    const activeTimestamps = counter.timestamps.filter((stamp) => now - stamp < limit.windowMs);
    const resetAt = (activeTimestamps[0] || now) + limit.windowMs;
    const penalty = currentPenalty(counter, limit, now);

    return {
      key,
      userId,
      operation: op,
      count: activeTimestamps.length,
      remaining: Math.max(limit.max - activeTimestamps.length, 0),
      resetAt: new Date(resetAt).toISOString(),
      violations: counter.violations,
      penalty,
      blockedUntil: counter.blockedUntil ? new Date(counter.blockedUntil).toISOString() : undefined,
      trusted: trusted.has(userId),
      effectiveWindowMs: limit.windowMs,
      effectiveMax: limit.max,
      avgIntervalMs: counter.avgIntervalMs ?? 0,
      totalAccepted: counter.totalAccepted ?? 0,
    };
  });
}

export function getSensitiveRateLimitConfig(): Record<Operation, OperationLimit> {
  return operationLimits;
}

export function resetSensitiveRateLimits(): void {
  counters.clear();
}

/**
 * Report the current network congestion index in basis points.
 *
 * This is the API-layer counterpart to the Soroban `rl_report_congestion`
 * entrypoint. Off-chain monitors call this to push congestion signals into
 * the middleware's adaptive throttler without requiring an on-chain tx.
 *
 * @param congestionBps  Congestion index. 10_000 = normal, 20_000 = 2× congested.
 * @param reportedBy     Identifier of the reporter (e.g. service name or address).
 * @param ttlMs          Optional TTL override for how long this report stays active.
 */
export function reportAdaptiveCongestion(
  congestionBps: number,
  reportedBy: string,
  ttlMs?: number
): void {
  const scaleFactor = scaleFactorFromCongestion(congestionBps);
  adaptiveState = {
    congestionBps,
    scaleFactor,
    updatedAt: Date.now(),
    reportedBy,
    reportTtlMs: ttlMs ?? adaptiveState.reportTtlMs,
  };
}

/**
 * Read-only: return the current adaptive throttling state.
 *
 * Exposed so the `/api/rate-limit/analytics` route can surface it to
 * operators alongside per-user counter data.
 */
export function getAdaptiveState(): AdaptiveState & { isStale: boolean } {
  const now = Date.now();
  const isStale =
    adaptiveState.updatedAt > 0 &&
    now - adaptiveState.updatedAt > adaptiveState.reportTtlMs;
  return { ...adaptiveState, isStale };
}

/**
 * Reset the adaptive congestion state back to "normal" (10_000 bps, scale=1.0).
 *
 * Called by the `/api/rate-limit/reset-congestion` admin endpoint.
 */
export function resetAdaptiveCongestion(): void {
  adaptiveState = {
    congestionBps: 10_000,
    scaleFactor: 1.0,
    updatedAt: Date.now(),
    reportedBy: 'manual-reset',
    reportTtlMs: adaptiveState.reportTtlMs,
  };
}


interface OperationLimit {
  windowMs: number;
  max: number;
  throttleAfter: number;
  blockAfter: number;
}

interface Counter {
  timestamps: number[];
  violations: number;
  blockedUntil?: number;
  lastViolationAt?: string;
}

interface AnalyticsRow {
  key: string;
  userId: string;
  operation: Operation;
  count: number;
  remaining: number;
  resetAt: string;
  violations: number;
  penalty: Penalty;
  blockedUntil?: string;
  trusted: boolean;
}

const DEFAULT_LIMITS: Record<Operation, OperationLimit> = {
  borrow: { windowMs: 60_000, max: 8, throttleAfter: 1, blockAfter: 3 },
  withdraw: { windowMs: 60_000, max: 10, throttleAfter: 1, blockAfter: 3 },
  liquidate: { windowMs: 60_000, max: 5, throttleAfter: 1, blockAfter: 2 },
};

const counters = new Map<string, Counter>();

function parseLimit(operation: Operation): OperationLimit {
  const envName = `SENSITIVE_RATE_LIMIT_${operation.toUpperCase()}`;
  const raw = process.env[envName];
  if (!raw) {
    return DEFAULT_LIMITS[operation];
  }

  const parts = raw.split(':').map((part) => Number(part));
  const m = parts[0];
  const w = parts[1];
  const t = parts[2];
  const b = parts[3];

  return {
    max: m !== undefined && Number.isFinite(m) && m > 0 ? m : DEFAULT_LIMITS[operation].max,
    windowMs:
      w !== undefined && Number.isFinite(w) && w > 0 ? w : DEFAULT_LIMITS[operation].windowMs,
    throttleAfter:
      t !== undefined && Number.isFinite(t) && t >= 0
        ? t
        : DEFAULT_LIMITS[operation].throttleAfter,
    blockAfter:
      b !== undefined && Number.isFinite(b) && b > 0
        ? b
        : DEFAULT_LIMITS[operation].blockAfter,
  };
}

const operationLimits: Record<Operation, OperationLimit> = {
  borrow: parseLimit('borrow'),
  withdraw: parseLimit('withdraw'),
  liquidate: parseLimit('liquidate'),
};

function trustedUsers(): Set<string> {
  return new Set(
    (process.env.RATE_LIMIT_TRUSTED_USERS || '')
      .split(',')
      .map((value) => value.trim())
      .filter(Boolean)
  );
}

function userIdFromRequest(req: Request): string {
  const bodyUser = typeof req.body?.userAddress === 'string' ? req.body.userAddress : undefined;
  const queryUser = typeof req.query?.userAddress === 'string' ? req.query.userAddress : undefined;
  const headerUser =
    typeof req.headers['x-user-address'] === 'string' ? req.headers['x-user-address'] : undefined;
  return bodyUser || queryUser || headerUser || req.ip || 'anonymous';
}

function operationFromRequest(req: Request): Operation | undefined {
  const bodyOperation =
    typeof req.body?.operation === 'string' ? req.body.operation.toLowerCase() : undefined;
  const pathOperation =
    typeof req.params?.operation === 'string'
      ? req.params.operation.toLowerCase()
      : req.path.split('/').find((part) => ['borrow', 'withdraw', 'liquidate'].includes(part));
  const operation = bodyOperation || pathOperation;

  if (operation === 'borrow' || operation === 'withdraw' || operation === 'liquidate') {
    return operation;
  }
  return undefined;
}

function counterKey(userId: string, operation: Operation): string {
  return `${operation}:${userId}`;
}

function currentPenalty(counter: Counter, limit: OperationLimit, now: number): Penalty {
  if (counter.blockedUntil && counter.blockedUntil > now) {
    return 'block';
  }
  if (counter.violations >= limit.blockAfter) {
    return 'block';
  }
  if (counter.violations >= limit.throttleAfter) {
    return 'throttle';
  }
  if (counter.violations > 0) {
    return 'warning';
  }
  return 'none';
}

function setHeaders(
  res: Response,
  limit: OperationLimit,
  count: number,
  resetAt: number,
  penalty: Penalty
): void {
  res.setHeader('RateLimit-Limit', String(limit.max));
  res.setHeader('RateLimit-Remaining', String(Math.max(limit.max - count, 0)));
  res.setHeader('RateLimit-Reset', String(Math.ceil(resetAt / 1000)));
  res.setHeader('X-RateLimit-Penalty', penalty);
}

export function sensitiveOperationRateLimiter(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  const operation = operationFromRequest(req);
  if (!operation) {
    next();
    return;
  }

  const userId = userIdFromRequest(req);
  if (trustedUsers().has(userId)) {
    res.setHeader('X-RateLimit-Bypass', 'trusted-user');
    next();
    return;
  }

  const now = Date.now();
  const limit = operationLimits[operation];
  const key = counterKey(userId, operation);
  const counter = counters.get(key) || { timestamps: [], violations: 0 };

  counter.timestamps = counter.timestamps.filter((stamp) => now - stamp < limit.windowMs);
  const resetAt = (counter.timestamps[0] || now) + limit.windowMs;

  if (counter.blockedUntil && counter.blockedUntil > now) {
    setHeaders(res, limit, counter.timestamps.length, resetAt, 'block');
    res.setHeader('Retry-After', String(Math.ceil((counter.blockedUntil - now) / 1000)));
    res.status(429).json({
      success: false,
      error: 'Sensitive operation temporarily blocked after repeated rate limit violations',
      operation,
      penalty: 'block',
    });
    return;
  }

  counter.timestamps.push(now);

  if (counter.timestamps.length > limit.max) {
    counter.violations += 1;
    counter.lastViolationAt = new Date(now).toISOString();

    const penalty = currentPenalty(counter, limit, now);
    if (penalty === 'block') {
      counter.blockedUntil = now + limit.windowMs * 2;
    }

    counters.set(key, counter);
    setHeaders(res, limit, counter.timestamps.length, resetAt, penalty);
    res.setHeader('Retry-After', String(Math.ceil(limit.windowMs / 1000)));
    res.status(429).json({
      success: false,
      error:
        penalty === 'warning'
          ? 'Sensitive operation rate limit warning'
          : 'Sensitive operation rate limit exceeded',
      operation,
      penalty,
    });
    return;
  }

  counters.set(key, counter);
  setHeaders(res, limit, counter.timestamps.length, resetAt, currentPenalty(counter, limit, now));
  next();
}

export function getSensitiveRateLimitAnalytics(): AnalyticsRow[] {
  const now = Date.now();
  const trusted = trustedUsers();

  return [...counters.entries()].map(([key, counter]) => {
    const [operation, ...userParts] = key.split(':');
    const op = operation as Operation;
    const userId = userParts.join(':');
    const limit = operationLimits[op];
    const activeTimestamps = counter.timestamps.filter((stamp) => now - stamp < limit.windowMs);
    const resetAt = (activeTimestamps[0] || now) + limit.windowMs;
    const penalty = currentPenalty(counter, limit, now);

    return {
      key,
      userId,
      operation: op,
      count: activeTimestamps.length,
      remaining: Math.max(limit.max - activeTimestamps.length, 0),
      resetAt: new Date(resetAt).toISOString(),
      violations: counter.violations,
      penalty,
      blockedUntil: counter.blockedUntil ? new Date(counter.blockedUntil).toISOString() : undefined,
      trusted: trusted.has(userId),
    };
  });
}

export function getSensitiveRateLimitConfig(): Record<Operation, OperationLimit> {
  return operationLimits;
}

export function resetSensitiveRateLimits(): void {
  counters.clear();
}
