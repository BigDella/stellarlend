import { Request, Response, NextFunction } from 'express';
import { requestLogger } from '../middleware/requestLogger';

jest.mock('../config', () => ({
  config: {
    logging: { level: 'silent' },
  },
}));

jest.mock('../utils/logger', () => ({
  info: jest.fn(),
  warn: jest.fn(),
  error: jest.fn(),
}));

jest.mock('../utils/requestContext', () => ({
  requestContext: { getStore: jest.fn().mockReturnValue(undefined) },
}));

import logger from '../utils/logger';

function makeReq(overrides: Partial<Request> = {}): Request {
  return {
    id: 'test-req-id',
    method: 'GET',
    path: '/api/health',
    ip: '127.0.0.1',
    headers: { 'user-agent': 'jest' },
    ...overrides,
  } as unknown as Request;
}

function makeRes(statusCode: number): Response & { emit: jest.Mock; on: jest.Mock } {
  const listeners: Record<string, (() => void)[]> = {};
  const res = {
    statusCode,
    on: jest.fn((event: string, cb: () => void) => {
      listeners[event] = listeners[event] || [];
      listeners[event].push(cb);
    }),
    emit: jest.fn((event: string) => {
      (listeners[event] || []).forEach((cb) => cb());
    }),
  };
  return res as unknown as Response & { emit: jest.Mock; on: jest.Mock };
}

describe('requestLogger middleware', () => {
  const next: NextFunction = jest.fn();

  beforeEach(() => jest.clearAllMocks());

  it('calls next immediately', () => {
    const req = makeReq();
    const res = makeRes(200);
    requestLogger(req, res, next);
    expect(next).toHaveBeenCalledTimes(1);
  });

  it('logs at info level for 2xx responses', () => {
    const req = makeReq();
    const res = makeRes(200);
    requestLogger(req, res, next);
    res.emit('finish');
    expect(logger.info).toHaveBeenCalledTimes(1);
    const [msg, meta] = (logger.info as jest.Mock).mock.calls[0];
    expect(msg).toBe('HTTP request');
    expect(meta).toMatchObject({
      method: 'GET',
      path: '/api/health',
      statusCode: 200,
      requestId: 'test-req-id',
    });
  });

  it('logs at warn level for 4xx responses', () => {
    const req = makeReq({ method: 'POST', path: '/api/lending' } as Partial<Request>);
    const res = makeRes(404);
    requestLogger(req, res, next);
    res.emit('finish');
    expect(logger.warn).toHaveBeenCalledTimes(1);
    const [, meta] = (logger.warn as jest.Mock).mock.calls[0];
    expect(meta.statusCode).toBe(404);
  });

  it('logs at error level for 5xx responses', () => {
    const req = makeReq();
    const res = makeRes(500);
    requestLogger(req, res, next);
    res.emit('finish');
    expect(logger.error).toHaveBeenCalledTimes(1);
  });

  it('redacts sensitive headers', () => {
    const req = makeReq({
      headers: {
        'user-agent': 'jest',
        authorization: 'Bearer super-secret-token',
        'x-api-key': 'raw-api-key',
        'x-request-id': 'req-123',
      },
    } as Partial<Request>);
    const res = makeRes(200);
    requestLogger(req, res, next);
    res.emit('finish');
    const [, meta] = (logger.info as jest.Mock).mock.calls[0];
    expect(meta.headers['authorization']).toBe('[REDACTED]');
    expect(meta.headers['x-api-key']).toBe('[REDACTED]');
    expect(meta.headers['x-request-id']).toBe('req-123');
  });

  it('includes a numeric durationMs field', () => {
    const req = makeReq();
    const res = makeRes(201);
    requestLogger(req, res, next);
    res.emit('finish');
    const [, meta] = (logger.info as jest.Mock).mock.calls[0];
    expect(typeof meta.durationMs).toBe('number');
    expect(meta.durationMs).toBeGreaterThanOrEqual(0);
  });
});
