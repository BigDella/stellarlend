import { Request, Response } from 'express';
import { yieldCurveService } from '../services/yieldCurve.service';

export class YieldCurveController {
  public predictYieldCurve = (req: Request, res: Response): void => {
    try {
      const { config, stepBps } = req.body || {};
      const step = typeof stepBps === 'number' && stepBps > 0 ? stepBps : 500;
      const result = yieldCurveService.predictYieldCurve(config, step);
      res.status(200).json(result);
    } catch (error: any) {
      res.status(400).json({ error: error.message || 'Failed to generate yield curve prediction' });
    }
  };

  public optimizeRates = (req: Request, res: Response): void => {
    try {
      const { currentConfig, targetUtilizationBps, maxAcceptableRiskScore } = req.body || {};
      if (typeof targetUtilizationBps !== 'number') {
        res.status(400).json({ error: 'targetUtilizationBps is required and must be a number' });
        return;
      }
      const result = yieldCurveService.optimizeRateParameters({
        currentConfig: currentConfig || {},
        targetUtilizationBps,
        maxAcceptableRiskScore,
      });
      res.status(200).json(result);
    } catch (error: any) {
      res.status(400).json({ error: error.message || 'Failed to optimize rate parameters' });
    }
  };

  public stressTest = (req: Request, res: Response): void => {
    try {
      const { config, baseUtilizationBps, shocksBps } = req.body || {};
      if (typeof baseUtilizationBps !== 'number') {
        res.status(400).json({ error: 'baseUtilizationBps is required and must be a number' });
        return;
      }
      const shocks = Array.isArray(shocksBps) ? shocksBps : [-2000, -1000, 1000, 2000, 3000];
      const result = yieldCurveService.runStressTest({
        config: config || {},
        baseUtilizationBps,
        shocksBps: shocks,
      });
      res.status(200).json(result);
    } catch (error: any) {
      res.status(400).json({ error: error.message || 'Failed to execute yield curve stress test' });
    }
  };

  public getConfig = (_req: Request, res: Response): void => {
    try {
      const defaultPrediction = yieldCurveService.predictYieldCurve();
      res.status(200).json(defaultPrediction.config);
    } catch (error: any) {
      res.status(500).json({ error: error.message || 'Failed to fetch default yield curve config' });
    }
  };
}

export const yieldCurveController = new YieldCurveController();
