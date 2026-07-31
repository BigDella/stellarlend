/**
 * DTOs for lending operations (deposit, borrow, repay, withdraw, flash loan).
 *
 * Each class:
 *  - Defines typed, immutable properties.
 *  - Exposes a static `validate()` returning structured field errors.
 *  - Exposes a static `fromBody()` / `fromQuery()` factory for Express handlers.
 *
 * @openapi
 * components:
 *   schemas:
 *     LendingOperationDto:
 *       type: object
 *       required: [userAddress, amount]
 *       properties:
 *         userAddress:
 *           type: string
 *           description: Valid Ed25519 Stellar public key (56 chars, starts with G)
 *         assetAddress:
 *           type: string
 *           description: Optional contract address of the asset
 *         amount:
 *           type: string
 *           description: Positive integer ≤ i128 max
 */

import { FieldError, ValidationResult, isValidStellarAddress, isValidAmount } from './base.dto';
import type { LendingOperation } from '../types';

const VALID_OPERATIONS: LendingOperation[] = ['deposit', 'borrow', 'repay', 'withdraw'];
const MAX_XDR_LENGTH = 20_000;

// ─── Lending operation ────────────────────────────────────────────────────────

export class LendingOperationDto {
  readonly userAddress: string;
  readonly assetAddress?: string;
  readonly amount: string;

  private constructor(data: { userAddress: string; assetAddress?: string; amount: string }) {
    this.userAddress = data.userAddress;
    this.assetAddress = data.assetAddress;
    this.amount = data.amount;
  }

  static validate(data: Partial<LendingOperationDto>): ValidationResult {
    const errors: FieldError[] = [];

    if (!isValidStellarAddress(data.userAddress)) {
      errors.push({ field: 'userAddress', message: 'Must be a valid Stellar Ed25519 public key' });
    }
    if (!isValidAmount(data.amount)) {
      errors.push({
        field: 'amount',
        message: 'Must be a positive integer not exceeding i128 max',
      });
    }
    if (data.assetAddress !== undefined && data.assetAddress !== null) {
      if (typeof data.assetAddress !== 'string' || data.assetAddress.trim() === '') {
        errors.push({ field: 'assetAddress', message: 'When provided, must be a non-empty string' });
      }
    }

    return new ValidationResult(errors);
  }

  static fromBody(body: Record<string, unknown>): LendingOperationDto {
    return new LendingOperationDto({
      userAddress: String(body.userAddress ?? ''),
      assetAddress: body.assetAddress != null ? String(body.assetAddress) : undefined,
      amount: String(body.amount ?? ''),
    });
  }

  static fromQuery(query: Record<string, unknown>): LendingOperationDto {
    return LendingOperationDto.fromBody(query);
  }
}

// ─── Prepare ─────────────────────────────────────────────────────────────────

export class PrepareRequestDto {
  readonly operation: LendingOperation;
  readonly userAddress: string;
  readonly assetAddress?: string;
  readonly amount: string;

  private constructor(data: {
    operation: LendingOperation;
    userAddress: string;
    assetAddress?: string;
    amount: string;
  }) {
    this.operation = data.operation;
    this.userAddress = data.userAddress;
    this.assetAddress = data.assetAddress;
    this.amount = data.amount;
  }

  static validate(data: Record<string, unknown>): ValidationResult {
    const errors: FieldError[] = [];

    if (!VALID_OPERATIONS.includes(data.operation as LendingOperation)) {
      errors.push({
        field: 'operation',
        message: `Must be one of: ${VALID_OPERATIONS.join(', ')}`,
      });
    }

    const inner = LendingOperationDto.validate({
      userAddress: data.userAddress as string,
      assetAddress: data.assetAddress as string | undefined,
      amount: data.amount as string,
    });
    errors.push(...inner.errors);

    return new ValidationResult(errors);
  }

  static from(
    operation: string,
    source: Record<string, unknown>,
  ): PrepareRequestDto {
    return new PrepareRequestDto({
      operation: operation as LendingOperation,
      userAddress: String(source.userAddress ?? ''),
      assetAddress: source.assetAddress != null ? String(source.assetAddress) : undefined,
      amount: String(source.amount ?? ''),
    });
  }
}

// ─── Submit ───────────────────────────────────────────────────────────────────

export class SubmitRequestDto {
  readonly signedXdr: string;
  readonly operation?: LendingOperation;
  readonly userAddress?: string;
  readonly amount?: string;
  readonly assetAddress?: string;

  private constructor(data: {
    signedXdr: string;
    operation?: LendingOperation;
    userAddress?: string;
    amount?: string;
    assetAddress?: string;
  }) {
    this.signedXdr = data.signedXdr;
    this.operation = data.operation;
    this.userAddress = data.userAddress;
    this.amount = data.amount;
    this.assetAddress = data.assetAddress;
  }

  static validate(body: Record<string, unknown>): ValidationResult {
    const errors: FieldError[] = [];

    if (typeof body.signedXdr !== 'string' || body.signedXdr.trim() === '') {
      errors.push({ field: 'signedXdr', message: 'Required non-empty string' });
    } else if ((body.signedXdr as string).length > MAX_XDR_LENGTH) {
      errors.push({ field: 'signedXdr', message: `Must not exceed ${MAX_XDR_LENGTH} characters` });
    }

    if (body.operation != null && !VALID_OPERATIONS.includes(body.operation as LendingOperation)) {
      errors.push({
        field: 'operation',
        message: `When provided, must be one of: ${VALID_OPERATIONS.join(', ')}`,
      });
    }

    if (body.userAddress != null && !isValidStellarAddress(body.userAddress)) {
      errors.push({ field: 'userAddress', message: 'Must be a valid Stellar Ed25519 public key' });
    }

    if (body.amount != null && !isValidAmount(body.amount)) {
      errors.push({
        field: 'amount',
        message: 'When provided, must be a positive integer not exceeding i128 max',
      });
    }

    return new ValidationResult(errors);
  }

  static fromBody(body: Record<string, unknown>): SubmitRequestDto {
    return new SubmitRequestDto({
      signedXdr: String(body.signedXdr ?? ''),
      operation: body.operation != null ? (body.operation as LendingOperation) : undefined,
      userAddress: body.userAddress != null ? String(body.userAddress) : undefined,
      amount: body.amount != null ? String(body.amount) : undefined,
      assetAddress: body.assetAddress != null ? String(body.assetAddress) : undefined,
    });
  }
}

// ─── Relay delegated ─────────────────────────────────────────────────────────

export class RelayDelegatedDto {
  readonly delegatorAddress: string;
  readonly nonce: string;
  readonly deadline: string;
  readonly callsXdr: string;

  private constructor(data: {
    delegatorAddress: string;
    nonce: string;
    deadline: string;
    callsXdr: string;
  }) {
    this.delegatorAddress = data.delegatorAddress;
    this.nonce = data.nonce;
    this.deadline = data.deadline;
    this.callsXdr = data.callsXdr;
  }

  static validate(body: Record<string, unknown>): ValidationResult {
    const errors: FieldError[] = [];

    if (!isValidStellarAddress(body.delegatorAddress)) {
      errors.push({
        field: 'delegatorAddress',
        message: 'Must be a valid Stellar Ed25519 public key',
      });
    }
    if (typeof body.nonce !== 'string' || body.nonce.trim() === '') {
      errors.push({ field: 'nonce', message: 'Required non-empty string' });
    }
    if (typeof body.deadline !== 'string' || body.deadline.trim() === '') {
      errors.push({ field: 'deadline', message: 'Required non-empty string' });
    }
    if (typeof body.callsXdr !== 'string' || body.callsXdr.trim() === '') {
      errors.push({ field: 'callsXdr', message: 'Required non-empty string' });
    } else if ((body.callsXdr as string).length > MAX_XDR_LENGTH) {
      errors.push({ field: 'callsXdr', message: `Must not exceed ${MAX_XDR_LENGTH} characters` });
    }

    return new ValidationResult(errors);
  }

  static fromBody(body: Record<string, unknown>): RelayDelegatedDto {
    return new RelayDelegatedDto({
      delegatorAddress: String(body.delegatorAddress ?? ''),
      nonce: String(body.nonce ?? ''),
      deadline: String(body.deadline ?? ''),
      callsXdr: String(body.callsXdr ?? ''),
    });
  }
}

// ─── Response DTOs ────────────────────────────────────────────────────────────

export class PrepareResponseDto {
  readonly unsignedXdr: string;
  readonly operation: LendingOperation;
  readonly expiresAt: string;

  constructor(data: { unsignedXdr: string; operation: LendingOperation; expiresAt: string }) {
    this.unsignedXdr = data.unsignedXdr;
    this.operation = data.operation;
    this.expiresAt = data.expiresAt;
  }

  toJSON() {
    return {
      unsignedXdr: this.unsignedXdr,
      operation: this.operation,
      expiresAt: this.expiresAt,
    };
  }
}

export class TransactionResponseDto {
  readonly success: boolean;
  readonly transactionHash?: string;
  readonly status: 'pending' | 'success' | 'failed' | 'cancelled';
  readonly message?: string;
  readonly error?: string;

  constructor(data: {
    success: boolean;
    transactionHash?: string;
    status: 'pending' | 'success' | 'failed' | 'cancelled';
    message?: string;
    error?: string;
  }) {
    this.success = data.success;
    this.transactionHash = data.transactionHash;
    this.status = data.status;
    this.message = data.message;
    this.error = data.error;
  }

  toJSON() {
    return {
      success: this.success,
      ...(this.transactionHash !== undefined && { transactionHash: this.transactionHash }),
      status: this.status,
      ...(this.message !== undefined && { message: this.message }),
      ...(this.error !== undefined && { error: this.error }),
    };
  }
}
