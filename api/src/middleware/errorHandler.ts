import { Request, Response, NextFunction } from 'express';
import { ApiError, ErrorCode, type ErrorResponse } from '../utils/errors';
import logger from '../utils/logger';

/**
 * Generate a request ID from the request or create a new one
 */
function getRequestId(req: Request): string {
  return (req.headers['x-request-id'] as string) || `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
}

export const errorHandler = (err: Error, req: Request, res: Response, next: NextFunction) => {
  const requestId = getRequestId(req);

  logger.error('Error occurred:', {
    requestId,
    error: err.message,
    stack: err.stack,
    path: req.path,
    method: req.method,
  });

  if (err instanceof SyntaxError) {
    const errorResponse: ErrorResponse = {
      success: false,
      error: {
        code: ErrorCode.VALIDATION_ERROR,
        message: 'Invalid JSON',
        details: { syntax: true },
      },
      requestId,
    };
    return res.status(400).json(errorResponse);
  }

  if (err instanceof ApiError) {
    const errorResponse: ErrorResponse = {
      success: false,
      error: {
        code: err.code,
        message: err.message,
        ...(err.details ? { details: err.details } : {}),
      },
      requestId,
    };
    return res.status(err.statusCode).json(errorResponse);
  }

  const errorResponse: ErrorResponse = {
    success: false,
    error: {
      code: ErrorCode.INTERNAL_SERVER_ERROR,
      message: 'Internal server error',
    },
    requestId,
  };
  return res.status(500).json(errorResponse);
};
