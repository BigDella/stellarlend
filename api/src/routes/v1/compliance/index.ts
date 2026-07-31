import { Router } from 'express';
import { complianceController } from '../../../controllers/compliance.controller';

const router = Router();

// ─── Sanctions ──────────────────────────────────────────────────────────────
router.post('/sanctions', (req, res) => complianceController.addSanction(req, res));
router.delete('/sanctions', (req, res) => complianceController.removeSanction(req, res));
router.get('/sanctions/check', (req, res) => complianceController.checkSanctioned(req, res));

// ─── KYC ────────────────────────────────────────────────────────────────────
router.post('/kyc', (req, res) => complianceController.setKyc(req, res));
router.delete('/kyc', (req, res) => complianceController.revokeKyc(req, res));
router.get('/kyc/check', (req, res) => complianceController.checkKyc(req, res));
router.get('/kyc/list', (req, res) => complianceController.listKycVerifications(req, res));

// ─── AML ────────────────────────────────────────────────────────────────────
router.post('/aml/assess/:address', (req, res) => complianceController.assessAmlRisk(req, res));
router.get('/aml/risk/:address', (req, res) => complianceController.getAmlRisk(req, res));
router.get('/aml/assessments', (req, res) => complianceController.listAmlAssessments(req, res));

// ─── Transaction Compliance ─────────────────────────────────────────────────
router.post('/transaction/check', (req, res) => complianceController.checkTransaction(req, res));

// ─── SAR ────────────────────────────────────────────────────────────────────
router.post('/sar', (req, res) => complianceController.fileSar(req, res));
router.get('/sar/:sarId', (req, res) => complianceController.getSar(req, res));
router.get('/sar', (req, res) => complianceController.listSars(req, res));
router.patch('/sar/:sarId/status', (req, res) => complianceController.updateSarStatus(req, res));

// ─── Compliance Reporting ───────────────────────────────────────────────────
router.get('/report', (req, res) => complianceController.getReport(req, res));

// ─── Audit Trail ────────────────────────────────────────────────────────────
router.get('/audit-trail', (req, res) => complianceController.getAuditTrail(req, res));
router.get('/audit-trail/verify', (req, res) => complianceController.verifyAuditTrailIntegrity(req, res));

// ─── Compliance Configuration ───────────────────────────────────────────────
router.get('/config', (req, res) => complianceController.getConfig(req, res));
router.put('/config', (req, res) => complianceController.updateConfig(req, res));
router.post('/config/jurisdiction-limits', (req, res) => complianceController.setJurisdictionLimits(req, res));
router.post('/config/restricted-jurisdictions', (req, res) => complianceController.addRestrictedJurisdiction(req, res));
router.delete('/config/restricted-jurisdictions', (req, res) => complianceController.removeRestrictedJurisdiction(req, res));

// ─── Compliance Dashboard ───────────────────────────────────────────────────
router.get('/dashboard', (req, res) => complianceController.getDashboard(req, res));

// ─── Regulatory Limits ──────────────────────────────────────────────────────
router.post('/regulatory-limits/check', (req, res) => complianceController.checkRegulatoryLimits(req, res));

export default router;
