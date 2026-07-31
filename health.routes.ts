import { Router } from 'express';
import {
  getHealthStatus,
  getLivenessStatus,
  getReadinessStatus,
} from '../../controllers/lending.controller';

const router = Router();

// Health checks
router.get('/', getHealthStatus);
router.get('/live', getLivenessStatus);
router.get('/ready', getReadinessStatus);

export default router;