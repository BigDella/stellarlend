import { Router } from 'express';
import * as feesController from '../controllers/fees.controller';

const router: Router = Router();

router.get('/current', feesController.getCurrentFees);
router.get('/history', feesController.getFeeHistory);
router.post('/compute', feesController.computeFee);

export default router;
