import { Request, Response, NextFunction } from 'express';
import jwt from 'jsonwebtoken';
import { AuthConfig, JWTPayload, RequestWithUser } from './types';

export const validateApiKey = (config: AuthConfig) => {
  return async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const apiKeyHeader = config.apiKeyHeader || 'x-api-key';
      const apiKey = req.headers[apiKeyHeader] as string;

      if (!apiKey) {
        res.status(401).json({ error: 'API key required' });
        return;
      }

      if (config.validateApiKey) {
        const isValid = await config.validateApiKey(apiKey);
        if (!isValid) {
          res.status(401).json({ error: 'Invalid API key' });
          return;
        }
      }

      next();
    } catch (error) {
      res.status(500).json({ error: 'Authentication error' });
    }
  };
};

export const validateJWT = (config: AuthConfig) => {
  return (req: RequestWithUser, res: Response, next: NextFunction): void => {
    try {
      const authHeader = req.headers.authorization;

      if (!authHeader || !authHeader.startsWith('Bearer ')) {
        res.status(401).json({ error: 'JWT token required' });
        return;
      }

      const token = authHeader.substring(7);

      if (!config.jwtSecret) {
        res.status(500).json({ error: 'JWT secret not configured' });
        return;
      }

      const decoded = jwt.verify(token, config.jwtSecret) as JWTPayload;
      req.user = decoded;
      next();
    } catch (error) {
      if (error instanceof jwt.JsonWebTokenError) {
        res.status(401).json({ error: 'Invalid JWT token' });
        return;
      }
      res.status(500).json({ error: 'Authentication error' });
    }
  };
};

export const authMiddleware = {
  apiKey: validateApiKey,
  jwt: validateJWT,
};
