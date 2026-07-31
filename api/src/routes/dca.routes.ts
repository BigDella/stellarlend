import { Router } from 'express';
import * as dcaController from '../controllers/dca.controller';

const router: Router = Router();

router.post('/plan', dcaController.createPlan);
router.get('/plans/:userAddress', dcaController.getPlans);
router.get('/plan/:planId', dcaController.getPlan);
router.post('/pause/:planId', dcaController.pausePlan);
router.post('/resume/:planId', dcaController.resumePlan);
router.post('/cancel/:planId', dcaController.cancelPlan);
router.get('/history/:planId', dcaController.getHistory);

export default router;
