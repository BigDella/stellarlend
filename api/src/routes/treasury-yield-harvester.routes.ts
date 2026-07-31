import { Router } from 'express';
import { treasuryYieldHarvesterController } from '../controllers/treasury-yield-harvester.controller';

const router = Router();

router.post('/protocol', (req, res) => treasuryYieldHarvesterController.registerProtocol(req, res));

router.post('/whitelist', (req, res) => treasuryYieldHarvesterController.updateWhitelist(req, res));

router.get('/protocols', (req, res) => treasuryYieldHarvesterController.getProtocols(req, res));

router.get('/whitelisted-protocols', (req, res) => treasuryYieldHarvesterController.getWhitelistedProtocols(req, res));

router.get('/risk-score/:protocolId', (req, res) => treasuryYieldHarvesterController.calculateRiskScore(req, res));

router.post('/deploy', (req, res) => treasuryYieldHarvesterController.deployToProtocol(req, res));

router.get('/positions', (req, res) => treasuryYieldHarvesterController.getPositions(req, res));

router.post('/harvest', (req, res) => treasuryYieldHarvesterController.harvestYield(req, res));

router.get('/withdrawal-simulation/:positionId', (req, res) => treasuryYieldHarvesterController.simulateWithdrawal(req, res));

router.post('/withdraw/:positionId', (req, res) => treasuryYieldHarvesterController.withdrawFromProtocol(req, res));

router.post('/emergency-withdraw/:positionId', (req, res) => treasuryYieldHarvesterController.emergencyWithdraw(req, res));

router.get('/yield-report', (req, res) => treasuryYieldHarvesterController.getYieldReport(req, res));

router.get('/emergency-withdrawals', (req, res) => treasuryYieldHarvesterController.getEmergencyWithdrawals(req, res));

export const treasuryYieldHarvesterRoutes = router;
