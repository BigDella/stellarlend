import { Request, Response, NextFunction } from 'express';
import {
  getHistoricalRates,
  getPoolUtilization,
  getRateComparison,
  getProtocolRevenue,
  getAnalyticsSummary,
  exportAnalytics,
} from '../services/analytics.service';
import { AnalyticsQuery } from '../types/analytics';

export const historicalRates = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const query: AnalyticsQuery = {
      timeRange: (req.query.timeRange as AnalyticsQuery['timeRange']) || '7d',
      poolAddress: req.query.poolAddress as string,
      limit: req.query.limit ? Number(req.query.limit) : undefined,
    };
    const rates = await getHistoricalRates(query);
    res.status(200).json(rates);
  } catch (error) {
    next(error);
    return;
  }
};

export const poolUtilization = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const query: AnalyticsQuery = {
      timeRange: (req.query.timeRange as AnalyticsQuery['timeRange']) || '7d',
      poolAddress: req.query.poolAddress as string,
    };
    const utilization = await getPoolUtilization(query);
    res.status(200).json(utilization);
  } catch (error) {
    next(error);
    return;
  }
};

export const rateComparison = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const comparisons = await getRateComparison();
    res.status(200).json(comparisons);
  } catch (error) {
    next(error);
    return;
  }
};

export const protocolRevenue = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const query: AnalyticsQuery = {
      timeRange: (req.query.timeRange as AnalyticsQuery['timeRange']) || '30d',
    };
    const revenue = await getProtocolRevenue(query);
    res.status(200).json(revenue);
  } catch (error) {
    next(error);
    return;
  }
};

export const analyticsSummary = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const summary = await getAnalyticsSummary();
    res.status(200).json(summary);
  } catch (error) {
    next(error);
    return;
  }
};

export const analyticsExport = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const format = (req.query.format as string) ?? 'json';
    const query: AnalyticsQuery = {
      timeRange: (req.query.timeRange as AnalyticsQuery['timeRange']) || '7d',
      poolAddress: req.query.poolAddress as string,
    };

    const data = await exportAnalytics(query, format as 'csv' | 'json');

    if (format === 'csv') {
      res.setHeader('Content-Type', 'text/csv');
      res.setHeader(
        'Content-Disposition',
        'attachment; filename="analytics-export.csv"'
      );
      res.status(200).send(data);
      return;
    }

    res.setHeader('Content-Type', 'application/json');
    res.setHeader(
      'Content-Disposition',
      'attachment; filename="analytics-export.json"'
    );
    res.status(200).json(data);
  } catch (error) {
    next(error);
    return;
  }
};
