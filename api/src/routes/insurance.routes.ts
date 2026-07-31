import { Response, Router } from 'express';
import { insuranceService } from '../services/insurance/insurance.service';

const router = Router();
const handle = (res: Response, fn: () => unknown) => {
  try {
    res.json({ success: true, data: fn() });
  } catch (error) {
    res.status(400).json({
      success: false,
      error: error instanceof Error ? error.message : 'Invalid request',
    });
  }
};
router.get('/policies', (_req, res) => handle(res, () => insuranceService.listPolicies()));
router.post('/providers', (req, res) =>
  handle(res, () => insuranceService.onboardProvider(req.body))
);
router.post('/policies', (req, res) => handle(res, () => insuranceService.createPolicy(req.body)));
router.post('/coverages', (req, res) =>
  handle(res, () =>
    insuranceService.purchase(
      req.body.policyId,
      req.body.lender,
      req.body.positionId,
      req.body.coverageAmount
    )
  )
);
router.post('/claims', (req, res) =>
  handle(res, () =>
    insuranceService.submitClaim(
      req.body.coverageId,
      req.body.trigger,
      req.body.evidence,
      Number(req.body.amount)
    )
  )
);
router.post('/claims/:id/dispute', (req, res) =>
  handle(res, () => insuranceService.disputeClaim(req.params.id))
);
router.get('/claims', (req, res) =>
  handle(res, () => insuranceService.dashboard(req.query.providerId as string | undefined))
);
export default router;
