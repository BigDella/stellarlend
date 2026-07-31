import { Request, Response } from 'express';
import { collateralRatioMonitorService } from '../services/collateralRatioMonitor.service';

export class CollateralRatioController {
  async getCurrentSnapshots(req: Request, res: Response): Promise<void> {
    try {
      const snapshots = collateralRatioMonitorService.getCurrentSnapshots();
      res.json(snapshots);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch collateral ratio snapshots' });
    }
  }

  async getSnapshot(req: Request, res: Response): Promise<void> {
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

  async getPositionRisks(req: Request, res: Response): Promise<void> {
    try {
      const { address } = req.query;
      const positions = collateralRatioMonitorService.getPositionRisks(address as string);
      res.json(positions);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch position risks' });
    }
  }

  async getAlerts(req: Request, res: Response): Promise<void> {
    try {
      const { severity, limit } = req.query;
      const alerts = collateralRatioMonitorService.getAlerts(
        severity as string,
        limit ? parseInt(limit as string) : undefined
      );
      res.json(alerts);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch alerts' });
    }
  }

  async acknowledgeAlert(req: Request, res: Response): Promise<void> {
    try {
      const { alertId } = req.params;
      collateralRatioMonitorService.acknowledgeAlert(alertId);
      res.json({ success: true });
    } catch (error) {
      res.status(500).json({ error: 'Failed to acknowledge alert' });
    }
  }

  async getHistoricalTrends(req: Request, res: Response): Promise<void> {
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

  async getAssetMetrics(req: Request, res: Response): Promise<void> {
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

  async getAllAssetMetrics(req: Request, res: Response): Promise<void> {
    try {
      const metrics = collateralRatioMonitorService.getAllAssetMetrics();
      res.json(metrics);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch asset metrics' });
    }
  }

  async updateThresholds(req: Request, res: Response): Promise<void> {
    try {
      const config = req.body;
      await collateralRatioMonitorService.updateThresholds(config);
      res.json({ success: true, thresholds: collateralRatioMonitorService.getThresholds() });
    } catch (error) {
      res.status(500).json({ error: 'Failed to update thresholds' });
    }
  }

  async getThresholds(req: Request, res: Response): Promise<void> {
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

export const collateralRatioController = new CollateralRatioController();
