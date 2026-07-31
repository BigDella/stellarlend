import { Request, Response, NextFunction } from 'express';
import { feeService } from '../services/fee.service';

export const getCurrentFees = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { asset } = req.query;
    const fees = feeService.getCurrentFees(asset as string);
    return res.status(200).json({ success: true, fees });
  } catch (err) {
    next(err);
    return;
  }
};

export const getFeeHistory = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { asset, limit } = req.query;
    const history = feeService.getFeeHistory(asset as string, Number(limit) || 50);
    return res.status(200).json({ success: true, history });
  } catch (err) {
    next(err);
    return;
  }
};

export const computeFee = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { asset, operation, amount } = req.body;
    const fee = feeService.computeFee(asset, operation, Number(amount));
    return res.status(200).json({ success: true, fee });
  } catch (err) {
    next(err);
    return;
  }
};
