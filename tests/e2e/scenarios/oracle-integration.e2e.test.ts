/**
 * E2E: Oracle integration — Issue #485
 *
 * Verifies that oracle price feed updates propagate into borrow capacity
 * and liquidation eligibility, and that borrowing is blocked without a
 * price feed for the requested asset.
 */

import request from 'supertest';
import { Application } from 'express';
import { buildLendingApp, setPrice, reset } from './harness';

describe('E2E: oracle price feed affects borrow and liquidation', () => {
  let app: Application;
  const USER = 'GUSERORACLEINTEGRATION0000000000000000000000000000000';

  beforeEach(() => {
    reset();
    app = buildLendingApp();
  });

  it('blocks borrowing an asset with no oracle price', async () => {
    setPrice('XLM', 0.12);
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 100_000 });

    const borrow = await request(app)
      .post('/api/lending/borrow')
      .send({ userAddress: USER, asset: 'USDC', amount: 100 });
    expect(borrow.status).toBe(400);
    expect(borrow.body.error).toMatch(/oracle price/i);
  });

  it('increases available borrow capacity when the collateral price rises', async () => {
    setPrice('XLM', 0.1);
    setPrice('USDC', 1.0);
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 10_000 });
    // Collateral value $1,000; 75% cap = $750.
    const tooMuch = await request(app)
      .post('/api/lending/borrow')
      .send({ userAddress: USER, asset: 'USDC', amount: 800 });
    expect(tooMuch.status).toBe(400);

    // Oracle price update: XLM doubles in value.
    await request(app).post('/api/oracle/update-price').send({ asset: 'XLM', price: 0.2 });
    // Collateral value now $2,000; 75% cap = $1,500 — the same borrow now succeeds.
    const nowAllowed = await request(app)
      .post('/api/lending/borrow')
      .send({ userAddress: USER, asset: 'USDC', amount: 800 });
    expect(nowAllowed.status).toBe(200);
  });

  it('makes a previously healthy position liquidatable after a collateral price drop', async () => {
    setPrice('XLM', 1.0);
    setPrice('USDC', 1.0);
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 1_000 });
    await request(app).post('/api/lending/borrow').send({ userAddress: USER, asset: 'USDC', amount: 700 });

    const healthy = await request(app).get(`/api/positions/${USER}`);
    expect(healthy.body.liquidatable).toBe(false);

    // Oracle reports a sharp price drop.
    await request(app).post('/api/oracle/update-price').send({ asset: 'XLM', price: 0.6 });

    const afterCrash = await request(app).get(`/api/positions/${USER}`);
    expect(afterCrash.body.liquidatable).toBe(true);
    expect(afterCrash.body.healthFactor).toBeLessThan(1);
  });

  it('restores solvency when the price recovers', async () => {
    setPrice('XLM', 1.0);
    setPrice('USDC', 1.0);
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 1_000 });
    await request(app).post('/api/lending/borrow').send({ userAddress: USER, asset: 'USDC', amount: 700 });
    await request(app).post('/api/oracle/update-price').send({ asset: 'XLM', price: 0.6 });

    let position = await request(app).get(`/api/positions/${USER}`);
    expect(position.body.liquidatable).toBe(true);

    await request(app).post('/api/oracle/update-price').send({ asset: 'XLM', price: 1.0 });
    position = await request(app).get(`/api/positions/${USER}`);
    expect(position.body.liquidatable).toBe(false);
  });
});
