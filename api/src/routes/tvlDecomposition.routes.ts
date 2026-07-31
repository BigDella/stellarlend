import { Router } from 'express';
import * as controller from '../controllers/tvlDecomposition.controller';

const router: Router = Router();

router.get('/breakdown', controller.getBreakdown);
router.get('/attribution', controller.getAttribution);
router.get('/history', controller.getHistory);
router.get('/competitors', controller.getCompetitorComparison);

export default router;
