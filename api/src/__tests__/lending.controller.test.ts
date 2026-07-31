// Robust global Axios mock to prevent real network calls
import axios from 'axios';
jest.mock('axios');
const mockedAxios = axios as jest.Mocked<typeof axios>;
beforeAll(() => {
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
afterEach(() => {
  jest.clearAllMocks();
});

// Mock StellarService before importing app
import { StellarService } from '../services/stellar.service';
import { idempotencyStore } from '../middleware/idempotency';
jest.mock('../services/stellar.service');
const mockStellarService: jest.Mocked<StellarService> = {
  buildUnsignedTransaction: jest.fn().mockResolvedValue('unsigned_xdr_string'),
  submitTransaction: jest.fn().mockResolvedValue({
    success: true,
    transactionHash: 'mock_tx_hash',
    status: 'success',
  }),
  monitorTransaction: jest.fn().mockResolvedValue({
    success: true,
    transactionHash: 'mock_tx_hash',
    status: 'success',
    ledger: 12345,
  }),
  getProtocolStats: jest.fn().mockResolvedValue({
    totalDeposits: '1000000',
    totalBorrows: '500000',
    utilizationRate: '0.50',
    numberOfUsers: 150,
    tvl: '1500000',
  }),
  healthCheck: jest.fn().mockResolvedValue({
    horizon: true,
    sorobanRpc: true,
  }),
  getTransactionHistory: jest.fn().mockResolvedValue({
    data: [
      {
        transactionHash: 'tx_hash_1',
        type: 'deposit',
        amount: '1000000',
        assetAddress: 'GTEST123...',
        timestamp: '2023-01-01T00:00:00Z',
        status: 'success',
        ledger: 12345,
      },
    ],
    pagination: {
      cursor: null,
      hasMore: false,
      limit: 10,
      total: null,
    },
  }),
  streamTransactionHistory: jest.fn().mockImplementation(async function* () {
    yield {
      transactionHash: 'tx_hash_stream_1',
      type: 'deposit',
      amount: '500000',
      assetAddress: 'GTEST123...',
      timestamp: '2023-01-01T00:00:00Z',
      status: 'success',
      ledger: 11111,
    };
    yield {
      transactionHash: 'tx_hash_stream_2',
      type: 'borrow',
      amount: '250000',
      timestamp: '2023-01-02T00:00:00Z',
      status: 'success',
      ledger: 11112,
    };
  }),
} as any;
(StellarService as jest.Mock).mockImplementation(() => mockStellarService);

// Mock logger to capture audit logs
import logger from '../utils/logger';
jest.mock('../utils/logger');
const mockLogger = logger as jest.Mocked<typeof logger>;

import request from 'supertest';
import app, { resetRateLimiters } from '../app';

describe('Lending Controller', () => {
  beforeEach(async () => {
    jest.clearAllMocks();
    idempotencyStore.clear();
    await resetRateLimiters();
  });

  describe('GET /api/lending/prepare/:operation', () => {
    const validBody = {
      userAddress: 'GDZZJ3UPZZCKY5DBH6ZGMPMRORRBG4ECIORASBUAXPPNCL4SYRHNLYU2',
      amount: '1000000',
    };

    it.each(['deposit', 'borrow', 'repay', 'withdraw'])(
      'should return unsigned XDR for %s',
      async (operation) => {
        const response = await request(app)
          .get(`/api/lending/prepare/${operation}`)
          .send(validBody);

        expect(response.status).toBe(200);
        expect(response.body.unsignedXdr).toBe('unsigned_xdr_string');
        expect(response.body.operation).toBe(operation);
        expect(response.body.expiresAt).toBeDefined();
      }
    );

    it('should return 400 for invalid operation', async () => {
      const response = await request(app).get('/api/lending/prepare/invalid_op').send(validBody);

      expect(response.status).toBe(400);
    });

    it('should return 400 for missing userAddress', async () => {
      const response = await request(app)
        .get('/api/lending/prepare/deposit')
        .send({ amount: '1000000' });

      expect(response.status).toBe(400);
    });

    it('should return 400 for zero amount', async () => {
      const response = await request(app)
        .get('/api/lending/prepare/deposit')
        .send({ ...validBody, amount: '0' });

      expect(response.status).toBe(400);
    });

    it('should not accept userSecret in request body', async () => {
      const response = await request(app)
        .get('/api/lending/prepare/deposit')
        .send({ ...validBody, userSecret: 'SXXXXX' });

      expect(response.status).toBe(200);
      // userSecret must never be forwarded to the service
      expect(mockStellarService.buildUnsignedTransaction).toHaveBeenCalledWith(
        'deposit',
        validBody.userAddress,
        undefined,
        validBody.amount
      );
    });
  });

  describe('POST /api/lending/submit', () => {
    it('should submit signed XDR and return transaction result', async () => {
      const response = await request(app)
        .post('/api/lending/submit')
        .send({ signedXdr: 'signed_xdr_string' });

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);
      expect(response.body.transactionHash).toBe('mock_tx_hash');
    });

    it('should log audit entry when transaction succeeds with full audit data', async () => {
      const auditData = {
        signedXdr: 'signed_xdr_string',
        operation: 'deposit',
        userAddress: 'GDZZJ3UPZZCKY5DBH6ZGMPMRORRBG4ECIORASBUAXPPNCL4SYRHNLYU2',
        amount: '1000000',
        assetAddress: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAH2U',
      };

      const response = await request(app).post('/api/lending/submit').send(auditData);

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);

      // Verify audit log was called with correct structure
      expect(mockLogger.info).toHaveBeenCalledWith(
        'AUDIT',
        expect.objectContaining({
          action: 'DEPOSIT',
          actor: 'GDZZJ3UPZZCKY5DBH6ZGMPMRORRBG4ECIORASBUAXPPNCL4SYRHNLYU2',
          txHash: 'mock_tx_hash',
          timestamp: expect.any(String),
          status: 'success',
          ledger: 12345,
          id: expect.any(String),
          sequence: expect.any(Number),
          hash: expect.any(String),
        })
      );
    });

    it('should log audit entry with redacted data when audit fields are missing', async () => {
      const response = await request(app)
        .post('/api/lending/submit')
        .send({ signedXdr: 'signed_xdr_string' });

      expect(response.status).toBe(200);
      expect(response.body.success).toBe(true);

      // Verify audit log was called with redacted values
      expect(mockLogger.info).toHaveBeenCalledWith(
        'AUDIT',
        expect.objectContaining({
          action: 'TRANSACTION_EXECUTED',
          actor: 'REDACTED',
          txHash: 'mock_tx_hash',
          timestamp: expect.any(String),
          status: 'success',
          ledger: 12345,
          id: expect.any(String),
          sequence: expect.any(Number),
          hash: expect.any(String),
        })
      );
    });

    it('should validate optional audit fields when provided', async () => {
      const response = await request(app).post('/api/lending/submit').send({
        signedXdr: 'signed_xdr_string',
        operation: 'invalid_operation',
        userAddress: 'invalid_address',
        amount: 'invalid_amount',
      });

      expect(response.status).toBe(400);
    });

    it('should return 400 when transaction fails', async () => {
      mockStellarService.submitTransaction.mockResolvedValueOnce({
        success: false,
        status: 'failed',
        error: 'Insufficient collateral',
      });

      const response = await request(app)
        .post('/api/lending/submit')
        .send({ signedXdr: 'signed_xdr_string' });

      expect(response.status).toBe(400);
      expect(response.body.success).toBe(false);

      // No audit log should be generated for failed transactions
      expect(mockLogger.info).not.toHaveBeenCalledWith('AUDIT', expect.any(Object));
    });

    it('should return 400 when signedXdr is missing', async () => {
      const response = await request(app).post('/api/lending/submit').send({});

      expect(response.status).toBe(400);
    });

    it('should never log secrets in audit entries', async () => {
      const response = await request(app).post('/api/lending/submit').send({
        signedXdr: 'signed_xdr_string',
        operation: 'deposit',
        userAddress: 'GDZZJ3UPZZCKY5DBH6ZGMPMRORRBG4ECIORASBUAXPPNCL4SYRHNLYU2',
        amount: '1000000',
        // Note: userSecret should not be a field that gets logged
      });

      expect(response.status).toBe(200);

      // Verify audit log does not contain any secret fields
      const auditCall = (mockLogger.info.mock.calls as Array<any[]>).find(
        (call) => call[0] === 'AUDIT' && typeof call[1] === 'object' && call[1] !== null
      );
      expect(auditCall).toBeDefined();
      const auditData = (auditCall?.[1] ?? {}) as Record<string, unknown>;

      // Ensure no secret fields are present
      expect(Object.keys(auditData)).not.toContain('userSecret');
      expect(Object.keys(auditData)).not.toContain('privateKey');
      expect(Object.keys(auditData)).not.toContain('secret');
    });
  });

  describe('GET /api/lending/transactions/:userAddress', () => {
    const userAddress = 'GDZZJ3UPZZCKY5DBH6ZGMPMRORRBG4ECIORASBUAXPPNCL4SYRHNLYU2';

    it('should return paginated transaction history with default limit', async () => {
      const response = await request(app).get(`/api/lending/transactions/${userAddress}`);

      expect(response.status).toBe(200);
      expect(response.body).toHaveProperty('data');
      expect(Array.isArray(response.body.data)).toBe(true);
      expect(response.body).toHaveProperty('pagination');
      expect(response.body.pagination).toEqual({
        cursor: null,
        hasMore: false,
        limit: 10,
        total: null,
      });
      expect(response.body.data[0]).toMatchObject({ transactionHash: 'tx_hash_1' });
    });

    it('should validate query limit', async () => {
      const response = await request(app)
        .get(`/api/lending/transactions/${userAddress}`)
        .query({ limit: 9999 });

      expect(response.status).toBe(400);
      expect(response.body.error.message).toMatch(/limit/i);
    });

    it('should validate cursor is non-empty', async () => {
      const response = await request(app)
        .get(`/api/lending/transactions/${userAddress}`)
        .query({ cursor: '' });

      expect(response.status).toBe(400);
      expect(response.body.error.message).toMatch(/cursor/i);
    });

    it('should reject a cursor that is not valid base64url', async () => {
      const response = await request(app)
        .get(`/api/lending/transactions/${userAddress}`)
        .query({ cursor: '!!!not-a-valid-cursor!!!' });

      expect(response.status).toBe(400);
      expect(response.body.error.message).toMatch(/cursor/i);
    });

    it('should return an opaque base64url cursor when hasMore is true', async () => {
      mockStellarService.getTransactionHistory.mockResolvedValueOnce({
        data: [],
        pagination: {
          cursor: 'aGVsbG8',
          hasMore: true,
          limit: 10,
          total: null,
        },
      });

      const response = await request(app).get(`/api/lending/transactions/${userAddress}`);

      expect(response.status).toBe(200);
      expect(response.body.pagination.hasMore).toBe(true);
      expect(response.body.pagination.cursor).not.toBeNull();
      // Cursor must be a non-empty string opaque to the client
      expect(typeof response.body.pagination.cursor).toBe('string');
      expect(response.body.pagination.cursor.length).toBeGreaterThan(0);
    });

    it('should include total: null on the first page when total count is unavailable', async () => {
      const response = await request(app).get(`/api/lending/transactions/${userAddress}`);

      expect(response.status).toBe(200);
      expect(response.body.pagination).toHaveProperty('total');
      expect(response.body.pagination.total).toBeNull();
    });

    it('should respect a custom limit parameter', async () => {
      mockStellarService.getTransactionHistory.mockResolvedValueOnce({
        data: [],
        pagination: { cursor: null, hasMore: false, limit: 5, total: null },
      });

      const response = await request(app)
        .get(`/api/lending/transactions/${userAddress}`)
        .query({ limit: 5 });

      expect(response.status).toBe(200);
      expect(response.body.pagination.limit).toBe(5);
    });

    it('should accept a valid base64url cursor and forward it', async () => {
      const rawCursor = 'some-horizon-cursor-token';
      const encodedCursor = Buffer.from(rawCursor, 'utf8').toString('base64url');

      const response = await request(app)
        .get(`/api/lending/transactions/${userAddress}`)
        .query({ cursor: encodedCursor });

      expect(response.status).toBe(200);
      // Service should have been called with the decoded raw cursor
      expect(mockStellarService.getTransactionHistory).toHaveBeenCalledWith(
        expect.objectContaining({ cursor: rawCursor })
      );
    });
  });

  describe('GET /api/health', () => {
    it('should return healthy status when all services are up', async () => {
      const response = await request(app).get('/api/health');

      expect(response.status).toBe(200);
      expect(response.body.status).toBe('healthy');
    });

    it('should return unhealthy status when services are down', async () => {
      mockStellarService.healthCheck.mockResolvedValueOnce({
        horizon: false,
        sorobanRpc: true,
      });

      const response = await request(app).get('/api/health');

      expect(response.status).toBe(503);
      expect(response.body.status).toBe('unhealthy');
    });
  });

  describe('GET /api/health/live', () => {
    it('should return ok without checking upstream dependencies', async () => {
      const response = await request(app).get('/api/health/live');

      expect(response.status).toBe(200);
      expect(response.body).toMatchObject({ status: 'ok' });
      expect(mockStellarService.healthCheck).not.toHaveBeenCalled();
    });
  });

  describe('GET /api/health/ready', () => {
    it('should return ok when all dependencies are up', async () => {
      const response = await request(app).get('/api/health/ready');

      expect(response.status).toBe(200);
      expect(response.body).toMatchObject({
        status: 'ok',
        horizon: 'up',
        soroban: 'up',
      });
    });

    it('should return 503 when a dependency is unavailable', async () => {
      mockStellarService.healthCheck.mockResolvedValueOnce({
        horizon: false,
        sorobanRpc: true,
      });

      const response = await request(app).get('/api/health/ready');

      expect(response.status).toBe(503);
      expect(response.body).toMatchObject({
        status: 'error',
        horizon: 'down',
        soroban: 'up',
      });
    });
  });

  describe('GET /api/protocol/stats', () => {
    it('should return protocol statistics', async () => {
      const response = await request(app).get('/api/protocol/stats');

      expect(response.status).toBe(200);
      expect(response.body).toMatchObject({
        totalDeposits: '1000000',
        totalBorrows: '500000',
        utilizationRate: '0.50',
        numberOfUsers: 150,
        tvl: '1500000',
      });
      expect(response.headers['cache-control']).toBe('public, max-age=30');
    });
  });

  describe('Idempotency-Key', () => {
    const idemKey = '123e4567-e89b-12d3-a456-426614174000';

    it('should replay a cached submit response for duplicate POST requests', async () => {
      const firstResponse = await request(app)
        .post('/api/lending/submit')
        .set('Idempotency-Key', idemKey)
        .send({ signedXdr: 'signed_xdr_string' });

      const secondResponse = await request(app)
        .post('/api/lending/submit')
        .set('Idempotency-Key', idemKey)
        .send({ signedXdr: 'signed_xdr_string' });

      expect(firstResponse.status).toBe(200);
      expect(firstResponse.headers['idempotency-status']).toBe('created');
      expect(secondResponse.status).toBe(200);
      expect(secondResponse.headers['idempotency-status']).toBe('cached');
      expect(secondResponse.body).toEqual(firstResponse.body);
      expect(mockStellarService.submitTransaction).toHaveBeenCalledTimes(1);
      expect(mockStellarService.monitorTransaction).toHaveBeenCalledTimes(1);
    });

    it('should reject non-UUID idempotency keys', async () => {
      const response = await request(app)
        .post('/api/lending/submit')
        .set('Idempotency-Key', 'not-a-uuid')
        .send({ signedXdr: 'signed_xdr_string' });

      expect(response.status).toBe(400);
      expect(response.body.error.message).toMatch(/Idempotency-Key/i);
    });
  });

  describe('GET /api/lending/transactions/:userAddress/stream', () => {
    const userAddress = 'GDZZJ3UPZZCKY5DBH6ZGMPMRORRBG4ECIORASBUAXPPNCL4SYRHNLYU2';

    it('should respond with content-type application/x-ndjson', async () => {
      const response = await request(app).get(`/api/lending/transactions/${userAddress}/stream`);

      expect(response.status).toBe(200);
      expect(response.headers['content-type']).toMatch(/application\/x-ndjson/);
    });

    it('should stream each transaction as a separate JSON line', async () => {
      const response = await request(app).get(`/api/lending/transactions/${userAddress}/stream`);

      expect(response.status).toBe(200);
      const lines = response.text.trim().split('\n').filter(Boolean);
      expect(lines.length).toBe(2);

      const first = JSON.parse(lines[0]);
      expect(first.transactionHash).toBe('tx_hash_stream_1');
      expect(first.type).toBe('deposit');

      const second = JSON.parse(lines[1]);
      expect(second.transactionHash).toBe('tx_hash_stream_2');
      expect(second.type).toBe('borrow');
    });

    it('should call streamTransactionHistory with the user address', async () => {
      await request(app).get(`/api/lending/transactions/${userAddress}/stream`);

      expect(mockStellarService.streamTransactionHistory).toHaveBeenCalledWith(
        userAddress,
        undefined,
        expect.any(AbortSignal)
      );
    });

    it('should stream an empty body when there are no transactions', async () => {
      mockStellarService.streamTransactionHistory.mockImplementationOnce(async function* () {});

      const response = await request(app).get(`/api/lending/transactions/${userAddress}/stream`);

      expect(response.status).toBe(200);
      expect(response.text.trim()).toBe('');
    });

    it('should write a terminal error line when the stream throws after headers are sent', async () => {
      mockStellarService.streamTransactionHistory.mockImplementationOnce(async function* () {
        yield {
          transactionHash: 'tx_before_error',
          type: 'deposit',
          amount: '100',
          timestamp: '2023-01-01T00:00:00Z',
          status: 'success',
          ledger: 1,
        };
        throw new Error('upstream failure');
      });

      const response = await request(app).get(`/api/lending/transactions/${userAddress}/stream`);

      // Headers were sent with the first item, so we get a 200 with an error line at the end
      expect(response.status).toBe(200);
      const lines = response.text.trim().split('\n').filter(Boolean);
      const lastLine = JSON.parse(lines[lines.length - 1]);
      expect(lastLine).toHaveProperty('error');
    });
  });
});
