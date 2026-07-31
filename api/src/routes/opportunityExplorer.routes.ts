import { Router } from 'express';
import { opportunityExplorerController } from '../controllers/opportunityExplorer.controller';

const router = Router();

router.get('/opportunities', opportunityExplorerController.getOpportunities);
router.get('/history', opportunityExplorerController.getHistoricalLiquidations);
router.get('/gas-estimate', opportunityExplorerController.getGasEstimate);

export default router;
