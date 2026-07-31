import { Router } from 'express';
import { protocolOwnedLiquidityController } from '../controllers/protocol-owned-liquidity.controller';

const router = Router();

router.post('/pool', (req, res) => protocolOwnedLiquidityController.initializePool(req, res));

router.post('/deployment-proposal', (req, res) => protocolOwnedLiquidityController.createDeploymentProposal(req, res));

router.post('/deployment-proposal/:proposalId/approve', (req, res) =>
  protocolOwnedLiquidityController.approveDeploymentProposal(req, res)
);

router.post('/deployment-proposal/:proposalId/execute', (req, res) =>
  protocolOwnedLiquidityController.executeDeploymentProposal(req, res)
);

router.post('/deployment-proposal/:proposalId/reject', (req, res) =>
  protocolOwnedLiquidityController.rejectDeploymentProposal(req, res)
);

router.post('/withdrawal-proposal', (req, res) => protocolOwnedLiquidityController.createWithdrawalProposal(req, res));

router.post('/withdrawal-proposal/:proposalId/approve', (req, res) =>
  protocolOwnedLiquidityController.approveWithdrawalProposal(req, res)
);

router.post('/withdrawal-proposal/:proposalId/execute', (req, res) =>
  protocolOwnedLiquidityController.executeWithdrawalProposal(req, res)
);

router.post('/withdrawal-proposal/:proposalId/reject', (req, res) =>
  protocolOwnedLiquidityController.rejectWithdrawalProposal(req, res)
);

router.post('/rebalance-proposal', (req, res) => protocolOwnedLiquidityController.createRebalanceProposal(req, res));

router.post('/rebalance-proposal/:proposalId/approve', (req, res) =>
  protocolOwnedLiquidityController.approveRebalanceProposal(req, res)
);

router.post('/rebalance-proposal/:proposalId/execute', (req, res) =>
  protocolOwnedLiquidityController.executeRebalanceProposal(req, res)
);

router.post('/rebalance-proposal/:proposalId/reject', (req, res) =>
  protocolOwnedLiquidityController.rejectRebalanceProposal(req, res)
);

router.get('/positions', (req, res) => protocolOwnedLiquidityController.getPositions(req, res));

router.post('/harvest', (req, res) => protocolOwnedLiquidityController.harvestYield(req, res));

router.get('/dashboard', (req, res) => protocolOwnedLiquidityController.getDashboard(req, res));

router.get('/history', (req, res) => protocolOwnedLiquidityController.getHistory(req, res));

router.get('/deployment-proposals', (req, res) => protocolOwnedLiquidityController.getDeploymentProposals(req, res));

router.get('/withdrawal-proposals', (req, res) => protocolOwnedLiquidityController.getWithdrawalProposals(req, res));

router.get('/rebalance-proposals', (req, res) => protocolOwnedLiquidityController.getRebalanceProposals(req, res));

export const protocolOwnedLiquidityRoutes = router;
