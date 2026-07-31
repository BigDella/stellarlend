import rateLimit from 'express-rate-limit';
import { RateLimitConfig } from './types';

export const createRateLimiter = (config: RateLimitConfig = {}) => {
  return rateLimit({
    windowMs: config.windowMs || 15 * 60 * 1000,
    max: config.max || 100,
    message: config.message || 'Too many requests from this IP, please try again later',
    standardHeaders: config.standardHeaders !== false,
    legacyHeaders: config.legacyHeaders !== false,
  });
};

export const rateLimitMiddleware = {
  standard: createRateLimiter(),
  strict: createRateLimiter({ windowMs: 15 * 60 * 1000, max: 50 }),
  lenient: createRateLimiter({ windowMs: 15 * 60 * 1000, max: 200 }),
};
