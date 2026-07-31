import { ratePredictorService } from '../services/rate-predictor/ratePredictor.service';
import { featureEngine } from '../services/rate-predictor/featureEngine';
import { backtestingFramework } from '../services/rate-predictor/backtester';
import { historicalRateStore } from '../services/rate-predictor/historicalRateStore';

describe('RatePredictorService & Time-Series Engine', () => {
  describe('HistoricalRateStore', () => {
    it('should retrieve historical snapshots for an asset', () => {
      const history = historicalRateStore.getHistory('USDC', 30);
      expect(history.length).toBeGreaterThan(0);
      expect(history[0].asset).toBe('USDC');
    });
  });

  describe('FeatureEngine', () => {
    it('should extract engineered features from historical snapshots', () => {
      const history = historicalRateStore.getHistory('USDC', 90);
      const features = featureEngine.extractFeatures(history);

      expect(features.asset).toBe('USDC');
      expect(features.currentUtilizationBps).toBeGreaterThan(0);
      expect(features.volatility30d).toBeGreaterThanOrEqual(0);
    });
  });

  describe('BacktestingFramework', () => {
    it('should calculate MAPE, RMSE, MAE and verify MAPE < 20%', () => {
      const history = historicalRateStore.getHistory('USDC', 30);
      const predicted = history.map((h) => h.borrowRateBps + 5); // 5 BPS slight error

      const result = backtestingFramework.evaluatePredictions(history, predicted);
      expect(result.mape).toBeLessThan(20);
      expect(result.accuracyPassed).toBe(true);
    });
  });

  describe('RatePredictorService', () => {
    it('should generate rate forecast for 1d, 7d, 30d horizons', () => {
      const forecast = ratePredictorService.getRateForecast('USDC');

      expect(forecast.asset).toBe('USDC');
      expect(forecast.forecasts).toHaveLength(3);

      const [f1d, f7d, f30d] = forecast.forecasts;
      expect(f1d.horizon).toBe('1d');
      expect(f7d.horizon).toBe('7d');
      expect(f30d.horizon).toBe('30d');

      // Check 95% confidence intervals
      expect(f7d.confidenceInterval.lowerBps).toBeLessThanOrEqual(f7d.predictedBorrowRateBps);
      expect(f7d.confidenceInterval.upperBps).toBeGreaterThanOrEqual(f7d.predictedBorrowRateBps);
      expect(f7d.confidenceInterval.confidenceLevel).toBe(0.95);
    });

    it('should filter forecast by single horizon when specified', () => {
      const forecast = ratePredictorService.getRateForecast('USDC', '7d');
      expect(forecast.forecasts).toHaveLength(1);
      expect(forecast.forecasts[0].horizon).toBe('7d');
    });

    it('should retrain model without throwing errors', () => {
      expect(() => ratePredictorService.retrainModel()).not.toThrow();
    });
  });
});
