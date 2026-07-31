import { Request } from 'express';

export interface AuthConfig {
  jwtSecret?: string;
  apiKeyHeader?: string;
  validateApiKey?: (key: string) => Promise<boolean>;
}

export interface JWTPayload {
  userId: string;
  email?: string;
  role?: string;
  [key: string]: any;
}

export interface RateLimitConfig {
  windowMs?: number;
  max?: number;
  message?: string;
  standardHeaders?: boolean;
  legacyHeaders?: boolean;
}

export interface LoggerConfig {
  level?: string;
  format?: string;
  silent?: boolean;
}

export interface RequestWithUser extends Request {
  user?: JWTPayload;
  correlationId?: string;
}
