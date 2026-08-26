import {
  ExportData,
  ImportData,
  ImportHistoryEntry,
  ImportResult,
  SubscriptionRecord,
  ValidationResult,
} from '../types';
import { exportSubscriptions, importSubscriptions, validateImport } from '../utils/importExport';

const subscriptionsByMerchant = new Map<string, Map<string, SubscriptionRecord>>();
const importHistoryByMerchant = new Map<string, ImportHistoryEntry[]>();

function getMerchantStore(merchantId: string): Map<string, SubscriptionRecord> {
  const existing = subscriptionsByMerchant.get(merchantId);
  if (existing) {
    return existing;
  }

  const created = new Map<string, SubscriptionRecord>();
  subscriptionsByMerchant.set(merchantId, created);
  return created;
}

function appendHistory(entry: ImportHistoryEntry): void {
  const existing = importHistoryByMerchant.get(entry.merchantId) ?? [];
  importHistoryByMerchant.set(entry.merchantId, [entry, ...existing]);
}

export class SubscriptionService {
  validateImport(input: ImportData): ValidationResult {
    const existingSubscriptions = this.listSubscriptions(input.merchantId);
    return validateImport(input, existingSubscriptions);
  }

  previewImport(input: ImportData): ValidationResult {
    return this.validateImport(input);
  }

  importSubscriptions(input: ImportData): ImportResult {
    const existingSubscriptions = this.listSubscriptions(input.merchantId);
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
    return exportSubscriptions(merchantId, this.listSubscriptions(merchantId));
  }

  listSubscriptions(merchantId: string): SubscriptionRecord[] {
    return Array.from(getMerchantStore(merchantId).values()).sort((left, right) =>
      left.subscriptionId.localeCompare(right.subscriptionId)
    );
  }

  getImportHistory(merchantId: string): ImportHistoryEntry[] {
    return [...(importHistoryByMerchant.get(merchantId) ?? [])];
  }
}

export function resetSubscriptionStore(): void {
  subscriptionsByMerchant.clear();
  importHistoryByMerchant.clear();
}
