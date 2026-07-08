import { Request, Response, NextFunction } from 'express';
import { referralService } from '../services/referral.service';

export const generateCode = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.body;
    if (!userAddress) return res.status(400).json({ error: 'userAddress is required' });
    const code = referralService.generateCode(userAddress);
    return res.status(200).json({ success: true, code });
  } catch (err) {
    next(err);
    return;
  }
};

export const registerReferral = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { refereeAddress, referralCode } = req.body;
    if (!refereeAddress || !referralCode) {
      return res.status(400).json({ error: 'refereeAddress and referralCode are required' });
    }
    const result = referralService.register(refereeAddress, referralCode);
    return res.status(200).json({ success: true, ...result });
  } catch (err) {
    next(err);
    return;
  }
};

export const getStats = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params;
    if (!userAddress) return res.status(400).json({ error: 'userAddress is required' });
    const stats = referralService.getStats(userAddress);
    return res.status(200).json({ success: true, stats });
  } catch (err) {
    next(err);
    return;
  }
};

export const claim = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.body;
    if (!userAddress) return res.status(400).json({ error: 'userAddress is required' });
    const result = referralService.claim(userAddress);
    return res.status(200).json({ success: true, ...result });
  } catch (err) {
    next(err);
    return;
  }
};

export const getReferralLink = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params;
    if (!userAddress) return res.status(400).json({ error: 'userAddress is required' });
    const link = referralService.getReferralLink(userAddress);
    return res.status(200).json({ success: true, link });
  } catch (err) {
    next(err);
    return;
  }
};
