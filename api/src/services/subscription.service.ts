import {
  RecurringSubscription,
  CreateRecurringSubscriptionRequest,
  UpdateRecurringSubscriptionRequest,
  ExecutionRecord,
  SubscriptionAnalytics,
  ExecutionStatus,
  RecurringAction,
  SubscriptionInterval,
  SubscriptionStatus,
  ExportData,
  ImportData,
  ImportHistoryEntry,
  ImportResult,
  SubscriptionRecord,
  ValidationResult,
} from '../types';
import { exportSubscriptions, importSubscriptions, validateImport } from '../utils/importExport';
import { StellarService } from './stellar.service';
import { config } from '../config';
import { ValidationError } from '../utils/errors';
import logger from '../utils/logger';

// ─── In-memory stores ─────────────────────────────────────────────────────────

const subscriptionsByUser = new Map<string, Map<string, RecurringSubscription>>();
const executionRecords: ExecutionRecord[] = [];
const subscriptionsByMerchant = new Map<string, Map<string, SubscriptionRecord>>();
const importHistoryByMerchant = new Map<string, ImportHistoryEntry[]>();
let keeperIntervalId: ReturnType<typeof setInterval> | undefined;

// ─── Frequency helpers ─────────────────────────────────────────────────────────

const INTERVAL_MS: Record<SubscriptionInterval, number> = {
  daily: 86400000,
  weekly: 604800000,
  monthly: 2592000000,
  quarterly: 7776000000,
  yearly: 31536000000,
};

function computeNextExecution(
  currentDate: Date,
  interval: SubscriptionInterval,
  frequency: number
): Date {
  const ms = INTERVAL_MS[interval] * frequency;
  return new Date(currentDate.getTime() + ms);
}

function generateId(): string {
  return `sub_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

function generateExecutionId(): string {
  return `exec_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

// ─── Merchant store helpers (preserved) ────────────────────────────────────────

function getMerchantStore(merchantId: string): Map<string, SubscriptionRecord> {
  const existing = subscriptionsByMerchant.get(merchantId);
  if (existing) return existing;
  const created = new Map<string, SubscriptionRecord>();
  subscriptionsByMerchant.set(merchantId, created);
  return created;
}

function appendHistory(entry: ImportHistoryEntry): void {
  const existing = importHistoryByMerchant.get(entry.merchantId) ?? [];
  importHistoryByMerchant.set(entry.merchantId, [entry, ...existing]);
}

function getUserStore(userAddress: string): Map<string, RecurringSubscription> {
  const existing = subscriptionsByUser.get(userAddress);
  if (existing) return existing;
  const created = new Map<string, RecurringSubscription>();
  subscriptionsByUser.set(userAddress, created);
  return created;
}

// ─── Subscription class ────────────────────────────────────────────────────────

export class SubscriptionService {
  // ─── Recurring Operations CRUD ─────────────────────────────────────────────

  createSubscription(req: CreateRecurringSubscriptionRequest): RecurringSubscription {
    const now = new Date();
    const startDate = req.startDate ? new Date(req.startDate) : now;
    const nextExecution = computeNextExecution(
      startDate,
      req.interval,
      req.frequency || 1
    );

    const subscription: RecurringSubscription = {
      id: generateId(),
      userAddress: req.userAddress,
      action: req.action,
      amount: req.amount,
      assetAddress: req.assetAddress,
      interval: req.interval,
      frequency: req.frequency || 1,
      startDate: startDate.toISOString(),
      endDate: req.endDate,
      nextExecutionAt: nextExecution.toISOString(),
      status: 'active',
      maxRetries: req.maxRetries ?? config.subscriptions.maxRetries,
      retryCount: 0,
      totalExecutions: 0,
      successfulExecutions: 0,
      failedExecutions: 0,
      totalAmountProcessed: '0',
      createdAt: now.toISOString(),
      updatedAt: now.toISOString(),
    };

    const store = getUserStore(req.userAddress);
    store.set(subscription.id, subscription);
    return subscription;
  }

  getSubscription(userAddress: string, subscriptionId: string): RecurringSubscription | undefined {
    const store = getUserStore(userAddress);
    return store.get(subscriptionId);
  }

  listSubscriptions(
    userAddress: string,
    status?: SubscriptionStatus,
    action?: RecurringAction
  ): RecurringSubscription[] {
    const store = getUserStore(userAddress);
    let subs = Array.from(store.values());
    if (status) subs = subs.filter((s) => s.status === status);
    if (action) subs = subs.filter((s) => s.action === action);
    return subs.sort((a, b) => a.createdAt.localeCompare(b.createdAt));
  }

  updateSubscription(
    userAddress: string,
    subscriptionId: string,
    req: UpdateRecurringSubscriptionRequest
  ): RecurringSubscription {
    const store = getUserStore(userAddress);
    const sub = store.get(subscriptionId);
    if (!sub) throw new ValidationError('Subscription not found');
    if (sub.status === 'cancelled') throw new ValidationError('Cannot update cancelled subscription');

    if (req.amount) sub.amount = req.amount;
    if (req.interval) {
      sub.interval = req.interval;
      sub.nextExecutionAt = computeNextExecution(new Date(), req.interval, req.frequency || sub.frequency).toISOString();
    }
    if (req.frequency) sub.frequency = req.frequency;
    if (req.endDate) sub.endDate = req.endDate;
    if (req.maxRetries !== undefined) sub.maxRetries = req.maxRetries;
    if (req.metadata) sub.metadata = { ...sub.metadata, ...req.metadata };
    sub.updatedAt = new Date().toISOString();

    store.set(subscriptionId, sub);
    return sub;
  }

  pauseSubscription(userAddress: string, subscriptionId: string): RecurringSubscription {
    const store = getUserStore(userAddress);
    const sub = store.get(subscriptionId);
    if (!sub) throw new ValidationError('Subscription not found');
    if (sub.status === 'cancelled') throw new ValidationError('Cannot pause cancelled subscription');
    sub.status = 'paused';
    sub.updatedAt = new Date().toISOString();
    store.set(subscriptionId, sub);
    return sub;
  }

  resumeSubscription(userAddress: string, subscriptionId: string): RecurringSubscription {
    const store = getUserStore(userAddress);
    const sub = store.get(subscriptionId);
    if (!sub) throw new ValidationError('Subscription not found');
    if (sub.status === 'cancelled') throw new ValidationError('Cannot resume cancelled subscription');
    sub.status = 'active';
    sub.nextExecutionAt = computeNextExecution(new Date(), sub.interval, sub.frequency).toISOString();
    sub.updatedAt = new Date().toISOString();
    store.set(subscriptionId, sub);
    return sub;
  }

  cancelSubscription(userAddress: string, subscriptionId: string): RecurringSubscription {
    const store = getUserStore(userAddress);
    const sub = store.get(subscriptionId);
    if (!sub) throw new ValidationError('Subscription not found');
    sub.status = 'cancelled';
    sub.updatedAt = new Date().toISOString();
    store.set(subscriptionId, sub);
    return sub;
  }

  getExecutionHistory(subscriptionId: string): ExecutionRecord[] {
    return executionRecords
      .filter((e) => e.subscriptionId === subscriptionId)
      .sort((a, b) => b.executedAt.localeCompare(a.executedAt));
  }

  getSubscriptionAnalytics(userAddress: string): SubscriptionAnalytics[] {
    const subs = this.listSubscriptions(userAddress);
    return subs.map((sub) => {
      const execs = executionRecords.filter((e) => e.subscriptionId === sub.id);
      const avgExecTimeMs =
        execs.length > 0
          ? execs.reduce((sum, e) => sum + (e.retryNumber || 0), 0) * 1000
          : 0;
      const lastExec = execs[0];
      const daysSinceLastExec = lastExec
        ? Math.floor(
            (Date.now() - new Date(lastExec.executedAt).getTime()) / 86400000
          )
        : undefined;

      return {
        subscriptionId: sub.id,
        userAddress: sub.userAddress,
        action: sub.action,
        totalExecutions: sub.totalExecutions,
        successfulExecutions: sub.successfulExecutions,
        failedExecutions: sub.failedExecutions,
        successRate:
          sub.totalExecutions > 0
            ? (sub.successfulExecutions / sub.totalExecutions) * 100
            : 0,
        totalAmountProcessed: sub.totalAmountProcessed,
        averageExecutionTimeMs: avgExecTimeMs,
        lastExecutionStatus: lastExec?.status,
        daysSinceLastExecution: daysSinceLastExec,
      };
    });
  }

  // ─── Keeper execution engine ────────────────────────────────────────────────

  async executeDueSubscriptions(): Promise<ExecutionRecord[]> {
    const now = new Date();
    const due: RecurringSubscription[] = [];

    for (const [, store] of subscriptionsByUser) {
      for (const sub of store.values()) {
        if (
          sub.status === 'active' &&
          new Date(sub.nextExecutionAt) <= now &&
          (!sub.endDate || new Date(sub.endDate) > now)
        ) {
          due.push(sub);
        }
      }
    }

    const results: ExecutionRecord[] = [];
    for (const sub of due) {
      const result = await this.executeSingleSubscription(sub, now);
      results.push(result);
    }

    return results;
  }

  private async executeSingleSubscription(
    sub: RecurringSubscription,
    now: Date
  ): Promise<ExecutionRecord> {
    const executionId = generateExecutionId();
    const record: ExecutionRecord = {
      id: executionId,
      subscriptionId: sub.id,
      userAddress: sub.userAddress,
      action: sub.action,
      amount: sub.amount,
      status: 'pending',
      executedAt: now.toISOString(),
      retryNumber: sub.retryCount,
    };

    sub.totalExecutions++;

    try {
      record.status = 'executing';
      const stellarService = new StellarService();
      const txResult = await stellarService.executeRecurringOperation(
        sub.userAddress,
        sub.action,
        sub.amount,
        sub.assetAddress
      );

      if (txResult.success) {
        record.status = 'success';
        record.transactionHash = txResult.transactionHash;
        sub.successfulExecutions++;
        sub.retryCount = 0;
        sub.totalAmountProcessed = (
          BigInt(sub.totalAmountProcessed) + BigInt(sub.amount)
        ).toString();
        sub.lastExecutionAt = now.toISOString();
      } else if (txResult.error?.includes('insufficient')) {
        record.status = 'insufficient_balance';
        sub.failedExecutions++;
      } else {
        record.status = 'failed';
        record.errorMessage = txResult.error;
        sub.failedExecutions++;
        sub.retryCount++;
      }
    } catch (error) {
      record.status = 'failed';
      record.errorMessage = (error as Error).message;
      sub.failedExecutions++;
      sub.retryCount++;
    }

    // Retry logic
    if (record.status === 'failed' && sub.retryCount <= sub.maxRetries) {
      const backoffMs = config.subscriptions.retryBackoffMs * sub.retryCount;
      sub.nextExecutionAt = new Date(now.getTime() + backoffMs).toISOString();
    } else if (record.status === 'insufficient_balance' && sub.retryCount <= sub.maxRetries) {
      sub.nextExecutionAt = new Date(now.getTime() + config.subscriptions.retryBackoffMs).toISOString();
      sub.retryCount++;
    } else {
      sub.nextExecutionAt = computeNextExecution(now, sub.interval, sub.frequency).toISOString();
    }

    sub.updatedAt = now.toISOString();
    executionRecords.unshift(record);

    if (executionRecords.length > 10000) {
      executionRecords.length = 10000;
    }

    logger.info('Subscription execution completed', {
      subscriptionId: sub.id,
      action: sub.action,
      status: record.status,
      retryCount: sub.retryCount,
    });

    return record;
  }

  // ─── Keeper lifecycle ──────────────────────────────────────────────────────

  startKeeper(): void {
    if (keeperIntervalId) return;
    logger.info('Subscription keeper started', {
      intervalMs: config.subscriptions.executionIntervalMs,
    });
    keeperIntervalId = setInterval(() => {
      this.executeDueSubscriptions().catch((err) =>
        logger.error('Subscription keeper cycle failed', { error: err.message })
      );
    }, config.subscriptions.executionIntervalMs);
  }

  stopKeeper(): void {
    if (keeperIntervalId) {
      clearInterval(keeperIntervalId);
      keeperIntervalId = undefined;
      logger.info('Subscription keeper stopped');
    }
  }

  async triggerManualExecution(subscriptionId: string, userAddress: string): Promise<ExecutionRecord> {
    const sub = this.getSubscription(userAddress, subscriptionId);
    if (!sub) throw new ValidationError('Subscription not found');
    if (sub.status !== 'active') throw new ValidationError('Subscription is not active');
    return this.executeSingleSubscription(sub, new Date());
  }

  // ─── Import/Export (preserved) ──────────────────────────────────────────────

  validateImport(input: ImportData): ValidationResult {
    const existingSubscriptions = this.listMerchantSubscriptions(input.merchantId);
    return validateImport(input, existingSubscriptions);
  }

  previewImport(input: ImportData): ValidationResult {
    return this.validateImport(input);
  }

  importSubscriptions(input: ImportData): ImportResult {
    const existingSubscriptions = this.listMerchantSubscriptions(input.merchantId);
    const result = importSubscriptions(input, existingSubscriptions);
    appendHistory(result.historyEntry);
    if (result.historyEntry.status === 'completed') {
      const merchantStore = getMerchantStore(input.merchantId);
      result.appliedSubscriptions.forEach((subscription) => {
        merchantStore.set(subscription.subscriptionId, subscription);
      });
    }
    return {
      merchantId: result.merchantId,
      importId: result.importId,
      importedCount: result.importedCount,
      updatedCount: result.updatedCount,
      skippedCount: result.skippedCount,
      errorCount: result.errorCount,
      errors: result.errors,
      warnings: result.warnings,
      historyEntry: result.historyEntry,
    };
  }

  exportSubscriptions(merchantId: string): ExportData {
    return exportSubscriptions(merchantId, this.listMerchantSubscriptions(merchantId));
  }

  listMerchantSubscriptions(merchantId: string): SubscriptionRecord[] {
    return Array.from(getMerchantStore(merchantId).values()).sort((left, right) =>
      left.subscriptionId.localeCompare(right.subscriptionId)
    );
  }

  getImportHistory(merchantId: string): ImportHistoryEntry[] {
    return [...(importHistoryByMerchant.get(merchantId) ?? [])];
  }
}

// ─── Reset for testing ─────────────────────────────────────────────────────────

export function resetSubscriptionStore(): void {
  subscriptionsByUser.clear();
  subscriptionsByMerchant.clear();
  importHistoryByMerchant.clear();
  executionRecords.length = 0;
}
