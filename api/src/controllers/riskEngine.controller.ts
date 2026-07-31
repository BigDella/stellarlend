/**
 * Risk Engine Controller
 * Handles HTTP requests for issues #450–#453.
 *
 * Routes served:
 *   GET  /api/risk/correlations                               → full matrix (30d default)
 *   GET  /api/risk/correlations/:asset1/:asset2               → single pair (all windows)
 *   POST /api/risk/correlations/position-risk                 → position correlation risk
 *   PUT  /api/risk/correlations/config                        → update alert threshold
 *
 *   GET  /api/risk/volatility                                 → all assets, all windows
 *   GET  /api/risk/volatility/:asset                          → single asset
 *   GET  /api/risk/ltv/:asset                                 → volatility-adjusted LTV
 *   GET  /api/risk/ltv/:asset/history                         → LTV adjustment history
 *   POST /api/risk/ltv/:asset/governance-override             → governance LTV override
 *   GET  /api/risk/ltv/:asset/collateral-factor               → effective collateral factor
 *
 *   GET  /api/risk/concentration                              → all assets dashboard
 *   GET  /api/risk/concentration/:asset                       → single asset metrics
 *   GET  /api/risk/concentration/:asset/history               → historical snapshots
 *   GET  /api/risk/concentration/alerts                       → open alerts
 *   PUT  /api/risk/concentration/config                       → update concentration config
 *
 *   GET  /api/risk/collateral-ratio/:asset                    → risk-adjusted ratio
 *   GET  /api/risk/collateral-ratio/:asset/history            → ratio history
 *   POST /api/risk/ratio/backtest                             → backtest a proposed ratio
 *   PUT  /api/risk/ratio/weights                              → update factor weights
 */

import { Request, Response } from 'express';
import { correlationMatrixService } from '../services/risk-engine/correlationMatrix.service';
import { volatilityOracleService } from '../services/risk-engine/volatilityOracle.service';
import { dynamicLiquidationService } from '../services/risk-engine/dynamicLiquidation.service';
import { concentrationMonitorService } from '../services/risk-engine/concentrationMonitor.service';
import { riskAdjustedRatioService } from '../services/risk-engine/riskAdjustedRatio.service';
import { ValidationError } from '../utils/errors';
import type { CorrelationWindow, VolatilityWindow } from '../types/riskEngine';

// ─── Helper ───────────────────────────────────────────────────────────────────

function parseWindow<T extends number>(
  raw: unknown,
  allowed: T[],
  defaultVal: T,
): T {
  const n = parseInt(String(raw), 10);
  return (allowed.includes(n as T) ? n : defaultVal) as T;
}

// ─── Controller class ─────────────────────────────────────────────────────────

class RiskEngineController {
  // ── #450 Correlation ───────────────────────────────────────────────────────

  /** GET /correlations?window=30 */
  async getCorrelationMatrix(req: Request, res: Response): Promise<void> {
    try {
      const window = parseWindow<CorrelationWindow>(req.query['window'], [30, 60, 90], 30);
      const matrix = await correlationMatrixService.getFullMatrix(window);
      res.json({ success: true, data: matrix });
    } catch (error) {
      this.handleError(res, error, 'Failed to compute correlation matrix');
    }
  }

  /** GET /correlations/:asset1/:asset2?window=30 */
  async getPairCorrelation(req: Request, res: Response): Promise<void> {
    try {
      const { asset1, asset2 } = req.params as { asset1: string; asset2: string };
      if (!asset1 || !asset2) {
        throw new ValidationError('asset1 and asset2 are required path parameters');
      }
      const data = await correlationMatrixService.getPairCorrelationAllWindows(asset1, asset2);
      res.json({ success: true, data });
    } catch (error) {
      this.handleError(res, error, 'Failed to compute pair correlation');
    }
  }

  /** POST /correlations/position-risk  body: { userAddress, collateralAssets, windowDays? } */
  async getPositionCorrelationRisk(req: Request, res: Response): Promise<void> {
    try {
      const { userAddress, collateralAssets, windowDays } = req.body as {
        userAddress?: string;
        collateralAssets?: string[];
        windowDays?: number;
      };

      if (!userAddress) throw new ValidationError('userAddress is required');
      if (!Array.isArray(collateralAssets) || collateralAssets.length === 0) {
        throw new ValidationError('collateralAssets must be a non-empty array');
      }

      const window = parseWindow<CorrelationWindow>(windowDays, [30, 60, 90], 30);
      const risk = await correlationMatrixService.getPositionCorrelationRisk(
        userAddress,
        collateralAssets,
        window,
      );
      res.json({ success: true, data: risk });
    } catch (error) {
      this.handleError(res, error, 'Failed to compute position correlation risk');
    }
  }

  /** PUT /correlations/config  body: { threshold?, windowDays? } */
  async updateCorrelationConfig(req: Request, res: Response): Promise<void> {
    try {
      correlationMatrixService.updateAlertConfig(req.body as Record<string, unknown>);
      res.json({ success: true, data: correlationMatrixService.getAlertConfig() });
    } catch (error) {
      this.handleError(res, error, 'Failed to update correlation config');
    }
  }

  // ── #451 Volatility & Dynamic LTV ─────────────────────────────────────────

  /** GET /volatility */
  async getAllVolatility(_req: Request, res: Response): Promise<void> {
    try {
      const data = await volatilityOracleService.getAllVolatilities();
      res.json({ success: true, data });
    } catch (error) {
      this.handleError(res, error, 'Failed to fetch volatility data');
    }
  }

  /** GET /volatility/:asset?window=5 */
  async getAssetVolatility(req: Request, res: Response): Promise<void> {
    try {
      const asset = req.params['asset'] as string;
      if (!asset) throw new ValidationError('asset is required');
      const window = parseWindow<VolatilityWindow>(req.query['window'], [5, 20], 20);
      const data = await volatilityOracleService.getVolatility(asset, window);
      res.json({ success: true, data });
    } catch (error) {
      this.handleError(res, error, 'Failed to fetch asset volatility');
    }
  }

  /** GET /ltv/:asset */
  async getVolatilityAdjustedLtv(req: Request, res: Response): Promise<void> {
    try {
      const asset = req.params['asset'] as string;
      if (!asset) throw new ValidationError('asset is required');
      const data = await dynamicLiquidationService.getVolatilityAdjustedLtv(asset);
      res.json({ success: true, data });
    } catch (error) {
      this.handleError(res, error, 'Failed to compute volatility-adjusted LTV');
    }
  }

  /** GET /ltv/:asset/history */
  async getLtvHistory(req: Request, res: Response): Promise<void> {
    try {
      const asset = req.params['asset'] as string;
      if (!asset) throw new ValidationError('asset is required');
      const data = dynamicLiquidationService.getAdjustmentHistory(asset);
      res.json({ success: true, data });
    } catch (error) {
      this.handleError(res, error, 'Failed to fetch LTV history');
    }
  }

  /** POST /ltv/:asset/governance-override  body: { overrideLtv } */
  async applyGovernanceOverride(req: Request, res: Response): Promise<void> {
    try {
      const asset = req.params['asset'] as string;
      if (!asset) throw new ValidationError('asset is required');
      const { overrideLtv } = req.body as { overrideLtv?: number };
      if (typeof overrideLtv !== 'number') throw new ValidationError('overrideLtv (number, bps) is required');
      const data = await dynamicLiquidationService.applyGovernanceOverride(asset, overrideLtv);
      res.json({ success: true, data });
    } catch (error) {
      this.handleError(res, error, 'Failed to apply governance LTV override');
    }
  }

  /** GET /ltv/:asset/collateral-factor */
  async getCollateralFactor(req: Request, res: Response): Promise<void> {
    try {
      const asset = req.params['asset'] as string;
      if (!asset) throw new ValidationError('asset is required');
      const factor = await dynamicLiquidationService.getCollateralFactor(asset);
      res.json({ success: true, data: { asset, collateralFactor: factor } });
    } catch (error) {
      this.handleError(res, error, 'Failed to fetch collateral factor');
    }
  }

  // ── #452 Concentration ────────────────────────────────────────────────────

  /** GET /concentration */
  async getConcentrationDashboard(_req: Request, res: Response): Promise<void> {
    try {
      const data = await concentrationMonitorService.getDashboard();
      res.json({ success: true, data });
    } catch (error) {
      this.handleError(res, error, 'Failed to fetch concentration dashboard');
    }
  }

  /** GET /concentration/:asset */
  async getAssetConcentration(req: Request, res: Response): Promise<void> {
    try {
      const asset = req.params['asset'] as string;
      if (!asset) throw new ValidationError('asset is required');
      const data = await concentrationMonitorService.getConcentrationMetrics(asset);
      res.json({ success: true, data });
    } catch (error) {
      this.handleError(res, error, 'Failed to fetch concentration metrics');
    }
  }

  /** GET /concentration/:asset/history */
  async getConcentrationHistory(req: Request, res: Response): Promise<void> {
    try {
      const asset = req.params['asset'] as string;
      if (!asset) throw new ValidationError('asset is required');
      const data = concentrationMonitorService.getHistory(asset);
      res.json({ success: true, data });
    } catch (error) {
      this.handleError(res, error, 'Failed to fetch concentration history');
    }
  }

  /** GET /concentration/alerts?asset=XLM */
  async getConcentrationAlerts(req: Request, res: Response): Promise<void> {
    try {
      const asset = req.query['asset'] as string | undefined;
      const data = concentrationMonitorService.getActiveAlerts(asset);
      res.json({ success: true, data });
    } catch (error) {
      this.handleError(res, error, 'Failed to fetch concentration alerts');
    }
  }

  /** PUT /concentration/config  body: ConcentrationConfig */
  async updateConcentrationConfig(req: Request, res: Response): Promise<void> {
    try {
      concentrationMonitorService.updateConfig(req.body as Record<string, unknown>);
      res.json({ success: true, data: concentrationMonitorService.getConfig() });
    } catch (error) {
      this.handleError(res, error, 'Failed to update concentration config');
    }
  }

  // ── #453 Collateral Ratio ─────────────────────────────────────────────────

  /** GET /collateral-ratio/:asset */
  async getCollateralRatio(req: Request, res: Response): Promise<void> {
    try {
      const asset = req.params['asset'] as string;
      if (!asset) throw new ValidationError('asset is required');
      const data = await riskAdjustedRatioService.getCollateralRatio(asset);
      res.json({ success: true, data });
    } catch (error) {
      this.handleError(res, error, 'Failed to compute collateral ratio');
    }
  }

  /** GET /collateral-ratio/:asset/history */
  async getCollateralRatioHistory(req: Request, res: Response): Promise<void> {
    try {
      const asset = req.params['asset'] as string;
      if (!asset) throw new ValidationError('asset is required');
      const data = riskAdjustedRatioService.getHistory(asset);
      res.json({ success: true, data });
    } catch (error) {
      this.handleError(res, error, 'Failed to fetch collateral ratio history');
    }
  }

  /** POST /ratio/backtest  body: BacktestRequest */
  async backtestRatio(req: Request, res: Response): Promise<void> {
    try {
      const { asset, startDate, endDate, proposedRatio } = req.body as {
        asset?: string;
        startDate?: string;
        endDate?: string;
        proposedRatio?: number;
      };

      if (!asset) throw new ValidationError('asset is required');
      if (!startDate) throw new ValidationError('startDate is required');
      if (!endDate) throw new ValidationError('endDate is required');
      if (typeof proposedRatio !== 'number') throw new ValidationError('proposedRatio (bps) is required');

      const data = await riskAdjustedRatioService.backtest({ asset, startDate, endDate, proposedRatio });
      res.json({ success: true, data });
    } catch (error) {
      this.handleError(res, error, 'Backtest failed');
    }
  }

  /** PUT /ratio/weights  body: RatioFactorWeights */
  async updateRatioWeights(req: Request, res: Response): Promise<void> {
    try {
      riskAdjustedRatioService.updateWeights(req.body as Record<string, unknown>);
      res.json({ success: true, data: riskAdjustedRatioService.getWeights() });
    } catch (error) {
      this.handleError(res, error, 'Failed to update ratio weights');
    }
  }

  // ── Error handler ─────────────────────────────────────────────────────────

  private handleError(res: Response, error: unknown, fallback: string): void {
    if (error instanceof ValidationError) {
      res.status(400).json({ success: false, error: { code: 'VALIDATION_ERROR', message: error.message } });
      return;
    }
    const message = error instanceof Error ? error.message : fallback;
    res.status(500).json({ success: false, error: { code: 'INTERNAL_SERVER_ERROR', message } });
  }
}

export const riskEngineController = new RiskEngineController();
