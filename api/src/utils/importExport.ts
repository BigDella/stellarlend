import {
  ExportData,
  ImportAction,
  ImportData,
  ImportPreviewRow,
  ImportResult,
  ImportRowError,
  ImportRowWarning,
  SubscriptionRecord,
  SUBSCRIPTION_INTERVALS,
  SUBSCRIPTION_STATUSES,
  ValidationResult,
} from '../types';

const REQUIRED_FIELDS = [
  'subscriptionId',
  'customerEmail',
  'planId',
  'status',
  'amount',
  'currency',
  'interval',
  'startDate',
] as const;

const CANONICAL_FIELDS = [...REQUIRED_FIELDS, 'nextBillingDate', 'metadata'] as const;

type CanonicalField = (typeof CANONICAL_FIELDS)[number];
type RawImportRow = Record<string, unknown>;
type DraftSubscription = Omit<SubscriptionRecord, 'createdAt' | 'updatedAt'>;

const DEFAULT_PREVIEW_LIMIT = 25;
const MAX_PREVIEW_LIMIT = 100;
const MAX_IMPORT_ROWS = 5000;

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function normalizeString(value: unknown): string {
  return String(value ?? '').trim();
}

function isCanonicalField(value: string): value is CanonicalField {
  return (CANONICAL_FIELDS as readonly string[]).includes(value);
}

function normalizeRowKeys(
  row: RawImportRow,
  columnMapping: Record<string, string> = {}
): RawImportRow {
  const normalized: RawImportRow = {};

  for (const [key, value] of Object.entries(row)) {
    const trimmedKey = key.trim();
    const mappedKey = columnMapping[trimmedKey] ?? trimmedKey;

    if (isCanonicalField(mappedKey)) {
      normalized[mappedKey] = value;
    }
  }

  return normalized;
}

function parseCsvLine(line: string): string[] {
  const values: string[] = [];
  let current = '';
  let inQuotes = false;

  for (let index = 0; index < line.length; index += 1) {
    const char = line[index];
    const nextChar = line[index + 1];

    if (char === '"') {
      if (inQuotes && nextChar === '"') {
        current += '"';
        index += 1;
        continue;
      }

      inQuotes = !inQuotes;
      continue;
    }

    if (char === ',' && !inQuotes) {
      values.push(current);
      current = '';
      continue;
    }

    current += char;
  }

  values.push(current);
  return values.map((value) => value.trim());
}

function parseCsvRows(csv: string, columnMapping?: Record<string, string>): RawImportRow[] {
  const lines = csv
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  if (lines.length === 0) {
    return [];
  }

  const firstLine = lines[0]!;
  const headers = parseCsvLine(firstLine);

  return lines.slice(1).map((line) => {
    const values = parseCsvLine(line);
    const row: RawImportRow = {};

    headers.forEach((header, index) => {
      row[header] = values[index] ?? '';
    });

    return normalizeRowKeys(row, columnMapping);
  });
}

function parseJsonRows(
  input: string | unknown[],
  columnMapping?: Record<string, string>
): RawImportRow[] {
  let parsed: unknown;

  if (typeof input === 'string') {
    try {
      parsed = JSON.parse(input);
    } catch {
      throw new Error('Invalid JSON import payload');
    }
  } else {
    parsed = input;
  }

  if (!Array.isArray(parsed)) {
    throw new Error('JSON import payload must be an array of subscription objects');
  }

  return parsed.map((row) => {
    if (!isObject(row)) {
      throw new Error('Each JSON import row must be an object');
    }

    return normalizeRowKeys(row, columnMapping);
  });
}

function normalizeDate(
  value: unknown,
  field: string,
  rowNumber: number,
  errors: ImportRowError[]
): string | undefined {
  const raw = normalizeString(value);
  if (!raw) {
    return undefined;
  }

  const date = new Date(raw);
  if (Number.isNaN(date.getTime())) {
    errors.push({ rowNumber, field, message: `${field} must be a valid ISO date` });
    return undefined;
  }

  return date.toISOString();
}

function normalizeMetadata(
  value: unknown,
  rowNumber: number,
  errors: ImportRowError[]
): Record<string, unknown> | undefined {
  if (value === undefined || value === null || value === '') {
    return undefined;
  }

  if (isObject(value)) {
    return value;
  }

  const raw = normalizeString(value);
  try {
    const parsed = JSON.parse(raw);
    if (!isObject(parsed)) {
      throw new Error('Metadata must be an object');
    }
    return parsed;
  } catch {
    errors.push({
      rowNumber,
      field: 'metadata',
      message: 'metadata must be a valid JSON object',
    });
    return undefined;
  }
}

function normalizeAmount(
  value: unknown,
  rowNumber: number,
  errors: ImportRowError[]
): number | undefined {
  const raw = normalizeString(value);
  if (!raw) {
    errors.push({ rowNumber, field: 'amount', message: 'amount is required' });
    return undefined;
  }

  const amount = Number(raw);
  if (!Number.isFinite(amount) || amount < 0) {
    errors.push({
      rowNumber,
      field: 'amount',
      message: 'amount must be a valid non-negative number',
    });
    return undefined;
  }

  return amount;
}

function normalizeSubscriptionRow(
  merchantId: string,
  row: RawImportRow,
  rowNumber: number
): { subscription?: DraftSubscription; errors: ImportRowError[] } {
  const errors: ImportRowError[] = [];

  const subscriptionId = normalizeString(row.subscriptionId);
  if (!subscriptionId) {
    errors.push({ rowNumber, field: 'subscriptionId', message: 'subscriptionId is required' });
  }

  const customerEmail = normalizeString(row.customerEmail).toLowerCase();
  if (!customerEmail) {
    errors.push({ rowNumber, field: 'customerEmail', message: 'customerEmail is required' });
  } else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(customerEmail)) {
    errors.push({
      rowNumber,
      field: 'customerEmail',
      message: 'customerEmail must be a valid email address',
    });
  }

  const planId = normalizeString(row.planId);
  if (!planId) {
    errors.push({ rowNumber, field: 'planId', message: 'planId is required' });
  }

  const status = normalizeString(row.status).toLowerCase();
  if (!status) {
    errors.push({ rowNumber, field: 'status', message: 'status is required' });
  } else if (!SUBSCRIPTION_STATUSES.includes(status as (typeof SUBSCRIPTION_STATUSES)[number])) {
    errors.push({
      rowNumber,
      field: 'status',
      message: `status must be one of: ${SUBSCRIPTION_STATUSES.join(', ')}`,
    });
  }

  const currency = normalizeString(row.currency).toUpperCase();
  if (!currency) {
    errors.push({ rowNumber, field: 'currency', message: 'currency is required' });
  } else if (!/^[A-Z]{3}$/.test(currency)) {
    errors.push({
      rowNumber,
      field: 'currency',
      message: 'currency must be a 3-letter ISO code',
    });
  }

  const interval = normalizeString(row.interval).toLowerCase();
  if (!interval) {
    errors.push({ rowNumber, field: 'interval', message: 'interval is required' });
  } else if (
    !SUBSCRIPTION_INTERVALS.includes(interval as (typeof SUBSCRIPTION_INTERVALS)[number])
  ) {
    errors.push({
      rowNumber,
      field: 'interval',
      message: `interval must be one of: ${SUBSCRIPTION_INTERVALS.join(', ')}`,
    });
  }

  const startDate = normalizeDate(row.startDate, 'startDate', rowNumber, errors);
  if (!normalizeString(row.startDate)) {
    errors.push({ rowNumber, field: 'startDate', message: 'startDate is required' });
  }

  const nextBillingDate = normalizeDate(row.nextBillingDate, 'nextBillingDate', rowNumber, errors);
  const metadata = normalizeMetadata(row.metadata, rowNumber, errors);
  const amount = normalizeAmount(row.amount, rowNumber, errors);

  if (errors.length > 0 || amount === undefined || !startDate) {
    return { errors };
  }

  return {
    subscription: {
      merchantId,
      subscriptionId,
      customerEmail,
      planId,
      status: status as DraftSubscription['status'],
      amount,
      currency,
      interval: interval as DraftSubscription['interval'],
      startDate,
      nextBillingDate,
      metadata,
    },
    errors,
  };
}

function subscriptionsMatch(
  left: DraftSubscription,
  right: Omit<SubscriptionRecord, 'createdAt' | 'updatedAt'>
): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function getPreviewLimit(rawLimit?: number): number {
  if (!rawLimit || !Number.isFinite(rawLimit) || rawLimit <= 0) {
    return DEFAULT_PREVIEW_LIMIT;
  }

  return Math.min(Math.floor(rawLimit), MAX_PREVIEW_LIMIT);
}

function getRawRows(input: ImportData): RawImportRow[] {
  const rows =
    input.format === 'csv'
      ? parseCsvRows(String(input.data ?? ''), input.columnMapping)
      : parseJsonRows(input.data, input.columnMapping);

  if (rows.length > MAX_IMPORT_ROWS) {
    throw new Error(`Import payload exceeds the maximum of ${MAX_IMPORT_ROWS} rows`);
  }

  return rows;
}

export function validateImport(
  input: ImportData,
  existingSubscriptions: SubscriptionRecord[] = []
): ValidationResult {
  const errors: ImportRowError[] = [];
  const warnings: ImportRowWarning[] = [];
  const normalizedRows: ImportPreviewRow[] = [];
  const existingById = new Map(existingSubscriptions.map((item) => [item.subscriptionId, item]));
  const seenSubscriptionIds = new Set<string>();
  const rows = getRawRows(input);
  const previewLimit = getPreviewLimit(input.options?.previewLimit);
  const upsert = input.options?.upsert !== false;

  rows.forEach((row, index) => {
    const rowNumber = input.format === 'csv' ? index + 2 : index + 1;
    const { subscription, errors: rowErrors } = normalizeSubscriptionRow(
      input.merchantId,
      row,
      rowNumber
    );

    if (rowErrors.length > 0 || !subscription) {
      errors.push(...rowErrors);
      return;
    }

    if (seenSubscriptionIds.has(subscription.subscriptionId)) {
      errors.push({
        rowNumber,
        field: 'subscriptionId',
        message: 'Duplicate subscriptionId found in import payload',
      });
      return;
    }

    seenSubscriptionIds.add(subscription.subscriptionId);

    const existing = existingById.get(subscription.subscriptionId);
    let action: ImportAction = 'create';

    if (existing) {
      if (!upsert) {
        errors.push({
          rowNumber,
          field: 'subscriptionId',
          message: 'subscriptionId already exists for this merchant',
        });
        return;
      }

      const comparableExisting: DraftSubscription = {
        merchantId: existing.merchantId,
        subscriptionId: existing.subscriptionId,
        customerEmail: existing.customerEmail,
        planId: existing.planId,
        status: existing.status,
        amount: existing.amount,
        currency: existing.currency,
        interval: existing.interval,
        startDate: existing.startDate,
        nextBillingDate: existing.nextBillingDate,
        metadata: existing.metadata,
      };

      action = subscriptionsMatch(subscription, comparableExisting) ? 'skip' : 'update';
      warnings.push({
        rowNumber,
        field: 'subscriptionId',
        message:
          action === 'skip'
            ? 'Existing subscription is unchanged and will be skipped'
            : 'Existing subscription will be updated',
      });
    }

    normalizedRows.push({ rowNumber, action, subscription });
  });

  const invalidRows = new Set(errors.map((item) => item.rowNumber)).size;
  const creates = normalizedRows.filter((item) => item.action === 'create').length;
  const updates = normalizedRows.filter((item) => item.action === 'update').length;
  const skips = normalizedRows.filter((item) => item.action === 'skip').length;

  return {
    merchantId: input.merchantId,
    format: input.format,
    isValid: errors.length === 0,
    summary: {
      totalRows: rows.length,
      validRows: normalizedRows.length,
      invalidRows,
      creates,
      updates,
      skips,
    },
    errors,
    warnings,
    previewRows: normalizedRows.slice(0, previewLimit),
    normalizedRows,
  };
}

export function importSubscriptions(
  input: ImportData,
  existingSubscriptions: SubscriptionRecord[] = []
): ImportResult & { appliedSubscriptions: SubscriptionRecord[] } {
  const validation = validateImport(input, existingSubscriptions);
  const createdAt = new Date().toISOString();
  const importId = `imp_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

  if (!validation.isValid) {
    return {
      merchantId: input.merchantId,
      importId,
      importedCount: 0,
      updatedCount: 0,
      skippedCount: 0,
      errorCount: validation.errors.length,
      errors: validation.errors,
      warnings: validation.warnings,
      appliedSubscriptions: existingSubscriptions,
      historyEntry: {
        importId,
        merchantId: input.merchantId,
        format: input.format,
        createdAt,
        totalRows: validation.summary.totalRows,
        importedCount: 0,
        updatedCount: 0,
        skippedCount: 0,
        errorCount: validation.errors.length,
        status: 'failed',
      },
    };
  }

  const subscriptionMap = new Map(existingSubscriptions.map((item) => [item.subscriptionId, item]));
  let importedCount = 0;
  let updatedCount = 0;
  let skippedCount = 0;

  validation.normalizedRows.forEach(({ action, subscription }) => {
    if (action === 'skip') {
      skippedCount += 1;
      return;
    }

    if (action === 'update') {
      const existing = subscriptionMap.get(subscription.subscriptionId);
      subscriptionMap.set(subscription.subscriptionId, {
        ...existing!,
        ...subscription,
        updatedAt: createdAt,
      });
      updatedCount += 1;
      return;
    }

    subscriptionMap.set(subscription.subscriptionId, {
      ...subscription,
      createdAt,
      updatedAt: createdAt,
    });
    importedCount += 1;
  });

  const appliedSubscriptions = Array.from(subscriptionMap.values()).sort((left, right) =>
    left.subscriptionId.localeCompare(right.subscriptionId)
  );

  return {
    merchantId: input.merchantId,
    importId,
    importedCount,
    updatedCount,
    skippedCount,
    errorCount: 0,
    errors: [],
    warnings: validation.warnings,
    appliedSubscriptions,
    historyEntry: {
      importId,
      merchantId: input.merchantId,
      format: input.format,
      createdAt,
      totalRows: validation.summary.totalRows,
      importedCount,
      updatedCount,
      skippedCount,
      errorCount: 0,
      status: 'completed',
    },
  };
}

export function exportSubscriptions(
  merchantId: string,
  subscriptions: SubscriptionRecord[]
): ExportData {
  const orderedSubscriptions = [...subscriptions].sort((left, right) =>
    left.subscriptionId.localeCompare(right.subscriptionId)
  );

  return {
    merchantId,
    exportedAt: new Date().toISOString(),
    format: 'json',
    count: orderedSubscriptions.length,
    subscriptions: orderedSubscriptions,
  };
}
