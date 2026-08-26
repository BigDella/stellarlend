import { Router } from 'express';
import * as subscriptionController from '../controllers/subscription.controller';
import { importRequestValidation, merchantParamValidation } from '../middleware/validation';

const router = Router();

router.post(
  '/import/validate',
  importRequestValidation,
  subscriptionController.validateImportRequest
);
router.post(
  '/import/preview',
  importRequestValidation,
  subscriptionController.previewImportRequest
);
router.post('/import', importRequestValidation, subscriptionController.importSubscriptionsRequest);
router.get(
  '/export/:merchantId',
  merchantParamValidation,
  subscriptionController.exportSubscriptionsRequest
);
router.get(
  '/import/history/:merchantId',
  merchantParamValidation,
  subscriptionController.getImportHistoryRequest
);

export default router;
