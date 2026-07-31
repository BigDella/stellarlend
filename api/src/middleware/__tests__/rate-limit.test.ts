import { Request, Response } from 'express';
import {
  getSensitiveRateLimitAnalytics,
  resetSensitiveRateLimits,
  sensitiveOperationRateLimiter,
} from '../rate-limit';

function makeResponse(): Response {
  const res = {
    setHeader: jest.fn(),
    status: jest.fn().mockReturnThis(),
    json: jest.fn().mockReturnThis(),
  };
  return res as unknown as Response;
}

function runLimiter(req: Partial<Request>, res: Response = makeResponse()): Response {
  const next = jest.fn();
  sensitiveOperationRateLimiter(req as Request, res, next);
  return res;
}

describe('sensitiveOperationRateLimiter', () => {
  beforeEach(() => {
    delete process.env.RATE_LIMIT_TRUSTED_USERS;
    resetSensitiveRateLimits();
  });

  it('sets standard rate limit headers for sensitive operations', () => {
    const res = runLimiter({
      path: '/prepare/borrow',
      params: {},
      query: { userAddress: 'GBORROWER' },
      body: {},
      headers: {},
      ip: '127.0.0.1',
    });

    expect(res.setHeader).toHaveBeenCalledWith('RateLimit-Limit', '8');
    expect(res.setHeader).toHaveBeenCalledWith('RateLimit-Remaining', '7');
    expect(res.setHeader).toHaveBeenCalledWith('X-RateLimit-Penalty', 'none');
  });

  it('returns a graduated throttle response after repeated violations', () => {
    let res = makeResponse();
    for (let i = 0; i < 10; i += 1) {
      res = runLimiter(
        {
          path: '/submit',
          params: {},
          query: {},
          body: { operation: 'borrow', userAddress: 'GBORROWER' },
          headers: {},
          ip: '127.0.0.1',
        },
        makeResponse()
      );
    }

    expect(res.status).toHaveBeenCalledWith(429);
    expect(res.json).toHaveBeenCalledWith(
      expect.objectContaining({ operation: 'borrow', penalty: 'throttle' })
    );
    expect(getSensitiveRateLimitAnalytics()[0]).toEqual(
      expect.objectContaining({ operation: 'borrow', userId: 'GBORROWER' })
    );
  });

  it('bypasses limits for trusted institutional users', () => {
    process.env.RATE_LIMIT_TRUSTED_USERS = 'GINSTITUTION';
    const res = runLimiter({
      path: '/prepare/withdraw',
      params: {},
      query: { userAddress: 'GINSTITUTION' },
      body: {},
      headers: {},
      ip: '127.0.0.1',
    });

    expect(res.setHeader).toHaveBeenCalledWith('X-RateLimit-Bypass', 'trusted-user');
    expect(res.status).not.toHaveBeenCalled();
  });
});
