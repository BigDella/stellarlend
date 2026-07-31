import { Router } from 'express';
import * as controller from '../controllers/liquidationProfitCalculator.controller';

const router: Router = Router();

router.post('/profitability', controller.getProfitability);

export default router;
