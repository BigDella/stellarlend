import { NextFunction, Request, Response } from 'express';
import { ValidationError } from '../utils/errors';
import { SubscriptionService } from '../services/subscription.service';
import type { CreateRecurringSubscriptionRequest, UpdateRecurringSubscriptionRequest } from '../types';

const subscriptionService = new SubscriptionService();

function normalizeImportError(error: unknown): Error {
  if (error instanceof ValidationError) return error;
  if (error instanceof Error) return new ValidationError(error.message);
  return new ValidationError('Invalid import request');
}

// ─── Recurring Operations ──────────────────────────────────────────────────────

export const createSubscription = (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as CreateRecurringSubscriptionRequest;
    const subscription = subscriptionService.createSubscription(body);
    res.status(201).json({ success: true, subscription });
  } catch (error) {
    next(error);
  }
};

export const getSubscription = (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress, subscriptionId } = req.params;
    const subscription = subscriptionService.getSubscription(userAddress!, subscriptionId!);
    if (!subscription) {
      res.status(404).json({ success: false, error: 'Subscription not found' });
      return;
    }
    res.status(200).json({ success: true, subscription });
  } catch (error) {
    next(error);
  }
};

export const listSubscriptions = (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params;
    const status = req.query.status as string | undefined;
    const action = req.query.action as string | undefined;
    const subscriptions = subscriptionService.listSubscriptions(
      userAddress!,
      status as any,
      action as any
    );
    res.status(200).json({ success: true, subscriptions, count: subscriptions.length });
  } catch (error) {
    next(error);
  }
};

export const updateSubscription = (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress, subscriptionId } = req.params;
    const body = req.body as UpdateRecurringSubscriptionRequest;
    const subscription = subscriptionService.updateSubscription(userAddress!, subscriptionId!, body);
    res.status(200).json({ success: true, subscription });
  } catch (error) {
    next(error);
  }
};

export const pauseSubscription = (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress, subscriptionId } = req.params;
    const subscription = subscriptionService.pauseSubscription(userAddress!, subscriptionId!);
    res.status(200).json({ success: true, subscription });
  } catch (error) {
    next(error);
  }
};

export const resumeSubscription = (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress, subscriptionId } = req.params;
    const subscription = subscriptionService.resumeSubscription(userAddress!, subscriptionId!);
    res.status(200).json({ success: true, subscription });
  } catch (error) {
    next(error);
  }
};

export const cancelSubscription = (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress, subscriptionId } = req.params;
    const subscription = subscriptionService.cancelSubscription(userAddress!, subscriptionId!);
    res.status(200).json({ success: true, subscription });
  } catch (error) {
    next(error);
  }
};

export const getExecutionHistory = (req: Request, res: Response, next: NextFunction) => {
  try {
    const { subscriptionId } = req.params;
    const history = subscriptionService.getExecutionHistory(subscriptionId!);
    res.status(200).json({ success: true, executionHistory: history, count: history.length });
  } catch (error) {
    next(error);
  }
};

export const getSubscriptionAnalytics = (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params;
    const analytics = subscriptionService.getSubscriptionAnalytics(userAddress!);
    res.status(200).json({ success: true, analytics });
  } catch (error) {
    next(error);
  }
};

export const triggerManualExecution = (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress, subscriptionId } = req.params;
    const result = subscriptionService.triggerManualExecution(subscriptionId!, userAddress!);
    res.status(200).json({ success: true, execution: result });
  } catch (error) {
    next(error);
  }
};

export const executeDueSubscriptions = async (_req: Request, res: Response, next: NextFunction) => {
  try {
    const results = await subscriptionService.executeDueSubscriptions();
    res.status(200).json({ success: true, executed: results.length, results });
  } catch (error) {
    next(error);
  }
};

// ─── Import/Export (preserved) ─────────────────────────────────────────────────

export const validateImportRequest = (req: Request, res: Response, next: NextFunction) => {
  try {
    const result = subscriptionService.validateImport(req.body);
    res.status(200).json(result);
  } catch (error) {
    next(normalizeImportError(error));
  }
};

export const previewImportRequest = (req: Request, res: Response, next: NextFunction) => {
  try {
    const result = subscriptionService.previewImport(req.body);
    res.status(200).json(result);
  } catch (error) {
    next(normalizeImportError(error));
  }
};

export const importSubscriptionsRequest = (req: Request, res: Response, next: NextFunction) => {
  try {
    const result = subscriptionService.importSubscriptions(req.body);
    if (result.errorCount > 0) {
      res.status(400).json({ success: false, ...result });
      return;
    }
    res.status(200).json({ success: true, ...result });
  } catch (error) {
    next(normalizeImportError(error));
  }
};

export const exportSubscriptionsRequest = (req: Request, res: Response, next: NextFunction) => {
  try {
    const result = subscriptionService.exportSubscriptions(req.params.merchantId!);
    res.status(200).json(result);
  } catch (error) {
    next(error);
  }
};

export const getImportHistoryRequest = (req: Request, res: Response, next: NextFunction) => {
  try {
    const history = subscriptionService.getImportHistory(req.params.merchantId!);
    res.status(200).json({ merchantId: req.params.merchantId, history });
  } catch (error) {
    next(error);
  }
};
