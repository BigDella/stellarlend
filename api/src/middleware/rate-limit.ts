import { NextFunction, Request, Response } from 'express';

type Penalty = 'none' | 'warning' | 'throttle' | 'block';
type Operation = 'borrow' | 'withdraw' | 'liquidate';

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
