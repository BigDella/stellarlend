// Mock StellarService before importing app
import { StellarService } from '../services/stellar.service';
import { idempotencyStore } from '../middleware/idempotency';
jest.mock('../services/stellar.service');

// Robust global Axios mock to prevent real network calls
import axios from 'axios';
jest.mock('axios');
const mockedAxios = axios as jest.Mocked<typeof axios>;

import request from 'supertest';
import app, { resetRateLimiters } from '../app';
import { emergencyPauseService } from '../services/emergencyPause.service';

const VALID_ADDRESS = 'GDZZJ3UPZZCKY5DBH6ZGMPMRORRBG4ECIORASBUAXPPNCL4SYRHNLYU2';
const VALID_AMOUNT = '10000000';

const mockStellarService: jest.Mocked<StellarService> = {
  buildUnsignedTransaction: jest.fn(),
  submitTransaction: jest.fn(),
  monitorTransaction: jest.fn(),
  getProtocolStats: jest.fn(),
  healthCheck: jest.fn(),
} as any;

beforeAll(() => {
  (StellarService as jest.Mock).mockImplementation(() => mockStellarService);

  mockedAxios.create.mockReturnThis();
  const axiosResponse = {
    data: {},
    status: 200,
    statusText: 'OK',
    headers: {},
    config: { url: '' },
  };
  mockedAxios.get.mockResolvedValue(axiosResponse);
  mockedAxios.post.mockResolvedValue(axiosResponse);
  mockedAxios.request.mockResolvedValue(axiosResponse);
});

beforeEach(async () => {
  jest.clearAllMocks();
  idempotencyStore.clear();
  emergencyPauseService.resume();
  emergencyPauseService.drainWithdrawalQueue();
  await resetRateLimiters();
  // Default happy-path mock responses
  mockStellarService.buildUnsignedTransaction.mockResolvedValue('unsigned_xdr_string');
  mockStellarService.submitTransaction.mockResolvedValue({
    success: true,
    transactionHash: 'abc123txhash',
    status: 'success',
  });
  mockStellarService.monitorTransaction.mockResolvedValue({
    success: true,
    transactionHash: 'abc123txhash',
    status: 'success',
    ledger: 12345,
  });
  mockStellarService.getProtocolStats.mockResolvedValue({
    totalDeposits: '1000000',
    totalBorrows: '500000',
    utilizationRate: '0.50',
    numberOfUsers: 150,
    tvl: '1500000',
  });
  mockStellarService.healthCheck.mockResolvedValue({ horizon: true, sorobanRpc: true });
});

// ─── 1. Complete Deposit Flow ─────────────────────────────────────────────────

describe('Complete Deposit Flow', () => {
  it('prepare returns unsigned XDR with correct shape', async () => {
    const res = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: VALID_ADDRESS, amount: VALID_AMOUNT });

    expect(res.status).toBe(200);
    expect(res.body).toMatchObject({
      unsignedXdr: 'unsigned_xdr_string',
      operation: 'deposit',
    });
    expect(typeof res.body.expiresAt).toBe('string');
    expect(new Date(res.body.expiresAt).getTime()).toBeGreaterThan(Date.now());
  });

  it('prepare calls buildUnsignedTransaction with correct args', async () => {
    await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: VALID_ADDRESS, amount: VALID_AMOUNT });

    expect(mockStellarService.buildUnsignedTransaction).toHaveBeenCalledTimes(1);
    expect(mockStellarService.buildUnsignedTransaction).toHaveBeenCalledWith(
      'deposit',
      VALID_ADDRESS,
      undefined,
      VALID_AMOUNT
    );
  });

  it('submit returns success with transaction hash and ledger', async () => {
    const res = await request(app)
      .post('/api/lending/submit')
      .send({ signedXdr: 'signed_xdr_payload' });

    expect(res.status).toBe(200);
    expect(res.body).toMatchObject({
      success: true,
      transactionHash: 'abc123txhash',
      status: 'success',
      ledger: 12345,
    });
  });

  it('submit calls monitorTransaction after successful submitTransaction', async () => {
    await request(app).post('/api/lending/submit').send({ signedXdr: 'signed_xdr_payload' });

    expect(mockStellarService.submitTransaction).toHaveBeenCalledWith('signed_xdr_payload');
    expect(mockStellarService.monitorTransaction).toHaveBeenCalledWith('abc123txhash');
  });

  it('full prepare → submit lifecycle returns consistent data', async () => {
    const prepareRes = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: VALID_ADDRESS, amount: VALID_AMOUNT });

    expect(prepareRes.status).toBe(200);
    expect(prepareRes.body.unsignedXdr).toBe('unsigned_xdr_string');

    const submitRes = await request(app)
      .post('/api/lending/submit')
      .send({ signedXdr: 'client_signed_xdr' });

    expect(submitRes.status).toBe(200);
    expect(submitRes.body.success).toBe(true);
    expect(submitRes.body.transactionHash).toBe('abc123txhash');
  });
});

// ─── 2. Error Handling ────────────────────────────────────────────────────────

describe('Error Handling', () => {
  it('returns 400 for an invalid operation name', async () => {
    const res = await request(app)
      .get('/api/lending/prepare/invalid_op')
      .query({ userAddress: VALID_ADDRESS, amount: VALID_AMOUNT });

    expect(res.status).toBe(400);
    expect(res.body).toHaveProperty('error');
  });

  it('returns 400 when userAddress is missing', async () => {
    const res = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ amount: VALID_AMOUNT });

    expect(res.status).toBe(400);
    expect(res.body.error.message).toMatch(/address/i);
  });

  it('returns 400 when amount is missing', async () => {
    const res = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: VALID_ADDRESS });

    expect(res.status).toBe(400);
    expect(res.body.error.message).toMatch(/amount/i);
  });

  it('returns 400 when userAddress is not a valid Stellar key', async () => {
    const res = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: 'NOT_A_STELLAR_ADDRESS', amount: VALID_AMOUNT });

    expect(res.status).toBe(400);
    expect(res.body.error.message).toMatch(/stellar address/i);
  });

  it('returns 400 when signedXdr is missing on submit', async () => {
    const res = await request(app).post('/api/lending/submit').send({});

    expect(res.status).toBe(400);
    expect(res.body.error.message).toMatch(/signedXdr|invalid value/i);
  });

  it('returns 400 when submit receives malformed JSON', async () => {
    const res = await request(app)
      .post('/api/lending/submit')
      .set('Content-Type', 'application/json')
      .send('{ bad json }');

    expect(res.status).toBe(400);
  });

  it('returns 500 when stellar service fails to build transaction', async () => {
    mockStellarService.buildUnsignedTransaction.mockRejectedValueOnce(
      new Error('Stellar network error')
    );

    const res = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: VALID_ADDRESS, amount: VALID_AMOUNT });

    expect(res.status).toBe(500);
    expect(res.body).toHaveProperty('error');
  });

  it('returns 400 from submit when submitTransaction reports failure', async () => {
    mockStellarService.submitTransaction.mockResolvedValueOnce({
      success: false,
      status: 'failed',
      error: 'tx_bad_seq',
    });

    const res = await request(app).post('/api/lending/submit').send({ signedXdr: 'bad_xdr' });

    expect(res.status).toBe(400);
    expect(res.body.success).toBe(false);
    expect(res.body.error).toBe('tx_bad_seq');
  });

  it('health endpoint returns 503 when services are down', async () => {
    mockStellarService.healthCheck.mockResolvedValueOnce({
      horizon: false,
      sorobanRpc: false,
    });

    const res = await request(app).get('/api/health');

    expect(res.status).toBe(503);
    expect(res.body.status).toBe('unhealthy');
    expect(res.body.services.horizon).toBe(false);
    expect(res.body.services.sorobanRpc).toBe(false);
  });

  it('liveness endpoint returns immediately without upstream checks', async () => {
    const res = await request(app).get('/api/health/live');

    expect(res.status).toBe(200);
    expect(res.body).toMatchObject({ status: 'ok' });
    expect(mockStellarService.healthCheck).not.toHaveBeenCalled();
  });

  it('readiness endpoint returns dependency status details', async () => {
    mockStellarService.healthCheck.mockResolvedValueOnce({
      horizon: true,
      sorobanRpc: false,
    });

    const res = await request(app).get('/api/health/ready');

    expect(res.status).toBe(503);
    expect(res.body).toMatchObject({
      status: 'error',
      horizon: 'up',
      soroban: 'down',
    });
  });
});

// ─── 3. Edge Cases ────────────────────────────────────────────────────────────

describe('Edge Cases', () => {
  it('rejects amount of zero', async () => {
    const res = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: VALID_ADDRESS, amount: '0' });

    expect(res.status).toBe(400);
    expect(res.body.error.message).toMatch(/amount/i);
  });

  it('rejects negative amount', async () => {
    const res = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: VALID_ADDRESS, amount: '-500' });

    expect(res.status).toBe(400);
    expect(res.body.error.message).toMatch(/amount/i);
  });

  it('accepts optional assetAddress when provided', async () => {
    const res = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: VALID_ADDRESS, amount: VALID_AMOUNT, assetAddress: VALID_ADDRESS });

    expect(res.status).toBe(200);
    expect(mockStellarService.buildUnsignedTransaction).toHaveBeenCalledWith(
      'deposit',
      VALID_ADDRESS,
      VALID_ADDRESS,
      VALID_AMOUNT
    );
  });

  it('works without optional assetAddress', async () => {
    const res = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: VALID_ADDRESS, amount: VALID_AMOUNT });

    expect(res.status).toBe(200);
    expect(mockStellarService.buildUnsignedTransaction).toHaveBeenCalledWith(
      'deposit',
      VALID_ADDRESS,
      undefined,
      VALID_AMOUNT
    );
  });

  it('all four valid operations are accepted by prepare', async () => {
    for (const op of ['deposit', 'borrow', 'repay', 'withdraw']) {
      const res = await request(app)
        .get(`/api/lending/prepare/${op}`)
        .query({ userAddress: VALID_ADDRESS, amount: VALID_AMOUNT });

      expect(res.status).toBe(200);
      expect(res.body.operation).toBe(op);
    }
  });
});

// ─── 4. Security Headers ──────────────────────────────────────────────────────

describe('Protocol Stats', () => {
  it('returns protocol statistics with cache headers', async () => {
    const res = await request(app).get('/api/protocol/stats');

    expect(res.status).toBe(200);
    expect(res.body).toMatchObject({
      totalDeposits: '1000000',
      totalBorrows: '500000',
      utilizationRate: '0.50',
      numberOfUsers: 150,
      tvl: '1500000',
    });
    expect(res.headers['cache-control']).toBe('public, max-age=30');
  });
});

describe('Idempotency', () => {
  const idemKey = '123e4567-e89b-12d3-a456-426614174000';

  it('replays cached submit responses for duplicate POST requests', async () => {
    const first = await request(app)
      .post('/api/lending/submit')
      .set('Idempotency-Key', idemKey)
      .send({ signedXdr: 'signed_xdr_payload' });

    const second = await request(app)
      .post('/api/lending/submit')
      .set('Idempotency-Key', idemKey)
      .send({ signedXdr: 'signed_xdr_payload' });

    expect(first.status).toBe(200);
    expect(first.headers['idempotency-status']).toBe('created');
    expect(second.status).toBe(200);
    expect(second.headers['idempotency-status']).toBe('cached');
    expect(second.body).toEqual(first.body);
    expect(mockStellarService.submitTransaction).toHaveBeenCalledTimes(1);
    expect(mockStellarService.monitorTransaction).toHaveBeenCalledTimes(1);
  });

  it('rejects invalid idempotency keys', async () => {
    const res = await request(app)
      .post('/api/lending/submit')
      .set('Idempotency-Key', 'not-a-uuid')
      .send({ signedXdr: 'signed_xdr_payload' });

    expect(res.status).toBe(400);
    expect(res.body.error.message).toMatch(/Idempotency-Key/i);
  });
});

describe('Security Headers', () => {
  it('includes x-content-type-options header', async () => {
    const res = await request(app).get('/api/health');
    expect(res.headers['x-content-type-options']).toBe('nosniff');
  });

  it('includes x-frame-options header', async () => {
    const res = await request(app).get('/api/health');
    expect(res.headers['x-frame-options']).toBeDefined();
  });

  it('includes strict-transport-security header', async () => {
    const res = await request(app).get('/api/health');
    expect(res.headers['strict-transport-security']).toMatch(/max-age/);
  });

  it('responds to OPTIONS preflight requests', async () => {
    const res = await request(app).options('/api/lending/prepare/deposit');
    expect([200, 204]).toContain(res.status);
  });

  it('health endpoint returns healthy status with correct shape', async () => {
    const res = await request(app).get('/api/health');

    expect(res.status).toBe(200);
    expect(res.body).toMatchObject({
      status: 'healthy',
      services: { horizon: true, sorobanRpc: true },
    });
    expect(typeof res.body.timestamp).toBe('string');
  });
});

// ─── 6. Per-User Rate Limiting ──────────────────────────────────────────────────────

describe('Per-User Rate Limiting', () => {
  const USER_1 = 'GDZZJ3UPZZCKY5DBH6ZGMPMRORRBG4ECIORASBUAXPPNCL4SYRHNLYU2';
  const USER_2 = 'GBBM6BKZPEHWYO3E3YKREDPQXMS4VK35YLNU7NFBRI26RAN7GI5POFBB';

  beforeEach(() => {
    // Clear rate limit memory stores by restarting the app
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('allows different users to make requests independently', async () => {
    // User 1 makes 5 requests
    const user1Requests = Array.from({ length: 5 }, () =>
      request(app)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: USER_1, amount: VALID_AMOUNT })
    );

    // User 2 makes 5 requests
    const user2Requests = Array.from({ length: 5 }, () =>
      request(app)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: USER_2, amount: VALID_AMOUNT })
    );

    const allResponses = await Promise.all([...user1Requests, ...user2Requests]);

    // All should succeed since each user is under their 10 req/min limit
    allResponses.forEach((res) => {
      expect(res.status).toBe(200);
    });
  });

  it('enforces per-user rate limit for requests with userAddress in query params', async () => {
    // Make 10 successful requests (at the limit)
    const successfulRequests = Array.from({ length: 10 }, () =>
      request(app)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: USER_1, amount: VALID_AMOUNT })
    );

    const successfulResponses = await Promise.all(successfulRequests);
    successfulResponses.forEach((res) => {
      expect(res.status).toBe(200);
    });

    // 11th request should be rate limited
    const rateLimitedResponse = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: USER_1, amount: VALID_AMOUNT });

    expect(rateLimitedResponse.status).toBe(429);
    expect(rateLimitedResponse.body).toMatchObject({
      success: false,
      error: 'Too many requests for this account',
    });
  });

  it('enforces per-user rate limit for requests with userAddress in request body', async () => {
    // Make 10 successful POST requests (at the limit)
    const successfulRequests = Array.from({ length: 10 }, () =>
      request(app).post('/api/lending/submit').send({
        signedXdr: 'signed_xdr_payload',
        userAddress: USER_1,
      })
    );

    const successfulResponses = await Promise.all(successfulRequests);
    successfulResponses.forEach((res) => {
      expect([200, 400]).toContain(res.status); // 400 if XDR is invalid, but not 429
    });

    // 11th request should be rate limited
    const rateLimitedResponse = await request(app).post('/api/lending/submit').send({
      signedXdr: 'signed_xdr_payload',
      userAddress: USER_1,
    });

    expect(rateLimitedResponse.status).toBe(429);
    expect(rateLimitedResponse.body).toMatchObject({
      success: false,
      error: 'Too many requests for this account',
    });
  });

  it('falls back to IP-based limiting when userAddress is not provided', async () => {
    // Make requests without userAddress - should fall back to IP limiting
    const requestsWithoutAddress = Array.from(
      { length: 5 },
      () => request(app).post('/api/lending/submit').send({ signedXdr: 'signed_xdr_payload' }) // No userAddress
    );

    const responses = await Promise.all(requestsWithoutAddress);

    // These should be handled by the IP-based limiter
    // Since we're only making 5 requests, they should succeed
    responses.forEach((res) => {
      expect(res.status).toBe(200);
    });
  });

  it('resets per-user rate limit after window expires', async () => {
    // Make 10 requests to hit the limit
    const initialRequests = Array.from({ length: 10 }, () =>
      request(app)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: USER_1, amount: VALID_AMOUNT })
    );

    await Promise.all(initialRequests);

    // 11th request should be rate limited
    const rateLimitedResponse = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: USER_1, amount: VALID_AMOUNT });
    expect(rateLimitedResponse.status).toBe(429);

    // Advance time by 61 seconds (past the 60-second window)
    jest.advanceTimersByTime(61 * 1000);

    // New request should succeed after window reset
    const resetResponse = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: USER_1, amount: VALID_AMOUNT });
    expect(resetResponse.status).toBe(200);
  });

  it('does not affect non-lending endpoints', async () => {
    // Make many requests to health endpoint - should not be affected by user rate limiting
    const healthRequests = Array.from({ length: 15 }, () => request(app).get('/api/health'));

    const responses = await Promise.all(healthRequests);
    responses.forEach((res) => {
      expect(res.status).toBe(200);
    });
  });

  it('handles mixed userAddress sources (query vs body) correctly', async () => {
    // Mix requests with userAddress in query params and body
    const queryRequests = Array.from({ length: 5 }, () =>
      request(app)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: USER_1, amount: VALID_AMOUNT })
    );

    const bodyRequests = Array.from({ length: 5 }, () =>
      request(app).post('/api/lending/submit').send({
        signedXdr: 'signed_xdr_payload',
        userAddress: USER_1,
      })
    );

    const allResponses = await Promise.all([...queryRequests, ...bodyRequests]);

    // All should succeed since they're from the same user but under the limit
    allResponses.forEach((res) => {
      expect([200, 400]).toContain(res.status); // 400 for invalid XDR, but not 429
    });

    // 11th request for the same user should be rate limited
    const rateLimitedResponse = await request(app)
      .get('/api/lending/prepare/deposit')
      .query({ userAddress: USER_1, amount: VALID_AMOUNT });
    expect(rateLimitedResponse.status).toBe(429);
  });
});

// ─── 7. Contract Verification ──────────────────────────────────────

describe('Contract Verification', () => {
  it('returns 400 when contractId is missing', async () => {
    const res = await request(app).get('/api/verification');

    expect(res.status).toBe(400);
    expect(res.body.error).toMatch(/contractId/);
  });

  it('returns 400 for invalid contract ID', async () => {
    const res = await request(app).get('/api/verification').query({ contractId: 'invalid' });

    expect(res.status).toBe(400);
    expect(res.body.error).toMatch(/Unable to determine source path/);
  });

  // Note: Full verification test would require mocking exec and the script,
  // but for now we test the basic validation
});

// ─── 8. IP-based Rate Limiting (Outer Layer) ──────────────────────────────────────

describe('IP-based Rate Limiting (Outer Layer)', () => {
  it('still applies to all API endpoints', async () => {
    // This test verifies that the original IP-based limiter still works
    // We'll make requests to different endpoints to ensure the outer layer is active

    const requests = Array.from({ length: 105 }, () =>
      Promise.race([
        request(app).get('/api/health'),
        request(app).get('/api/lending/prepare/deposit').query({
          userAddress: VALID_ADDRESS,
          amount: VALID_AMOUNT,
        }),
        request(app).get('/api/openapi.json'),
      ])
    );

    const responses = await Promise.all(requests);
    const statuses = responses.map((r: { status: number }) => r.status);

    // Should have some successful requests
    expect(statuses.some((s: number) => s === 200)).toBe(true);
    // Should have some rate limited requests (429)
    expect(statuses.some((s: number) => s === 429)).toBe(true);
  });
});

// ─── 9. Concurrent Request Handling ──────────────────────────────────────────
//
// Race condition coverage:
//  - Simultaneous deposits from the same user must each get a unique response
//  - No double-processing: submitTransaction called exactly once per request
//  - Idempotency key deduplication holds under concurrency
//  - Concurrent requests from different users are isolated

describe('Concurrent Request Handling', () => {
  it('processes simultaneous deposits independently without double-processing', async () => {
    const CONCURRENCY = 5;

    const responses = await Promise.all(
      Array.from({ length: CONCURRENCY }, () =>
        request(app)
          .get('/api/lending/prepare/deposit')
          .query({ userAddress: VALID_ADDRESS, amount: VALID_AMOUNT })
      )
    );

    // Every request must succeed
    responses.forEach((res) => {
      expect(res.status).toBe(200);
      expect(res.body.unsignedXdr).toBe('unsigned_xdr_string');
      expect(res.body.operation).toBe('deposit');
    });

    // buildUnsignedTransaction called exactly once per concurrent request — no merging
    expect(mockStellarService.buildUnsignedTransaction).toHaveBeenCalledTimes(CONCURRENCY);
  });

  it('submits concurrent transactions without double-processing', async () => {
    const CONCURRENCY = 5;

    const responses = await Promise.all(
      Array.from({ length: CONCURRENCY }, () =>
        request(app)
          .post('/api/lending/submit')
          .send({ signedXdr: 'signed_xdr_payload' })
      )
    );

    // All must succeed
    responses.forEach((res) => {
      expect(res.status).toBe(200);
      expect(res.body.success).toBe(true);
      expect(res.body.transactionHash).toBe('abc123txhash');
    });

    // submitTransaction called exactly once per request — no batching or skipping
    expect(mockStellarService.submitTransaction).toHaveBeenCalledTimes(CONCURRENCY);
    expect(mockStellarService.monitorTransaction).toHaveBeenCalledTimes(CONCURRENCY);
  });

  it('deduplicates concurrent requests sharing the same idempotency key', async () => {
    const idemKey = 'a1b2c3d4-e5f6-4890-abcd-ef1234567890';
    const CONCURRENCY = 5;

    const responses = await Promise.all(
      Array.from({ length: CONCURRENCY }, () =>
        request(app)
          .post('/api/lending/submit')
          .set('Idempotency-Key', idemKey)
          .send({ signedXdr: 'signed_xdr_payload' })
      )
    );

    // All responses must be 200
    responses.forEach((res) => {
      expect(res.status).toBe(200);
      expect(res.body.success).toBe(true);
    });

    // Despite 5 concurrent requests, the transaction is submitted only once
    expect(mockStellarService.submitTransaction).toHaveBeenCalledTimes(1);

    // All responses carry the same transaction hash
    const hashes = responses.map((r) => r.body.transactionHash);
    expect(new Set(hashes).size).toBe(1);
  });

  it('isolates concurrent requests from different users', async () => {
    const USER_A = 'GDZZJ3UPZZCKY5DBH6ZGMPMRORRBG4ECIORASBUAXPPNCL4SYRHNLYU2';
    const USER_B = 'GBBM6BKZPEHWYO3E3YKREDPQXMS4VK35YLNU7NFBRI26RAN7GI5POFBB';

    const [userAResponses, userBResponses] = await Promise.all([
      Promise.all(
        Array.from({ length: 3 }, () =>
          request(app)
            .get('/api/lending/prepare/deposit')
            .query({ userAddress: USER_A, amount: VALID_AMOUNT })
        )
      ),
      Promise.all(
        Array.from({ length: 3 }, () =>
          request(app)
            .get('/api/lending/prepare/deposit')
            .query({ userAddress: USER_B, amount: VALID_AMOUNT })
        )
      ),
    ]);

    // Both users get successful responses
    [...userAResponses, ...userBResponses].forEach((res) => {
      expect(res.status).toBe(200);
    });

    // Each request triggers exactly one service call — 6 total
    expect(mockStellarService.buildUnsignedTransaction).toHaveBeenCalledTimes(6);

    // User A's calls used USER_A address
    const userACalls = mockStellarService.buildUnsignedTransaction.mock.calls.filter(
      (args) => args[1] === USER_A
    );
    expect(userACalls).toHaveLength(3);

    // User B's calls used USER_B address
    const userBCalls = mockStellarService.buildUnsignedTransaction.mock.calls.filter(
      (args) => args[1] === USER_B
    );
    expect(userBCalls).toHaveLength(3);
  });

  it('handles concurrent failures without affecting successful requests', async () => {
    // First call fails, rest succeed
    mockStellarService.buildUnsignedTransaction
      .mockRejectedValueOnce(new Error('Stellar network error'))
      .mockResolvedValue('unsigned_xdr_string');

    const responses = await Promise.all(
      Array.from({ length: 4 }, () =>
        request(app)
          .get('/api/lending/prepare/deposit')
          .query({ userAddress: VALID_ADDRESS, amount: VALID_AMOUNT })
      )
    );

    const statuses = responses.map((r) => r.status);

    // Exactly one failure
    expect(statuses.filter((s) => s === 500)).toHaveLength(1);
    // Remaining three succeed
    expect(statuses.filter((s) => s === 200)).toHaveLength(3);
  });
});
