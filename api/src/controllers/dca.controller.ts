import { Request, Response, NextFunction } from 'express';
import { dcaService } from '../services/dca.service';

export const createPlan = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const plan = dcaService.createPlan(req.body);
    return res.status(201).json({ success: true, plan });
  } catch (err) {
    next(err);
    return;
  }
};

export const getPlans = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params!;
    const plans = dcaService.getUserPlans(userAddress!);
    return res.status(200).json({ success: true, plans });
  } catch (err) {
    next(err);
    return;
  }
};

export const getPlan = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { planId } = req.params!;
    const plan = dcaService.getPlan(planId!);
    return res.status(200).json({ success: true, plan });
  } catch (err) {
    next(err);
    return;
  }
};

export const pausePlan = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { planId } = req.params!;
    const { userAddress } = req.body;
    dcaService.pause(planId!, userAddress);
    return res.status(200).json({ success: true });
  } catch (err) {
    next(err);
    return;
  }
};

export const resumePlan = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { planId } = req.params!;
    const { userAddress } = req.body;
    dcaService.resume(planId!, userAddress);
    return res.status(200).json({ success: true });
  } catch (err) {
    next(err);
    return;
  }
};

export const cancelPlan = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { planId } = req.params!;
    const { userAddress } = req.body;
    const result = dcaService.cancel(planId!, userAddress);
    return res.status(200).json({ success: true, ...result });
  } catch (err) {
    next(err);
    return;
  }
};

export const getHistory = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { planId } = req.params!;
    const history = dcaService.getExecutionHistory(planId!);
    return res.status(200).json({ success: true, history });
  } catch (err) {
    next(err);
    return;
  }
};
