import { Request, Response } from 'express';
import { treasuryDashboardService } from '../services/treasury-dashboard.service';
import { ValidationError } from '../utils/errors';
import logger from '../utils/logger';

export class TreasuryDashboardController {
  async getOverview(req: Request, res: Response): Promise<void> {
    try {
      const overview = treasuryDashboardService.getOverview();
      res.json({
        success: true,
        data: overview,
      });
    } catch (error) {
      logger.error('Failed to fetch treasury overview:', error);
      res.status(500).json({ error: 'Failed to fetch treasury overview' });
    }
  }

  async setAssetBalance(req: Request, res: Response): Promise<void> {
    try {
      const { asset, balance, usdPrice } = req.body;

      if (!asset || balance === undefined || usdPrice === undefined) {
        throw new ValidationError('asset, balance, and usdPrice are required');
      }

      treasuryDashboardService.setAssetBalance(asset, balance, usdPrice);

      res.json({
        success: true,
        message: `Balance updated for ${asset}`,
      });
    } catch (error) {
      logger.error('Failed to set asset balance:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to set asset balance' });
      }
    }
  }

  async recordRevenue(req: Request, res: Response): Promise<void> {
    try {
      const { source, amount, asset } = req.body;

      if (!source || !amount || !asset) {
        throw new ValidationError('source, amount, and asset are required');
      }

      if (!['fees', 'liquidation_penalties', 'yield'].includes(source)) {
        throw new ValidationError('Invalid revenue source');
      }

      treasuryDashboardService.recordRevenue(source, amount, asset);

      res.json({
        success: true,
        message: 'Revenue recorded',
      });
    } catch (error) {
      logger.error('Failed to record revenue:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to record revenue' });
      }
    }
  }

  async recordExpense(req: Request, res: Response): Promise<void> {
    try {
      const { category, amount, description } = req.body;

      if (!category || !amount || !description) {
        throw new ValidationError('category, amount, and description are required');
      }

      if (!['development_grants', 'operational_costs', 'marketing'].includes(category)) {
        throw new ValidationError('Invalid expense category');
      }

      treasuryDashboardService.recordExpense(category, amount, description);

      res.json({
        success: true,
        message: 'Expense recorded',
      });
    } catch (error) {
      logger.error('Failed to record expense:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to record expense' });
      }
    }
  }

  async getRevenueTracking(req: Request, res: Response): Promise<void> {
    try {
      const { days = 30 } = req.query;
      const revenue = treasuryDashboardService.getRevenueTracking(parseInt(days as string) || 30);

      res.json({
        success: true,
        data: revenue,
      });
    } catch (error) {
      logger.error('Failed to fetch revenue tracking:', error);
      res.status(500).json({ error: 'Failed to fetch revenue tracking' });
    }
  }

  async getExpenseTracking(req: Request, res: Response): Promise<void> {
    try {
      const { days = 30 } = req.query;
      const expenses = treasuryDashboardService.getExpenseTracking(parseInt(days as string) || 30);

      res.json({
        success: true,
        data: expenses,
      });
    } catch (error) {
      logger.error('Failed to fetch expense tracking:', error);
      res.status(500).json({ error: 'Failed to fetch expense tracking' });
    }
  }

  async getCashFlowForecasts(req: Request, res: Response): Promise<void> {
    try {
      const forecasts = treasuryDashboardService.getCashFlowForecasts();

      res.json({
        success: true,
        data: forecasts,
      });
    } catch (error) {
      logger.error('Failed to fetch cash flow forecasts:', error);
      res.status(500).json({ error: 'Failed to fetch cash flow forecasts' });
    }
  }

  async getScenarioAnalysis(req: Request, res: Response): Promise<void> {
    try {
      const scenarios = treasuryDashboardService.getScenarioAnalysis();

      res.json({
        success: true,
        data: scenarios,
      });
    } catch (error) {
      logger.error('Failed to fetch scenario analysis:', error);
      res.status(500).json({ error: 'Failed to fetch scenario analysis' });
    }
  }

  async getBurnRateAndRunway(req: Request, res: Response): Promise<void> {
    try {
      const analysis = treasuryDashboardService.getBurnRateAndRunway();

      res.json({
        success: true,
        data: analysis,
      });
    } catch (error) {
      logger.error('Failed to fetch burn rate and runway:', error);
      res.status(500).json({ error: 'Failed to fetch burn rate and runway' });
    }
  }

  async getCashFlowHistory(req: Request, res: Response): Promise<void> {
    try {
      const { months = 12 } = req.query;
      const history = treasuryDashboardService.getCashFlowHistory(parseInt(months as string) || 12);

      res.json({
        success: true,
        data: history,
      });
    } catch (error) {
      logger.error('Failed to fetch cash flow history:', error);
      res.status(500).json({ error: 'Failed to fetch cash flow history' });
    }
  }
}

export const treasuryDashboardController = new TreasuryDashboardController();
