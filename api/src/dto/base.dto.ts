/**
 * Base DTO utilities.
 *
 * Provides a lightweight validation framework that mirrors the class-validator
 * API surface without introducing an additional runtime dependency.
 * Each DTO exposes a static `validate()` that returns a list of field errors,
 * and a `fromRequest()` factory that extracts typed fields from an Express
 * request body/query.
 */

export interface FieldError {
  field: string;
  message: string;
}

export class ValidationResult {
  readonly errors: FieldError[];

  constructor(errors: FieldError[]) {
    this.errors = errors;
  }

  get isValid(): boolean {
    return this.errors.length === 0;
  }

  toErrorString(): string {
    return this.errors.map((e) => `${e.field}: ${e.message}`).join(', ');
  }
}

/** Maximum i128 value (2^127 - 1). */
export const MAX_I128 = (1n << 127n) - 1n;

export function isValidStellarAddress(value: unknown): boolean {
  if (typeof value !== 'string') return false;
  if (!/^G[A-Z0-9]{55}$/.test(value)) return false;
  return true;
}

export function isValidAmount(value: unknown): boolean {
  if (value === undefined || value === null) return false;
  try {
    const str = String(value).trim();
    if (!/^\+?\d+$/.test(str)) return false;
    const amount = BigInt(str);
    return amount > 0n && amount <= MAX_I128;
  } catch {
    return false;
  }
}

export function isOptionalString(value: unknown): boolean {
  return value === undefined || value === null || typeof value === 'string';
}
