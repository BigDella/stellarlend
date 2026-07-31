import request from 'supertest';
import app from '../app';
import express, { Request, Response } from 'express';
import { prepareValidation } from '../middleware/validation';
import { errorHandler } from '../middleware/errorHandler';

const VALID_ADDRESS = 'GDZZJ3UPZZCKY5DBH6ZGMPMRORRBG4ECIORASBUAXPPNCL4SYRHNLYU2';

describe('Validation Middleware', () => {
  describe('Prepare Validation (GET /api/lending/prepare/:operation)', () => {
    it('should reject empty userAddress', async () => {
      const response = await request(app)
        .get('/api/lending/prepare/deposit')
        .query({ amount: '1000000' });

      expect(response.status).toBe(400);
      expect(response.body.error).toBeDefined();
      expect(response.body.error.message).toContain('User address is required');
    });

    it('should reject invalid Stellar public key', async () => {
      const response = await request(app).get('/api/lending/prepare/deposit').query({
        userAddress: 'invalid-address',
        assetAddress: 'G...',
        amount: '100',
      });

      expect(response.status).toBe(400);
      expect(response.body.error).toBeDefined();
      expect(response.body.error.message).toContain('Invalid Stellar address');
    });

    it('should reject Stellar address with wrong prefix', async () => {
      const response = await request(app).get('/api/lending/prepare/deposit').query({
        userAddress: 'S...', // Secret key prefix
        assetAddress: 'G...',
        amount: '100',
      });

      expect(response.status).toBe(400);
      expect(response.body.error).toBeDefined();
      expect(response.body.error.message).toContain('Invalid Stellar address');
    });

    it('should accept valid Stellar public key', async () => {
      const validationOnlyApp = express();
      validationOnlyApp.get(
        '/prepare/:operation',
        prepareValidation,
        (_req: Request, res: Response) => res.status(204).send()
      );
      validationOnlyApp.use(errorHandler);

      const response = await request(validationOnlyApp).get('/prepare/deposit').query({
        userAddress: VALID_ADDRESS,
        assetAddress: 'G...',
        amount: '100',
      });

      expect(response.status).toBe(204);
    }, 15000);

    it('should reject missing amount', async () => {
      const response = await request(app).get('/api/lending/prepare/deposit').query({
        userAddress: VALID_ADDRESS,
        assetAddress: 'G...',
      });

      expect(response.status).toBe(400);
      expect(response.body.error.message).toContain('Amount is required');
    });

    it('should reject zero amount', async () => {
      const response = await request(app).get('/api/lending/prepare/deposit').query({
        userAddress: VALID_ADDRESS,
        assetAddress: 'G...',
        amount: '0',
      });

      expect(response.status).toBe(400);
      expect(response.body.error).toBeDefined();
      expect(response.body.error.message).toContain('Amount must be a valid positive integer');
    });

    it('should reject negative amount', async () => {
      const response = await request(app).get('/api/lending/prepare/deposit').query({
        userAddress: VALID_ADDRESS,
        assetAddress: 'G...',
        amount: '-1',
      });

      expect(response.status).toBe(400);
      expect(response.body.error).toBeDefined();
      expect(response.body.error.message).toContain('Amount must be a valid positive integer');
    });

    it('should reject non-integer amount strings', async () => {
      const res = await request(app)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: VALID_ADDRESS, assetAddress: 'G...', amount: '1.5' });

      expect(res.status).toBe(400);
      expect(res.body.error.message).toContain('Amount must be a valid positive integer');
    });

    it('should reject non-numeric amount strings', async () => {
      const res = await request(app)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: VALID_ADDRESS, assetAddress: 'G...', amount: 'abc' });

      expect(res.status).toBe(400);
      expect(res.body.error.message).toContain('Amount must be a valid positive integer');
    });

    it('should reject empty amount strings', async () => {
      const res = await request(app)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: VALID_ADDRESS, assetAddress: 'G...', amount: '' });

      expect(res.status).toBe(400);
      expect(res.body.error.message).toContain('Amount is required');
    });

    it('should accept very large valid integers (within i128)', async () => {
      // Max i128 = 2^127 - 1
      const maxI128 = '170141183460469231731687303715884105727';

      // Validate middleware acceptance without relying on external Horizon/Soroban availability.
      const testApp = express();
      testApp.use(express.json());
      testApp.get(
        '/api/lending/prepare/:operation',
        prepareValidation,
        (_req: Request, res: Response) => {
          res.status(200).json({ ok: true });
        }
      );
      testApp.use(errorHandler);

      const res = await request(testApp)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: VALID_ADDRESS, assetAddress: 'G...', amount: maxI128 });

      expect(res.status).toBe(200);
    });

    // BigInt edge case tests
    it('should accept MAX_SAFE_INTEGER', async () => {
      const maxSafeInt = '9007199254740991';

      const testApp = express();
      testApp.use(express.json());
      testApp.get(
        '/api/lending/prepare/:operation',
        prepareValidation,
        (_req: Request, res: Response) => {
          res.status(200).json({ ok: true });
        }
      );
      testApp.use(errorHandler);

      const res = await request(testApp)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: VALID_ADDRESS, assetAddress: 'G...', amount: maxSafeInt });

      expect(res.status).toBe(200);
    });

    it('should accept very large numbers', async () => {
      const veryLargeNumber = '99999999999999999999999999999';

      const testApp = express();
      testApp.use(express.json());
      testApp.get(
        '/api/lending/prepare/:operation',
        prepareValidation,
        (_req: Request, res: Response) => {
          res.status(200).json({ ok: true });
        }
      );
      testApp.use(errorHandler);

      const res = await request(testApp)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: VALID_ADDRESS, assetAddress: 'G...', amount: veryLargeNumber });

      expect(res.status).toBe(200);
    });

    it('should reject float amounts', async () => {
      const res = await request(app)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: VALID_ADDRESS, assetAddress: 'G...', amount: '1.5' });

      expect(res.status).toBe(400);
      expect(res.body.error.message).toContain('Amount must be a valid positive integer');
    });

    it('should reject scientific notation', async () => {
      const res = await request(app)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: VALID_ADDRESS, assetAddress: 'G...', amount: '1e18' });

      expect(res.status).toBe(400);
      expect(res.body.error.message).toContain('Amount must be a valid positive integer');
    });

    it('should reject negative zero', async () => {
      const res = await request(app)
        .get('/api/lending/prepare/deposit')
        .query({ userAddress: VALID_ADDRESS, assetAddress: 'G...', amount: '-0' });

      expect(res.status).toBe(400);
      expect(res.body.error.message).toContain('Amount must be a valid positive integer');
    });

    it('should reject invalid operation', async () => {
      const response = await request(app).get('/api/lending/prepare/invalid_op').query({
        userAddress: 'GDH7NBM22WUCYOLJJZ7ALN3QZ6W2G5YCHDP2YQWJZ76L2GZFPHSYZ4Y3',
        amount: '1000000',
      });

      expect(response.status).toBe(400);
    });
  });

  describe('Submit Validation (POST /api/lending/submit)', () => {
    it('should reject missing signedXdr', async () => {
      const response = await request(app).post('/api/lending/submit').send({});

      expect(response.status).toBe(400);
    });

    it('should reject empty signedXdr', async () => {
      const response = await request(app).post('/api/lending/submit').send({ signedXdr: '' });

      expect(response.status).toBe(400);
    });
  });
});
