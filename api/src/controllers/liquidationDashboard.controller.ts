import { Request, Response } from 'express';
import { liquidationMonitorService } from '../services/liquidation-monitor/liquidationMonitor.service';

export class LiquidationDashboardController {
  public getPositions = (_req: Request, res: Response): void => {
    try {
      const positions = liquidationMonitorService.getAllPositions();
      res.status(200).json({ positions, total: positions.length });
    } catch (error: any) {
      res.status(500).json({ error: error.message || 'Failed to fetch positions' });
    }
  };

  public getPositionDetail = (req: Request, res: Response): void => {
    try {
      const address = req.params.address;
      if (!address) {
        res.status(400).json({ error: 'Address parameter is required' });
        return;
      }
      const position = liquidationMonitorService.getPosition(address);
      if (!position) {
        res.status(404).json({ error: 'Position not found' });
        return;
      }
      res.status(200).json(position);
    } catch (error: any) {
      res.status(500).json({ error: error.message || 'Failed to fetch position' });
    }
  };

  public getAlerts = (_req: Request, res: Response): void => {
    try {
      const alerts = liquidationMonitorService.getAlerts();
      res.status(200).json({ alerts, total: alerts.length });
    } catch (error: any) {
      res.status(500).json({ error: error.message || 'Failed to fetch alerts' });
    }
  };

  public setThreshold = (req: Request, res: Response): void => {
    try {
      const { address, dangerHf, warningHf } = req.body;
      if (!address) {
        res.status(400).json({ error: 'address is required' });
        return;
      }
      liquidationMonitorService.setThreshold({
        address,
        dangerHf: dangerHf ?? 1.1,
        warningHf: warningHf ?? 1.3,
        enabled: true,
      });
      res.status(200).json({ message: 'Threshold updated', address });
    } catch (error: any) {
      res.status(500).json({ error: error.message || 'Failed to set threshold' });
    }
  };
}

export const liquidationDashboardController = new LiquidationDashboardController();
