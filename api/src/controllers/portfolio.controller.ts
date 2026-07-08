import { Request, Response, NextFunction } from 'express';
import { StellarService } from '../services/stellar.service';
import {
  analyzePortfolio,
  toCSV,
  computeInterestAccrualProjection,
  computeLiquidationPrice,
  getHealthFactorMonitor as buildHealthFactorMonitor,
} from '../services/portfolio.service';
import { PortfolioAnalyticsResponse } from '../types/portfolio';
import { redisCacheService } from '../services/redisCache.service';
import { config } from '../config/index';

const PORTFOLIO_CACHE_TTL_S = Math.floor(config.cache.positionTtlMs / 1000);

export const getPortfolioAnalytics = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const { userAddress } = req.params!;
    const cacheKey = redisCacheService.buildKey('position', `portfolio:${userAddress}`);

    const cached = await redisCacheService.get<PortfolioAnalyticsResponse>(cacheKey);
    if (cached) {
      res.status(200).json(cached);
      return;
    }

    const stellarService = new StellarService();
    const [position, history] = await Promise.all([
      stellarService.getUserPosition(userAddress!),
      stellarService.getTransactionHistory({ userAddress: userAddress!, limit: 200 }),
    ]);

    const analytics = analyzePortfolio(userAddress!, position, history.data);

    await redisCacheService.set(cacheKey, analytics, PORTFOLIO_CACHE_TTL_S);

    res.status(200).json(analytics);
  } catch (error) {
    next(error);
    return;
  }
};

export const getPortfolioRisk = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const { userAddress } = req.params!;
    const stellarService = new StellarService();
    const position = await stellarService.getUserPosition(userAddress!);
    const analytics = analyzePortfolio(userAddress!, position!, []);

    res.status(200).json({
      userAddress,
      riskMetrics: analytics.riskMetrics,
      suggestions: analytics.suggestions,
      generatedAt: analytics.generatedAt,
    });
  } catch (error) {
    next(error);
    return;
  }
};

export const getPortfolioPerformance = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const { userAddress } = req.params!;
    const limit = req.query.limit ? Number(req.query.limit) : 200;

    const stellarService = new StellarService();
    const [position, history] = await Promise.all([
      stellarService.getUserPosition(userAddress!),
      stellarService.getTransactionHistory({ userAddress: userAddress!, limit }),
    ]);

    const analytics = analyzePortfolio(userAddress!, position, history.data);

    res.status(200).json({
      userAddress,
      performance: analytics.performance,
      pagination: history.pagination,
      generatedAt: analytics.generatedAt,
    });
  } catch (error) {
    next(error);
    return;
  }
};

export const exportPortfolio = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const { userAddress } = req.params!;
    const format = (req.query.format as string) ?? 'json';

    const stellarService = new StellarService();
    const [position, history] = await Promise.all([
      stellarService.getUserPosition(userAddress!),
      stellarService.getTransactionHistory({ userAddress: userAddress!, limit: 200 }),
    ]);

    const analytics = analyzePortfolio(userAddress!, position, history.data);

    if (format === 'csv') {
      res.setHeader('Content-Type', 'text/csv');
      res.setHeader(
        'Content-Disposition',
        `attachment; filename="portfolio-${userAddress!.slice(0, 8)}.csv"`
      );
      res.status(200).send(toCSV(history.data));
      return;
    }

    res.setHeader('Content-Type', 'application/json');
    res.setHeader(
      'Content-Disposition',
      `attachment; filename="portfolio-${userAddress!.slice(0, 8)}.json"`
    );
    res.status(200).json({
      exportedAt: new Date().toISOString(),
      userAddress,
      analytics,
      transactions: history.data,
    });
  } catch (error) {
    next(error);
    return;
  }
};

export const getInterestProjection = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const { userAddress } = req.params!;
    const stellarService = new StellarService();
    const position = await stellarService.getUserPosition(userAddress!);
    const borrowApy = req.query.borrowApy ? Number(req.query.borrowApy) : 0.05;

    const projection = computeInterestAccrualProjection(position, borrowApy);
    res.status(200).json({
      userAddress,
      projection,
      generatedAt: new Date().toISOString(),
    });
  } catch (error) {
    next(error);
    return;
  }
};

export const getLiquidationPrice = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const { userAddress } = req.params!;
    const currentPrice = req.query.price ? Number(req.query.price) : 1;

    const stellarService = new StellarService();
    const position = await stellarService.getUserPosition(userAddress!);

    const liquidation = computeLiquidationPrice(position, currentPrice);
    res.status(200).json({
      userAddress,
      currentPrice,
      liquidation,
      generatedAt: new Date().toISOString(),
    });
  } catch (error) {
    next(error);
    return;
  }
};

export const getHealthFactorMonitor = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const { userAddress } = req.params!;

    const stellarService = new StellarService();
    const position = await stellarService.getUserPosition(userAddress!);

    const monitor = buildHealthFactorMonitor(position);
    res.status(200).json({
      userAddress,
      ...monitor,
      generatedAt: new Date().toISOString(),
    });
  } catch (error) {
    next(error);
    return;
  }
};
