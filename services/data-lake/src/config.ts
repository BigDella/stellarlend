import { z } from 'zod';
import dotenv from 'dotenv';
import type { DataLakeConfig } from './types.js';

dotenv.config();

const envSchema = z.object({
  DATA_LAKE_ROOT: z.string().default('./.data-lake'),
  STORAGE_BACKEND: z.enum(['local', 's3', 'gcs']).default('local'),
  S3_BUCKET: z.string().default('stellarlend-raw-blockchain'),
  S3_REGION: z.string().default('us-east-1'),
  GCS_BUCKET: z.string().default(''),
  RAW_RETENTION_DAYS: z.coerce.number().int().positive().default(90),
  CONTRACT_ID: z.string().default(''),
  STELLAR_RPC_URL: z.string().url().default('https://soroban-testnet.stellar.org'),
  LOG_LEVEL: z.enum(['debug', 'info', 'warn', 'error']).default('info'),
});

export function loadConfig(env: NodeJS.ProcessEnv = process.env): DataLakeConfig {
  const parsed = envSchema.safeParse(env);
  if (!parsed.success) {
    const issues = parsed.error.issues.map((i) => `${i.path.join('.')}: ${i.message}`).join('; ');
    throw new Error(`Invalid data-lake configuration: ${issues}`);
  }
  const data = parsed.data;
  return {
    root: data.DATA_LAKE_ROOT,
    storageBackend: data.STORAGE_BACKEND,
    s3Bucket: data.S3_BUCKET,
    s3Region: data.S3_REGION,
    gcsBucket: data.GCS_BUCKET,
    rawRetentionDays: data.RAW_RETENTION_DAYS,
    contractId: data.CONTRACT_ID,
    stellarRpcUrl: data.STELLAR_RPC_URL,
    logLevel: data.LOG_LEVEL,
  };
}
