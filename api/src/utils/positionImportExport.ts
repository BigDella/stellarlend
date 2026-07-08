import {
  Position,
  PositionImportData,
  PositionImportRow,
  PositionImportError,
  PositionImportWarning,
  PositionValidationResult,
  PositionImportResult,
  PositionExportData,
  PositionExportOptions,
  PositionImportTemplate,
  PositionImportHistory,
} from '../types/positions';

const REQUIRED_FIELDS = ['userAddress', 'collateral', 'debt'] as const;
const OPTIONAL_FIELDS = ['assetAddress'] as const;
const MAX_IMPORT_ROWS = 5000;
const DEFAULT_PREVIEW_LIMIT = 25;
const MAX_PREVIEW_LIMIT = 100;
const DEFAULT_BATCH_SIZE = 50;

type RawImportRow = Record<string, unknown>;

function isValidAddress(address: string): boolean {
  return /^G[A-Z0-9]{55}$/.test(address) || /^C[A-Z0-9]{55}$/.test(address);
}

function normalizeString(value: unknown): string {
  return String(value ?? '').trim();
}

function parseCsvLine(line: string): string[] {
  const values: string[] = [];
  let current = '';
  let inQuotes = false;

  for (let i = 0; i < line.length; i++) {
    const char = line[i];
    const nextChar = line[i + 1];

    if (char === '"') {
      if (inQuotes && nextChar === '"') {
        current += '"';
        i++;
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
  return values.map((v) => v.trim());
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
      const mappedKey = columnMapping?.[header] ?? header;
      row[mappedKey] = values[index] ?? '';
    });

    return row;
  });
}

function parseJsonRows(input: string | unknown[], columnMapping?: Record<string, string>): RawImportRow[] {
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
    throw new Error('JSON import payload must be an array');
  }

  return parsed.map((row) => {
    if (typeof row !== 'object' || row === null) {
      throw new Error('Each JSON row must be an object');
    }

    const normalized: RawImportRow = {};
    for (const [key, value] of Object.entries(row)) {
      const mappedKey = columnMapping?.[key] ?? key;
      normalized[mappedKey] = value;
    }

    return normalized;
  });
}

function normalizePositionRow(
  row: RawImportRow,
  rowNumber: number
): { position?: PositionImportRow; errors: PositionImportError[] } {
  const errors: PositionImportError[] = [];

  const userAddress = normalizeString(row.userAddress);
  if (!userAddress) {
    errors.push({ rowNumber, field: 'userAddress', message: 'userAddress is required' });
  } else if (!isValidAddress(userAddress)) {
    errors.push({ rowNumber, field: 'userAddress', message: 'Invalid Stellar address format', userAddress });
  }

  const collateral = normalizeString(row.collateral);
  if (!collateral) {
    errors.push({ rowNumber, field: 'collateral', message: 'collateral is required', userAddress });
  } else if (isNaN(Number(collateral)) || Number(collateral) < 0) {
    errors.push({ rowNumber, field: 'collateral', message: 'collateral must be a non-negative number', userAddress });
  }

  const debt = normalizeString(row.debt);
  if (!debt) {
    errors.push({ rowNumber, field: 'debt', message: 'debt is required', userAddress });
  } else if (isNaN(Number(debt)) || Number(debt) < 0) {
    errors.push({ rowNumber, field: 'debt', message: 'debt must be a non-negative number', userAddress });
  }

  const assetAddress = normalizeString(row.assetAddress);
  if (assetAddress && !isValidAddress(assetAddress)) {
    errors.push({ rowNumber, field: 'assetAddress', message: 'Invalid asset address format', userAddress });
  }

  if (errors.length > 0) {
    return { errors };
  }

  return {
    position: {
      userAddress,
      assetAddress: assetAddress || undefined,
      collateral,
      debt,
      action: 'create',
    },
    errors,
  };
}

function getRawRows(input: PositionImportData): RawImportRow[] {
  const rows = input.format === 'csv'
    ? parseCsvRows(String(input.data ?? ''), input.columnMapping)
    : parseJsonRows(input.data, input.columnMapping);

  if (rows.length > MAX_IMPORT_ROWS) {
    throw new Error(`Import exceeds maximum of ${MAX_IMPORT_ROWS} rows`);
  }

  return rows;
}

export function validatePositionImport(
  input: PositionImportData,
  existingPositions: Position[] = []
): PositionValidationResult {
  const errors: PositionImportError[] = [];
  const warnings: PositionImportWarning[] = [];
  const normalizedRows: PositionImportRow[] = [];
  const existingByAddress = new Map(existingPositions.map((p) => [p.userAddress, p]));
  const seenAddresses = new Set<string>();
  const rows = getRawRows(input);
  const previewLimit = Math.min(
    Math.floor(input.options?.previewLimit ?? DEFAULT_PREVIEW_LIMIT),
    MAX_PREVIEW_LIMIT
  );
  const allowUpdates = input.options?.allowUpdates !== false;

  rows.forEach((row, index) => {
    const rowNumber = input.format === 'csv' ? index + 2 : index + 1;
    const { position, errors: rowErrors } = normalizePositionRow(row, rowNumber);

    if (rowErrors.length > 0 || !position) {
      errors.push(...rowErrors);
      return;
    }

    if (seenAddresses.has(position.userAddress)) {
      errors.push({
        rowNumber,
        field: 'userAddress',
        message: 'Duplicate userAddress in import',
        userAddress: position.userAddress,
      });
      return;
    }

    seenAddresses.add(position.userAddress);

    const existing = existingByAddress.get(position.userAddress);
    if (existing) {
      if (!allowUpdates) {
        errors.push({
          rowNumber,
          field: 'userAddress',
          message: 'Position already exists and updates are disabled',
          userAddress: position.userAddress,
        });
        return;
      }

      const unchanged = existing.collateral === position.collateral && existing.debt === position.debt;
      position.action = unchanged ? 'skip' : 'update';
      
      warnings.push({
        rowNumber,
        field: 'userAddress',
        message: position.action === 'skip' 
          ? 'Position unchanged, will be skipped' 
          : 'Existing position will be updated',
        userAddress: position.userAddress,
      });
    }

    normalizedRows.push(position);
  });

  const invalidRows = new Set(errors.map((e) => e.rowNumber)).size;
  const creates = normalizedRows.filter((r) => r.action === 'create').length;
  const updates = normalizedRows.filter((r) => r.action === 'update').length;
  const skips = normalizedRows.filter((r) => r.action === 'skip').length;

  return {
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
  };
}

export function preparePositionImport(
  input: PositionImportData,
  existingPositions: Position[] = []
): PositionImportResult {
  const validation = validatePositionImport(input, existingPositions);
  const timestamp = new Date().toISOString();
  const importId = `imp_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

  if (!validation.isValid) {
    return {
      importId,
      timestamp,
      importedCount: 0,
      updatedCount: 0,
      skippedCount: 0,
      errorCount: validation.errors.length,
      errors: validation.errors,
      warnings: validation.warnings,
      transactionHashes: [],
      status: 'failed',
    };
  }

  return {
    importId,
    timestamp,
    importedCount: validation.summary.creates,
    updatedCount: validation.summary.updates,
    skippedCount: validation.summary.skips,
    errorCount: 0,
    errors: [],
    warnings: validation.warnings,
    transactionHashes: [],
    status: 'completed',
  };
}

export function exportPositions(
  positions: Position[],
  options: Partial<PositionExportOptions> = {}
): PositionExportData {
  const { format = 'json', includeZeroBalances = false, userAddresses, assetAddress } = options as PositionExportOptions;

  let filtered = positions;

  if (!includeZeroBalances) {
    filtered = filtered.filter((p) => Number(p.collateral) > 0 || Number(p.debt) > 0);
  }

  if (userAddresses && userAddresses.length > 0) {
    const addressSet = new Set(userAddresses);
    filtered = filtered.filter((p) => addressSet.has(p.userAddress));
  }

  if (assetAddress) {
    filtered = filtered.filter((p) => p.assetAddress === assetAddress);
  }

  const sorted = [...filtered].sort((a, b) => a.userAddress.localeCompare(b.userAddress));

  let csvData: string | undefined;
  if (format === 'csv') {
    const headers = ['userAddress', 'assetAddress', 'collateral', 'debt', 'borrowInterest', 'collateralRatio'];
    const rows = sorted.map((p) => [
      p.userAddress,
      p.assetAddress || '',
      p.collateral,
      p.debt,
      p.borrowInterest,
      p.collateralRatio || '',
    ]);
    
    csvData = [
      headers.join(','),
      ...rows.map((row) => row.map((val) => `"${val}"`).join(',')),
    ].join('\n');
  }

  return {
    exportedAt: new Date().toISOString(),
    format,
    count: sorted.length,
    positions: sorted,
    csvData,
  };
}

export function generateImportTemplate(format: 'csv' | 'json' = 'csv'): PositionImportTemplate {
  const exampleRow = {
    userAddress: 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF',
    assetAddress: 'CBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB',
    collateral: '1000000',
    debt: '500000',
  };

  let template: string | Record<string, unknown>[];

  if (format === 'csv') {
    template = 'userAddress,assetAddress,collateral,debt\n' +
      'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF,CBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB,1000000,500000';
  } else {
    template = [exampleRow];
  }

  return {
    format,
    template,
    instructions: `Import positions in ${format.toUpperCase()} format. Required fields: userAddress, collateral, debt. Optional: assetAddress.`,
    requiredFields: [...REQUIRED_FIELDS],
    optionalFields: [...OPTIONAL_FIELDS],
    exampleRow,
  };
}

export function createImportHistory(result: PositionImportResult): PositionImportHistory {
  return {
    importId: result.importId,
    timestamp: result.timestamp,
    format: 'json',
    totalRows: result.importedCount + result.updatedCount + result.skippedCount + result.errorCount,
    importedCount: result.importedCount,
    updatedCount: result.updatedCount,
    skippedCount: result.skippedCount,
    errorCount: result.errorCount,
    status: result.status,
    canRollback: result.transactionHashes.length > 0,
  };
}
