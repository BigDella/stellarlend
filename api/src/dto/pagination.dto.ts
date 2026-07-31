/**
 * Shared pagination DTO used by list endpoints.
 *
 * @openapi
 * components:
 *   schemas:
 *     PaginationQuery:
 *       type: object
 *       properties:
 *         limit:
 *           type: integer
 *           minimum: 1
 *           maximum: 100
 *         cursor:
 *           type: string
 *           maxLength: 256
 */

import { FieldError, ValidationResult } from './base.dto';

const DEFAULT_LIMIT = 20;
const MAX_LIMIT_DEFAULT = 100;

export class PaginationQueryDto {
  readonly limit: number;
  readonly cursor?: string;

  private constructor(data: { limit: number; cursor?: string }) {
    this.limit = data.limit;
    this.cursor = data.cursor;
  }

  static validate(
    query: Record<string, unknown>,
    maxLimit = MAX_LIMIT_DEFAULT,
  ): ValidationResult {
    const errors: FieldError[] = [];

    if (query.limit != null) {
      const l = Number(query.limit);
      if (!Number.isInteger(l) || l < 1 || l > maxLimit) {
        errors.push({ field: 'limit', message: `Must be a positive integer at most ${maxLimit}` });
      }
    }
    if (query.cursor != null) {
      const c = String(query.cursor);
      if (c.trim() === '' || c.length > 256) {
        errors.push({ field: 'cursor', message: 'Must be a non-empty string ≤ 256 characters' });
      }
    }

    return new ValidationResult(errors);
  }

  static fromQuery(
    query: Record<string, unknown>,
    maxLimit = MAX_LIMIT_DEFAULT,
  ): PaginationQueryDto {
    const rawLimit = query.limit != null ? Number(query.limit) : DEFAULT_LIMIT;
    return new PaginationQueryDto({
      limit: Math.min(Math.max(rawLimit, 1), maxLimit),
      cursor: query.cursor != null ? String(query.cursor) : undefined,
    });
  }
}
