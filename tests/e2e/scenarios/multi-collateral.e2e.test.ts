/**
 * E2E: Multiple collateral types — Issue #485
 *
 * Deposits several distinct collateral assets simultaneously and verifies
 * borrow capacity, position accounting, and partial withdrawal all account
 * for the combined collateral value correctly.
 */

import request from 'supertest';
import { Application } from 'express';
import { buildLendingApp, setPrice, reset } from './harness';

describe('E2E: multiple collateral types', () => {
  let app: Application;
  const USER = 'GUSERMULTICOLLATERAL000000000000000000000000000000000';

  beforeEach(() => {
    reset();
    app = buildLendingApp();
    setPrice('XLM', 0.1);
    setPrice('BTC', 50_000);
    setPrice('ETH', 3_000);
    setPrice('USDC', 1.0);
  });

  it('accepts deposits of several collateral types and sums their USD value', async () => {
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 10_000 }); // $1,000
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'BTC', amount: 0.02 }); // $1,000
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'ETH', amount: 1 }); // $3,000

    const position = await request(app).get(`/api/positions/${USER}`);
    expect(position.body.collateral).toEqual({ XLM: 10_000, BTC: 0.02, ETH: 1 });
    expect(position.body.collateralValueUsd).toBeCloseTo(5_000);
  });

  it('borrows against the combined value of multiple collateral assets', async () => {
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 10_000 }); // $1,000
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'BTC', amount: 0.02 }); // $1,000
    // Combined collateral = $2,000; 75% LTV cap = $1,500

    const withinCap = await request(app)
      .post('/api/lending/borrow')
      .send({ userAddress: USER, asset: 'USDC', amount: 1_000 });
    expect(withinCap.status).toBe(200);

    const overCap = await request(app)
      .post('/api/lending/borrow')
      .send({ userAddress: USER, asset: 'USDC', amount: 600 }); // would bring total to 1,600 > 1,500 cap
    expect(overCap.status).toBe(400);
  });

  it('allows withdrawing one collateral type while the position stays solvent on the remainder', async () => {
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 10_000 }); // $1,000
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'BTC', amount: 0.02 }); // $1,000
    await request(app).post('/api/lending/borrow').send({ userAddress: USER, asset: 'USDC', amount: 500 });

    // Withdrawing all XLM leaves $1,000 BTC collateral against $500 debt — still solvent.
    const withdraw = await request(app)
      .post('/api/lending/withdraw')
      .send({ userAddress: USER, asset: 'XLM', amount: 10_000 });
    expect(withdraw.status).toBe(200);
    expect(withdraw.body.position.collateral.XLM).toBeUndefined();
    expect(withdraw.body.position.collateral.BTC).toBe(0.02);
  });

  it('borrows a different asset than either collateral type', async () => {
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'ETH', amount: 1 }); // $3,000
    const borrow = await request(app)
      .post('/api/lending/borrow')
      .send({ userAddress: USER, asset: 'USDC', amount: 1_500 });
    expect(borrow.status).toBe(200);
    expect(borrow.body.position.debt).toEqual({ USDC: 1_500 });
  });
});
