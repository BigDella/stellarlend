import { config } from '../config';
import logger from '../utils/logger';

type PauseReason = 'manual' | 'auto-failure-threshold' | 'governance-vote' | 'oracle-failure';

interface PauseState {
  paused: boolean;
  reason: PauseReason | null;
  since: number | null;
}

interface QueuedWithdrawal {
  userAddress: string;
  assetAddress?: string;
  amount: string;
  queuedAt: string;
}

interface EmergencyEvent {
  type: 'pause' | 'resume' | 'withdrawal' | 'notification';
  timestamp: string;
  details: Record<string, unknown>;
}

interface EmergencyNotification {
  id: string;
  message: string;
  severity: 'info' | 'warning' | 'critical';
  timestamp: string;
  read: boolean;
}

class EmergencyPauseService {
  private state: PauseState = { paused: false, reason: null, since: null };
  private consecutiveFailures = 0;
  private readonly withdrawalQueue: QueuedWithdrawal[] = [];
  private readonly emergencyHistory: EmergencyEvent[] = [];
  private readonly notifications: EmergencyNotification[] = [];
  private notificationCounter = 0;

  isPaused(): PauseState {
    return { ...this.state };
  }

  recordSuccess(): void {
    this.consecutiveFailures = 0;
  }

  recordFailure(): void {
    this.consecutiveFailures += 1;
    if (this.consecutiveFailures >= config.emergency.autoPauseFailureThreshold) {
      this.pause('auto-failure-threshold');
    }
  }

  pause(reason: PauseReason): void {
    this.state = {
      paused: true,
      reason,
      since: Date.now(),
    };

    this.recordEvent('pause', { reason });
    this.addNotification(
      `Protocol paused: ${reason}. Lenders can queue emergency withdrawals.`,
      reason === 'auto-failure-threshold' ? 'critical' : 'warning',
    );

    logger.info(`Emergency pause triggered: ${reason}`);
  }

  resume(): void {
    const previousReason = this.state.reason;
    this.state = { paused: false, reason: null, since: null };
    this.consecutiveFailures = 0;

    this.recordEvent('resume', { previousReason });
    this.addNotification('Protocol resumed normal operations.', 'info');

    logger.info('Emergency pause resumed');
  }

  queueWithdrawal(entry: Omit<QueuedWithdrawal, 'queuedAt'>): void {
    const queuedEntry: QueuedWithdrawal = {
      ...entry,
      queuedAt: new Date().toISOString(),
    };
    this.withdrawalQueue.push(queuedEntry);

    this.recordEvent('withdrawal', { userAddress: entry.userAddress, amount: entry.amount });
    this.addNotification(
      `Emergency withdrawal queued for ${entry.userAddress}: ${entry.amount}`,
      'warning',
    );

    logger.info(`Emergency withdrawal queued: ${entry.userAddress} - ${entry.amount}`);
  }

  drainWithdrawalQueue(): QueuedWithdrawal[] {
    const drained = [...this.withdrawalQueue];
    this.withdrawalQueue.length = 0;
    return drained;
  }

  getWithdrawalQueue(): QueuedWithdrawal[] {
    return [...this.withdrawalQueue];
  }

  getEmergencyHistory(): EmergencyEvent[] {
    return [...this.emergencyHistory];
  }

  getNotifications(): EmergencyNotification[] {
    return [...this.notifications];
  }

  getUnreadNotifications(): EmergencyNotification[] {
    return this.notifications.filter(n => !n.read);
  }

  markNotificationRead(id: string): boolean {
    const notification = this.notifications.find(n => n.id === id);
    if (notification) {
      notification.read = true;
      return true;
    }
    return false;
  }

  markAllNotificationsRead(): void {
    this.notifications.forEach(n => { n.read = true; });
  }

  private recordEvent(type: EmergencyEvent['type'], details: Record<string, unknown>): void {
    this.emergencyHistory.push({
      type,
      timestamp: new Date().toISOString(),
      details,
    });
  }

  private addNotification(message: string, severity: EmergencyNotification['severity']): void {
    this.notificationCounter++;
    this.notifications.push({
      id: `notif-${this.notificationCounter}`,
      message,
      severity,
      timestamp: new Date().toISOString(),
      read: false,
    });
  }
}

export const emergencyPauseService = new EmergencyPauseService();
