import request from 'supertest';
import app from '../app';

describe('Yield Curve API Routes (/api/yield-curve)', () => {
  describe('GET /api/yield-curve/config', () => {
    it('should return default yield curve configuration', async () => {
      const res = await request(app).get('/api/yield-curve/config');
      expect(res.status).toBe(200);
      expect(res.body).toHaveProperty('curveType');
      expect(res.body).toHaveProperty('baseRateBps');
      expect(res.body).toHaveProperty('kinkUtilizationBps');
    });
  });

  describe('POST /api/yield-curve/predict', () => {
    it('should generate yield curve prediction points', async () => {
      const res = await request(app)
        .post('/api/yield-curve/predict')
        .send({
          stepBps: 1000,
        });

      expect(res.status).toBe(200);
      expect(res.body).toHaveProperty('points');
      expect(res.body.points).toBeInstanceOf(Array);
      expect(res.body).toHaveProperty('summary');
    });
  });

  describe('POST /api/yield-curve/optimize', () => {
    it('should optimize rate configuration parameters', async () => {
      const res = await request(app)
        .post('/api/yield-curve/optimize')
        .send({
          targetUtilizationBps: 7500,
        });

      expect(res.status).toBe(200);
      expect(res.body).toHaveProperty('recommendedConfig');
      expect(res.body.recommendedConfig.kinkUtilizationBps).toBe(7500);
    });

    it('should return 400 when targetUtilizationBps is missing', async () => {
      const res = await request(app)
        .post('/api/yield-curve/optimize')
        .send({});

      expect(res.status).toBe(400);
      expect(res.body).toHaveProperty('error');
    });
  });

  describe('POST /api/yield-curve/stress-test', () => {
    it('should execute liquidity stress test simulation', async () => {
      const res = await request(app)
        .post('/api/yield-curve/stress-test')
        .send({
          baseUtilizationBps: 6000,
          shocksBps: [-1000, 1000, 2000],
        });

      expect(res.status).toBe(200);
      expect(res.body).toHaveProperty('basePoint');
      expect(res.body).toHaveProperty('shockResults');
      expect(res.body.shockResults).toHaveLength(3);
    });
  });
});
