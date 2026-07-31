import { z } from 'zod';
import dotenv from 'dotenv';
import type { MetricsCollectorConfig } from './types.js';

dotenv.config();

const envSchema = z.object({
  PROTOCOL_STATS_URL: z.string().url().default('http://localhost:3000/api/protocol/stats'),
  DATABASE_URL: z.string().min(1, 'DATABASE_URL is required'),
  COLLECT_INTERVAL_MS: z.coerce.number().positive().default(60_000),
  RAW_RETENTION_DAYS: z.coerce.number().int().positive().default(30),
  AGGREGATED_RETENTION_DAYS: z.coerce.number().int().positive().default(365),
  LOG_LEVEL: z.enum(['debug', 'info', 'warn', 'error']).default('info'),
});

export function loadConfig(env: NodeJS.ProcessEnv = process.env): MetricsCollectorConfig {
  const parsed = envSchema.safeParse(env);
  if (!parsed.success) {
    const issues = parsed.error.issues.map((i) => `${i.path.join('.')}: ${i.message}`).join('; ');
    throw new Error(`Invalid metrics-collector configuration: ${issues}`);
  }
  const data = parsed.data;
  return {
    protocolStatsUrl: data.PROTOCOL_STATS_URL,
    databaseUrl: data.DATABASE_URL,
    collectIntervalMs: data.COLLECT_INTERVAL_MS,
    rawRetentionDays: data.RAW_RETENTION_DAYS,
    aggregatedRetentionDays: data.AGGREGATED_RETENTION_DAYS,
    logLevel: data.LOG_LEVEL,
  };
}
