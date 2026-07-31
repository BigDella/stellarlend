import { NotificationMessage, NotificationChannel, AlertType, NotificationPreference, DeliveryStatus, NotificationTemplate } from '../../types/notifications';
import { emailChannel } from './channels/email.channel';
import { telegramChannel } from './channels/telegram.channel';
import { discordChannel } from './channels/discord.channel';
import { pushChannel } from './channels/push.channel';
import logger from '../../utils/logger';
import { v4 as uuid } from 'uuid';

const RATE_LIMIT_MS = 5 * 60 * 1000; // 5 minutes
const MAX_NOTIFICATIONS_PER_USER = 1000;

class NotificationEngine {
  private preferences: Map<string, NotificationPreference[]> = new Map();
  private history: Map<string, NotificationMessage[]> = new Map();
  private templates: Map<AlertType, NotificationTemplate> = new Map();
  private rateLimitTracker: Map<string, number> = new Map();
  private channels: Map<NotificationChannel, { send: (msg: NotificationMessage) => Promise<boolean> }> = new Map();

  constructor() {
    this.channels.set('email', emailChannel);
    this.channels.set('telegram', telegramChannel);
    this.channels.set('discord', discordChannel);
    this.channels.set('push', pushChannel);
    this.initializeTemplates();
  }

  private initializeTemplates(): void {
    this.templates.set('health_factor_low', {
      id: 'health_factor_low',
      alertType: 'health_factor_low',
      titleTemplate: '⚠️ Health Factor Low',
      bodyTemplate: 'Your health factor has dropped to {healthFactor}. Current value: {collateralValue}, Debt: {debtValue}. Consider adding collateral.',
      variables: ['healthFactor', 'collateralValue', 'debtValue'],
    });
    this.templates.set('approaching_liquidation', {
      id: 'approaching_liquidation',
      alertType: 'approaching_liquidation',
      titleTemplate: '🚨 Approaching Liquidation',
      bodyTemplate: 'Your position is approaching liquidation (HF: {healthFactor}). Price at liquidation: {liquidationPrice}. Current price: {currentPrice}.',
      variables: ['healthFactor', 'liquidationPrice', 'currentPrice'],
    });
    this.templates.set('position_liquidated', {
      id: 'position_liquidated',
      alertType: 'position_liquidated',
      titleTemplate: '❌ Position Liquidated',
      bodyTemplate: 'Your position has been liquidated. Liquidated amount: {liquidatedAmount}. Remaining collateral: {remainingCollateral}.',
      variables: ['liquidatedAmount', 'remainingCollateral'],
    });
    this.templates.set('liquidatable_position', {
      id: 'liquidatable_position',
      alertType: 'liquidatable_position',
      titleTemplate: '💰 Liquidatable Position Available',
      bodyTemplate: 'A position is available for liquidation at {liquidationAddress}. Estimated profit: {estimatedProfit}. Health factor: {healthFactor}.',
      variables: ['liquidationAddress', 'estimatedProfit', 'healthFactor'],
    });
    this.templates.set('price_alert', {
      id: 'price_alert',
      alertType: 'price_alert',
      titleTemplate: '📊 Price Alert',
      bodyTemplate: '{asset} price moved {direction} by {changePercent}%. Current price: {currentPrice}. Threshold: {threshold}.',
      variables: ['asset', 'direction', 'changePercent', 'currentPrice', 'threshold'],
    });
  }

  subscribe(userId: string, channel: NotificationChannel, recipient: string, alertTypes: AlertType[]): NotificationPreference[] {
    const prefs: NotificationPreference[] = alertTypes.map(alertType => ({
      userId,
      channel,
      alertType,
      enabled: true,
    }));
    this.preferences.set(userId, [...(this.preferences.get(userId) || []), ...prefs]);
    logger.info('Notification subscription created', { userId, channel, alertTypes });
    return prefs;
  }

  getPreferences(userId: string): NotificationPreference[] {
    return this.preferences.get(userId) || [];
  }

  updatePreference(userId: string, channel: NotificationChannel, alertType: AlertType, enabled: boolean, threshold?: number): NotificationPreference | null {
    const prefs = this.preferences.get(userId) || [];
    const index = prefs.findIndex(p => p.channel === channel && p.alertType === alertType);
    if (index === -1) return null;
    
    const updated: NotificationPreference = { ...(prefs[index] as NotificationPreference), enabled, threshold };
    prefs[index] = updated;
    this.preferences.set(userId, prefs);
    return updated;
  }

  async sendAlert(
    userId: string,
    alertType: AlertType,
    variables: Record<string, string>,
    data?: Record<string, unknown>
  ): Promise<NotificationMessage[]> {
    const prefs = this.preferences.get(userId) || [];
    const activePrefs = prefs.filter(p => p.enabled && this.matchesAlertType(p.alertType, alertType));

    if (activePrefs.length === 0) return [];

    const template = this.templates.get(alertType);
    if (!template) {
      logger.warn('No template for alert type', { alertType });
      return [];
    }

    const messages: NotificationMessage[] = [];

    for (const pref of activePrefs) {
      if (this.isRateLimited(userId, alertType)) {
        logger.warn('Rate limited notification', { userId, alertType });
        continue;
      }

      const message: NotificationMessage = {
        id: uuid(),
        userId,
        channel: pref.channel,
        alertType,
        title: this.renderTemplate(template.titleTemplate, variables),
        body: this.renderTemplate(template.bodyTemplate, variables),
        data,
        status: 'pending',
        createdAt: new Date().toISOString(),
      };

      try {
        const channel = this.channels.get(pref.channel);
        if (channel) {
          const sent = await channel.send(message);
          message.status = sent ? 'sent' : 'failed';
          message.sentAt = sent ? new Date().toISOString() : undefined;
        }
      } catch (error) {
        message.status = 'failed';
        logger.error('Failed to send notification', { error, userId, channel: pref.channel });
      }

      this.rateLimitTracker.set(`${userId}:${alertType}`, Date.now());
      this.addToHistory(userId, message);
      messages.push(message);
    }

    return messages;
  }

  getHistory(userId: string, query?: { alertType?: AlertType; channel?: NotificationChannel; limit?: number; cursor?: string }): { messages: NotificationMessage[]; nextCursor?: string } {
    let messages = this.history.get(userId) || [];

    if (query?.alertType) {
      messages = messages.filter(m => m.alertType === query.alertType);
    }
    if (query?.channel) {
      messages = messages.filter(m => m.channel === query.channel);
    }

    messages.sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());

    const limit = query?.limit || 20;
    const cursorIndex = query?.cursor ? messages.findIndex(m => m.id === query.cursor) : -1;
    const startIndex = cursorIndex >= 0 ? cursorIndex + 1 : 0;
    const slice = messages.slice(startIndex, startIndex + limit);

    return {
      messages: slice,
      nextCursor: messages.length > startIndex + limit ? slice[slice.length - 1]?.id : undefined,
    };
  }

  markDelivered(messageId: string): void {
    this.updateMessageStatus(messageId, 'delivered');
  }

  markRead(messageId: string): void {
    this.updateMessageStatus(messageId, 'read');
  }

  private updateMessageStatus(messageId: string, status: DeliveryStatus): void {
    for (const [, messages] of this.history) {
      const index = messages.findIndex(m => m.id === messageId);
      if (index >= 0) {
        const updated: NotificationMessage = { ...(messages[index] as NotificationMessage), status };
        if (status === 'delivered') updated.deliveredAt = new Date().toISOString();
        if (status === 'read') updated.readAt = new Date().toISOString();
        messages[index] = updated;
        break;
      }
    }
  }

  private renderTemplate(template: string, variables: Record<string, string>): string {
    return template.replace(/\{(\w+)\}/g, (_, key) => variables[key] || `{${key}}`);
  }

  private matchesAlertType(prefType: AlertType, alertType: AlertType): boolean {
    return prefType === alertType;
  }

  private isRateLimited(userId: string, alertType: AlertType): boolean {
    const key = `${userId}:${alertType}`;
    const lastSent = this.rateLimitTracker.get(key);
    if (!lastSent) return false;
    return Date.now() - lastSent < RATE_LIMIT_MS;
  }

  private addToHistory(userId: string, message: NotificationMessage): void {
    const history = this.history.get(userId) || [];
    history.push(message);
    if (history.length > MAX_NOTIFICATIONS_PER_USER) {
      history.splice(0, history.length - MAX_NOTIFICATIONS_PER_USER);
    }
    this.history.set(userId, history);
  }
}

export const notificationEngine = new NotificationEngine();
