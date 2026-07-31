export interface ServerConfig {
  port: number;
  env: string;
}

export interface StellarConfig {
  network: string;
  horizonUrl: string;
  sorobanRpcUrl: string;
  networkPassphrase: string;
  contractId: string;
  readOnlySimulationAccount: string;
  relayerSecret: string;
}

export interface AuthConfig {
  jwtSecret: string;
  jwtExpiresIn: string;
}

export interface RateLimitConfig {
  windowMs: number;
  maxRequests: number;
}

export interface LoggingConfig {
  level: string;
}

export interface RequestConfig {
  timeout: number;
  maxRetries: number;
  retryInitialDelayMs: number;
  retryMaxDelayMs: number;
}

export interface BodySizeLimitConfig {
  limit: string;
}

export interface PaginationConfig {
  defaultLimit: number;
  maxLimit: number;
}

export interface CacheConfig {
  redisEnabled: boolean;
  redisUrl: string;
  idempotencyTtlMs: number;
  idempotencyMaxEntries: number;
  protocolStatsTtlMs: number;
  positionTtlMs: number;
  poolTtlMs: number;
}

export interface WsConfig {
  priceUpdateIntervalMs: number;
  heartbeatIntervalMs: number;
  oracleApiUrl: string;
}

export interface EmergencyConfig {
  autoPauseFailureThreshold: number;
}

export interface AnalyticsConfig {
  historyRetentionDays: number;
  snapshotIntervalMs: number;
}

export interface SubscriptionConfig {
  maxRetries: number;
  retryBackoffMs: number;
  executionIntervalMs: number;
}

export interface CorsConfig {
  allowedOrigins: string[];
}

export interface AppConfig {
  server: ServerConfig;
  stellar: StellarConfig;
  auth: AuthConfig;
  rateLimit: RateLimitConfig;
  logging: LoggingConfig;
  request: RequestConfig;
  bodySizeLimit: BodySizeLimitConfig;
  pagination: PaginationConfig;
  cache: CacheConfig;
  ws: WsConfig;
  emergency: EmergencyConfig;
  analytics: AnalyticsConfig;
  subscriptions: SubscriptionConfig;
  cors: CorsConfig;
}

export interface ConfigAuditEntry {
  timestamp: string;
  action: 'loaded' | 'reloaded' | 'updated' | 'validated';
  source: string;
  changes?: string[];
  validationErrors?: string[];
}
