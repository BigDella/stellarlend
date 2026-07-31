/**
 * Shared E2E test harness — Issue #485
 *
 * Extends the mocked oracle/contract/API pattern established in
 * `pipeline.e2e.test.ts` with a minimal, in-memory lending state machine so
 * the full user journey (deposit → borrow → check position → repay →
 * withdraw), multi-collateral positions, oracle-driven borrow/liquidation
 * limits, and role-gated admin/liquidator actions can be exercised without a
 * live Soroban devnet, API, or oracle process. Every store resets via
 * `reset()` between tests.
 */

import express, { Application, NextFunction, Request, Response } from 'express';

// ─── Roles ──────────────────────────────────────────────────────────────────

export type Role = 'admin' | 'user' | 'liquidator';

const roles = new Map<string, Role>();

export function assignRole(address: string, role: Role): void {
  roles.set(address, role);
}

function requireRole(role: Role) {
  return (req: Request, res: Response, next: NextFunction) => {
    const caller = (req.body?.callerAddress || req.query.callerAddress) as string | undefined;
    if (!caller || roles.get(caller) !== role) {
      res.status(403).json({ error: `caller must have the '${role}' role` });
      return;
    }
    next();
  };
}

// ─── Oracle price store ─────────────────────────────────────────────────────

const prices = new Map<string, number>();

export function setPrice(asset: string, price: number): void {
  prices.set(asset.toUpperCase(), price);
}

function getPrice(asset: string): number {
  return prices.get(asset.toUpperCase()) ?? 0;
}

// ─── Protocol pause state ───────────────────────────────────────────────────

let paused = false;

// ─── Positions ──────────────────────────────────────────────────────────────

export interface Position {
  userAddress: string;
  collateral: Record<string, number>;
  debt: Record<string, number>;
}

const positions = new Map<string, Position>();

function getOrCreatePosition(userAddress: string): Position {
  let position = positions.get(userAddress);
  if (!position) {
    position = { userAddress, collateral: {}, debt: {} };
    positions.set(userAddress, position);
  }
  return position;
}

// Same collateral factor and liquidation threshold applied to every asset —
// enough to model cross-asset borrow capacity and liquidation eligibility
// without needing a full per-asset risk-parameter table for this harness.
const MAX_LTV = 0.75;
const LIQUIDATION_THRESHOLD = 0.85;
const CLOSE_FACTOR = 0.5; // max fraction of debt a liquidator may repay per call
const LIQUIDATION_BONUS = 0.1;

function collateralValue(position: Position): number {
  return Object.entries(position.collateral).reduce(
    (sum, [asset, amount]) => sum + amount * getPrice(asset),
    0
  );
}

function debtValue(position: Position): number {
  return Object.entries(position.debt).reduce((sum, [asset, amount]) => sum + amount * getPrice(asset), 0);
}

function healthFactor(position: Position): number {
  const debt = debtValue(position);
  if (debt === 0) return Number.POSITIVE_INFINITY;
  return (collateralValue(position) * LIQUIDATION_THRESHOLD) / debt;
}

export function positionSummary(userAddress: string) {
  const position = getOrCreatePosition(userAddress);
  return {
    userAddress,
    collateral: { ...position.collateral },
    debt: { ...position.debt },
    collateralValueUsd: collateralValue(position),
    debtValueUsd: debtValue(position),
    healthFactor: healthFactor(position),
    liquidatable: healthFactor(position) < 1,
  };
}

// ─── App ────────────────────────────────────────────────────────────────────

export function buildLendingApp(): Application {
  const app = express();
  app.use(express.json());

  app.get('/api/health', (_req, res) => {
    res.json({ status: paused ? 'paused' : 'healthy' });
  });

  app.get('/api/prices/:asset', (req, res) => {
    const asset = req.params.asset.toUpperCase();
    if (!prices.has(asset)) {
      res.status(404).json({ error: `no price for ${asset}` });
      return;
    }
    res.json({ asset, price: prices.get(asset) });
  });

  function guardPaused(res: Response): boolean {
    if (paused) {
      res.status(503).json({ error: 'protocol is paused' });
      return true;
    }
    return false;
  }

  app.post('/api/lending/deposit', (req, res) => {
    if (guardPaused(res)) return;
    const { userAddress, asset, amount } = req.body;
    if (!userAddress || !asset || !(amount > 0)) {
      res.status(400).json({ error: 'userAddress, asset, and a positive amount are required' });
      return;
    }
    const position = getOrCreatePosition(userAddress);
    position.collateral[asset] = (position.collateral[asset] ?? 0) + amount;
    res.json({ success: true, position: positionSummary(userAddress) });
  });

  app.post('/api/lending/borrow', (req, res) => {
    if (guardPaused(res)) return;
    const { userAddress, asset, amount } = req.body;
    if (!userAddress || !asset || !(amount > 0)) {
      res.status(400).json({ error: 'userAddress, asset, and a positive amount are required' });
      return;
    }
    if (getPrice(asset) === 0) {
      res.status(400).json({ error: `no oracle price available for ${asset}` });
      return;
    }
    const position = getOrCreatePosition(userAddress);
    const requestedValue = amount * getPrice(asset);
    const maxBorrowValue = collateralValue(position) * MAX_LTV;
    const projectedDebtValue = debtValue(position) + requestedValue;
    if (projectedDebtValue > maxBorrowValue) {
      res.status(400).json({
        error: 'borrow exceeds available collateral capacity',
        maxBorrowValue,
        projectedDebtValue,
      });
      return;
    }
    position.debt[asset] = (position.debt[asset] ?? 0) + amount;
    res.json({ success: true, position: positionSummary(userAddress) });
  });

  app.get('/api/positions/:userAddress', (req, res) => {
    res.json(positionSummary(req.params.userAddress));
  });

  app.post('/api/lending/repay', (req, res) => {
    if (guardPaused(res)) return;
    const { userAddress, asset, amount } = req.body;
    const position = getOrCreatePosition(userAddress);
    const owed = position.debt[asset] ?? 0;
    if (!(amount > 0) || amount > owed) {
      res.status(400).json({ error: `repay amount must be > 0 and <= outstanding debt (${owed})` });
      return;
    }
    position.debt[asset] = owed - amount;
    if (position.debt[asset] === 0) delete position.debt[asset];
    res.json({ success: true, position: positionSummary(userAddress) });
  });

  app.post('/api/lending/withdraw', (req, res) => {
    if (guardPaused(res)) return;
    const { userAddress, asset, amount } = req.body;
    const position = getOrCreatePosition(userAddress);
    const held = position.collateral[asset] ?? 0;
    if (!(amount > 0) || amount > held) {
      res.status(400).json({ error: `withdraw amount must be > 0 and <= held collateral (${held})` });
      return;
    }

    // Simulate the withdrawal, then verify it doesn't break solvency.
    const projected: Position = {
      ...position,
      collateral: { ...position.collateral, [asset]: held - amount },
    };
    if (debtValue(projected) > 0 && healthFactor(projected) < 1) {
      res.status(400).json({ error: 'withdrawal would leave the position undercollateralized' });
      return;
    }

    position.collateral[asset] = held - amount;
    if (position.collateral[asset] === 0) delete position.collateral[asset];
    res.json({ success: true, position: positionSummary(userAddress) });
  });

  app.post('/api/liquidations/liquidate', requireRole('liquidator'), (req, res) => {
    if (guardPaused(res)) return;
    const { targetUser, debtAsset, collateralAsset, repayAmount } = req.body;
    const position = getOrCreatePosition(targetUser);

    if (!position.debt[debtAsset]) {
      res.status(400).json({ error: `target has no ${debtAsset} debt` });
      return;
    }
    if (healthFactor(position) >= 1) {
      res.status(400).json({ error: 'target position is not liquidatable' });
      return;
    }

    const maxRepay = position.debt[debtAsset]! * CLOSE_FACTOR;
    if (!(repayAmount > 0) || repayAmount > maxRepay) {
      res.status(400).json({ error: `repayAmount must be > 0 and <= close-factor-limited max (${maxRepay})` });
      return;
    }

    const seizedValue = repayAmount * getPrice(debtAsset) * (1 + LIQUIDATION_BONUS);
    const seizedAmount = seizedValue / getPrice(collateralAsset);
    const heldCollateral = position.collateral[collateralAsset] ?? 0;
    if (seizedAmount > heldCollateral) {
      res.status(400).json({ error: 'insufficient collateral of the requested type to seize' });
      return;
    }

    position.debt[debtAsset] = position.debt[debtAsset]! - repayAmount;
    if (position.debt[debtAsset] === 0) delete position.debt[debtAsset];
    position.collateral[collateralAsset] = heldCollateral - seizedAmount;
    if (position.collateral[collateralAsset] === 0) delete position.collateral[collateralAsset];

    res.json({
      success: true,
      repaidAmount: repayAmount,
      seizedAmount,
      position: positionSummary(targetUser),
    });
  });

  app.post('/api/protocol/pause', requireRole('admin'), (_req, res) => {
    paused = true;
    res.json({ success: true, paused });
  });

  app.post('/api/protocol/resume', requireRole('admin'), (_req, res) => {
    paused = false;
    res.json({ success: true, paused });
  });

  app.post('/api/oracle/update-price', (req, res) => {
    const { asset, price } = req.body;
    if (!asset || !(price > 0)) {
      res.status(400).json({ error: 'asset and a positive price are required' });
      return;
    }
    setPrice(asset, price);
    res.json({ success: true, asset: asset.toUpperCase(), price });
  });

  return app;
}

/** Resets every in-memory store — call from `beforeEach` in each scenario file. */
export function reset(): void {
  roles.clear();
  prices.clear();
  positions.clear();
  paused = false;
}
