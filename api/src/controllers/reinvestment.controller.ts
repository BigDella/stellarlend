import { Request, Response, NextFunction } from 'express';
import { reinvestmentService } from '../services/earnings/reinvestment.service';

export const createPlan = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const plan = reinvestmentService.createPlan(req.body);
    return res.status(201).json({ success: true, plan });
  } catch (err) {
    next(err);
    return;
  }
};

export const getPlan = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { planId } = req.params!;
    const plan = reinvestmentService.getPlan(planId!);
    return res.status(200).json({ success: true, plan });
  } catch (err) {
    next(err);
    return;
  }
};

export const getUserPlans = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params!;
    const plans = reinvestmentService.getUserPlans(userAddress!);
    return res.status(200).json({ success: true, plans });
  } catch (err) {
    next(err);
    return;
  }
};

export const pausePlan = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { planId } = req.params!;
    const { userAddress } = req.body;
    const plan = reinvestmentService.pause(planId!, userAddress);
    return res.status(200).json({ success: true, plan });
  } catch (err) {
    next(err);
    return;
  }
};

export const resumePlan = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { planId } = req.params!;
    const { userAddress } = req.body;
    const plan = reinvestmentService.resume(planId!, userAddress);
    return res.status(200).json({ success: true, plan });
  } catch (err) {
    next(err);
    return;
  }
};

export const recordSweep = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { planId } = req.params!;
    const events = reinvestmentService.recordSweep(planId!, req.body);
    return res.status(201).json({ success: true, events });
  } catch (err) {
    next(err);
    return;
  }
};

export const getHistory = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { planId } = req.params!;
    const history = reinvestmentService.getHistory(planId!);
    return res.status(200).json({ success: true, history });
  } catch (err) {
    next(err);
    return;
  }
};

export const getAnalytics = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { planId } = req.params!;
    const apyBps = req.query.assumedApyBps ? Number(req.query.assumedApyBps) : undefined;
    const analytics = reinvestmentService.getAnalytics(planId!, apyBps);
    return res.status(200).json({ success: true, analytics });
  } catch (err) {
    next(err);
    return;
  }
};
