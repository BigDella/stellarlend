/**
 * E2E: Concurrent multi-user interactions and role-gated actions — Issue #485
 *
 * Verifies that concurrent deposits/borrows across many independent users
 * don't cross-contaminate state, and that admin/liquidator-only actions are
 * correctly gated by account role.
 */

import request from 'supertest';
import { Application } from 'express';
import { buildLendingApp, setPrice, assignRole, reset } from './harness';

describe('E2E: concurrent multi-user interactions', () => {
  let app: Application;

  beforeEach(() => {
    reset();
    app = buildLendingApp();
    setPrice('XLM', 0.12);
    setPrice('USDC', 1.0);
  });

  it('keeps each user position isolated under concurrent deposits and borrows', async () => {
    const users = Array.from({ length: 10 }, (_, i) => `GCONCURRENTUSER${i.toString().padStart(4, '0')}`);

    await Promise.all(
      users.map((user, i) =>
        request(app)
          .post('/api/lending/deposit')
          .send({ userAddress: user, asset: 'XLM', amount: 100_000 + i * 1_000 })
      )
    );

    await Promise.all(
      users.map((user, i) =>
        request(app)
          .post('/api/lending/borrow')
          .send({ userAddress: user, asset: 'USDC', amount: 1_000 + i * 10 })
      )
    );

    const positions = await Promise.all(users.map((user) => request(app).get(`/api/positions/${user}`)));

    positions.forEach((res, i) => {
      expect(res.status).toBe(200);
      expect(res.body.collateral.XLM).toBe(100_000 + i * 1_000);
      expect(res.body.debt.USDC).toBe(1_000 + i * 10);
    });
  });

  it('processes concurrent repay and withdraw calls from different users without interference', async () => {
    const userA = 'GCONCURRENTREPAYA0000000000000000000000000000000000000';
    const userB = 'GCONCURRENTREPAYB0000000000000000000000000000000000000';

    await request(app).post('/api/lending/deposit').send({ userAddress: userA, asset: 'XLM', amount: 100_000 });
    await request(app).post('/api/lending/deposit').send({ userAddress: userB, asset: 'XLM', amount: 200_000 });
    await request(app).post('/api/lending/borrow').send({ userAddress: userA, asset: 'USDC', amount: 2_000 });
    await request(app).post('/api/lending/borrow').send({ userAddress: userB, asset: 'USDC', amount: 3_000 });

    const [repayA, withdrawB] = await Promise.all([
      request(app).post('/api/lending/repay').send({ userAddress: userA, asset: 'USDC', amount: 500 }),
      request(app).post('/api/lending/withdraw').send({ userAddress: userB, asset: 'XLM', amount: 10_000 }),
    ]);

    expect(repayA.status).toBe(200);
    expect(repayA.body.position.debt.USDC).toBe(1_500);
    expect(withdrawB.status).toBe(200);
    expect(withdrawB.body.position.collateral.XLM).toBe(190_000);

    // Confirm cross-contamination didn't happen: A's collateral and B's debt untouched.
    const posA = await request(app).get(`/api/positions/${userA}`);
    const posB = await request(app).get(`/api/positions/${userB}`);
    expect(posA.body.collateral.XLM).toBe(100_000);
    expect(posB.body.debt.USDC).toBe(3_000);
  });
});

describe('E2E: role-gated account configurations (admin, user, liquidator)', () => {
  let app: Application;
  const ADMIN = 'GACCOUNTADMIN00000000000000000000000000000000000000000';
  const REGULAR_USER = 'GACCOUNTUSER0000000000000000000000000000000000000000000';
  const LIQUIDATOR = 'GACCOUNTLIQUIDATOR000000000000000000000000000000000000';

  beforeEach(() => {
    reset();
    app = buildLendingApp();
    assignRole(ADMIN, 'admin');
    assignRole(REGULAR_USER, 'user');
    assignRole(LIQUIDATOR, 'liquidator');
    setPrice('XLM', 0.12);
    setPrice('USDC', 1.0);
  });

  it('allows only an admin account to pause and resume the protocol', async () => {
    const deniedPause = await request(app)
      .post('/api/protocol/pause')
      .send({ callerAddress: REGULAR_USER });
    expect(deniedPause.status).toBe(403);

    const allowedPause = await request(app).post('/api/protocol/pause').send({ callerAddress: ADMIN });
    expect(allowedPause.status).toBe(200);
    expect(allowedPause.body.paused).toBe(true);

    const health = await request(app).get('/api/health');
    expect(health.body.status).toBe('paused');

    const deniedResume = await request(app)
      .post('/api/protocol/resume')
      .send({ callerAddress: REGULAR_USER });
    expect(deniedResume.status).toBe(403);

    const allowedResume = await request(app).post('/api/protocol/resume').send({ callerAddress: ADMIN });
    expect(allowedResume.status).toBe(200);
    expect(allowedResume.body.paused).toBe(false);
  });

  it('blocks user actions while the protocol is paused', async () => {
    await request(app).post('/api/protocol/pause').send({ callerAddress: ADMIN });

    const deposit = await request(app)
      .post('/api/lending/deposit')
      .send({ userAddress: REGULAR_USER, asset: 'XLM', amount: 1_000 });
    expect(deposit.status).toBe(503);
  });

  it('restricts liquidation calls to accounts with the liquidator role', async () => {
    await request(app).post('/api/oracle/update-price').send({ asset: 'XLM', price: 1.0 });
    await request(app)
      .post('/api/lending/deposit')
      .send({ userAddress: REGULAR_USER, asset: 'XLM', amount: 1_100 });
    await request(app)
      .post('/api/lending/borrow')
      .send({ userAddress: REGULAR_USER, asset: 'USDC', amount: 700 });
    await request(app).post('/api/oracle/update-price').send({ asset: 'XLM', price: 0.55 });

    const deniedLiquidation = await request(app).post('/api/liquidations/liquidate').send({
      callerAddress: REGULAR_USER, // a plain user, not a liquidator
      targetUser: REGULAR_USER,
      debtAsset: 'USDC',
      collateralAsset: 'XLM',
      repayAmount: 100,
    });
    expect(deniedLiquidation.status).toBe(403);

    const allowedLiquidation = await request(app).post('/api/liquidations/liquidate').send({
      callerAddress: LIQUIDATOR,
      targetUser: REGULAR_USER,
      debtAsset: 'USDC',
      collateralAsset: 'XLM',
      repayAmount: 100,
    });
    expect(allowedLiquidation.status).toBe(200);
  });
});
