import { Router } from 'express';
import * as debtTokenController from '../controllers/debtToken.controller';

const router: Router = Router();

// POST /api/debt-token/mint
router.post('/mint', debtTokenController.mintDebtToken);

// POST /api/debt-token/transfer
router.post('/transfer', debtTokenController.transferDebtToken);

// POST /api/debt-token/burn
router.post('/burn', debtTokenController.burnDebtToken);

// GET /api/debt-token/position/:tokenId
router.get('/position/:tokenId', debtTokenController.getDebtPosition);

// GET /api/debt-token/tokens/:userAddress
router.get('/tokens/:userAddress', debtTokenController.getUserDebtTokens);

// GET /api/debt-token/total-supply
router.get('/total-supply', debtTokenController.getDebtTokenTotalSupply);

// POST /api/debt-token/transfer-pause
router.post('/transfer-pause', debtTokenController.setDebtTokenTransferPause);

// POST /api/debt-token/address-blocked
router.post('/address-blocked', debtTokenController.setDebtTokenAddressBlocked);

export default router;
