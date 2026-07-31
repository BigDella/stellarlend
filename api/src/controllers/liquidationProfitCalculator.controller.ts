import { Request, Response, NextFunction } from 'express';
import {
  calculateLiquidationProfitability,
  ProfitabilityRequest,
} from '../services/liquidationProfitCalculator.service';

export const getProfitability = (req: Request, res: Response, next: NextFunction): void => {
  try {
    const body = req.body as ProfitabilityRequest;
    const result = calculateLiquidationProfitability(body);
    res.status(200).json(result);
  } catch (error) {
    next(error);
    return;
  }
};
