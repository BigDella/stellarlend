import { Router } from 'express';
import {
  getAuditFindings,
  createAuditFinding,
  getAuditFindingById,
  updateAuditFinding,
  getAuditMetrics,
  getAuditReport
} from '../controllers/audit-findings.controller';

const router = Router();

router.get('/metrics', getAuditMetrics);
router.get('/report', getAuditReport);

router.route('/')
  .get(getAuditFindings)
  .post(createAuditFinding);

router.route('/:id')
  .get(getAuditFindingById)
  .patch(updateAuditFinding);

export default router;
