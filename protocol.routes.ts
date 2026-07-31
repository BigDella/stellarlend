import { Router } from 'express';
import {
  getProtocolStats,
} from '../../controllers/lending.controller';

const router = Router();

// Protocol stats
router.get('/stats', getProtocolStats);

export default router;