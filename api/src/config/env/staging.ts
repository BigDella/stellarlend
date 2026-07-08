import { AppConfig } from '../types';

const stagingOverrides: Partial<AppConfig> = {
  server: {
    port: 3000,
    env: 'staging',
  },
  logging: {
    level: 'info',
  },
  cache: {
    redisEnabled: true,
    redisUrl: process.env.REDIS_URL || '',
    idempotencyTtlMs: 86400000,
    idempotencyMaxEntries: 5000,
    protocolStatsTtlMs: 15000,
    positionTtlMs: 10000,
    poolTtlMs: 15000,
  },
  rateLimit: {
    windowMs: 60000,
    maxRequests: 200,
  },
  analytics: {
    historyRetentionDays: 180,
    snapshotIntervalMs: 30000,
  },
  subscriptions: {
    maxRetries: 5,
    retryBackoffMs: 10000,
    executionIntervalMs: 120000,
  },
};

export default stagingOverrides;
