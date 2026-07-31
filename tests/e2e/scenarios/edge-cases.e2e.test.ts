/**
 * E2E: Edge cases — Issue #485
 *
 * Partial repay, partial withdraw, and liquidation through to full
 * collateral seizure.
 */

import request from 'supertest';
import { Application } from 'express';
import { buildLendingApp, setPrice, assignRole, reset } from './harness';

describe('E2E: edge cases', () => {
  let app: Application;
  const USER = 'GUSEREDGECASES00000000000000000000000000000000000000';
  const LIQUIDATOR = 'GLIQUIDATOREDGECASES000000000000000000000000000000000';

  beforeEach(() => {
    reset();
    app = buildLendingApp();
    assignRole(LIQUIDATOR, 'liquidator');
  });

  describe('partial repay', () => {
    it('leaves the remaining debt outstanding and keeps the position healthy', async () => {
      setPrice('XLM', 0.12);
      setPrice('USDC', 1.0);
      await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 100_000 });
      await request(app).post('/api/lending/borrow').send({ userAddress: USER, asset: 'USDC', amount: 5_000 });

      const partialRepay = await request(app)
        .post('/api/lending/repay')
        .send({ userAddress: USER, asset: 'USDC', amount: 2_000 });
      expect(partialRepay.status).toBe(200);
      expect(partialRepay.body.position.debt.USDC).toBe(3_000);
      expect(partialRepay.body.position.liquidatable).toBe(false);
    });

    it('rejects repaying more than the outstanding debt', async () => {
      setPrice('XLM', 0.12);
      setPrice('USDC', 1.0);
      await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 100_000 });
      await request(app).post('/api/lending/borrow').send({ userAddress: USER, asset: 'USDC', amount: 5_000 });

      const overRepay = await request(app)
        .post('/api/lending/repay')
        .send({ userAddress: USER, asset: 'USDC', amount: 5_001 });
      expect(overRepay.status).toBe(400);
    });
  });

  describe('partial withdraw', () => {
    it('leaves the remaining collateral in place and keeps the position solvent', async () => {
      setPrice('XLM', 0.12);
      setPrice('USDC', 1.0);
      await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 100_000 });
      await request(app).post('/api/lending/borrow').send({ userAddress: USER, asset: 'USDC', amount: 1_000 });

      const partialWithdraw = await request(app)
        .post('/api/lending/withdraw')
        .send({ userAddress: USER, asset: 'XLM', amount: 40_000 });
      expect(partialWithdraw.status).toBe(200);
      expect(partialWithdraw.body.position.collateral.XLM).toBe(60_000);
      expect(partialWithdraw.body.position.liquidatable).toBe(false);
    });
  });

  describe('full liquidation', () => {
    it('liquidates a position through to full collateral seizure after a price crash', async () => {
      setPrice('XLM', 1.5);
      setPrice('USDC', 1.0);

      await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 1_100 });
      // Collateral value $1,650; 75% LTV cap = $1,237.50 — comfortably covers this borrow.
      await request(app).post('/api/lending/borrow').send({ userAddress: USER, asset: 'USDC', amount: 1_000 });

      // Oracle price crash: collateral value drops to $550 against $1,000 debt.
      await request(app).post('/api/oracle/update-price').send({ asset: 'XLM', price: 0.5 });

      const before = await request(app).get(`/api/positions/${USER}`);
      expect(before.body.liquidatable).toBe(true);
      expect(before.body.healthFactor).toBeLessThan(1);

      // Close-factor-limited liquidation call: repay the max allowed 50% of debt.
      const liquidate = await request(app).post('/api/liquidations/liquidate').send({
        callerAddress: LIQUIDATOR,
        targetUser: USER,
        debtAsset: 'USDC',
        collateralAsset: 'XLM',
        repayAmount: 500,
      });
      expect(liquidate.status).toBe(200);
      expect(liquidate.body.seizedAmount).toBeCloseTo(1_100); // exactly all held XLM collateral
      expect(liquidate.body.position.collateral.XLM).toBeUndefined();
      expect(liquidate.body.position.debt.USDC).toBe(500);

      // Remaining debt is now fully uncollateralized — a further liquidation
      // attempt fails because there is no collateral left to seize.
      const secondAttempt = await request(app).post('/api/liquidations/liquidate').send({
        callerAddress: LIQUIDATOR,
        targetUser: USER,
        debtAsset: 'USDC',
        collateralAsset: 'XLM',
        repayAmount: 250,
      });
      expect(secondAttempt.status).toBe(400);
    });

    it('rejects liquidating a healthy position', async () => {
      setPrice('XLM', 0.12);
      setPrice('USDC', 1.0);
      await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 100_000 });
      await request(app).post('/api/lending/borrow').send({ userAddress: USER, asset: 'USDC', amount: 1_000 });

      const liquidate = await request(app).post('/api/liquidations/liquidate').send({
        callerAddress: LIQUIDATOR,
        targetUser: USER,
        debtAsset: 'USDC',
        collateralAsset: 'XLM',
        repayAmount: 100,
      });
      expect(liquidate.status).toBe(400);
    });

    it('rejects a liquidation repay amount above the close-factor cap', async () => {
      setPrice('XLM', 1.5);
      setPrice('USDC', 1.0);
      await request(app).post('/api/lending/deposit').send({ userAddress: USER, asset: 'XLM', amount: 1_100 });
      await request(app).post('/api/lending/borrow').send({ userAddress: USER, asset: 'USDC', amount: 1_000 });
      await request(app).post('/api/oracle/update-price').send({ asset: 'XLM', price: 0.5 });

      const liquidate = await request(app).post('/api/liquidations/liquidate').send({
        callerAddress: LIQUIDATOR,
        targetUser: USER,
        debtAsset: 'USDC',
        collateralAsset: 'XLM',
        repayAmount: 900, // exceeds 50% close-factor cap of 500
      });
      expect(liquidate.status).toBe(400);
    });
  });
});
