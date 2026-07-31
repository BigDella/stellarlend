/**
 * Protocol Health Score Controller — Issue #484
 */

import { Request, Response } from 'express';
import { protocolHealthScoreService } from '../services/protocol-health/healthScore.service';
import { ValidationError } from '../utils/errors';
import logger from '../utils/logger';

export class ProtocolHealthController {
  /** GET /api/protocol/health-score */
  async getHealthScore(_req: Request, res: Response): Promise<void> {
    try {
      const score = await protocolHealthScoreService.getHealthScore();
      res.json({ success: true, data: score });
    } catch (error) {
      this.handleError(res, error, 'Failed to compute protocol health score');
    }
  }

  /** GET /api/protocol/health-score/history?limit=30 */
  async getHistory(req: Request, res: Response): Promise<void> {
    try {
      const limit = req.query.limit ? Number(req.query.limit) : undefined;
      const history = protocolHealthScoreService.getHistory(limit);
      res.json({ success: true, data: history });
    } catch (error) {
      this.handleError(res, error, 'Failed to fetch protocol health score history');
    }
  }

  /** GET /api/protocol/health-score/weights */
  async getWeights(_req: Request, res: Response): Promise<void> {
    try {
      res.json({ success: true, data: protocolHealthScoreService.getWeights() });
    } catch (error) {
      this.handleError(res, error, 'Failed to fetch health score weights');
    }
  }

  /** PUT /api/protocol/health-score/weights (governance/admin-controlled) */
  async updateWeights(req: Request, res: Response): Promise<void> {
    try {
      const partial = req.body ?? {};
      if (typeof partial !== 'object' || Array.isArray(partial)) {
        throw new ValidationError('Request body must be an object of component weights');
      }
      const updated = protocolHealthScoreService.updateWeights(partial);
      res.json({ success: true, data: updated });
    } catch (error) {
      this.handleError(res, error, 'Failed to update health score weights');
    }
  }

  /** GET /api/protocol/health-score/alerts */
  async getAlerts(_req: Request, res: Response): Promise<void> {
    try {
      const alerts = await protocolHealthScoreService.getAlerts();
      res.json({ success: true, data: alerts });
    } catch (error) {
      this.handleError(res, error, 'Failed to fetch health score alerts');
    }
  }

  /** PUT /api/protocol/health-score/alert-threshold (governance/admin-controlled) */
  async updateAlertThreshold(req: Request, res: Response): Promise<void> {
    try {
      const { threshold } = req.body ?? {};
      if (threshold === undefined || threshold === null || Number.isNaN(Number(threshold))) {
        throw new ValidationError('threshold is required and must be a number');
      }
      protocolHealthScoreService.setAlertThreshold(Number(threshold));
      res.json({ success: true, data: { threshold: protocolHealthScoreService.getAlertThreshold() } });
    } catch (error) {
      this.handleError(res, error, 'Failed to update health score alert threshold');
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

export const protocolHealthController = new ProtocolHealthController();
