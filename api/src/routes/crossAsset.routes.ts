import { Router } from 'express';
import {
  getCrossAssetPositionSummary,
  depositCrossAsset,
  borrowCrossAsset,
  withdrawCrossAsset,
  liquidateCrossAsset,
  computeUnifiedHealth,
  getCorrelationMatrix,
  getArbitrageOpportunities,
  getPairThreshold,
} from '../controllers/crossAsset.controller';

const router = Router();

router.get('/position/:userAddress', getCrossAssetPositionSummary);
router.post('/deposit', depositCrossAsset);
router.post('/borrow', borrowCrossAsset);
router.post('/withdraw', withdrawCrossAsset);
router.post('/liquidate', liquidateCrossAsset);
router.post('/health', computeUnifiedHealth);
router.post('/correlation', getCorrelationMatrix);
router.post('/arbitrage', getArbitrageOpportunities);
router.post('/pair-threshold', getPairThreshold);

export default router;
