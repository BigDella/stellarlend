import { Router } from 'express';
import { autoCompoundVaultController } from '../controllers/autoCompoundVault.controller';

const router = Router();

router.get('/config', autoCompoundVaultController.getConfig);
router.get('/snapshot', autoCompoundVaultController.getSnapshot);
router.get('/position/:address', autoCompoundVaultController.getUserPosition);
router.get('/apy-boost', autoCompoundVaultController.computeApyBoost);

export default router;