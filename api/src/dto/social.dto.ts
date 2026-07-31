import { FieldError, ValidationResult, isValidStellarAddress, isValidAmount } from './base.dto';

export class FollowRequestDto {
  readonly leaderAddress: string;
  readonly amount: string;
  readonly acknowledgeRisk: boolean;

  private constructor(data: { leaderAddress: string; amount: string; acknowledgeRisk: boolean }) {
    this.leaderAddress = data.leaderAddress;
    this.amount = data.amount;
    this.acknowledgeRisk = data.acknowledgeRisk;
  }

  static validate(body: Record<string, unknown>): ValidationResult {
    const errors: FieldError[] = [];
    if (!isValidStellarAddress(body.leaderAddress)) {
      errors.push({ field: 'leaderAddress', message: 'Must be a valid Stellar Ed25519 public key' });
    }
    if (!isValidAmount(body.amount)) {
      errors.push({ field: 'amount', message: 'Must be a positive integer not exceeding i128 max' });
    }
    if (body.acknowledgeRisk !== true) {
      errors.push({ field: 'acknowledgeRisk', message: 'You must acknowledge the risk disclosure' });
    }
    return new ValidationResult(errors);
  }

  static fromBody(body: Record<string, unknown>): FollowRequestDto {
    return new FollowRequestDto({
      leaderAddress: String(body.leaderAddress ?? ''),
      amount: String(body.amount ?? ''),
      acknowledgeRisk: body.acknowledgeRisk === true,
    });
  }
}

export class UnfollowRequestDto {
  readonly leaderAddress: string;

  private constructor(data: { leaderAddress: string }) {
    this.leaderAddress = data.leaderAddress;
  }

  static validate(body: Record<string, unknown>): ValidationResult {
    const errors: FieldError[] = [];
    if (!isValidStellarAddress(body.leaderAddress)) {
      errors.push({ field: 'leaderAddress', message: 'Must be a valid Stellar Ed25519 public key' });
    }
    return new ValidationResult(errors);
  }

  static fromBody(body: Record<string, unknown>): UnfollowRequestDto {
    return new UnfollowRequestDto({
      leaderAddress: String(body.leaderAddress ?? ''),
    });
  }
}

export class LeaderboardQueryDto {
  readonly sortBy: string = 'apy';
  readonly limit: number = 20;
  readonly offset: number = 0;
  readonly riskLevel?: string;

  static fromQuery(query: Record<string, unknown>): LeaderboardQueryDto {
    const dto = new LeaderboardQueryDto();
    (dto as unknown as Record<string, unknown>).sortBy = String(query.sortBy ?? 'apy');
    (dto as unknown as Record<string, unknown>).limit = parseInt(String(query.limit ?? '20'), 10);
    (dto as unknown as Record<string, unknown>).offset = parseInt(String(query.offset ?? '0'), 10);
    (dto as unknown as Record<string, unknown>).riskLevel = query.riskLevel ? String(query.riskLevel) : undefined;
    return dto;
  }
}
