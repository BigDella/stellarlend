import type { LedgerIntegrityResult } from '../types.js';

/**
 * Verify archived event counts against expected per-ledger totals.
 * When expectedCount is unknown (null), integrity is considered OK if archivedCount >= 0.
 */
export function verifyLedgerIntegrity(
  ledger: number,
  archivedCount: number,
  expectedCount: number | null
): LedgerIntegrityResult {
  const integrityOk =
    expectedCount === null ? archivedCount >= 0 : archivedCount === expectedCount;

  return {
    ledger,
    expectedCount,
    archivedCount,
    integrityOk,
  };
}

export function summarizeIntegrity(results: LedgerIntegrityResult[]): {
  checked: number;
  failed: number;
  failedLedgers: number[];
} {
  const failedLedgers = results.filter((r) => !r.integrityOk).map((r) => r.ledger);
  return {
    checked: results.length,
    failed: failedLedgers.length,
    failedLedgers,
  };
}
