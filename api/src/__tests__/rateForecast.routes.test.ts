import request from 'supertest';
import app from '../app';

describe('Rate Forecast API Routes (/api/rates)', () => {
  describe('GET /api/rates/forecast', () => {
    it('should return rate forecast for default asset USDC', async () => {
      const res = await request(app).get('/api/rates/forecast');
      expect(res.status).toBe(200);
      expect(res.body).toHaveProperty('asset', 'USDC');
      expect(res.body).toHaveProperty('forecasts');
      expect(res.body.forecasts).toBeInstanceOf(Array);
      expect(res.body).toHaveProperty('backtest');
      expect(res.body.backtest.accuracyPassed).toBe(true);
      expect(res.body.backtest.mape).toBeLessThan(20);
    });

    it('should filter forecast by specified asset and horizon', async () => {
      const res = await request(app).get('/api/rates/forecast?asset=XLM&horizon=7d');
      expect(res.status).toBe(200);
      expect(res.body.asset).toBe('XLM');
      expect(res.body.forecasts).toHaveLength(1);
      expect(res.body.forecasts[0].horizon).toBe('7d');
    });

    it('should include 95% confidence intervals in response', async () => {
      const res = await request(app).get('/api/rates/forecast?asset=USDC');
      expect(res.status).toBe(200);
      const f1d = res.body.forecasts[0];
      expect(f1d).toHaveProperty('confidenceInterval');
      expect(f1d.confidenceInterval).toHaveProperty('lowerBps');
      expect(f1d.confidenceInterval).toHaveProperty('upperBps');
      expect(f1d.confidenceInterval.confidenceLevel).toBe(0.95);
    });
  });

  describe('POST /api/rates/retrain', () => {
    it('should retrain model successfully', async () => {
      const res = await request(app).post('/api/rates/retrain');
      expect(res.status).toBe(200);
      expect(res.body).toHaveProperty('message');
    });
  });
});
