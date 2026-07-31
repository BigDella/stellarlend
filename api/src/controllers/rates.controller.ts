import { Request, Response, NextFunction } from 'express';
import { getRateHistoryRange } from '../services/analytics.service';
import { RateGranularity, RateHistoryQuery } from '../types/analytics';

export const rateHistory = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const query: RateHistoryQuery = {
      asset: req.query.asset as string,
      from: req.query.from as string,
      to: req.query.to as string,
      granularity: req.query.granularity as RateGranularity,
    };
    const history = await getRateHistoryRange(query);
    res.status(200).json(history);
  } catch (error) {
    next(error);
    return;
  }
};
