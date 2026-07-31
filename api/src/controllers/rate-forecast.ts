import { Request, Response } from 'express';
import { ratePredictorService } from '../services/rate-predictor/ratePredictor.service';

export class RateForecastController {
  /**
   * GET /api/rates/forecast?asset=USDC&horizon=7d
   */
  public getForecast = (req: Request, res: Response): void => {
    try {
      const asset = (req.query.asset as string) || 'USDC';
      const horizon = req.query.horizon as string | undefined;

      const forecast = ratePredictorService.getRateForecast(asset, horizon);
      res.status(200).json(forecast);
    } catch (error: any) {
      res.status(500).json({
        error: error.message || 'Failed to generate rate forecast',
      });
    }
  };

  /**
   * POST /api/rates/retrain
   */
  public retrainModel = (_req: Request, res: Response): void => {
    try {
      ratePredictorService.retrainModel();
      res.status(200).json({
        message: 'Rate prediction model successfully retrained',
        timestamp: new Date().toISOString(),
      });
    } catch (error: any) {
      res.status(500).json({
        error: error.message || 'Failed to retrain model',
      });
    }
  };
}

export const rateForecastController = new RateForecastController();
