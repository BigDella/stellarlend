import { AppConfig } from '../types';

const developmentOverrides: Partial<AppConfig> = {
  server: {
    port: 3000,
    env: 'development',
  },
  logging: {
    level: 'debug',
  },
  cache: {
    redisEnabled: false,
    redisUrl: 'redis://127.0.0.1:6379',
    idempotencyTtlMs: 86400000,
    idempotencyMaxEntries: 1000,
    protocolStatsTtlMs: 30000,
    positionTtlMs: 15000,
    poolTtlMs: 30000,
  },
  rateLimit: {
    windowMs: 900000,
    maxRequests: 100,
  },
  analytics: {
    historyRetentionDays: 90,
    snapshotIntervalMs: 60000,
  },
  subscriptions: {
    maxRetries: 3,
    retryBackoffMs: 5000,
    executionIntervalMs: 300000,
  },
};

export default developmentOverrides;
