/**
 * Gas Cost Estimation Controller
 * 
 * Handles HTTP endpoints for gas cost estimation and optimization
 */

import { Request, Response } from 'express';
import { gasEstimatorService } from '../services/gas/estimator';
import { GasEstimateRequest, GasAlertConfig } from '../types/gas';
import { ValidationError } from '../utils/errors';
import logger from '../utils/logger';

export class GasController {
  /**
   * POST /api/gas/estimate
   * Estimate gas cost for a specific operation
   */
  async estimateGas(req: Request, res: Response): Promise<void> {
    try {
      const request: GasEstimateRequest = {
        operation: req.body.operation,
        userAddress: req.body.userAddress,
        assetAddress: req.body.assetAddress,
        amount: req.body.amount,
        includeOptimizations: req.body.includeOptimizations ?? true,
        includeHistorical: req.body.includeHistorical ?? true,
      };

      // Validation
      if (!request.operation || !request.userAddress || !request.amount) {
        throw new ValidationError('operation, userAddress, and amount are required');
      }

      const validOperations = ['deposit', 'withdraw', 'borrow', 'repay', 'liquidation', 'flash_loan'];
      if (!validOperations.includes(request.operation)) {
        throw new ValidationError(`Invalid operation. Must be one of: ${validOperations.join(', ')}`);
      }

      const estimate = await gasEstimatorService.estimateGasCost(request);
      res.json(estimate);
    } catch (error) {
      logger.error('Gas estimation failed:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to estimate gas cost' });
      }
    }
  }

  /**
   * GET /api/gas/historical/:operation
   * Get historical gas data for an operation
   */
  async getHistoricalData(req: Request, res: Response): Promise<void> {
    try {
      const { operation } = req.params!;
      const period = (req.query.period as string) || '30d';

      const validOperations = ['deposit', 'withdraw', 'borrow', 'repay', 'liquidation', 'flash_loan'];
      if (!validOperations.includes(operation)) {
        throw new ValidationError('Invalid operation');
      }

      const historical = await gasEstimatorService.getHistoricalData(
        operation as any,
        period
      );
      res.json(historical);
    } catch (error) {
      logger.error('Failed to get historical data:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to get historical data' });
      }
    }
  }

  /**
   * GET /api/gas/chart/:operation
   * Get historical gas chart data
   */
  async getChartData(req: Request, res: Response): Promise<void> {
    try {
      const { operation } = req.params!;
      const period = (req.query.period as string) || '7d';

      const validOperations = ['deposit', 'withdraw', 'borrow', 'repay', 'liquidation', 'flash_loan'];
      if (!validOperations.includes(operation)) {
        throw new ValidationError('Invalid operation');
      }

      const chartData = await gasEstimatorService.getHistoricalChart(
        operation as any,
        period
      );
      res.json(chartData);
    } catch (error) {
      logger.error('Failed to get chart data:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to get chart data' });
      }
    }
  }

  /**
   * GET /api/gas/compare
   * Compare gas costs across all operations
   */
  async compareOperations(req: Request, res: Response): Promise<void> {
    try {
      const comparison = await gasEstimatorService.compareOperations();
      res.json(comparison);
    } catch (error) {
      logger.error('Failed to compare operations:', error);
      res.status(500).json({ error: 'Failed to compare operations' });
    }
  }

  /**
   * POST /api/gas/alerts
   * Configure gas cost alert
   */
  async configureAlert(req: Request, res: Response): Promise<void> {
    try {
      const config: GasAlertConfig = {
        userAddress: req.body.userAddress,
        operation: req.body.operation,
        threshold: req.body.threshold,
        enabled: req.body.enabled ?? true,
      };

      // Validation
      if (!config.operation || !config.threshold) {
        throw new ValidationError('operation and threshold are required');
      }

      const validOperations = ['deposit', 'withdraw', 'borrow', 'repay', 'liquidation', 'flash_loan'];
      if (!validOperations.includes(config.operation)) {
        throw new ValidationError('Invalid operation');
      }

      await gasEstimatorService.configureAlert(config);
      res.json({ success: true, message: 'Alert configured successfully' });
    } catch (error) {
      logger.error('Failed to configure alert:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to configure alert' });
      }
    }
  }

  /**
   * GET /api/gas/alerts
   * Get all alerts for a user
   */
  async getAlerts(req: Request, res: Response): Promise<void> {
    try {
      const userAddress = req.query.userAddress as string | undefined;
      const alerts = await gasEstimatorService.getAlerts(userAddress);
      res.json(alerts);
    } catch (error) {
      logger.error('Failed to get alerts:', error);
      res.status(500).json({ error: 'Failed to get alerts' });
    }
  }

  /**
   * POST /api/gas/accuracy
   * Record actual gas cost for accuracy tracking
   */
  async recordActualCost(req: Request, res: Response): Promise<void> {
    try {
      const { operation, estimatedCost, actualCost, txHash } = req.body;

      if (!operation || !estimatedCost || !actualCost || !txHash) {
        throw new ValidationError('operation, estimatedCost, actualCost, and txHash are required');
      }

      await gasEstimatorService.recordActualCost(
        operation,
        estimatedCost,
        actualCost,
        txHash
      );
      res.json({ success: true, message: 'Actual cost recorded' });
    } catch (error) {
      logger.error('Failed to record actual cost:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to record actual cost' });
      }
    }
  }

  /**
   * GET /api/gas/accuracy
   * Get accuracy report
   */
  async getAccuracyReport(req: Request, res: Response): Promise<void> {
    try {
      const period = (req.query.period as string) || '7d';
      const report = await gasEstimatorService.getAccuracyReport(period);
      res.json(report);
    } catch (error) {
      logger.error('Failed to get accuracy report:', error);
      res.status(500).json({ error: 'Failed to get accuracy report' });
    }
  }

  /**
   * POST /api/gas/batch-estimate
   * Estimate gas cost for batch operations
   */
  async estimateBatchCost(req: Request, res: Response): Promise<void> {
    try {
      const { operations } = req.body;

      if (!Array.isArray(operations) || operations.length === 0) {
        throw new ValidationError('operations array is required');
      }

      const batchEstimate = await gasEstimatorService.estimateBatchCost(operations);
      res.json(batchEstimate);
    } catch (error) {
      logger.error('Failed to estimate batch cost:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to estimate batch cost' });
      }
    }
  }

  /**
   * GET /api/gas/timing/:operation
   * Get timing recommendation for an operation
   */
  async getTimingRecommendation(req: Request, res: Response): Promise<void> {
    try {
      const { operation } = req.params!;

      const validOperations = ['deposit', 'withdraw', 'borrow', 'repay', 'liquidation', 'flash_loan'];
      if (!validOperations.includes(operation)) {
        throw new ValidationError('Invalid operation');
      }

      const recommendation = await gasEstimatorService.getTimingRecommendation(
        operation as any
      );
      res.json(recommendation);
    } catch (error) {
      logger.error('Failed to get timing recommendation:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to get timing recommendation' });
      }
    }
  }
}

export const gasController = new GasController();
