import { Router } from 'express';
import { riskController } from '../controllers/risk.controller';
import { stressTestController } from '../controllers/stress-test.controller';

const router = Router();

router.get('/pool-health/:poolId', (req, res) => riskController.getPoolHealth(req, res));

router.get('/liquidation-heatmap', (req, res) => riskController.getLiquidationHeatmap(req, res));

router.get('/oracle-health', (req, res) => riskController.getOracleHealth(req, res));

router.get('/safety-score', (req, res) => riskController.getProtocolSafetyScore(req, res));

router.get('/metric-trends', (req, res) => riskController.getMetricTrends(req, res));

router.get('/alerts', (req, res) => riskController.getAlerts(req, res));

router.put('/alert-config', (req, res) => riskController.updateAlertConfig(req, res));

router.get('/user/:address/risk-profile', (req, res) => riskController.getUserRiskProfile(req, res));

router.get('/dashboard', (req, res) => riskController.getDashboard(req, res));

// Collateral Ratio Monitoring routes
router.get('/collateral-ratio/snapshots', (req, res) => riskController.getCollateralRatioSnapshots(req, res));

router.get('/collateral-ratio/snapshots/:asset', (req, res) => riskController.getCollateralRatioSnapshot(req, res));

router.get('/collateral-ratio/positions', (req, res) => riskController.getCollateralRatioPositionRisks(req, res));

router.get('/collateral-ratio/alerts', (req, res) => riskController.getCollateralRatioAlerts(req, res));

router.post('/collateral-ratio/alerts/:alertId/acknowledge', (req, res) => riskController.acknowledgeCollateralRatioAlert(req, res));

router.get('/collateral-ratio/trends/:asset', (req, res) => riskController.getCollateralRatioHistoricalTrends(req, res));

router.get('/collateral-ratio/metrics/:asset', (req, res) => riskController.getCollateralRatioAssetMetrics(req, res));

router.get('/collateral-ratio/metrics', (req, res) => riskController.getAllCollateralRatioAssetMetrics(req, res));

router.put('/collateral-ratio/thresholds', (req, res) => riskController.updateCollateralRatioThresholds(req, res));

router.get('/collateral-ratio/thresholds', (req, res) => riskController.getCollateralRatioThresholds(req, res));

router.get('/collateral-ratio/lending-limits/:asset', (req, res) => riskController.getRiskAdjustedLendingLimit(req, res));

router.post('/stress/run', (req, res) => stressTestController.runStressTest(req, res));

router.post('/stress/run-all', (req, res) => stressTestController.runAllScenarios(req, res));

router.get('/stress/category/:category', (req, res) => stressTestController.runByCategory(req, res));

router.get('/stress/scenarios', (req, res) => stressTestController.getScenarios(req, res));

router.get('/stress/scenarios/:id', (req, res) => stressTestController.getScenario(req, res));

router.post('/stress/scenarios/custom', (req, res) => stressTestController.createCustomScenario(req, res));

router.post('/stress/scenarios/build', (req, res) => stressTestController.buildCustomScenario(req, res));

router.delete('/stress/scenarios/:id', (req, res) => stressTestController.deleteScenario(req, res));

router.get('/stress/scenarios/:id/export', (req, res) => stressTestController.exportScenario(req, res));

router.post('/stress/scenarios/import', (req, res) => stressTestController.importScenario(req, res));

export default router;
