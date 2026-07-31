import { Router } from 'express';
import { collateralRatioController } from '../controllers/collateralRatio.controller';

const router = Router();

// Get all current collateral ratio snapshots
router.get('/snapshots', collateralRatioController.getCurrentSnapshots.bind(collateralRatioController));

// Get snapshot for a specific asset
router.get('/snapshots/:asset', collateralRatioController.getSnapshot.bind(collateralRatioController));

// Get position risks (optional address filter)
router.get('/positions', collateralRatioController.getPositionRisks.bind(collateralRatioController));

// Get alerts (optional severity and limit filters)
router.get('/alerts', collateralRatioController.getAlerts.bind(collateralRatioController));

// Acknowledge an alert
router.post('/alerts/:alertId/acknowledge', collateralRatioController.acknowledgeAlert.bind(collateralRatioController));

// Get historical trends for an asset
router.get('/trends/:asset', collateralRatioController.getHistoricalTrends.bind(collateralRatioController));

// Get metrics for a specific asset
router.get('/metrics/:asset', collateralRatioController.getAssetMetrics.bind(collateralRatioController));

// Get metrics for all assets
router.get('/metrics', collateralRatioController.getAllAssetMetrics.bind(collateralRatioController));

// Update risk thresholds
router.put('/thresholds', collateralRatioController.updateThresholds.bind(collateralRatioController));

// Get current risk thresholds
router.get('/thresholds', collateralRatioController.getThresholds.bind(collateralRatioController));

// Get risk-adjusted lending limit for an asset
router.get('/lending-limits/:asset', collateralRatioController.getRiskAdjustedLendingLimit.bind(collateralRatioController));

export default router;
