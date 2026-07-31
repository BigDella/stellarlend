import { Router } from 'express';
import lendingV1Routes from './v1/lending.routes';
import healthV1Routes from './v1/health.routes';
import protocolV1Routes from './v1/protocol.routes';

const router = Router();

// Version 1 API routes
router.use('/v1/lending', lendingV1Routes);
router.use('/v1/health', healthV1Routes);
router.use('/v1/protocol', protocolV1Routes);

export default router;