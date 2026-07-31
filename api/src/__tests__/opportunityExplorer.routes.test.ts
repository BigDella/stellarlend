import request from 'supertest';
import app from '../app';

describe('Opportunity Explorer API Routes (/api/liquidations)', () => {
  describe('GET /api/liquidations/opportunities', () => {
    it('should return liquidation opportunities', async () => {
      const res = await request(app).get('/api/liquidations/opportunities');
      expect(res.status).toBe(200);
      expect(res.body).toHaveProperty('opportunities');
      expect(res.body.opportunities).toBeInstanceOf(Array);
      expect(res.body.total).toBeGreaterThan(0);
    });

    it('should filter by max health factor', async () => {
      const res = await request(app).get('/api/liquidations/opportunities?maxHf=1.1');
      expect(res.status).toBe(200);
      expect(res.body.opportunities.every((o: any) => o.healthFactor <= 1.1)).toBe(true);
    });

    it('should filter by min profit', async () => {
      const res = await request(app).get('/api/liquidations/opportunities?minProfit=10000000');
      expect(res.status).toBe(200);
      expect(res.body.opportunities.every((o: any) => o.netProfitStroops >= 10000000)).toBe(true);
    });

    it('should filter by asset type', async () => {
      const res = await request(app).get('/api/liquidations/opportunities?asset=XLM');
      expect(res.status).toBe(200);
      expect(res.body.opportunities.every(
        (o: any) => o.collateralAsset === 'XLM' || o.debtAsset === 'XLM'
      )).toBe(true);
    });

    it('should sort by net profit descending', async () => {
      const res = await request(app).get('/api/liquidations/opportunities?sortBy=netProfitStroops&sortDir=desc');
      expect(res.status).toBe(200);
      const profits = res.body.opportunities.map((o: any) => o.netProfitStroops);
      for (let i = 1; i < profits.length; i++) {
        expect(profits[i]).toBeLessThanOrEqual(profits[i - 1]);
      }
    });
  });

  describe('GET /api/liquidations/history', () => {
    it('should return historical liquidation data', async () => {
      const res = await request(app).get('/api/liquidations/history');
      expect(res.status).toBe(200);
      expect(res.body).toHaveProperty('history');
      expect(res.body.history).toBeInstanceOf(Array);
    });
  });

  describe('GET /api/liquidations/gas-estimate', () => {
    it('should return gas cost estimate for liquidation', async () => {
      const res = await request(app).get('/api/liquidations/gas-estimate');
      expect(res.status).toBe(200);
      expect(res.body).toHaveProperty('operation', 'liquidation');
      expect(res.body).toHaveProperty('totalCostStroops');
    });
  });
});
