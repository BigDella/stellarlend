/**
 * DTOs for recurring subscription endpoints.
 *
 * @openapi
 * components:
 *   schemas:
 *     CreateSubscriptionDto:
 *       type: object
 *       required: [userAddress, action, amount, interval]
 *       properties:
 *         userAddress:
 *           type: string
 *         action:
 *           type: string
 *           enum: [deposit, borrow, repay]
 *         amount:
 *           type: string
 *         interval:
 *           type: string
 *           enum: [daily, weekly, monthly, quarterly, yearly]
 *         frequency:
 *           type: integer
 *           minimum: 1
 *           maximum: 365
 *         startDate:
 *           type: string
 *           format: date-time
 *         endDate:
 *           type: string
 *           format: date-time
 *         assetAddress:
 *           type: string
 *         maxRetries:
 *           type: integer
 *           minimum: 0
 *           maximum: 100
 */

import { FieldError, ValidationResult, isValidStellarAddress, isValidAmount } from './base.dto';

const VALID_ACTIONS = ['deposit', 'borrow', 'repay'] as const;
const VALID_INTERVALS = ['daily', 'weekly', 'monthly', 'quarterly', 'yearly'] as const;

export type SubscriptionAction = (typeof VALID_ACTIONS)[number];
export type SubscriptionInterval = (typeof VALID_INTERVALS)[number];

export class CreateSubscriptionDto {
  readonly userAddress!: string;
  readonly action!: SubscriptionAction;
  readonly amount!: string;
  readonly interval!: SubscriptionInterval;
  readonly frequency?: number;
  readonly startDate?: string;
  readonly endDate?: string;
  readonly assetAddress?: string;
  readonly maxRetries?: number;

  private constructor(data: {
    userAddress: string;
    action: SubscriptionAction;
    amount: string;
    interval: SubscriptionInterval;
    frequency?: number;
    startDate?: string;
    endDate?: string;
    assetAddress?: string;
    maxRetries?: number;
  }) {
    Object.assign(this, data);
  }

  static validate(body: Record<string, unknown>): ValidationResult {
    const errors: FieldError[] = [];

    if (!isValidStellarAddress(body.userAddress)) {
      errors.push({ field: 'userAddress', message: 'Must be a valid Stellar Ed25519 public key' });
    }
    if (!VALID_ACTIONS.includes(body.action as SubscriptionAction)) {
      errors.push({
        field: 'action',
        message: `Must be one of: ${VALID_ACTIONS.join(', ')}`,
      });
    }
    if (!isValidAmount(body.amount)) {
      errors.push({
        field: 'amount',
        message: 'Must be a positive integer not exceeding i128 max',
      });
    }
    if (!VALID_INTERVALS.includes(body.interval as SubscriptionInterval)) {
      errors.push({
        field: 'interval',
        message: `Must be one of: ${VALID_INTERVALS.join(', ')}`,
      });
    }
    if (body.frequency != null) {
      const freq = Number(body.frequency);
      if (!Number.isInteger(freq) || freq < 1 || freq > 365) {
        errors.push({ field: 'frequency', message: 'Must be an integer between 1 and 365' });
      }
    }
    if (body.startDate != null && isNaN(Date.parse(String(body.startDate)))) {
      errors.push({ field: 'startDate', message: 'Must be a valid ISO 8601 date string' });
    }
    if (body.endDate != null && isNaN(Date.parse(String(body.endDate)))) {
      errors.push({ field: 'endDate', message: 'Must be a valid ISO 8601 date string' });
    }
    if (body.maxRetries != null) {
      const r = Number(body.maxRetries);
      if (!Number.isInteger(r) || r < 0 || r > 100) {
        errors.push({ field: 'maxRetries', message: 'Must be an integer between 0 and 100' });
      }
    }

    return new ValidationResult(errors);
  }

  static fromBody(body: Record<string, unknown>): CreateSubscriptionDto {
    return new CreateSubscriptionDto({
      userAddress: String(body.userAddress ?? ''),
      action: body.action as SubscriptionAction,
      amount: String(body.amount ?? ''),
      interval: body.interval as SubscriptionInterval,
      frequency: body.frequency != null ? Number(body.frequency) : undefined,
      startDate: body.startDate != null ? String(body.startDate) : undefined,
      endDate: body.endDate != null ? String(body.endDate) : undefined,
      assetAddress: body.assetAddress != null ? String(body.assetAddress) : undefined,
      maxRetries: body.maxRetries != null ? Number(body.maxRetries) : undefined,
    });
  }
}
