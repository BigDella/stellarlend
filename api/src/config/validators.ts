import { AppConfig } from './types';
import { ValidationError } from '../utils/errors';
import { configAuditService } from '../services/configAudit.service';

const VALID_ENVS = ['development', 'staging', 'production', 'test'] as const;
const VALID_NETWORKS = ['testnet', 'mainnet'] as const;
const VALID_LOG_LEVELS = ['error', 'warn', 'info', 'debug'] as const;

function isNonEmptyString(value: string): boolean {
  return Boolean(value?.trim());
}

function isValidUrl(value: string | undefined): boolean {
  if (!value) return false;
  try {
    new URL(value);
    return true;
  } catch {
    return false;
  }
}

export function validateConfig(config: AppConfig): string[] {
  const errors: string[] = [];

  if (!VALID_ENVS.includes(config.server.env as any)) {
    errors.push('NODE_ENV must be one of development, staging, production, test');
  }

  if (!VALID_NETWORKS.includes(config.stellar.network as any)) {
    errors.push('STELLAR_NETWORK must be testnet or mainnet');
  }

  if (!isValidUrl(config.stellar.horizonUrl)) {
    errors.push('HORIZON_URL must be a valid URL');
  }

  if (!isValidUrl(config.stellar.sorobanRpcUrl)) {
    errors.push('SOROBAN_RPC_URL must be a valid URL');
  }

  if (!isNonEmptyString(config.stellar.networkPassphrase)) {
    errors.push('NETWORK_PASSPHRASE is required');
  }

  if (!isNonEmptyString(config.stellar.contractId)) {
    errors.push('CONTRACT_ID is required');
  }

  if (!config.auth.jwtSecret || config.auth.jwtSecret.length < 32) {
    errors.push('JWT_SECRET must be at least 32 characters');
  }

  if (!isNonEmptyString(config.auth.jwtExpiresIn)) {
    errors.push('JWT_EXPIRES_IN is required');
  }

  if (config.server.port < 1 || config.server.port > 65535) {
    errors.push('PORT must be between 1 and 65535');
  }

  if (!VALID_LOG_LEVELS.includes(config.logging.level as any)) {
    errors.push('LOG_LEVEL must be one of error, warn, info, debug');
  }

  if (!isNonEmptyString(config.bodySizeLimit.limit)) {
    errors.push('BODY_SIZE_LIMIT is required');
  }

  if (config.cache.idempotencyTtlMs < 1000) {
    errors.push('IDEMPOTENCY_TTL_MS must be at least 1000');
  }

  if (config.cache.redisEnabled && !isNonEmptyString(config.cache.redisUrl)) {
    errors.push('REDIS_URL is required when REDIS_ENABLED=true');
  }

  if (config.cache.idempotencyMaxEntries < 1) {
    errors.push('IDEMPOTENCY_MAX_ENTRIES must be at least 1');
  }

  if (config.pagination.maxLimit < config.pagination.defaultLimit) {
    errors.push('PAGINATION_MAX_LIMIT must be >= PAGINATION_DEFAULT_LIMIT');
  }

  if (config.analytics.historyRetentionDays < 1) {
    errors.push('ANALYTICS_HISTORY_RETENTION_DAYS must be >= 1');
  }

  if (config.subscriptions.maxRetries < 0) {
    errors.push('SUBSCRIPTION_MAX_RETRIES must be >= 0');
  }

  if (config.ws.priceUpdateIntervalMs < 1000) {
    errors.push('WS_PRICE_UPDATE_INTERVAL_MS must be >= 1000');
  }

  if (config.ws.oracleApiUrl && !isValidUrl(config.ws.oracleApiUrl)) {
    errors.push('ORACLE_API_URL must be a valid URL when set');
  }

  if (config.emergency.autoPauseFailureThreshold < 1) {
    errors.push('AUTO_PAUSE_FAILURE_THRESHOLD must be >= 1');
  }

  if (config.server.env === 'production') {
    if (!config.cors.allowedOrigins.length) {
      errors.push('ALLOWED_ORIGINS is required in production');
    }
    if (config.cors.allowedOrigins.includes('*')) {
      errors.push('ALLOWED_ORIGINS must not include wildcard (*) in production');
    }
  }

  return errors;
}

export function assertValidConfig(config: AppConfig): void {
  const errors = validateConfig(config);
  configAuditService.record({
    timestamp: new Date().toISOString(),
    action: 'validated',
    source: config.server.env || 'environment',
    validationErrors: errors,
  });

  if (errors.length > 0) {
    throw new ValidationError(`Config validation failed: ${errors.join('; ')}`);
  }
}
