import { Request, Response, NextFunction } from 'express';
import { v4 as uuidv4 } from 'uuid';
import { RequestWithUser } from './types';

export const requestIdMiddleware = (headerName: string = 'x-correlation-id') => {
  return (req: RequestWithUser, res: Response, next: NextFunction): void => {
    const existingId = req.headers[headerName] as string;
    const correlationId = existingId || uuidv4();

    req.correlationId = correlationId;
    res.setHeader(headerName, correlationId);

    next();
  };
};
