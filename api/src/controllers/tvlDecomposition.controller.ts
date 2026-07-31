import { Request, Response, NextFunction } from 'express';
import * as tvlDecompositionService from '../services/tvlDecomposition.service';

export const getBreakdown = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const breakdown = await tvlDecompositionService.getTvlBreakdown();
    res.status(200).json(breakdown);
  } catch (error) {
    next(error);
    return;
  }
};

export const getAttribution = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const timeRange = (req.query.timeRange as string) || '7d';
    const attribution = await tvlDecompositionService.getTvlAttribution(timeRange);
    res.status(200).json(attribution);
  } catch (error) {
    next(error);
    return;
  }
};

export const getHistory = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const timeRange = (req.query.timeRange as string) || '30d';
    const points = req.query.points ? Number(req.query.points) : undefined;
    const history = await tvlDecompositionService.getHistoricalTvlBreakdown(timeRange, points);
    res.status(200).json(history);
  } catch (error) {
    next(error);
    return;
  }
};

export const getCompetitorComparison = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const comparison = await tvlDecompositionService.getCompetitorTvlComparison();
    res.status(200).json(comparison);
  } catch (error) {
    next(error);
    return;
  }
};
