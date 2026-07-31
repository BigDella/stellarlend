import { describe, it, expect, beforeEach, vi } from 'vitest';
import { createClaimVerifier, ClaimVerifier } from '../src/claims/claim-verifier.js';
import { ClaimStatus, VerificationErrorCode } from '../src/claims/types.js';
import type { PriceAggregator } from '../src/services/price-aggregator.js';

describe('ClaimVerifier', () => {
  let mockAggregator: any;
  let verifier: ClaimVerifier;
  const now = Math.floor(Date.now() / 1000);

  beforeEach(() => {
    mockAggregator = {
      getPrice: vi.fn(),
    };
    verifier = createClaimVerifier(mockAggregator as unknown as PriceAggregator, {
        maxPriceAgeSeconds: 300,
        minOracleConfidence: 80,
    });
  });

  const validClaim: any = {
    id: 'claim-1',
    claimantAddress: 'GADDRESS123',
    asset: 'XLM',
    claimedAmount: 100000000n, // 10 XLM
    coverageLimit: 500000000n, // 50 XLM
    lossTimestamp: now - 3600,
    coveragePurchasedAt: now - 86400,
    status: ClaimStatus.PENDING,
  };

  it('should verify a valid claim successfully', async () => {
    mockAggregator.getPrice.mockResolvedValue({
      price: 1500000n,
      timestamp: now - 60,
      confidence: 95,
      sources: [{ source: 'binance' }],
    });

    const result = await verifier.verify(validClaim);

    expect(result.isValid).toBe(true);
    expect(result.oracleData).toBeDefined();
    expect(result.oracleData?.priceAtVerification).toBe(1500000n);
    expect(result.errors).toHaveLength(0);
  });

  it('should reject if claimed amount is zero or negative', async () => {
    const invalidClaim = { ...validClaim, claimedAmount: 0n };
    const result = await verifier.verify(invalidClaim);

    expect(result.isValid).toBe(false);
    expect(result.errors[0].code).toBe(VerificationErrorCode.INVALID_AMOUNT);
  });

  it('should reject if claimed amount exceeds coverage limit', async () => {
    const invalidClaim = { ...validClaim, claimedAmount: 1000n, coverageLimit: 500n };
    const result = await verifier.verify(invalidClaim);

    expect(result.isValid).toBe(false);
    expect(result.errors[0].code).toBe(VerificationErrorCode.AMOUNT_EXCEEDS_COVERAGE);
  });

  it('should reject if loss timestamp is in the future', async () => {
    const invalidClaim = { ...validClaim, lossTimestamp: now + 1000 };
    const result = await verifier.verify(invalidClaim);

    expect(result.isValid).toBe(false);
    expect(result.errors[0].code).toBe(VerificationErrorCode.LOSS_TIMESTAMP_IN_FUTURE);
  });

  it('should reject if loss occurred before coverage purchase', async () => {
    const invalidClaim = { ...validClaim, lossTimestamp: now - 1000, coveragePurchasedAt: now - 500 };
    const result = await verifier.verify(invalidClaim);

    expect(result.isValid).toBe(false);
    expect(result.errors[0].code).toBe(VerificationErrorCode.LOSS_BEFORE_COVERAGE);
  });

  it('should handle oracle unavailability', async () => {
    mockAggregator.getPrice.mockResolvedValue(null);

    const result = await verifier.verify(validClaim);

    expect(result.isValid).toBe(false);
    expect(result.errors[0].code).toBe(VerificationErrorCode.ORACLE_UNAVAILABLE);
  });

  it('should reject stale oracle prices', async () => {
    mockAggregator.getPrice.mockResolvedValue({
      price: 1500000n,
      timestamp: now - 600, // 10 minutes old
      confidence: 95,
      sources: [],
    });

    const result = await verifier.verify(validClaim);

    expect(result.isValid).toBe(false);
    expect(result.errors[0].code).toBe(VerificationErrorCode.PRICE_STALE);
  });

  it('should handle low oracle confidence', async () => {
    mockAggregator.getPrice.mockResolvedValue({
      price: 1500000n,
      timestamp: now - 60,
      confidence: 50, // Low confidence
      sources: [],
    });

    const result = await verifier.verify(validClaim);

    // Default config allows low confidence but logs warning
    expect(result.isValid).toBe(true);
  });

  it('should reject low oracle confidence if configured to do so', async () => {
    verifier = createClaimVerifier(mockAggregator, {
        rejectOnLowConfidence: true,
        minOracleConfidence: 80,
    });

    mockAggregator.getPrice.mockResolvedValue({
      price: 1500000n,
      timestamp: now - 60,
      confidence: 50,
      sources: [],
    });

    const result = await verifier.verify(validClaim);

    expect(result.isValid).toBe(false);
    expect(result.errors[0].code).toBe(VerificationErrorCode.LOW_ORACLE_CONFIDENCE);
  });
});
