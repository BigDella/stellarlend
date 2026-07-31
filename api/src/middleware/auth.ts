import { Request, Response, NextFunction } from 'express';
import jwt from 'jsonwebtoken';
import { config } from '../config';
import { UnauthorizedError } from '../utils/errors';
import { apiKeyService } from '../services/apiKey.service';

export interface AuthRequest extends Request {
  user?: {
    address: string;
  };
}

export const authenticateToken = (req: AuthRequest, res: Response, next: NextFunction) => {
  const authHeader = req.headers['authorization'];
  const token = authHeader && authHeader.split(' ')[1];

  if (!token) {
    throw new UnauthorizedError('Access token required');
  }

  try {
    const decoded = jwt.verify(token, config.auth.jwtSecret) as { address: string };
    req.user = decoded;
    next();
  } catch (error) {
    throw new UnauthorizedError('Invalid or expired token');
  }
};

export const generateToken = (address: string): string => {
  return jwt.sign({ address }, config.auth.jwtSecret, {
    expiresIn: config.auth.jwtExpiresIn,
  } as jwt.SignOptions);
};

/**
 * Issue #387 – API key authentication middleware.
 *
 * Accepts a raw API key in the X-API-Key header.
 * The key is verified against the bcrypt hash stored in apiKeyService.
 * The raw key is never logged.
 */
export const authenticateApiKey = async (
  req: AuthRequest,
  _res: Response,
  next: NextFunction
): Promise<void> => {
  const rawKey = req.headers['x-api-key'];

  if (!rawKey || typeof rawKey !== 'string') {
    throw new UnauthorizedError('X-API-Key header is required');
  }

  const result = await apiKeyService.verify(rawKey);
  if (!result.valid) {
    throw new UnauthorizedError('Invalid or revoked API key');
  }

  // Attach a synthetic user identity from the key record
  req.user = { address: result.record?.createdBy ?? 'api-key-user' };
  next();
};
