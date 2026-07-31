export interface Position {
  userAddress: string;
  assetAddress?: string;
  collateral: string;
  debt: string;
  borrowInterest: string;
  lastAccrualTime: number;
  collateralRatio?: string;
  healthFactor?: string;
  liquidationThreshold?: string;
}

export interface PositionImportRow {
  userAddress: string;
  assetAddress?: string;
  collateral: string;
  debt: string;
  action: 'create' | 'update' | 'skip';
}

export interface PositionImportOptions {
  validateOnly?: boolean;
  allowUpdates?: boolean;
  previewLimit?: number;
  batchSize?: number;
}

export interface PositionImportData {
  format: 'csv' | 'json';
  data: string | PositionImportRow[];
  columnMapping?: Record<string, string>;
  options?: PositionImportOptions;
}

export interface PositionImportError {
  rowNumber: number;
  field: string;
  message: string;
  userAddress?: string;
}

export interface PositionImportWarning {
  rowNumber: number;
  field: string;
  message: string;
  userAddress?: string;
}

export interface PositionValidationResult {
  isValid: boolean;
  summary: {
    totalRows: number;
    validRows: number;
    invalidRows: number;
    creates: number;
    updates: number;
    skips: number;
  };
  errors: PositionImportError[];
  warnings: PositionImportWarning[];
  previewRows: PositionImportRow[];
}

export interface PositionImportResult {
  importId: string;
  timestamp: string;
  importedCount: number;
  updatedCount: number;
  skippedCount: number;
  errorCount: number;
  errors: PositionImportError[];
  warnings: PositionImportWarning[];
  transactionHashes: string[];
  status: 'completed' | 'partial' | 'failed';
}

export interface PositionExportOptions {
  format: 'csv' | 'json';
  includeZeroBalances?: boolean;
  userAddresses?: string[];
  assetAddress?: string;
}

export interface PositionExportData {
  exportedAt: string;
  format: 'csv' | 'json';
  count: number;
  positions: Position[];
  csvData?: string;
}

export interface PositionImportTemplate {
  format: 'csv' | 'json';
  template: string | Record<string, unknown>[];
  instructions: string;
  requiredFields: string[];
  optionalFields: string[];
  exampleRow: Record<string, unknown>;
}

export interface PositionImportHistory {
  importId: string;
  timestamp: string;
  format: 'csv' | 'json';
  totalRows: number;
  importedCount: number;
  updatedCount: number;
  skippedCount: number;
  errorCount: number;
  status: 'completed' | 'partial' | 'failed';
  canRollback: boolean;
}
