/**
 * Risk Engine Routes — Issues #450–#453
 *
 * Mounted at /api/risk (legacy) and /api/v1/risk (versioned).
 *
 * #450 Correlation Matrix:
 *   GET  /correlations                            Full N×N matrix
 *   GET  /correlations/:asset1/:asset2            Single pair (all windows + trend)
 *   POST /correlations/position-risk              Position-level correlation risk
 *   PUT  /correlations/config                     Update alert threshold
 *
 * #451 Dynamic Liquidation Thresholds:
 *   GET  /volatility                              All assets, all windows
 *   GET  /volatility/:asset                       Single asset volatility
 *   GET  /ltv/:asset                              Volatility-adjusted LTV
 *   GET  /ltv/:asset/history                      LTV adjustment history
 *   POST /ltv/:asset/governance-override          Governance LTV override
 *   GET  /ltv/:asset/collateral-factor            Effective collateral factor
 *
 * #452 Concentration Risk:
 *   GET  /concentration                           Dashboard (all assets)
 *   GET  /concentration/alerts                    Open concentration alerts
 *   GET  /concentration/:asset                    Single asset metrics
 *   GET  /concentration/:asset/history            Historical snapshots
 *   PUT  /concentration/config                    Update concentration config
 *
 * #453 Risk-Adjusted Collateral Ratio:
 *   GET  /collateral-ratio/:asset                 Compute risk-adjusted ratio
 *   GET  /collateral-ratio/:asset/history         Ratio history
 *   POST /ratio/backtest                          Backtest a proposed ratio
 *   PUT  /ratio/weights                           Update factor weights
 */

import { Router } from 'express';
import { riskEngineController } from '../controllers/riskEngine.controller';

const router = Router();

// ─── #450 Correlation ─────────────────────────────────────────────────────────
router.get(
  '/correlations',
  (req, res) => riskEngineController.getCorrelationMatrix(req, res),
);
// NOTE: /correlations/position-risk must come before /correlations/:asset1/:asset2
// to avoid "position-risk" being captured as asset1.
router.post(
  '/correlations/position-risk',
  (req, res) => riskEngineController.getPositionCorrelationRisk(req, res),
);
router.put(
  '/correlations/config',
  (req, res) => riskEngineController.updateCorrelationConfig(req, res),
);
router.get(
  '/correlations/:asset1/:asset2',
  (req, res) => riskEngineController.getPairCorrelation(req, res),
);

// ─── #451 Volatility & LTV ────────────────────────────────────────────────────
router.get(
  '/volatility',
  (req, res) => riskEngineController.getAllVolatility(req, res),
);
router.get(
  '/volatility/:asset',
  (req, res) => riskEngineController.getAssetVolatility(req, res),
);
// Specific sub-paths before :asset catch-all
router.post(
  '/ltv/:asset/governance-override',
  (req, res) => riskEngineController.applyGovernanceOverride(req, res),
);
router.get(
  '/ltv/:asset/history',
  (req, res) => riskEngineController.getLtvHistory(req, res),
);
router.get(
  '/ltv/:asset/collateral-factor',
  (req, res) => riskEngineController.getCollateralFactor(req, res),
);
router.get(
  '/ltv/:asset',
  (req, res) => riskEngineController.getVolatilityAdjustedLtv(req, res),
);

// ─── #452 Concentration ───────────────────────────────────────────────────────
router.get(
  '/concentration',
  (req, res) => riskEngineController.getConcentrationDashboard(req, res),
);
router.get(
  '/concentration/alerts',
  (req, res) => riskEngineController.getConcentrationAlerts(req, res),
);
router.put(
  '/concentration/config',
  (req, res) => riskEngineController.updateConcentrationConfig(req, res),
);
router.get(
  '/concentration/:asset/history',
  (req, res) => riskEngineController.getConcentrationHistory(req, res),
);
router.get(
  '/concentration/:asset',
  (req, res) => riskEngineController.getAssetConcentration(req, res),
);

// ─── #453 Collateral Ratio ────────────────────────────────────────────────────
router.get(
  '/collateral-ratio/:asset/history',
  (req, res) => riskEngineController.getCollateralRatioHistory(req, res),
);
router.get(
  '/collateral-ratio/:asset',
  (req, res) => riskEngineController.getCollateralRatio(req, res),
);
router.post(
  '/ratio/backtest',
  (req, res) => riskEngineController.backtestRatio(req, res),
);
router.put(
  '/ratio/weights',
  (req, res) => riskEngineController.updateRatioWeights(req, res),
);

export default router;
