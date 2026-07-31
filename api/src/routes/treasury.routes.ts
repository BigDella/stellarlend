import { Router } from 'express';
import { treasuryController } from '../controllers/treasury';

const router = Router();

router.post('/proposal', (req, res) => treasuryController.createProposal(req, res));

router.post('/proposal/:proposalId/approve', (req, res) => treasuryController.approveProposal(req, res));

router.post('/proposal/:proposalId/execute', (req, res) => treasuryController.executeProposal(req, res));

router.get('/proposal/:proposalId', (req, res) => treasuryController.getProposal(req, res));

router.get('/proposals', (req, res) => treasuryController.listProposals(req, res));

router.put('/spending-limit', (req, res) => treasuryController.updateSpendingLimit(req, res));

router.get('/spending-limits', (req, res) => treasuryController.getSpendingLimits(req, res));

router.get('/audit-trail', (req, res) => treasuryController.getAuditTrail(req, res));

export const treasuryRoutes = router;
