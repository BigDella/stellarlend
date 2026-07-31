import { Request, Response, NextFunction } from 'express';
import { riskScoringService } from '../services/riskScoring.service';

export class RiskScoringController {
  async getPoolScore(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const { pool } = req.params;
      if (!pool) {
        res.status(400).json({ error: 'Pool parameter is required' });
        return;
      }
      const score = await riskScoringService.getPoolRiskScore(pool);
      if (!score) {
        res.status(404).json({ error: 'Pool not found' });
        return;
      }
      res.status(200).json({ success: true, score });
    } catch (err) {
      next(err);
    }
  }

  async getAllScores(_req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const scores = await riskScoringService.getAllPoolScores();
      res.status(200).json({ success: true, scores, total: scores.length });
    } catch (err) {
      next(err);
    }
  }

  async getPoolProfile(req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const { pool } = req.params;
      if (!pool) {
        res.status(400).json({ error: 'Pool parameter is required' });
        return;
      }
      const profile = await riskScoringService.getPoolRiskProfile(pool);
      if (!profile) {
        res.status(404).json({ error: 'Pool not found' });
        return;
      }
      res.status(200).json({ success: true, profile });
    } catch (err) {
      next(err);
    }
  }

  async getDistribution(_req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const distribution = await riskScoringService.getScoreDistribution();
      res.status(200).json({ success: true, distribution });
    } catch (err) {
      next(err);
    }
  }

  async getWeights(_req: Request, res: Response, next: NextFunction): Promise<void> {
    try {
      const weights = await riskScoringService.getDefaultWeights();
      res.status(200).json({ success: true, weights });
    } catch (err) {
      next(err);
    }
  }
}

export const riskScoringController = new RiskScoringController();