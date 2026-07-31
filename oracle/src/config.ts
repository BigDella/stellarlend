/**
 * Oracle Service Configuration
 *
 * Handles loading and validating environment variables and
 * provides typed configuration for the oracle service.
 */

import { z } from 'zod';
import dotenv from 'dotenv';
import type {
  OracleServiceConfig,
  ProviderConfig,
  AssetMapping,
  SupportedAsset,
} from './types/index.js';
import { configureLogger, logger } from './utils/logger.js';
import { developmentOverrides } from './config/development.js';
import { productionOverrides } from './config/production.js';

export type { OracleServiceConfig } from './types/index.js';

dotenv.config();

const VALID_LOG_LEVELS = new Set(['debug', 'info', 'warn', 'error']);
const MIN_STELLAR_FEE = 100;
const DEFAULT_MAX_FEE = 1_000_000;

function getBootstrapLogLevel(): 'debug' | 'info' | 'warn' | 'error' {
  const level = process.env.LOG_LEVEL;

  if (level && VALID_LOG_LEVELS.has(level)) {
    return level as 'debug' | 'info' | 'warn' | 'error';
  }

  return 'info';
}

configureLogger(getBootstrapLogLevel(), process.env.NODE_ENV === 'production');

const booleanFlagSchema = z
  .union([z.boolean(), z.string()])
  .optional()
  .transform((value, ctx) => {
    if (value === undefined) {
      return false;
    }

    if (typeof value === 'boolean') {
      return value;
    }

    const normalized = value.trim().toLowerCase();

    if (['true', '1', 'yes', 'on'].includes(normalized)) {
      return true;
    }

    if (['false', '0', 'no', 'off', ''].includes(normalized)) {
      return false;
    }

    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: 'Expected a boolean value like true/false',
    });

    return z.NEVER;
  });

/**
 * Network-specific defaults
 */
const NETWORK_DEFAULTS = {
  testnet: {
    rpcUrl: 'https://soroban-testnet.stellar.org',
    baseFee: 100000,
  },
  mainnet: {
    rpcUrl: 'https://soroban.stellar.org',
    baseFee: 200000,
  },
} as const;

/**
 * Environment variable validation schema
 */
const envSchema = z
  .object({
    STELLAR_NETWORK: z.enum(['testnet', 'mainnet']).default('testnet'),
    STELLAR_RPC_URL: z.string().url().optional(),
    STELLAR_BASE_FEE: z.coerce.number().int().min(MIN_STELLAR_FEE).optional(),
    STELLAR_MAX_FEE: z.coerce.number().int().min(MIN_STELLAR_FEE).default(DEFAULT_MAX_FEE),
    CONTRACT_ID: z.string().min(1, 'CONTRACT_ID is required'),
    ADMIN_SECRET_KEY: z.string().min(1, 'ADMIN_SECRET_KEY is required'),
    COINGECKO_API_KEY: z.string().optional(),
    COINMARKETCAP_API_KEY: z.string().optional(),
    REDIS_URL: z.string().url().optional().or(z.literal('')),
    CACHE_TTL_SECONDS: z.coerce.number().positive().default(30),
    UPDATE_INTERVAL_MS: z.coerce.number().positive().default(60000),
    DRY_RUN: booleanFlagSchema,
    MAX_PRICE_DEVIATION_PERCENT: z.coerce.number().positive().default(10),
    PRICE_STALENESS_THRESHOLD_SECONDS: z.coerce.number().positive().default(300),
    CIRCUIT_BREAKER_FAILURE_THRESHOLD: z.coerce.number().int().positive().default(3),
    CIRCUIT_BREAKER_BACKOFF_MS: z.coerce.number().positive().default(30_000),
    LOG_LEVEL: z.enum(['debug', 'info', 'warn', 'error']).default('info'),
    METRICS_PORT: z.coerce.number().int().positive().optional(),
  })
  .superRefine((env, ctx) => {
    const networkDefaults = NETWORK_DEFAULTS[env.STELLAR_NETWORK as keyof typeof NETWORK_DEFAULTS];
    const baseFee = env.STELLAR_BASE_FEE ?? networkDefaults.baseFee;

    if (baseFee > env.STELLAR_MAX_FEE) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: 'STELLAR_MAX_FEE must be greater than or equal to STELLAR_BASE_FEE',
        path: ['STELLAR_MAX_FEE'],
      });
    }
  });

/**
 * Parse and validate environment variables
 */
function parseEnv() {
  const result = envSchema.safeParse(process.env);

  if (!result.success) {
    logger.error('Environment validation failed', {
      issues: result.error.issues.map((issue) => ({
        path: issue.path.join('.'),
        message: issue.message,
      })),
    });
    throw new Error('Invalid environment configuration');
  }

  return result.data;
}

/**
 * Default provider configurations
 */
function getProviderConfigs(env: z.infer<typeof envSchema>): ProviderConfig[] {
  return [
    {
      name: 'coingecko',
      enabled: true,
      priority: 1,
      weight: 0.4,
      apiKey: env.COINGECKO_API_KEY,
      baseUrl: env.COINGECKO_API_KEY
        ? 'https://pro-api.coingecko.com/api/v3'
        : 'https://api.coingecko.com/api/v3',
      rateLimit: {
        maxRequests: env.COINGECKO_API_KEY ? 500 : 10,
        windowMs: 60000,
      },
    },
    {
      name: 'coinmarketcap',
      enabled: !!env.COINMARKETCAP_API_KEY,
      priority: 2,
      weight: 0.35,
      apiKey: env.COINMARKETCAP_API_KEY,
      baseUrl: 'https://pro-api.coinmarketcap.com/v2',
      rateLimit: {
        maxRequests: 30,
        windowMs: 60000,
      },
    },
    {
      name: 'binance',
      enabled: true,
      priority: 3,
      weight: 0.25,
      baseUrl: 'https://api.binance.com/api/v3',
      rateLimit: {
        maxRequests: 1200,
        windowMs: 60000,
      },
    },
  ];
}

/**
 * Asset mappings for different providers
 */
export const ASSET_MAPPINGS: AssetMapping[] = [
  {
    symbol: 'XLM',
    coingeckoId: 'stellar',
    coinmarketcapId: 512,
    binanceSymbol: 'XLMUSDT',
  },
  {
    symbol: 'USDC',
    coingeckoId: 'usd-coin',
    coinmarketcapId: 3408,
    binanceSymbol: 'USDCUSDT',
  },
  {
    symbol: 'USDT',
    coingeckoId: 'tether',
    coinmarketcapId: 825,
    binanceSymbol: 'USDTBUSD',
  },
  {
    symbol: 'BTC',
    coingeckoId: 'bitcoin',
    coinmarketcapId: 1,
    binanceSymbol: 'BTCUSDT',
  },
  {
    symbol: 'ETH',
    coingeckoId: 'ethereum',
    coinmarketcapId: 1027,
    binanceSymbol: 'ETHUSDT',
  },
];

/**
 * Get asset mapping by symbol
 */
export function getAssetMapping(symbol: SupportedAsset): AssetMapping | undefined {
  return ASSET_MAPPINGS.find((m) => m.symbol === symbol);
}

/**
 * Check if an asset is supported
 */
export function isSupportedAsset(symbol: string): symbol is SupportedAsset {
  return ASSET_MAPPINGS.some((m) => m.symbol === symbol);
}

/**
 * Build and export the service configuration
 */
function getEnvOverrides(): Partial<OracleServiceConfig> {
  const nodeEnv = process.env.NODE_ENV || 'development';
  switch (nodeEnv) {
    case 'production':
      return productionOverrides;
    case 'development':
    default:
      return developmentOverrides;
  }
}

export function loadConfig(): OracleServiceConfig {
  const env = parseEnv();
  const envOverrides = getEnvOverrides();

  // Get network-specific defaults
  const networkDefaults = NETWORK_DEFAULTS[env.STELLAR_NETWORK as keyof typeof NETWORK_DEFAULTS];

  // Use env vars if provided, otherwise use network defaults
  const stellarRpcUrl = env.STELLAR_RPC_URL || networkDefaults.rpcUrl;
  const baseFee = env.STELLAR_BASE_FEE || networkDefaults.baseFee;

  return {
    stellarNetwork: env.STELLAR_NETWORK,
    stellarRpcUrl,
    baseFee,
    maxFee: env.STELLAR_MAX_FEE,
    contractId: env.CONTRACT_ID,
    adminSecretKey: env.ADMIN_SECRET_KEY,
    dryRun: envOverrides.dryRun ?? env.DRY_RUN,
    updateIntervalMs: envOverrides.updateIntervalMs ?? env.UPDATE_INTERVAL_MS,
    maxPriceDeviationPercent: envOverrides.maxPriceDeviationPercent ?? env.MAX_PRICE_DEVIATION_PERCENT,
    priceStaleThresholdSeconds: envOverrides.priceStaleThresholdSeconds ?? env.PRICE_STALENESS_THRESHOLD_SECONDS,
    cacheTtlSeconds: envOverrides.cacheTtlSeconds ?? env.CACHE_TTL_SECONDS,
    redisUrl: env.REDIS_URL || undefined,
    logLevel: envOverrides.logLevel ?? env.LOG_LEVEL,
    providers: getProviderConfigs(env),
    circuitBreaker: envOverrides.circuitBreaker ?? {
      failureThreshold: env.CIRCUIT_BREAKER_FAILURE_THRESHOLD,
      backoffMs: env.CIRCUIT_BREAKER_BACKOFF_MS,
    },
    metricsPort: env.METRICS_PORT,
  };
}

/**
 * Masks a secret key for safe logging.
 * Shows first 2 and last 2 characters only.
 * Handles edge cases: empty string, very short keys.
 */
export function maskSecret(key: string): string {
  if (!key || key.length === 0) return '****';
  if (key.length <= 8) return '****';
  return key.slice(0, 2) + '*'.repeat(key.length - 4) + key.slice(-2);
}

/**
 * Returns a safe (redacted) version of the config for logging.
 * Strips adminSecretKey entirely.
 */
export function getSafeConfig(
  config: OracleServiceConfig
): Omit<OracleServiceConfig, 'adminSecretKey'> & { adminSecretKey: string } {
  return {
    ...config,
    adminSecretKey: maskSecret(config.adminSecretKey),
  };
}

export const PRICE_SCALE = 1_000_000n;

export function scalePrice(price: number): bigint {
  return BigInt(Math.round(price * Number(PRICE_SCALE)));
}

export function unscalePrice(price: bigint): number {
  return Number(price) / Number(PRICE_SCALE);
}
