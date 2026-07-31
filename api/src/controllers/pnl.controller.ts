import { Request, Response, NextFunction } from 'express';
import * as pnlService from '../services/pnl.service';
import { ExpenseCategory, PnlPeriod, RevenueSource } from '../services/pnl.service';
import { ValidationError } from '../utils/errors';

export const postRevenue = (req: Request, res: Response, next: NextFunction): void => {
  try {
    const { source, amount, asset, poolAddress } = req.body as {
      source: RevenueSource;
      amount: number;
      asset: string;
      poolAddress?: string;
    };
    pnlService.recordRevenue(source, amount, asset, poolAddress);
    res.status(201).json({ message: 'Revenue recorded' });
  } catch (error) {
    next(error);
    return;
  }
};

export const postExpense = (req: Request, res: Response, next: NextFunction): void => {
  try {
    const { category, amount, description } = req.body as {
      category: ExpenseCategory;
      amount: number;
      description: string;
    };
    pnlService.recordExpense(category, amount, description);
    res.status(201).json({ message: 'Expense recorded' });
  } catch (error) {
    next(error);
    return;
  }
};

export const getSummary = (_req: Request, res: Response, next: NextFunction): void => {
  try {
    res.status(200).json(pnlService.getSummary());
  } catch (error) {
    next(error);
    return;
  }
};

export const getBreakdown = (_req: Request, res: Response, next: NextFunction): void => {
  try {
    res.status(200).json(pnlService.getBreakdown());
  } catch (error) {
    next(error);
    return;
  }
};

const VALID_PERIODS: PnlPeriod[] = ['daily', 'monthly', 'annual'];

export const getHistory = (req: Request, res: Response, next: NextFunction): void => {
  try {
    const period = (req.query.period as PnlPeriod) || 'monthly';
    if (!VALID_PERIODS.includes(period)) {
      throw new ValidationError(`period must be one of: ${VALID_PERIODS.join(', ')}`);
    }
    const count = req.query.count ? Number(req.query.count) : undefined;
    res.status(200).json({ statements: pnlService.getStatements(period, count) });
  } catch (error) {
    next(error);
    return;
  }
};

export const getRevenueByPool = (_req: Request, res: Response, next: NextFunction): void => {
  try {
    res.status(200).json({ pools: pnlService.getRevenueByPool() });
  } catch (error) {
    next(error);
    return;
  }
};

export const getCumulativeChart = (_req: Request, res: Response, next: NextFunction): void => {
  try {
    res.status(200).json({ points: pnlService.getCumulativeChart() });
  } catch (error) {
    next(error);
    return;
  }
};

export const getYieldVsBenchmark = (req: Request, res: Response, next: NextFunction): void => {
  try {
    const tvlUsd = req.query.tvlUsd ? Number(req.query.tvlUsd) : 0;
    const lookbackDays = req.query.lookbackDays ? Number(req.query.lookbackDays) : undefined;
    res.status(200).json(pnlService.getYieldVsBenchmark(tvlUsd, lookbackDays));
  } catch (error) {
    next(error);
    return;
  }
};

export const getRevenueGrowth = (req: Request, res: Response, next: NextFunction): void => {
  try {
    const period = (req.query.period as PnlPeriod) || 'monthly';
    if (!VALID_PERIODS.includes(period)) {
      throw new ValidationError(`period must be one of: ${VALID_PERIODS.join(', ')}`);
    }
    res.status(200).json(pnlService.getRevenueGrowth(period));
  } catch (error) {
    next(error);
    return;
  }
};

export const getExport = (req: Request, res: Response, next: NextFunction): void => {
  try {
    const format = (req.query.format as 'quickbooks' | 'xero') || 'quickbooks';
    if (format !== 'quickbooks' && format !== 'xero') {
      throw new ValidationError('format must be one of: quickbooks, xero');
    }
    const csv = pnlService.exportToAccountingFormat(format);
    res.setHeader('Content-Type', 'text/csv');
    res.setHeader('Content-Disposition', `attachment; filename="pnl-export-${format}.csv"`);
    res.status(200).send(csv);
  } catch (error) {
    next(error);
    return;
  }
};
