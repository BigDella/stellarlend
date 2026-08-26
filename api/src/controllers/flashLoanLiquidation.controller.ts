import { Request, Response, NextFunction } from 'express';
import * as flashLoanLiquidation from '../services/flashLoanLiquidation.service';

export async function simulateCombo(req: Request, res: Response, next: NextFunction): Promise<void> {
  try {
    const { debtAmount, debtPrice, collateralPrice, feeBps, incentiveBps } = req.body ?? {};
    if (typeof debtAmount !== 'number' || debtAmount <= 0) {
      res.status(400).json({ success: false, error: 'debtAmount must be a positive number' });
      return;
    }
    const sim = flashLoanLiquidation.simulateFlashLoanLiquidation({
      debtAmount,
      debtPrice,
      collateralPrice,
      feeBps,
      incentiveBps,
    });
    res.status(200).json({ success: true, data: sim });
  } catch (error) {
    next(error);
  }
}

export async function executeCombo(req: Request, res: Response, next: NextFunction): Promise<void> {
  try {
    const { liquidator, borrower, debtAsset, collateralAsset, debtAmount, debtPrice, collateralPrice } =
      req.body ?? {};
    if (!liquidator || !borrower || !debtAsset || !collateralAsset || typeof debtAmount !== 'number') {
      res.status(400).json({ success: false, error: 'liquidator, borrower, assets, and debtAmount required' });
      return;
    }
    const result = await flashLoanLiquidation.prepareComboExecution({
      liquidator,
      borrower,
      debtAsset,
      collateralAsset,
      debtAmount,
      debtPrice,
      collateralPrice,
    });
    const status = result.status === 'rejected' ? 422 : 200;
    res.status(status).json({ success: result.status !== 'rejected', data: result });
  } catch (error) {
    next(error);
  }
}

export async function simulateMultiAsset(req: Request, res: Response, next: NextFunction): Promise<void> {
  try {
    const { legs, feeBps } = req.body ?? {};
    if (!Array.isArray(legs)) {
      res.status(400).json({ success: false, error: 'legs array required' });
      return;
    }
    const result = flashLoanLiquidation.simulateMultiAssetFlashLoan(legs, feeBps);
    res.status(result.ok ? 200 : 400).json({ success: result.ok, data: result });
  } catch (error) {
    next(error);
  }
}
