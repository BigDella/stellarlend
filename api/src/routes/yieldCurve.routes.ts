import { Router } from 'express';
import { yieldCurveController } from '../controllers/yieldCurve.controller';

const router = Router();

router.post('/predict', yieldCurveController.predictYieldCurve);
router.post('/optimize', yieldCurveController.optimizeRates);
router.post('/stress-test', yieldCurveController.stressTest);
router.get('/config', yieldCurveController.getConfig);

export default router;
