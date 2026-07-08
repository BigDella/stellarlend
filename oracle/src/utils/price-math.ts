/**
 * Shared Price Math Utilities
 *
 * Common mathematical operations used across oracle services:
 * - Price deviation calculation (basis points)
 * - Median computation
 * - Price normalization
 */

/**
 * Calculate price deviation in basis points between a reference price
 * and an observed price.
 *
 * Formula: |observed - reference| / reference * 10000
 *
 * Returns the deviation as a bigint to preserve precision for large values.
 * For display, convert to Number only at the boundary.
 *
 * @param reference The reference/baseline price (must be > 0)
 * @param observed The observed/comparison price (must be > 0)
 * @returns Deviation in basis points, or 0n if either price is non-positive
 */
export function calculateDeviationBps(reference: bigint, observed: bigint): bigint {
  if (reference <= 0n || observed <= 0n) {
    return 0n;
  }

  const diff = observed > reference ? observed - reference : reference - observed;
  return (diff * 10000n) / reference;
}

/**
 * Convert bigint deviation bps to a safe Number for display/metrics.
 * Caps at Number.MAX_SAFE_INTEGER to avoid precision loss.
 */
export function deviationBpsToNumber(bps: bigint): number {
  if (bps > BigInt(Number.MAX_SAFE_INTEGER)) {
    return Number.MAX_SAFE_INTEGER;
  }
  return Number(bps);
}

/**
 * Calculate median of a sorted array of bigint values.
 */
export function medianBigInt(values: bigint[]): bigint | null {
  if (values.length === 0) return null;

  const sorted = [...values].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
  const mid = Math.floor(sorted.length / 2);

  if (sorted.length % 2 === 0) {
    return (sorted[mid - 1]! + sorted[mid]!) / 2n;
  }

  return sorted[mid]!;
}

/**
 * Calculate mean (average) of bigint values.
 */
export function meanBigInt(values: bigint[]): bigint | null {
  if (values.length === 0) return null;
  const sum = values.reduce((acc, v) => acc + v, 0n);
  return sum / BigInt(values.length);
}
