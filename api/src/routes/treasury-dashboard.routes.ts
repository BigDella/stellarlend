import { Router } from 'express';
import { treasuryDashboardController } from '../controllers/treasury-dashboard.controller';

const router = Router();

router.get('/overview', (req, res) => treasuryDashboardController.getOverview(req, res));

router.post('/asset-balance', (req, res) => treasuryDashboardController.setAssetBalance(req, res));

router.post('/revenue', (req, res) => treasuryDashboardController.recordRevenue(req, res));

router.post('/expense', (req, res) => treasuryDashboardController.recordExpense(req, res));

router.get('/revenue-tracking', (req, res) => treasuryDashboardController.getRevenueTracking(req, res));

router.get('/expense-tracking', (req, res) => treasuryDashboardController.getExpenseTracking(req, res));

router.get('/cashflow-forecasts', (req, res) => treasuryDashboardController.getCashFlowForecasts(req, res));

router.get('/scenario-analysis', (req, res) => treasuryDashboardController.getScenarioAnalysis(req, res));

router.get('/burn-rate-runway', (req, res) => treasuryDashboardController.getBurnRateAndRunway(req, res));

router.get('/cashflow-history', (req, res) => treasuryDashboardController.getCashFlowHistory(req, res));

export const treasuryDashboardRoutes = router;
