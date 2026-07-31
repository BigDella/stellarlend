import request from 'supertest';
import app from '../app';

describe('Liquidation Dashboard API Routes (/api/liquidations)', () => {
  describe('GET /api/liquidations/positions', () => {
    it('should return all positions sorted by health factor', async () => {
      const res = await request(app).get('/api/liquidations/positions');
      expect(res.status).toBe(200);
      expect(res.body).toHaveProperty('positions');
      expect(res.body.positions).toBeInstanceOf(Array);
      expect(res.body.total).toBeGreaterThan(0);
    });
  });

  describe('GET /api/liquidations/positions/:address', () => {
    it('should return 404 for unknown address', async () => {
      const res = await request(app).get('/api/liquidations/positions/UNKNOWN');
      expect(res.status).toBe(404);
    });
  });

  describe('GET /api/liquidations/alerts', () => {
    it('should return active alerts', async () => {
      const res = await request(app).get('/api/liquidations/alerts');
      expect(res.status).toBe(200);
      expect(res.body).toHaveProperty('alerts');
    });
  });

  describe('POST /api/liquidations/threshold', () => {
    it('should set alert threshold', async () => {
      const res = await request(app)
        .post('/api/liquidations/threshold')
        .send({ address: 'GABCDE12345', dangerHf: 1.15, warningHf: 1.3 });
      expect(res.status).toBe(200);
      expect(res.body.message).toContain('Threshold updated');
    });

    it('should return 400 when address is missing', async () => {
      const res = await request(app)
        .post('/api/liquidations/threshold')
        .send({ dangerHf: 1.15 });
      expect(res.status).toBe(400);
    });
  });
});
