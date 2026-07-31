import type { OracleServiceConfig } from '../types/index.js';

export const developmentOverrides: Partial<OracleServiceConfig> = {
  logLevel: 'debug',
  dryRun: true,
  cacheTtlSeconds: 60,
  updateIntervalMs: 120000,
  maxPriceDeviationPercent: 15,
  priceStaleThresholdSeconds: 600,
  circuitBreaker: {
    failureThreshold: 5,
    backoffMs: 60000,
  },
};
