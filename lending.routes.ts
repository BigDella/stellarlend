import { Router } from 'express';
import {
  prepareLendingOperation,
  submitSignedXdr,
} from '../../controllers/lending.controller';
import {
  prepareValidation,
  submitValidation,
} from '../../middleware/validation';
import { authenticateToken } from '../../middleware/auth';
import { bodySizeLimitMiddleware } from '../../middleware/bodySizeLimit';
import { perUserRateLimit } from '../../middleware/rateLimit';
import { idempotencyMiddleware } from '../../middleware/idempotency';

const router = Router();

// Lending operations
router.get(
  '/prepare/:operation',
  authenticateToken,
  perUserRateLimit,
  prepareValidation,
  prepareLendingOperation
);
router.post(
  '/submit',
  authenticateToken,
  perUserRateLimit,
  bodySizeLimitMiddleware,
  idempotencyMiddleware,
  submitValidation,
  submitSignedXdr
);

export default router;