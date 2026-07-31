import { Router } from 'express';
import * as controller from '../controllers/poolPerformance.controller';

const router: Router = Router();

router.get('/snapshots/:poolAddress', controller.getPoolSnapshots);
router.get('/metrics/:poolAddress', controller.getPoolMetrics);
router.get('/compare', controller.comparePools);
router.get('/export/:poolAddress', controller.exportPerformanceData);
router.get('/summary', controller.getPerformanceSummary);

export default router;
