import { Router } from 'express';
import { rateForecastController } from '../controllers/rate-forecast';

const router = Router();

router.get('/forecast', rateForecastController.getForecast);
router.post('/retrain', rateForecastController.retrainModel);

export default router;
