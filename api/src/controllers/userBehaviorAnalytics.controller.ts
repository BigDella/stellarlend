import { Request, Response, NextFunction } from 'express';
import * as userBehaviorAnalyticsService from '../services/userBehaviorAnalytics.service';
import { FunnelStage, UserEventInput } from '../services/userBehaviorAnalytics.service';

export const postEvent = (req: Request, res: Response, next: NextFunction): void => {
  try {
    const body = req.body as UserEventInput;
    userBehaviorAnalyticsService.recordEvent(body);
    res.status(201).json({ recorded: !body.optedOut });
  } catch (error) {
    next(error);
    return;
  }
};

export const getFunnel = (_req: Request, res: Response, next: NextFunction): void => {
  try {
    res.status(200).json({ stages: userBehaviorAnalyticsService.getFunnel() });
  } catch (error) {
    next(error);
    return;
  }
};

export const getConversionRates = (_req: Request, res: Response, next: NextFunction): void => {
  try {
    res.status(200).json(userBehaviorAnalyticsService.getConversionRates());
  } catch (error) {
    next(error);
    return;
  }
};

export const getCohorts = (req: Request, res: Response, next: NextFunction): void => {
  try {
    const granularity = (req.query.granularity as 'weekly' | 'monthly') || 'weekly';
    const periods = req.query.periods ? Number(req.query.periods) : undefined;
    res.status(200).json({
      cohorts: userBehaviorAnalyticsService.getCohortRetention(granularity, periods),
    });
  } catch (error) {
    next(error);
    return;
  }
};

export const getPowerUsers = (req: Request, res: Response, next: NextFunction): void => {
  try {
    const topPercent = req.query.topPercent ? Number(req.query.topPercent) : undefined;
    res.status(200).json({ users: userBehaviorAnalyticsService.getPowerUsers(topPercent) });
  } catch (error) {
    next(error);
    return;
  }
};

export const getChurnRisk = (_req: Request, res: Response, next: NextFunction): void => {
  try {
    res.status(200).json({ users: userBehaviorAnalyticsService.getChurnRisk() });
  } catch (error) {
    next(error);
    return;
  }
};

export const getAbTestMetrics = (req: Request, res: Response, next: NextFunction): void => {
  try {
    const conversionStage = req.query.conversionStage as FunnelStage | undefined;
    res.status(200).json({
      variants: userBehaviorAnalyticsService.getAbTestMetrics(conversionStage),
    });
  } catch (error) {
    next(error);
    return;
  }
};
