import { Router } from 'express';
import { nonceRouter } from '../controllers/nonce.controller';

const router: Router = Router();

router.use('/', nonceRouter);

export default router;
