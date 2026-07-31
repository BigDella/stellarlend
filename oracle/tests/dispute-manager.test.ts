import { describe, it, expect, beforeEach, vi } from 'vitest';
import { createDisputeManager, DisputeManager, DisputeError } from '../src/claims/dispute-manager.js';
import { createClaimRepository } from '../src/claims/claim-repository.js';
import { ClaimStatus, DisputeResolution } from '../src/claims/types.js';

describe('DisputeManager', () => {
  let repository: any;
  let manager: DisputeManager;

  beforeEach(() => {
    repository = createClaimRepository();
    manager = createDisputeManager(repository);
  });

  const setupClaim = () => {
    return repository.create({
      claimantAddress: 'GADDRESS1',
      asset: 'XLM',
      claimedAmount: 100n,
      coverageLimit: 200n,
      lossDescription: 'Test loss',
      lossTimestamp: Math.floor(Date.now() / 1000) - 3600,
      coveragePurchasedAt: Math.floor(Date.now() / 1000) - 86400,
    });
  };

  it('should open a dispute successfully', () => {
    const claim = setupClaim();
    
    const dispute = manager.openDispute(
      claim.id,
      'GDISPUTANT1',
      'Incorrect payout calculation',
      ['ev1.pdf']
    );

    expect(dispute).toBeDefined();
    expect(dispute.claimId).toBe(claim.id);
    expect(dispute.reason).toBe('Incorrect payout calculation');
    
    const updatedClaim = repository.findById(claim.id);
    expect(updatedClaim.status).toBe(ClaimStatus.DISPUTED);
  });

  it('should reject opening dispute on non-existent claim', () => {
    expect(() => 
      manager.openDispute('invalid-id', 'G1', 'reason')
    ).toThrow(DisputeError);
  });

  it('should reject duplicate disputes', () => {
    const claim = setupClaim();
    manager.openDispute(claim.id, 'G1', 'reason');
    
    expect(() => 
      manager.openDispute(claim.id, 'G2', 'another reason')
    ).toThrow(DisputeError);
  });

  it('should resolve dispute as APPROVED', () => {
    const claim = setupClaim();
    manager.openDispute(claim.id, 'G1', 'reason');

    const result = manager.resolveDispute(
      claim.id,
      DisputeResolution.APPROVED,
      'GADMIN1',
      'Resolved in favor of claimant'
    );

    expect(result.dispute.resolution).toBe(DisputeResolution.APPROVED);
    expect(result.claim.status).toBe(ClaimStatus.APPROVED);
  });

  it('should resolve dispute as REJECTED', () => {
    const claim = setupClaim();
    manager.openDispute(claim.id, 'G1', 'reason');

    const result = manager.resolveDispute(
      claim.id,
      DisputeResolution.REJECTED,
      'GADMIN1',
      'Dispute rejected'
    );

    expect(result.claim.status).toBe(ClaimStatus.REJECTED);
  });

  it('should resolve dispute as ESCALATED', () => {
    const claim = setupClaim();
    manager.openDispute(claim.id, 'G1', 'reason');

    const result = manager.resolveDispute(
      claim.id,
      DisputeResolution.ESCALATED,
      'GADMIN1',
      'Needs manual intervention'
    );

    expect(result.claim.status).toBe(ClaimStatus.PENDING_MANUAL);
  });

  it('should reject resolution of non-existent dispute', () => {
    const claim = setupClaim();
    expect(() => 
      manager.resolveDispute(claim.id, DisputeResolution.APPROVED, 'G1')
    ).toThrow(DisputeError);
  });

  it('should reject multiple resolutions', () => {
    const claim = setupClaim();
    manager.openDispute(claim.id, 'G1', 'reason');
    manager.resolveDispute(claim.id, DisputeResolution.APPROVED, 'GADMIN1');

    expect(() => 
      manager.resolveDispute(claim.id, DisputeResolution.REJECTED, 'GADMIN1')
    ).toThrow(DisputeError);
  });
});
