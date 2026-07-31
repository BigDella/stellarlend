import { Router } from 'express';
import { feeTierService } from '../services/fees/tier.service';
const router = Router();
router.put('/config', (req, res) => {
  try {
    res.json({ success: true, data: feeTierService.configure(req.body.tiers) });
  } catch (error) {
    res
      .status(400)
      .json({ success: false, error: error instanceof Error ? error.message : 'Invalid tiers' });
  }
});
router.post('/status', (req, res) =>
  res.json({ success: true, data: feeTierService.status(req.body.userAddress, req.body.metrics) })
);
router.post('/apply', (req, res) =>
  res.json({
    success: true,
    data: feeTierService.apply(
      req.body.userAddress,
      Number(req.body.baseFee),
      req.body.metrics,
      Number(req.body.minimumFee ?? 0)
    ),
  })
);
export default router;
