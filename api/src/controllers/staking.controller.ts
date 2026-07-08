import { Request, Response, NextFunction } from 'express';
import { stakingService } from '../services/staking.service';
import type {
  StakeRequest,
  UnstakeRequest,
  DelegateRequest,
  RevokeDelegationRequest,
} from '../types/staking';

export const stake = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as StakeRequest;
    const position = stakingService.stake(body);
    return res.status(200).json({ success: true, position });
  } catch (err) {
    next(err);
    return;
  }
};

export const unstake = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as UnstakeRequest;
    const position = stakingService.unstake(body);
    return res.status(200).json({ success: true, position });
  } catch (err) {
    next(err);
    return;
  }
};

export const delegate = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as DelegateRequest;
    const result = stakingService.delegate(body);
    return res.status(200).json({ success: true, ...result });
  } catch (err) {
    next(err);
    return;
  }
};

export const revokeDelegation = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as RevokeDelegationRequest;
    const position = stakingService.revokeDelegation(body);
    return res.status(200).json({ success: true, position });
  } catch (err) {
    next(err);
    return;
  }
};

export const claimRewards = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params;
    const result = stakingService.claimRewards(userAddress!);
    return res.status(200).json({ success: true, ...result });
  } catch (err) {
    next(err);
    return;
  }
};

export const getPosition = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params;
    const position = stakingService.getPosition(userAddress!);
    return res.status(200).json({ success: true, position });
  } catch (err) {
    next(err);
    return;
  }
};

export const getAllPositions = async (_req: Request, res: Response, next: NextFunction) => {
  try {
    const positions = stakingService.getAllPositions();
    return res.status(200).json({ success: true, positions, total: positions.length });
  } catch (err) {
    next(err);
    return;
  }
};
