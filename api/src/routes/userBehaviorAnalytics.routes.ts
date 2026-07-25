import { Router } from 'express';
import * as controller from '../controllers/userBehaviorAnalytics.controller';

const router: Router = Router();

router.post('/events', controller.postEvent);
router.get('/funnel', controller.getFunnel);
router.get('/conversion', controller.getConversionRates);
router.get('/cohorts', controller.getCohorts);
router.get('/power-users', controller.getPowerUsers);
router.get('/churn-risk', controller.getChurnRisk);
router.get('/ab-tests', controller.getAbTestMetrics);

export default router;
