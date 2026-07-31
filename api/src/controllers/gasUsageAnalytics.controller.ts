/**
 * Gas Usage Analytics Controller — Issue #483
 */

import { Request, Response } from 'express';
import { gasUsageAnalyticsService } from '../services/analytics/gasUsageAnalytics.service';
import { ValidationError } from '../utils/errors';
import logger from '../utils/logger';

const VALID_PERIODS = ['24h', '7d', '30d'];
const VALID_GRANULARITIES = ['daily', 'weekly'];

function parsePeriod(raw: unknown): string {
  const period = (raw as string) || '30d';
  if (!VALID_PERIODS.includes(period)) {
    throw new ValidationError(`Invalid period. Must be one of: ${VALID_PERIODS.join(', ')}`);
  }
  return period;
}

export class GasUsageAnalyticsController {
  /** GET /api/analytics/gas/by-function/:functionName?period=30d */
  async getByFunction(req: Request, res: Response): Promise<void> {
    try {
      const { functionName } = req.params as { functionName: string };
      const period = parsePeriod(req.query.period);
      const stats = gasUsageAnalyticsService.getStats(functionName, period);
      res.json({ success: true, data: stats });
    } catch (error) {
      this.handleError(res, error, 'Failed to compute gas usage stats');
    }
  }

  /** GET /api/analytics/gas/by-function?period=30d */
  async getAllFunctions(req: Request, res: Response): Promise<void> {
    try {
      const period = parsePeriod(req.query.period);
      const stats = gasUsageAnalyticsService.getAllStats(period);
      res.json({ success: true, data: stats });
    } catch (error) {
      this.handleError(res, error, 'Failed to compute gas usage stats');
    }
  }

  /** GET /api/analytics/gas/anomalies?period=30d&stdDevThreshold=3 */
  async getAnomalies(req: Request, res: Response): Promise<void> {
    try {
      const period = parsePeriod(req.query.period);
      const stdDevThreshold = req.query.stdDevThreshold ? Number(req.query.stdDevThreshold) : undefined;
      const anomalies = gasUsageAnalyticsService.detectAnomalies(period, stdDevThreshold);
      res.json({ success: true, data: anomalies });
    } catch (error) {
      this.handleError(res, error, 'Failed to detect gas usage anomalies');
    }
  }

  /** GET /api/analytics/gas/trend/:functionName?granularity=daily&period=30d */
  async getTrend(req: Request, res: Response): Promise<void> {
    try {
      const { functionName } = req.params as { functionName: string };
      const period = parsePeriod(req.query.period);
      const granularity = (req.query.granularity as string) || 'daily';
      if (!VALID_GRANULARITIES.includes(granularity)) {
        throw new ValidationError(`Invalid granularity. Must be one of: ${VALID_GRANULARITIES.join(', ')}`);
      }
      const trend = gasUsageAnalyticsService.getTrend(functionName, granularity as 'daily' | 'weekly', period);
      res.json({ success: true, data: trend });
    } catch (error) {
      this.handleError(res, error, 'Failed to compute gas usage trend');
    }
  }

  /** GET /api/analytics/gas/compare?functionA=deposit&functionB=withdraw&period=30d */
  async compare(req: Request, res: Response): Promise<void> {
    try {
      const functionA = req.query.functionA as string;
      const functionB = req.query.functionB as string;
      if (!functionA || !functionB) {
        throw new ValidationError('functionA and functionB query parameters are required');
      }
      const period = parsePeriod(req.query.period);
      const comparison = gasUsageAnalyticsService.compareFunctions(functionA, functionB, period);
      res.json({ success: true, data: comparison });
    } catch (error) {
      this.handleError(res, error, 'Failed to compare functions');
    }
  }

  /** GET /api/analytics/gas/calldata-correlation/:functionName?period=30d */
  async getCalldataCorrelation(req: Request, res: Response): Promise<void> {
    try {
      const { functionName } = req.params as { functionName: string };
      const period = parsePeriod(req.query.period);
      const correlation = gasUsageAnalyticsService.getCalldataCorrelation(functionName, period);
      res.json({ success: true, data: correlation });
    } catch (error) {
      this.handleError(res, error, 'Failed to compute calldata correlation');
    }
  }

  /** GET /api/analytics/gas/report/:functionName?period=30d */
  async getReport(req: Request, res: Response): Promise<void> {
    try {
      const { functionName } = req.params as { functionName: string };
      const period = parsePeriod(req.query.period);
      const report = gasUsageAnalyticsService.getFunctionReport(functionName, period);
      res.json({ success: true, data: report });
    } catch (error) {
      this.handleError(res, error, 'Failed to generate gas usage report');
    }
  }

  /** POST /api/analytics/gas/record  body: { functionName, gasUsed, calldataSize?, txHash? } */
  async recordSample(req: Request, res: Response): Promise<void> {
    try {
      const { functionName, gasUsed, calldataSize, txHash } = req.body;
      if (!functionName || gasUsed === undefined || gasUsed === null) {
        throw new ValidationError('functionName and gasUsed are required');
      }
      gasUsageAnalyticsService.recordSample({
        functionName,
        gasUsed: Number(gasUsed),
        calldataSize: calldataSize !== undefined ? Number(calldataSize) : undefined,
        timestamp: Date.now(),
        txHash,
      });
      res.json({ success: true, message: 'Gas usage sample recorded' });
    } catch (error) {
      this.handleError(res, error, 'Failed to record gas usage sample');
    }
  }

  private handleError(res: Response, error: unknown, fallbackMessage: string): void {
    logger.error(fallbackMessage, error);
    if (error instanceof ValidationError) {
      res.status(400).json({ success: false, error: error.message });
    } else {
      res.status(500).json({ success: false, error: fallbackMessage });
    }
  }
}

export const gasUsageAnalyticsController = new GasUsageAnalyticsController();
