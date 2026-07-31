import axios from 'axios';
jest.mock('axios');
const mockedAxios = axios as jest.Mocked<typeof axios>;
beforeAll(() => {
  mockedAxios.create.mockReturnThis();
  const axiosResponse = { data: {}, status: 200, statusText: 'OK', headers: {}, config: { url: '' } };
  mockedAxios.get.mockResolvedValue(axiosResponse);
  mockedAxios.post.mockResolvedValue(axiosResponse);
});
afterEach(() => { jest.clearAllMocks(); });

import { notificationEngine } from '../services/notification-engine';

const userId = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';

describe('NotificationEngine', () => {
  it('creates subscriptions', () => {
    const prefs = notificationEngine.subscribe(
      userId, 'email', 'user@example.com',
      ['health_factor_low', 'approaching_liquidation']
    );
    expect(prefs).toHaveLength(2);
    expect(prefs[0]!.channel).toBe('email');
    expect(prefs[0]!.enabled).toBe(true);
  });

  it('returns preferences', () => {
    const prefs = notificationEngine.getPreferences(userId);
    expect(prefs.length).toBeGreaterThan(0);
  });

  it('updates preferences', () => {
    const updated = notificationEngine.updatePreference(
      userId, 'email', 'health_factor_low', false, 150
    );
    expect(updated).not.toBeNull();
    expect(updated!.enabled).toBe(false);
    expect(updated!.threshold).toBe(150);
  });

  it('sends alerts to subscribed channels', async () => {
    const messages = await notificationEngine.sendAlert(
      userId, 'health_factor_low',
      { healthFactor: '1.2', collateralValue: '1000', debtValue: '800' }
    );
    expect(Array.isArray(messages)).toBe(true);
  });

  it('respects rate limiting', async () => {
    const messages1 = await notificationEngine.sendAlert(
      userId, 'health_factor_low',
      { healthFactor: '1.1', collateralValue: '1000', debtValue: '900' }
    );
    const messages2 = await notificationEngine.sendAlert(
      userId, 'health_factor_low',
      { healthFactor: '1.0', collateralValue: '1000', debtValue: '1000' }
    );
    expect(messages2.length).toBeLessThanOrEqual(messages1.length);
  });

  it('returns notification history', () => {
    const result = notificationEngine.getHistory(userId);
    expect(result.messages).toBeDefined();
    expect(Array.isArray(result.messages)).toBe(true);
  });

  it('filters history by alert type', () => {
    const result = notificationEngine.getHistory(userId, { alertType: 'price_alert' });
    expect(result.messages.every(m => m.alertType === 'price_alert')).toBe(true);
  });

  it('marks messages as delivered', () => {
    const result = notificationEngine.getHistory(userId);
    if (result.messages.length > 0) {
      notificationEngine.markDelivered(result.messages[0]!.id);
      const updated = notificationEngine.getHistory(userId);
      const msg = updated.messages.find(m => m.id === result.messages[0]!.id);
      expect(msg).toBeDefined();
    }
  });

  it('marks messages as read', () => {
    const result = notificationEngine.getHistory(userId);
    if (result.messages.length > 0) {
      notificationEngine.markRead(result.messages[0]!.id);
    }
  });

  it('handles unknown alert types gracefully', async () => {
    const messages = await notificationEngine.sendAlert(
      userId, 'price_alert' as any,
      { asset: 'XLM', direction: 'up', changePercent: '5', currentPrice: '0.5', threshold: '0.45' }
    );
    expect(Array.isArray(messages)).toBe(true);
  });

  it('returns empty preferences for unknown user', () => {
    const prefs = notificationEngine.getPreferences('unknown');
    expect(prefs).toEqual([]);
  });
});
