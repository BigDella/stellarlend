import { Request, Response, NextFunction } from 'express';
import * as poolPerformanceService from '../services/poolPerformance.service';

export const getPoolSnapshots = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const { poolAddress } = req.params;
    const timeRange = (req.query.timeRange as string) || '7d';
    const snapshots = await poolPerformanceService.getPoolSnapshots(poolAddress, timeRange);
    res.status(200).json(snapshots);
  } catch (error) {
    next(error);
    return;
  }
};

export const getPoolMetrics = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const { poolAddress } = req.params;
    const period = (req.query.period as string) || '30d';
    const metrics = await poolPerformanceService.getPoolMetrics(poolAddress, period);
    res.status(200).json(metrics);
  } catch (error) {
    next(error);
    return;
  }
};

export const comparePools = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const timeRange = (req.query.timeRange as string) || '30d';
    const comparison = await poolPerformanceService.comparePools(timeRange);
    res.status(200).json(comparison);
  } catch (error) {
    next(error);
    return;
  }
};

export const exportPerformanceData = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const { poolAddress } = req.params;
    const format = (req.query.format as string) ?? 'json';
    const data = await poolPerformanceService.exportPerformanceData(
      poolAddress,
      format as 'csv' | 'json'
    );

    if (format === 'csv') {
      res.setHeader('Content-Type', 'text/csv');
      res.setHeader('Content-Disposition', 'attachment; filename="pool-performance-export.csv"');
      res.status(200).send(data);
      return;
    }

    res.setHeader('Content-Type', 'application/json');
    res.setHeader('Content-Disposition', 'attachment; filename="pool-performance-export.json"');
    res.status(200).json(data);
  } catch (error) {
    next(error);
    return;
  }
};

export const getPerformanceSummary = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const summary = await poolPerformanceService.getPerformanceSummary();
    res.status(200).json(summary);
  } catch (error) {
    next(error);
    return;
  }
};
