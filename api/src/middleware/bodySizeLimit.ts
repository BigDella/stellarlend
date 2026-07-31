import { Request, Response, NextFunction } from 'express';
import { config } from '../config';
import { PayloadTooLargeError } from '../utils/errors';

/**
 * Middleware to enforce a maximum request body size limit.
 * Returns 413 Payload Too Large when the limit is exceeded.
 */
export const bodySizeLimitMiddleware = (req: Request, res: Response, next: NextFunction): void => {
  const contentLength = req.headers['content-length'];

  if (contentLength) {
    const limitBytes = parseSizeLimit(config.bodySizeLimit.limit);
    const requestSize = parseInt(contentLength, 10);

    if (requestSize > limitBytes) {
      throw new PayloadTooLargeError(
        `Request body size (${requestSize} bytes) exceeds the allowed limit (${config.bodySizeLimit.limit})`
      );
    }
  }

  next();
};

/**
 * Parse size limit string (e.g., '100kb', '1mb', '1gb') to bytes.
 */
function parseSizeLimit(limit: string): number {
  const match = limit.toLowerCase().match(/^(\d+(?:\.\d+)?)\s*(b|kb|mb|gb)?$/);
  if (!match) {
    // Fallback to default 100kb if invalid format
    return 100 * 1024;
  }

  const value = parseFloat(match[1]!);
  const unit = match[2] || 'b';

  switch (unit) {
    case 'b':
      return Math.floor(value);
    case 'kb':
      return Math.floor(value * 1024);
    case 'mb':
      return Math.floor(value * 1024 * 1024);
    case 'gb':
      return Math.floor(value * 1024 * 1024 * 1024);
    default:
      return 100 * 1024;
  }
}
