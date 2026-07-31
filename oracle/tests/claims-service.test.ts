import { describe, it, expect, beforeEach, vi } from 'vitest';
import { createClaimsService, ClaimsService } from '../src/claims/claims-service.js';
import { ClaimStatus, RejectionReason, DisputeResolution } from '../src/claims/types.js';

describe('ClaimsService Integration', () => {
  let mockAggregator: any;
  let service: ClaimsService;
  const now = Math.floor(Date.now() / 1000);

  beforeEach(() => {
    mockAggregator = {
      getPrice: vi.fn(),
    };
    service = createClaimsService(mockAggregator, {
      maxPriceAgeSeconds: 300,
      minOracleConfidence: 80,
      deductiblePercent: 5,
      fallbackToPendingManual: true,
    });
  });

  const validRequest = {
    claimantAddress: 'GADDRESS1',
    asset: 'XLM',
    claimedAmount: 1000_0000000n,
    coverageLimit: 5000_0000000n,
    lossDescription: 'Water damage',
    lossTimestamp: now - 3600,
    coveragePurchasedAt: now - 86400,
  };

  it('should auto-approve a valid claim', async () => {
    mockAggregator.getPrice.mockResolvedValue({
      price: 1_5000000n,
      timestamp: now - 60,
      confidence: 90,
      sources: [{ source: 'binance' }],
    });

    const result = await service.submitClaim(validRequest);

    expect(result.autoApproved).toBe(true);
    expect(result.claim.status).toBe(ClaimStatus.APPROVED);
    expect(result.claim.payoutResult).toBeDefined();
    expect(result.claim.payoutResult?.netPayoutAmount).toBe(950_0000000n);
  });

  it('should reject a fraudulent claim (velocity)', async () => {
    // Fill up some history for velocity
    await service.submitClaim(validRequest);
    await service.submitClaim(validRequest);
    await service.submitClaim(validRequest);

    mockAggregator.getPrice.mockResolvedValue({
      price: 1_5000000n,
      timestamp: now - 60,
      confidence: 90,
      sources: [],
    });

    // 4th claim from same address in same window
    const result = await service.submitClaim(validRequest);

    expect(result.fraudRejected).toBe(true);
    expect(result.claim.status).toBe(ClaimStatus.REJECTED);
    expect(result.claim.rejectionReason).toBe(RejectionReason.FRAUD_DETECTED);
  });

  it('should put claim in pending manual if oracle is unavailable', async () => {
    mockAggregator.getPrice.mockResolvedValue(null);

    const result = await service.submitClaim(validRequest);

    expect(result.pendingManual).toBe(true);
    expect(result.claim.status).toBe(ClaimStatus.PENDING_MANUAL);
  });

  it('should process payout after approval', async () => {
    mockAggregator.getPrice.mockResolvedValue({
      price: 1_5000000n,
      timestamp: now - 60,
      confidence: 90,
      sources: [],
    });

    const { claim } = await service.submitClaim(validRequest);
    
    const paidClaim = service.processPayout(claim.id, 'TX123HASH', 'GADMIN1');

    expect(paidClaim.status).toBe(ClaimStatus.PAID_OUT);
    expect(paidClaim.payoutTransactionHash).toBe('TX123HASH');
  });

  it('should handle full dispute lifecycle', async () => {
    mockAggregator.getPrice.mockResolvedValue({
      price: 1_5000000n,
      timestamp: now - 60,
      confidence: 90,
      sources: [],
    });

    const { claim } = await service.submitClaim(validRequest);
    
    service.openDispute(claim.id, 'GCLAIMANT1', 'Payout too low');
    
    const statusBefore = service.getClaim(claim.id)?.status;
    expect(statusBefore).toBe(ClaimStatus.DISPUTED);

    service.resolveDispute(claim.id, DisputeResolution.APPROVED, 'GADMIN1', 'Adjusted');

    const statusAfter = service.getClaim(claim.id)?.status;
    expect(statusAfter).toBe(ClaimStatus.APPROVED);
  });

  it('should provide aggregate stats', async () => {
    mockAggregator.getPrice.mockResolvedValue({
      price: 1_5000000n,
      timestamp: now - 60,
      confidence: 90,
      sources: [],
    });

    await service.submitClaim(validRequest);
    
    const stats = service.getStats();
    expect(stats.totalClaims).toBe(1);
    expect(stats.byStatus[ClaimStatus.APPROVED]).toBe(1);
  });
});
