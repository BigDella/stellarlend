import type { OracleServiceConfig } from '../types/index.js';

export const productionOverrides: Partial<OracleServiceConfig> = {
  logLevel: 'warn',
  dryRun: false,
  cacheTtlSeconds: 15,
  updateIntervalMs: 15000,
  maxPriceDeviationPercent: 5,
  priceStaleThresholdSeconds: 120,
  circuitBreaker: {
    failureThreshold: 3,
    backoffMs: 30000,
  },
};
