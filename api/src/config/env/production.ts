import { AppConfig } from '../types';

const productionOverrides: Partial<AppConfig> = {
  server: {
    port: parseInt(process.env.PORT || '3000', 10),
    env: 'production',
  },
  logging: {
    level: 'warn',
  },
  cache: {
    redisEnabled: true,
    redisUrl: process.env.REDIS_URL || '',
    idempotencyTtlMs: 86400000,
    idempotencyMaxEntries: 10000,
    protocolStatsTtlMs: 10000,
    positionTtlMs: 5000,
    poolTtlMs: 10000,
  },
  rateLimit: {
    windowMs: 60000,
    maxRequests: 100,
  },
  analytics: {
    historyRetentionDays: 365,
    snapshotIntervalMs: 15000,
  },
  subscriptions: {
    maxRetries: 10,
    retryBackoffMs: 30000,
    executionIntervalMs: 60000,
  },
};

export default productionOverrides;
