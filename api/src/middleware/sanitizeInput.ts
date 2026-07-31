import { NextFunction, Request, Response } from 'express';
import { ValidationError } from '../utils/errors';

const MAX_STRING_LENGTH = 512;

function sanitizeObject(value: unknown): unknown {
  if (typeof value === 'string') {
    const trimmed = value.trim();
    if (trimmed.length > MAX_STRING_LENGTH) {
      throw new ValidationError(`Input exceeds maximum length (${MAX_STRING_LENGTH})`);
    }
    return trimmed.replace(/[<>"'`]/g, '');
  }

  if (Array.isArray(value)) {
    return value.map((item) => sanitizeObject(item));
  }

  if (value && typeof value === 'object') {
    const obj = value as Record<string, unknown>;
    const sanitized: Record<string, unknown> = {};
    for (const [key, inner] of Object.entries(obj)) {
      sanitized[key] = sanitizeObject(inner);
    }
    return sanitized;
  }

  return value;
}

/** Paths that carry structured payloads (CSV/JSON) where stripping quotes breaks parsing. */
function skipSanitize(path: string): boolean {
  return path.startsWith('/api/subscriptions/import');
}

export function sanitizeInput(req: Request, _res: Response, next: NextFunction): void {
  if (skipSanitize(req.path)) {
    return next();
  }

  req.body = sanitizeObject(req.body) as Request['body'];
  req.query = sanitizeObject(req.query) as Request['query'];
  req.params = sanitizeObject(req.params) as Request['params'];
  next();
}
