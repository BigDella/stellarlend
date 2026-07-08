import { Router } from 'express';
import { complianceController } from '../../../controllers/compliance.controller';

const router = Router();

router.post('/sanctions', (req, res) => complianceController.addSanction(req, res));
router.delete('/sanctions', (req, res) => complianceController.removeSanction(req, res));
router.get('/sanctions/check', (req, res) => complianceController.checkSanctioned(req, res));

router.post('/kyc', (req, res) => complianceController.setKyc(req, res));
router.delete('/kyc', (req, res) => complianceController.revokeKyc(req, res));
router.get('/kyc/check', (req, res) => complianceController.checkKyc(req, res));

router.post('/transaction/check', (req, res) => complianceController.checkTransaction(req, res));

router.post('/sar', (req, res) => complianceController.fileSar(req, res));
router.get('/sar/:sarId', (req, res) => complianceController.getSar(req, res));
router.get('/sar', (req, res) => complianceController.listSars(req, res));

router.get('/report', (req, res) => complianceController.getReport(req, res));
router.get('/audit-trail', (req, res) => complianceController.getAuditTrail(req, res));

export default router;
