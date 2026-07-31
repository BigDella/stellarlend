/**
 * E2E: Full user journey — Issue #485
 *
 * deposit → borrow → check position → repay → withdraw, including the case
 * where the collateral asset differs from the borrowed asset.
 */

import request from 'supertest';
import { Application } from 'express';
import { buildLendingApp, setPrice, reset } from './harness';

describe('E2E: full user journey', () => {
  let app: Application;
  const USER = 'GUSERFULLJOURNEY0000000000000000000000000000000000000';

  beforeEach(() => {
    reset();
    app = buildLendingApp();
    setPrice('XLM', 0.12);
    setPrice('USDC', 1.0);
  });

  it('walks deposit → borrow → check position → repay → withdraw end to end', async () => {
    // 1. Deposit collateral
    const deposit = await request(app)
      .post('/api/lending/deposit')
      .send({ userAddress: USER, asset: 'XLM', amount: 100_000 });
    expect(deposit.status).toBe(200);
    expect(deposit.body.position.collateral.XLM).toBe(100_000);

    // 2. Borrow a *different* asset against that collateral
    const collateralValueUsd = 100_000 * 0.12; // 12,000
    const borrowAmount = 5_000; // well within 75% LTV of $12,000
    const borrow = await request(app)
      .post('/api/lending/borrow')
      .send({ userAddress: USER, asset: 'USDC', amount: borrowAmount });
    expect(borrow.status).toBe(200);
    expect(borrow.body.position.debt.USDC).toBe(borrowAmount);
    expect(borrow.body.position.collateralValueUsd).toBeCloseTo(collateralValueUsd);

    // 3. Check position
    const position = await request(app).get(`/api/positions/${USER}`);
    expect(position.status).toBe(200);
    expect(position.body.debtValueUsd).toBeCloseTo(borrowAmount);
    expect(position.body.healthFactor).toBeGreaterThan(1);
    expect(position.body.liquidatable).toBe(false);

    // 4. Repay in full
    const repay = await request(app)
      .post('/api/lending/repay')
      .send({ userAddress: USER, asset: 'USDC', amount: borrowAmount });
    expect(repay.status).toBe(200);
    expect(repay.body.position.debt.USDC).toBeUndefined();

    // 5. Withdraw all collateral now that there's no outstanding debt
    const withdraw = await request(app)
      .post('/api/lending/withdraw')
      .send({ userAddress: USER, asset: 'XLM', amount: 100_000 });
    expect(withdraw.status).toBe(200);
    expect(withdraw.body.position.collateral.XLM).toBeUndefined();
    expect(withdraw.body.position.collateralValueUsd).toBe(0);
  });

  it('rejects a borrow that exceeds available collateral capacity', async () => {
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 1_000 });
    // Collateral value = 1000 * 0.12 = 120; 75% LTV cap = 90
    const borrow = await request(app)
      .post('/api/lending/borrow')
      .send({ userAddress: USER, asset: 'USDC', amount: 200 });
    expect(borrow.status).toBe(400);
  });

  it('rejects a withdrawal that would leave the position undercollateralized', async () => {
    await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 100_000 });
    await request(app).post('/api/lending/borrow').send({ userAddress: USER, asset: 'USDC', amount: 5_000 });

    const withdraw = await request(app)
      .post('/api/lending/withdraw')
      .send({ userAddress: USER, asset: 'XLM', amount: 99_000 });
    expect(withdraw.status).toBe(400);
  });
});
