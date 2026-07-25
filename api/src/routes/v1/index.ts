/**
 * V1 API Router
 *
 * Aggregates all v1 domain routers under versioned prefixes.
 * Route structure:
 *   /api/v1/lending     -> lending domain
 *   /api/v1/protocol    -> protocol domain
 *   /api/v1/governance  -> governance domain
 *   /api/v1/account     -> user account domain
 *   /api/v1/system      -> infrastructure domain
 *   /api/v1/security    -> security/privacy domain
 *   /api/v1/social      -> social trading domain
 *   /api/v1/notifications -> notification domain
 *   /api/v1/disputes    -> dispute resolution domain
 *   /api/v1/credit      -> credit delegation domain
 *   /api/v1/compliance  -> compliance/regulatory domain
 */

import { Router } from 'express';
import lendingV1Routes from './lending';
import protocolV1Routes from './protocol';
import governanceV1Routes from './governance';
import oracleV1Routes from './oracle';
import accountV1Routes from './account';
import systemV1Routes from './system';
import securityV1Routes from './security';
import socialRoutes from '../social.routes';
import notificationRoutes from '../notification.routes';
import disputeRoutes from '../dispute.routes';
import creditRoutes from '../credit.routes';
import complianceV1Routes from './compliance';
import riskV1Routes from './risk';

const router = Router();

router.use('/lending', lendingV1Routes);
router.use('/protocol', protocolV1Routes);
router.use('/governance', governanceV1Routes);
router.use('/oracle', oracleV1Routes);
router.use('/account', accountV1Routes);
router.use('/system', systemV1Routes);
router.use('/security', securityV1Routes);
router.use('/social', socialRoutes);
router.use('/notifications', notificationRoutes);
router.use('/disputes', disputeRoutes);
router.use('/credit', creditRoutes);
router.use('/compliance', complianceV1Routes);
router.use('/risk', riskV1Routes);

export default router;
