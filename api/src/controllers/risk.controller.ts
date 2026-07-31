import { Request, Response } from 'express';
import { riskMonitoringService } from '../services/riskMonitoring.service';
import { collateralRatioMonitorService } from '../services/collateralRatioMonitor.service';

export class RiskController {
  async getPoolHealth(req: Request, res: Response): Promise<void> {
    try {
      const { poolId } = req.params!;
      const metrics = await riskMonitoringService.getPoolHealthMetrics(poolId!);
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
      const profile = await riskMonitoringService.getUserRiskProfile(address!);
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

  // Collateral Ratio Monitoring endpoints
  async getCollateralRatioSnapshots(req: Request, res: Response): Promise<void> {
    try {
      const snapshots = collateralRatioMonitorService.getCurrentSnapshots();
      res.json(snapshots);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch collateral ratio snapshots' });
    }
  }

  async getCollateralRatioSnapshot(req: Request, res: Response): Promise<void> {
    try {
      const { asset } = req.params;
      const snapshot = collateralRatioMonitorService.getSnapshot(asset);
      
      if (!snapshot) {
        res.status(404).json({ error: 'Snapshot not found for asset' });
        return;
      }
      
      res.json(snapshot);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch collateral ratio snapshot' });
    }
  }

  async getCollateralRatioPositionRisks(req: Request, res: Response): Promise<void> {
    try {
      const { address } = req.query;
      const positions = collateralRatioMonitorService.getPositionRisks(address as string);
      res.json(positions);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch position risks' });
    }
  }

  async getCollateralRatioAlerts(req: Request, res: Response): Promise<void> {
    try {
      const { severity, limit } = req.query;
      const alerts = collateralRatioMonitorService.getAlerts(
        severity as string,
        limit ? parseInt(limit as string) : undefined
      );
      res.json(alerts);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch collateral ratio alerts' });
    }
  }

  async acknowledgeCollateralRatioAlert(req: Request, res: Response): Promise<void> {
    try {
      const { alertId } = req.params;
      collateralRatioMonitorService.acknowledgeAlert(alertId);
      res.json({ success: true });
    } catch (error) {
      res.status(500).json({ error: 'Failed to acknowledge alert' });
    }
  }

  async getCollateralRatioHistoricalTrends(req: Request, res: Response): Promise<void> {
    try {
      const { asset } = req.params;
      const { hours } = req.query;
      const trends = collateralRatioMonitorService.getHistoricalTrends(
        asset,
        hours ? parseInt(hours as string) : undefined
      );
      res.json(trends);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch historical trends' });
    }
  }

  async getCollateralRatioAssetMetrics(req: Request, res: Response): Promise<void> {
    try {
      const { asset } = req.params;
      const metrics = collateralRatioMonitorService.getAssetMetrics(asset);
      
      if (!metrics) {
        res.status(404).json({ error: 'Metrics not found for asset' });
        return;
      }
      
      res.json(metrics);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch asset metrics' });
    }
  }

  async getAllCollateralRatioAssetMetrics(req: Request, res: Response): Promise<void> {
    try {
      const metrics = collateralRatioMonitorService.getAllAssetMetrics();
      res.json(metrics);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch asset metrics' });
    }
  }

  async updateCollateralRatioThresholds(req: Request, res: Response): Promise<void> {
    try {
      const config = req.body;
      await collateralRatioMonitorService.updateThresholds(config);
      res.json({ success: true, thresholds: collateralRatioMonitorService.getThresholds() });
    } catch (error) {
      res.status(500).json({ error: 'Failed to update thresholds' });
    }
  }

  async getCollateralRatioThresholds(req: Request, res: Response): Promise<void> {
    try {
      const thresholds = collateralRatioMonitorService.getThresholds();
      res.json(thresholds);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch thresholds' });
    }
  }

  async getRiskAdjustedLendingLimit(req: Request, res: Response): Promise<void> {
    try {
      const { asset } = req.params;
      const limit = collateralRatioMonitorService.calculateRiskAdjustedLendingLimit(asset);
      res.json(limit);
    } catch (error) {
      res.status(500).json({ error: 'Failed to calculate risk-adjusted lending limit' });
    }
  }
}

export const riskController = new RiskController();
