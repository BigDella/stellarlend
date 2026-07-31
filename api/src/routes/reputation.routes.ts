import { Router } from 'express';
import * as reputationController from '../controllers/reputation.controller';

const router: Router = Router();

router.get('/tiers', reputationController.getReputationTiers);
router.get('/leaderboard', reputationController.getLeaderboard);
router.get('/:address', reputationController.getReputation);

export default router;
