import { Router } from 'express';
import * as controller from '../controllers/flashLoanLiquidation.controller';

const router: Router = Router();

router.post('/simulate', controller.simulateCombo);
router.post('/execute', controller.executeCombo);
router.post('/multi-asset', controller.simulateMultiAsset);

export default router;
