/**
 * Protocol Domain Routes (v1)
 *
 * Aggregates all protocol-level routes under /v1/protocol:
 * - Protocol statistics and status
 * - Pause/resume controls
 * - Role-based access management
 * - Audit logging and integrity verification
 */

import { Router } from 'express';
import protocolRoutes from '../../protocol.routes';

const router = Router();

// Protocol: /v1/protocol/*
router.use('/', protocolRoutes);

export default router;
