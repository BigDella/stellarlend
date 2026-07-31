import dotenv from 'dotenv';
import { AppConfig } from './types';
import { assertValidConfig } from './validators';
import { configAuditService } from '../services/configAudit.service';
import developmentOverrides from './env/development';
import stagingOverrides from './env/staging';
import productionOverrides from './env/production';

dotenv.config();

let configSource = 'environment';

const SENSITIVE_ENV_PATTERN = /secret|token|password|api[_-]?key|key/i;

function maskSecret(value: string | undefined): string {
  if (!value) return '****';
  if (value.length <= 8) return '****';
  return `${value.slice(0, 2)}${'*'.repeat(value.length - 4)}${value.slice(-2)}`;
}

function getSensitiveEnvAuditEntries(): string[] {
  return Object.entries(process.env)
    .filter(([key]) => SENSITIVE_ENV_PATTERN.test(key))
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([key, value]) => `${key}=${maskSecret(value)}`);
}

function parseCorsOrigins(): string[] {
  const raw = process.env.ALLOWED_ORIGINS;
  if (raw) {
    return raw
      .split(',')
      .map((o) => o.trim())
      .filter(Boolean);
  }
  // In production, default to restrictive (empty = deny all unlisted origins).
  // In other environments, allow all origins via wildcard sentinel '*'.
  return process.env.NODE_ENV === 'production' ? [] : ['*'];
}

const envConfigMap: Record<string, Partial<AppConfig>> = {
  development: developmentOverrides,
  staging: stagingOverrides,
  production: productionOverrides,
};

function loadEnvOverrides(): Partial<AppConfig> {
  const env = process.env.NODE_ENV || 'development';
  const overrides = envConfigMap[env];
  if (overrides) {
    configSource = `env/${env}`;
    configAuditService.record({
      timestamp: new Date().toISOString(),
      action: 'loaded',
      source: `env/${env}`,
    });
    return overrides;
  }
  configAuditService.record({
    timestamp: new Date().toISOString(),
    action: 'loaded',
    source: 'environment (no env file)',
  });
  return {};
}

function buildConfig(): AppConfig {
  const envOverrides = loadEnvOverrides();

  const cfg: AppConfig = {
    server: {
      port: parseInt(process.env.PORT || '3000', 10),
      env: process.env.NODE_ENV || 'development',
      ...(envOverrides.server || {}),
    },
    stellar: {
      network: process.env.STELLAR_NETWORK || 'testnet',
      horizonUrl: process.env.HORIZON_URL || 'https://horizon-testnet.stellar.org',
      sorobanRpcUrl:
        process.env.SOROBAN_RPC_URL || 'https://soroban-testnet.stellar.org',
      networkPassphrase:
        process.env.NETWORK_PASSPHRASE ||
        'Test SDF Network ; September 2015',
      contractId: process.env.CONTRACT_ID || '',
      readOnlySimulationAccount:
        process.env.READ_ONLY_SIMULATION_ACCOUNT ||
        'GDZZJ3UPZZCKY5DBH6ZGMPMRORRBG4ECIORASBUAXPPNCL4SYRHNLYU2',
      relayerSecret: process.env.RELAYER_SECRET || '',
      ...(envOverrides.stellar || {}),
    },
    auth: {
      jwtSecret: process.env.JWT_SECRET as string,
      jwtExpiresIn: process.env.JWT_EXPIRES_IN || '24h',
      ...(envOverrides.auth || {}),
    },
    rateLimit: {
      windowMs: parseInt(process.env.RATE_LIMIT_WINDOW_MS || '900000', 10),
      maxRequests: parseInt(process.env.RATE_LIMIT_MAX_REQUESTS || '100', 10),
      ...(envOverrides.rateLimit || {}),
    },
    logging: {
      level: process.env.LOG_LEVEL || 'info',
      ...(envOverrides.logging || {}),
    },
    request: {
      timeout: parseInt(process.env.REQUEST_TIMEOUT || '30000', 10),
      maxRetries: parseInt(process.env.MAX_RETRIES || '3', 10),
      retryInitialDelayMs: parseInt(process.env.RETRY_INITIAL_DELAY_MS || '1000', 10),
      retryMaxDelayMs: parseInt(process.env.RETRY_MAX_DELAY_MS || '10000', 10),
      ...(envOverrides.request || {}),
    },
    bodySizeLimit: {
      limit: process.env.BODY_SIZE_LIMIT || '100kb',
      ...(envOverrides.bodySizeLimit || {}),
    },
    pagination: {
      defaultLimit: parseInt(process.env.PAGINATION_DEFAULT_LIMIT || '10', 10),
      maxLimit: parseInt(process.env.PAGINATION_MAX_LIMIT || '100', 10),
      ...(envOverrides.pagination || {}),
    },
    cache: {
      redisEnabled: process.env.REDIS_ENABLED === 'true',
      redisUrl: process.env.REDIS_URL || '',
      idempotencyTtlMs: parseInt(
        process.env.IDEMPOTENCY_TTL_MS || '86400000',
        10
      ),
      idempotencyMaxEntries: parseInt(
        process.env.IDEMPOTENCY_MAX_ENTRIES || '1000',
        10
      ),
      protocolStatsTtlMs: parseInt(
        process.env.PROTOCOL_STATS_TTL_MS || '30000',
        10
      ),
      positionTtlMs: parseInt(process.env.POSITION_CACHE_TTL_MS || '15000', 10),
      poolTtlMs: parseInt(process.env.POOL_CACHE_TTL_MS || '30000', 10),
      ...(envOverrides.cache || {}),
    },
    ws: {
      priceUpdateIntervalMs: parseInt(
        process.env.WS_PRICE_UPDATE_INTERVAL_MS || '30000',
        10
      ),
      heartbeatIntervalMs: parseInt(
        process.env.WS_HEARTBEAT_INTERVAL_MS || '30000',
        10
      ),
      oracleApiUrl: process.env.ORACLE_API_URL || '',
      ...(envOverrides.ws || {}),
    },
    emergency: {
      autoPauseFailureThreshold: parseInt(
        process.env.AUTO_PAUSE_FAILURE_THRESHOLD || '5',
        10
      ),
      ...(envOverrides.emergency || {}),
    },
    analytics: {
      historyRetentionDays: parseInt(
        process.env.ANALYTICS_HISTORY_RETENTION_DAYS || '90',
        10
      ),
      snapshotIntervalMs: parseInt(
        process.env.ANALYTICS_SNAPSHOT_INTERVAL_MS || '60000',
        10
      ),
      ...(envOverrides.analytics || {}),
    },
    subscriptions: {
      maxRetries: parseInt(process.env.SUBSCRIPTION_MAX_RETRIES || '3', 10),
      retryBackoffMs: parseInt(
        process.env.SUBSCRIPTION_RETRY_BACKOFF_MS || '5000',
        10
      ),
      executionIntervalMs: parseInt(
        process.env.SUBSCRIPTION_EXECUTION_INTERVAL_MS || '300000',
        10
      ),
      ...(envOverrides.subscriptions || {}),
    },
    cors: {
      allowedOrigins: parseCorsOrigins(),
      ...(envOverrides.cors || {}),
    },
  };

  assertValidConfig(cfg);

  const sensitiveEntries = getSensitiveEnvAuditEntries();
  if (sensitiveEntries.length > 0) {
    configAuditService.record({
      timestamp: new Date().toISOString(),
      action: 'validated',
      source: configSource,
      changes: sensitiveEntries,
    });
  }

  return cfg;
}

export let config: AppConfig = buildConfig();

export function reloadConfig(): void {
  configSource = 'environment (reloaded)';
  config = buildConfig();
}

export function getConfigSource(): string {
  return configSource;
}

configAuditService.record({
  timestamp: new Date().toISOString(),
  action: 'loaded',
  source: configSource,
});

export * from './types';
