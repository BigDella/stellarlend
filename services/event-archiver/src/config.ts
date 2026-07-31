import { z } from 'zod';
import dotenv from 'dotenv';
import type { EventArchiverConfig } from './types.js';

dotenv.config();

const envSchema = z.object({
  STELLAR_NETWORK: z.enum(['testnet', 'mainnet']).default('testnet'),
  STELLAR_RPC_URL: z.string().url().optional(),
  CONTRACT_ID: z.string().min(1, 'CONTRACT_ID is required'),
  DATABASE_URL: z.string().min(1, 'DATABASE_URL is required'),
  START_LEDGER: z.coerce.number().int().nonnegative().default(0),
  POLL_INTERVAL_MS: z.coerce.number().positive().default(60_000),
  BATCH_SIZE: z.coerce.number().int().positive().max(1000).default(200),
  DETAIL_RETENTION_DAYS: z.coerce.number().int().positive().default(730),
  LOG_LEVEL: z.enum(['debug', 'info', 'warn', 'error']).default('info'),
});

const NETWORK_DEFAULTS = {
  testnet: { rpcUrl: 'https://soroban-testnet.stellar.org' },
  mainnet: { rpcUrl: 'https://soroban.stellar.org' },
} as const;

export function loadConfig(env: NodeJS.ProcessEnv = process.env): EventArchiverConfig {
  const parsed = envSchema.safeParse(env);
  if (!parsed.success) {
    const issues = parsed.error.issues.map((i) => `${i.path.join('.')}: ${i.message}`).join('; ');
    throw new Error(`Invalid event-archiver configuration: ${issues}`);
  }

  const data = parsed.data;
  return {
    stellarNetwork: data.STELLAR_NETWORK,
    stellarRpcUrl: data.STELLAR_RPC_URL ?? NETWORK_DEFAULTS[data.STELLAR_NETWORK].rpcUrl,
    contractId: data.CONTRACT_ID,
    databaseUrl: data.DATABASE_URL,
    startLedger: data.START_LEDGER,
    pollIntervalMs: data.POLL_INTERVAL_MS,
    batchSize: data.BATCH_SIZE,
    detailRetentionDays: data.DETAIL_RETENTION_DAYS,
    logLevel: data.LOG_LEVEL,
  };
}
