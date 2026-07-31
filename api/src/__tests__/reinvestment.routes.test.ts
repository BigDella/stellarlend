import request from 'supertest';
import app from '../app';
import { resetReinvestmentStore } from '../services/earnings/reinvestment.service';

const USER = 'GA6T6URCJEEWVTUFCFBP3OONDUTFOSAFUQDIITIUU2PYNTS4YEQKGP5E';
const POOL_A = 'GBNNVJG4O3HMCGM5C4ORI4O4H3K5CQA6OTKIW2KKWY2EEG2IL3TRXK32';

describe('Reinvestment routes', () => {
  beforeEach(() => {
    resetReinvestmentStore();
  });

  it('supports the full create -> pause -> resume -> sweep -> history -> analytics flow', async () => {
    const createResponse = await request(app).post('/api/reinvestment/plan').send({
      userAddress: USER,
      sourcePool: POOL_A,
      strategy: 'same_pool',
      schedule: 'real_time',
      thresholdAmount: '100',
    });
    expect(createResponse.status).toBe(201);
    expect(createResponse.body.success).toBe(true);
    const planId = createResponse.body.plan.id;
    expect(planId).toBeTruthy();

    const getResponse = await request(app).get(`/api/reinvestment/plan/${planId}`);
    expect(getResponse.status).toBe(200);
    expect(getResponse.body.plan.paused).toBe(false);

    const listResponse = await request(app).get(`/api/reinvestment/plans/${USER}`);
    expect(listResponse.status).toBe(200);
    expect(listResponse.body.plans).toHaveLength(1);

    const pauseResponse = await request(app)
      .post(`/api/reinvestment/plan/${planId}/pause`)
      .send({ userAddress: USER });
    expect(pauseResponse.status).toBe(200);
    expect(pauseResponse.body.plan.paused).toBe(true);

    const blockedSweep = await request(app).post(`/api/reinvestment/plan/${planId}/sweep`).send({
      earnedAmount: '500',
      estimatedGasCost: '1',
      poolPaused: false,
    });
    expect(blockedSweep.status).toBe(409);
    expect(blockedSweep.body.success).toBe(false);

    const resumeResponse = await request(app)
      .post(`/api/reinvestment/plan/${planId}/resume`)
      .send({ userAddress: USER });
    expect(resumeResponse.status).toBe(200);
    expect(resumeResponse.body.plan.paused).toBe(false);

    const sweepResponse = await request(app).post(`/api/reinvestment/plan/${planId}/sweep`).send({
      earnedAmount: '500',
      estimatedGasCost: '1',
      poolPaused: false,
    });
    expect(sweepResponse.status).toBe(201);
    expect(sweepResponse.body.events).toHaveLength(1);
    expect(sweepResponse.body.events[0].pool).toBe(POOL_A);

    const historyResponse = await request(app).get(`/api/reinvestment/plan/${planId}/history`);
    expect(historyResponse.status).toBe(200);
    expect(historyResponse.body.history).toHaveLength(1);

    const analyticsResponse = await request(app).get(`/api/reinvestment/plan/${planId}/analytics`);
    expect(analyticsResponse.status).toBe(200);
    expect(analyticsResponse.body.analytics.totalSweeps).toBe(1);
  });

  it('rejects plan creation with an invalid strategy', async () => {
    const response = await request(app).post('/api/reinvestment/plan').send({
      userAddress: USER,
      sourcePool: POOL_A,
      strategy: 'not_a_real_strategy',
      schedule: 'real_time',
      thresholdAmount: '0',
    });
    expect(response.status).toBe(400);
    expect(response.body.success).toBe(false);
  });

  it('returns 404 for an unknown plan id', async () => {
    const response = await request(app).get('/api/reinvestment/plan/does-not-exist');
    expect(response.status).toBe(404);
    expect(response.body.success).toBe(false);
  });

  it('rejects a sweep below the configured threshold', async () => {
    const createResponse = await request(app).post('/api/reinvestment/plan').send({
      userAddress: USER,
      sourcePool: POOL_A,
      strategy: 'same_pool',
      schedule: 'real_time',
      thresholdAmount: '1000',
    });
    const planId = createResponse.body.plan.id;

    const sweepResponse = await request(app).post(`/api/reinvestment/plan/${planId}/sweep`).send({
      earnedAmount: '10',
      estimatedGasCost: '1',
      poolPaused: false,
    });
    expect(sweepResponse.status).toBe(400);
    expect(sweepResponse.body.success).toBe(false);
  });
});
