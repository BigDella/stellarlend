import { Request, Response, NextFunction } from 'express';
import { autoCompoundVaultService } from '../services/autoCompoundVault.service';

export class AutoCompoundVaultController {
  async getConfig(_req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const config = await autoCompoundVaultService.getConfig();
      res.status(200).json({ success: true, config });
    } catch (err) {
      next(err);
    }
  }

  async getSnapshot(_req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const snapshot = await autoCompoundVaultService.getSnapshot();
      res.status(200).json({ success: true, snapshot });
    } catch (err) {
      next(err);
    }
  }

  async getUserPosition(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const { address } = req.params;
      if (!address) {
        res.status(400).json({ error: 'Address parameter is required' });
        return;
      }
      const position = await autoCompoundVaultService.getUserPosition(address);
      if (!position) {
        res.status(404).json({ error: 'Position not found' });
        return;
      }
      res.status(200).json({ success: true, position });
    } catch (err) {
      next(err);
    }
  }

  async computeApyBoost(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const { interval } = req.query;
      const result = await autoCompoundVaultService.computeApyBoost(
        (interval as string) || 'daily'
      );
      res.status(200).json({ success: true, ...result });
    } catch (err) {
      next(err);
    }
  }
}

export const autoCompoundVaultController = new AutoCompoundVaultController();