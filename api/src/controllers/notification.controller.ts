import { Request, Response, NextFunction } from 'express';
import { notificationEngine } from '../services/notification-engine';
import logger from '../utils/logger';

export const subscribe = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const channel: string = req.body.channel as string;
    const recipient: string = req.body.recipient as string;
    const alertTypes: string[] = req.body.alertTypes as string[];
    if (!channel || !recipient || !alertTypes) {
      return res.status(400).json({ success: false, error: 'Missing required fields: channel, recipient, alertTypes' });
    }
    const prefs = notificationEngine.subscribe(userAddress, channel as any, recipient, alertTypes as any);
    logger.info('Notification subscription', { userAddress, channel });
    return res.status(201).json({ success: true, data: prefs });
  } catch (error) {
    next(error);
    return;
  }
};

export const getPreferences = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const prefs = notificationEngine.getPreferences(userAddress);
    return res.status(200).json({ success: true, data: prefs });
  } catch (error) {
    next(error);
    return;
  }
};

export const updatePreference = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const channel: any = req.body.channel;
    const alertType: any = req.body.alertType;
    const enabled: boolean = req.body.enabled as boolean;
    const threshold: number | undefined = req.body.threshold as number | undefined;
    const updated = notificationEngine.updatePreference(userAddress, channel, alertType, enabled, threshold);
    if (!updated) {
      return res.status(404).json({ success: false, error: 'Preference not found' });
    }
    return res.status(200).json({ success: true, data: updated });
  } catch (error) {
    next(error);
    return;
  }
};

export const getHistory = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const { alertType, channel, limit, cursor } = req.query as Record<string, string>;
    const result = notificationEngine.getHistory(userAddress, {
      alertType: alertType as any,
      channel: channel as any,
      limit: limit ? parseInt(limit, 10) : undefined,
      cursor,
    });
    return res.status(200).json({ success: true, data: result.messages, nextCursor: result.nextCursor });
  } catch (error) {
    next(error);
    return;
  }
};

export const markDelivered = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const messageId = req.params.messageId as string;
    notificationEngine.markDelivered(messageId);
    return res.status(200).json({ success: true });
  } catch (error) {
    next(error);
    return;
  }
};

export const markRead = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const messageId = req.params.messageId as string;
    notificationEngine.markRead(messageId);
    return res.status(200).json({ success: true });
  } catch (error) {
    next(error);
    return;
  }
};
