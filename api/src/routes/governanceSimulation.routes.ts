import { Router } from 'express';
import * as controller from '../controllers/governanceSimulation.controller';

const router: Router = Router();

router.post('/', controller.simulate);
router.get('/share/:shareId', controller.getShared);

export default router;
