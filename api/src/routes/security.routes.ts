import { Router } from 'express';
import {
  submitVulnerabilityReport,
  getVulnerabilityReport,
  getTriageQueue
} from '../controllers/security.controller';

const router = Router();

router.post('/report', submitVulnerabilityReport);
router.get('/reports/:id', getVulnerabilityReport);
router.get('/triage-queue', getTriageQueue);

export default router;
