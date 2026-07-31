import { Router } from 'express';
import {
  getCrossAssetPositionSummary,
  depositCrossAsset,
  borrowCrossAsset,
  withdrawCrossAsset,
  liquidateCrossAsset,
} from '../controllers/crossAsset.controller';

const router = Router();

// GET /api/cross-asset/position/:userAddress
router.get('/position/:userAddress', getCrossAssetPositionSummary);

// POST /api/cross-asset/deposit
router.post('/deposit', depositCrossAsset);

// POST /api/cross-asset/borrow
router.post('/borrow', borrowCrossAsset);

// POST /api/cross-asset/withdraw
router.post('/withdraw', withdrawCrossAsset);

// POST /api/cross-asset/liquidate
router.post('/liquidate', liquidateCrossAsset);

export default router;
