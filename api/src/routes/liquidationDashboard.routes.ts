import { Router } from 'express';
import { liquidationDashboardController } from '../controllers/liquidationDashboard.controller';

const router = Router();

router.get('/positions', liquidationDashboardController.getPositions);
router.get('/positions/:address', liquidationDashboardController.getPositionDetail);
router.get('/alerts', liquidationDashboardController.getAlerts);
router.post('/threshold', liquidationDashboardController.setThreshold);

export default router;
