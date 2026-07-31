import { Router, Request, Response, NextFunction } from 'express';
import { referralService } from '../services/referral.service';
import logger from '../utils/logger';

const router: Router = Router();

const referralController = {
  async generateCode(req: Request, res: Response, next: NextFunction) {
    try {
      const { userAddress } = req.body;
      if (!userAddress) {
        return res.status(400).json({ success: false, error: 'userAddress is required' });
      }
      const code = referralService.generateCode(userAddress);
      const link = referralService.getReferralLink(userAddress);
      res.json({ success: true, data: { code, link } });
    } catch (error) {
      next(error);
    }
  },

  async register(req: Request, res: Response, next: NextFunction) {
    try {
      const { refereeAddress, referralCode } = req.body;
      if (!refereeAddress || !referralCode) {
        return res.status(400).json({ success: false, error: 'refereeAddress and referralCode required' });
      }
      const result = referralService.register(refereeAddress, referralCode);
      res.json({ success: true, data: result });
    } catch (error) {
      next(error);
    }
  },

  async accrueFee(req: Request, res: Response, next: NextFunction) {
    try {
      const { refereeAddress, feeAmount } = req.body;
      if (!refereeAddress || feeAmount === undefined) {
        return res.status(400).json({ success: false, error: 'refereeAddress and feeAmount required' });
      }
      referralService.accrueFee(refereeAddress, feeAmount);
      res.json({ success: true, data: { accrued: feeAmount } });
    } catch (error) {
      next(error);
    }
  },

  async getStats(req: Request, res: Response, next: NextFunction) {
    try {
      const { userAddress } = req.query;
      if (!userAddress || typeof userAddress !== 'string') {
        return res.status(400).json({ success: false, error: 'userAddress query param required' });
      }
      const stats = referralService.getStats(userAddress);
      if (!stats) {
        return res.status(404).json({ success: false, error: 'No stats found for user' });
      }
      res.json({ success: true, data: stats });
    } catch (error) {
      next(error);
    }
  },

  async claim(req: Request, res: Response, next: NextFunction) {
    try {
      const { userAddress } = req.body;
      if (!userAddress) {
        return res.status(400).json({ success: false, error: 'userAddress is required' });
      }
      const result = referralService.claim(userAddress);
      res.json({ success: true, data: result });
    } catch (error) {
      next(error);
    }
  },

  async getLink(req: Request, res: Response, next: NextFunction) {
    try {
      const { userAddress } = req.query;
      if (!userAddress || typeof userAddress !== 'string') {
        return res.status(400).json({ success: false, error: 'userAddress query param required' });
      }
      const link = referralService.getReferralLink(userAddress);
      res.json({ success: true, data: { link } });
    } catch (error) {
      next(error);
    }
  },

  async getConversionFunnel(req: Request, res: Response, next: NextFunction) {
    try {
      const { userAddress } = req.query;
      if (!userAddress || typeof userAddress !== 'string') {
        return res.status(400).json({ success: false, error: 'userAddress query param required' });
      }
      const funnel = referralService.getConversionFunnel(userAddress);
      if (!funnel) {
        return res.status(404).json({ success: false, error: 'No funnel data found' });
      }
      res.json({ success: true, data: funnel });
    } catch (error) {
      next(error);
    }
  },

  async getAntiSybilStatus(req: Request, res: Response, next: NextFunction) {
    try {
      const { userAddress, totalDeposit } = req.query;
      if (!userAddress || typeof userAddress !== 'string') {
        return res.status(400).json({ success: false, error: 'userAddress query param required' });
      }
      const deposit = parseFloat(totalDeposit as string) || 0;
      const status = referralService.getAntiSybilStatus(userAddress, deposit);
      res.json({ success: true, data: status });
    } catch (error) {
      next(error);
    }
  },
};

router.post('/generate-code', referralController.generateCode);
router.post('/register', referralController.register);
router.post('/accrue-fee', referralController.accrueFee);
router.post('/claim', referralController.claim);
router.get('/stats', referralController.getStats);
router.get('/link', referralController.getLink);
router.get('/funnel', referralController.getConversionFunnel);
router.get('/anti-sybil', referralController.getAntiSybilStatus);

export default router;
