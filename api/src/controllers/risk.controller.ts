import { Request, Response } from 'express';
import { riskMonitoringService } from '../services/riskMonitoring.service';

export class RiskController {
  async getPoolHealth(req: Request, res: Response): Promise<void> {
    try {
      const { poolId } = req.params!;
      const metrics = await riskMonitoringService.getPoolHealthMetrics(poolId);
      res.json(metrics);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch pool health metrics' });
    }
  }

  async getLiquidationHeatmap(req: Request, res: Response): Promise<void> {
    try {
      const { poolId } = req.query;
      const heatmap = await riskMonitoringService.getLiquidationRiskHeatmap(
        poolId as string | undefined
      );
      res.json(heatmap);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch liquidation heatmap' });
    }
  }

  async getOracleHealth(req: Request, res: Response): Promise<void> {
    try {
      const status = await riskMonitoringService.getOracleHealthStatus();
      res.json(status);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch oracle health status' });
    }
  }

  async getProtocolSafetyScore(req: Request, res: Response): Promise<void> {
    try {
      const score = await riskMonitoringService.getProtocolSafetyScore();
      res.json(score);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch protocol safety score' });
    }
  }

  async getMetricTrends(req: Request, res: Response): Promise<void> {
    try {
      const { metric, period } = req.query;
      const trends = await riskMonitoringService.getMetricTrends(
        metric as string,
        period as string
      );
      res.json(trends);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch metric trends' });
    }
  }

  async getAlerts(req: Request, res: Response): Promise<void> {
    try {
      const { severity, limit } = req.query;
      const alerts = await riskMonitoringService.getActiveAlerts(
        severity as string | undefined,
        limit ? parseInt(limit as string) : undefined
      );
      res.json(alerts);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch alerts' });
    }
  }

  async updateAlertConfig(req: Request, res: Response): Promise<void> {
    try {
      const config = req.body;
      await riskMonitoringService.updateAlertConfiguration(config);
      res.json({ success: true });
    } catch (error) {
      res.status(500).json({ error: 'Failed to update alert configuration' });
    }
  }

  async getUserRiskProfile(req: Request, res: Response): Promise<void> {
    try {
      const { address } = req.params!;
      const profile = await riskMonitoringService.getUserRiskProfile(address);
      res.json(profile);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch user risk profile' });
    }
  }

  async getDashboard(_req: Request, res: Response): Promise<void> {
    try {
      const snapshot = await riskMonitoringService.getDashboard();
      res.json(snapshot);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch dashboard snapshot' });
    }
  }
}

export const riskController = new RiskController();
