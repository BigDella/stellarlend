import { Router } from 'express';
import { treasuryRebalancerController } from '../controllers/treasury-rebalancer.controller';

const router = Router();

router.post('/target-allocations', (req, res) => treasuryRebalancerController.setTargetAllocations(req, res));

router.post('/asset-prices', (req, res) => treasuryRebalancerController.setAssetPrices(req, res));

router.post('/current-allocations', (req, res) => treasuryRebalancerController.setCurrentAllocations(req, res));

router.get('/allocation', (req, res) => treasuryRebalancerController.getCurrentAllocation(req, res));

router.get('/rebalance-trigger', (req, res) => treasuryRebalancerController.checkRebalanceTrigger(req, res));

router.get('/simulate', (req, res) => treasuryRebalancerController.simulateRebalance(req, res));

router.post('/execute', (req, res) => treasuryRebalancerController.executeRebalance(req, res));

router.get('/history', (req, res) => treasuryRebalancerController.getRebalanceHistory(req, res));

router.post('/governance-proposal', (req, res) => treasuryRebalancerController.createGovernanceProposal(req, res));

router.get('/governance-proposals', (req, res) => treasuryRebalancerController.getGovernanceProposals(req, res));

router.post('/governance-proposal/:proposalId/approve', (req, res) =>
  treasuryRebalancerController.approveGovernanceProposal(req, res)
);

router.post('/governance-proposal/:proposalId/reject', (req, res) =>
  treasuryRebalancerController.rejectGovernanceProposal(req, res)
);

router.post('/pause', (req, res) => treasuryRebalancerController.pauseRebalancer(req, res));

router.post('/resume', (req, res) => treasuryRebalancerController.resumeRebalancer(req, res));

router.get('/status', (req, res) => treasuryRebalancerController.getRebalancerStatus(req, res));

export const treasuryRebalancerRoutes = router;
