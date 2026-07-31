import { Router } from 'express';
import * as rebalancingController from '../controllers/rebalancing.controller';

const router: Router = Router();

// POST /api/rebalancing/configure
router.post('/configure', rebalancingController.configureRebalancing);

// POST /api/rebalancing/execute
router.post('/execute', rebalancingController.executeRebalancing);

// GET /api/rebalancing/config/:userAddress
router.get('/config/:userAddress', rebalancingController.getRebalancingConfig);

// POST /api/rebalancing/emergency-stop
router.post('/emergency-stop', rebalancingController.setRebalancingEmergencyStop);

// POST /api/rebalancing/pause
router.post('/pause', rebalancingController.setRebalancingPause);

export default router;
