import { Request, Response, NextFunction } from 'express';
import {
  getCrossProtocolComparison,
  getMarketShare,
  getLeaderboard,
  getAssetComparison,
  getFeeComparison,
  getLiquidationParamsComparison,
  getMarketShareHistory,
  getPositioningReport,
  getBenchmarkScore,
  getWeeklyDigest,
} from '../services/cross-protocol-etl/etl.service';
import { LeaderboardMetric } from '../services/cross-protocol-etl/types';
import { ValidationError } from '../utils/errors';

const VALID_LEADERBOARD_METRICS: LeaderboardMetric[] = ['supplyApy', 'borrowApy', 'tvlUsd'];

export const compare = async (_req: Request, res: Response, next: NextFunction): Promise<void> => {
  try {
    const result = await getCrossProtocolComparison();
    res.status(200).json(result);
  } catch (error) {
    next(error);
    return;
  }
};

export const marketShare = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const shares = await getMarketShare();
    res.status(200).json(shares);
  } catch (error) {
    next(error);
    return;
  }
};

export const leaderboard = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const metricParam = (req.query.metric as string) || 'tvlUsd';
    if (!VALID_LEADERBOARD_METRICS.includes(metricParam as LeaderboardMetric)) {
      throw new ValidationError(
        `Invalid metric "${metricParam}". Must be one of: ${VALID_LEADERBOARD_METRICS.join(', ')}`
      );
    }
    const limit = req.query.limit ? Number(req.query.limit) : undefined;
    const entries = await getLeaderboard(metricParam as LeaderboardMetric, limit);
    res.status(200).json(entries);
  } catch (error) {
    next(error);
    return;
  }
};

export const assetComparison = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
  try {
    const asset = req.query.asset as string;
    if (!asset) {
      throw new ValidationError('asset query parameter is required');
    }
    const result = await getAssetComparison(asset);
    res.status(200).json(result);
  } catch (error) {
    next(error);
    return;
  }
};

export const feeComparison = async (_req: Request, res: Response, next: NextFunction): Promise<void> => {
  try {
    const result = await getFeeComparison();
    res.status(200).json(result);
  } catch (error) {
    next(error);
    return;
  }
};

export const liquidationParamsComparison = (_req: Request, res: Response): void => {
  res.status(200).json(getLiquidationParamsComparison());
};

export const marketShareHistory = (req: Request, res: Response): void => {
  const limit = req.query.limit ? Number(req.query.limit) : undefined;
  res.status(200).json(getMarketShareHistory(limit));
};

export const positioningReport = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const result = await getPositioningReport();
    res.status(200).json(result);
  } catch (error) {
    next(error);
    return;
  }
};

export const benchmarkScore = async (_req: Request, res: Response, next: NextFunction): Promise<void> => {
  try {
    const result = await getBenchmarkScore();
    res.status(200).json(result);
  } catch (error) {
    next(error);
    return;
  }
};

export const weeklyDigest = async (_req: Request, res: Response, next: NextFunction): Promise<void> => {
  try {
    const result = await getWeeklyDigest();
    res.status(200).json(result);
  } catch (error) {
    next(error);
    return;
  }
};
