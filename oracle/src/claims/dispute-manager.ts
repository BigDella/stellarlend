/**
 * Dispute Manager
 *
 * Manages the lifecycle of claim disputes:
 *   PENDING / APPROVED / REJECTED / PAID_OUT → DISPUTED → (APPROVED | REJECTED | ESCALATED)
 *
 * Rules:
 *   - Only claims that are not already DISPUTED, CANCELLED, or PAID_OUT can be disputed.
 *   - Admin resolution applies an override payout (APPROVED) or cancels payout (REJECTED).
 *   - Full audit trail is maintained in ClaimRepository.
 */

import type { DisputeRecord, InsuranceClaim } from './types.js';
import { ClaimStatus, DisputeResolution } from './types.js';
import type { ClaimRepository } from './claim-repository.js';
import { logger } from '../utils/logger.js';

/**
 * Errors raised by DisputeManager.
 */
export class DisputeError extends Error {
  constructor(
    message: string,
    public readonly code:
      | 'CLAIM_NOT_FOUND'
      | 'INVALID_STATUS'
      | 'DISPUTE_NOT_FOUND'
      | 'ALREADY_RESOLVED'
  ) {
    super(message);
    this.name = 'DisputeError';
  }
}

/**
 * Dispute Manager.
 */
export class DisputeManager {
  /** claimId → DisputeRecord */
  private disputes: Map<string, DisputeRecord> = new Map();
  private repository: ClaimRepository;

  constructor(repository: ClaimRepository) {
    this.repository = repository;
  }

  // ── Open a Dispute ──────────────────────────────────────────────────────────

  /**
   * Open a dispute on an existing claim.
   *
   * @param claimId           - ID of the claim to dispute.
   * @param disputantAddress  - Stellar address opening the dispute.
   * @param reason            - Human-readable reason.
   * @param evidence          - List of evidence strings (URLs, hashes, descriptions).
   */
  openDispute(
    claimId: string,
    disputantAddress: string,
    reason: string,
    evidence: string[] = []
  ): DisputeRecord {
    const claim = this.repository.findById(claimId);

    if (!claim) {
      throw new DisputeError(`Claim '${claimId}' not found`, 'CLAIM_NOT_FOUND');
    }

    const nonDisputeableStatuses: ClaimStatus[] = [
      ClaimStatus.DISPUTED,
      ClaimStatus.CANCELLED,
      ClaimStatus.PAID_OUT,
    ];

    if (nonDisputeableStatuses.includes(claim.status)) {
      throw new DisputeError(
        `Claim '${claimId}' is in status '${claim.status}' and cannot be disputed`,
        'INVALID_STATUS'
      );
    }

    if (this.disputes.has(claimId)) {
      throw new DisputeError(
        `A dispute is already open for claim '${claimId}'`,
        'INVALID_STATUS'
      );
    }

    const now = Math.floor(Date.now() / 1000);

    const record: DisputeRecord = {
      claimId,
      disputantAddress,
      reason,
      evidence,
      openedAt: now,
    };

    this.disputes.set(claimId, record);

    this.repository.transition(
      claimId,
      ClaimStatus.DISPUTED,
      disputantAddress,
      `Dispute opened: ${reason}`,
      { evidence }
    );

    logger.info('Dispute opened', { claimId, disputant: disputantAddress, reason });
    return record;
  }

  // ── Resolve a Dispute ───────────────────────────────────────────────────────

  /**
   * Admin resolves a dispute.
   *
   * @param claimId          - The disputed claim ID.
   * @param resolution       - APPROVED, REJECTED, or ESCALATED.
   * @param adminAddress     - Admin Stellar address performing resolution.
   * @param resolutionNotes  - Optional notes.
   */
  resolveDispute(
    claimId: string,
    resolution: DisputeResolution,
    adminAddress: string,
    resolutionNotes?: string
  ): { claim: InsuranceClaim; dispute: DisputeRecord } {
    const claim = this.repository.findById(claimId);
    if (!claim) {
      throw new DisputeError(`Claim '${claimId}' not found`, 'CLAIM_NOT_FOUND');
    }

    const dispute = this.disputes.get(claimId);
    if (!dispute) {
      throw new DisputeError(`No dispute found for claim '${claimId}'`, 'DISPUTE_NOT_FOUND');
    }

    if (dispute.resolution !== undefined) {
      throw new DisputeError(
        `Dispute for claim '${claimId}' has already been resolved`,
        'ALREADY_RESOLVED'
      );
    }

    if (claim.status !== ClaimStatus.DISPUTED) {
      throw new DisputeError(
        `Claim '${claimId}' is not in DISPUTED status`,
        'INVALID_STATUS'
      );
    }

    const now = Math.floor(Date.now() / 1000);

    dispute.resolution = resolution;
    dispute.resolvedBy = adminAddress;
    dispute.resolutionNotes = resolutionNotes;
    dispute.resolvedAt = now;

    // Map dispute resolution to claim status
    const newStatus = this.mapResolutionToStatus(resolution);

    const updatedClaim = this.repository.transition(
      claimId,
      newStatus,
      adminAddress,
      `Dispute resolved: ${resolution}${resolutionNotes ? ` — ${resolutionNotes}` : ''}`,
      { resolution, admin: adminAddress }
    );

    if (!updatedClaim) {
      throw new DisputeError(`Failed to update claim '${claimId}'`, 'CLAIM_NOT_FOUND');
    }

    logger.info('Dispute resolved', {
      claimId,
      resolution,
      admin: adminAddress,
      newStatus,
    });

    return { claim: updatedClaim, dispute };
  }

  // ── Query ───────────────────────────────────────────────────────────────────

  getDispute(claimId: string): DisputeRecord | null {
    return this.disputes.get(claimId) ?? null;
  }

  getAllDisputes(): DisputeRecord[] {
    return Array.from(this.disputes.values());
  }

  getOpenDisputes(): DisputeRecord[] {
    return Array.from(this.disputes.values()).filter((d) => d.resolution === undefined);
  }

  // ── Helpers ─────────────────────────────────────────────────────────────────

  private mapResolutionToStatus(resolution: DisputeResolution): ClaimStatus {
    switch (resolution) {
      case DisputeResolution.APPROVED:
        return ClaimStatus.APPROVED;
      case DisputeResolution.REJECTED:
        return ClaimStatus.REJECTED;
      case DisputeResolution.ESCALATED:
        return ClaimStatus.PENDING_MANUAL;
    }
  }
}

/**
 * Factory function.
 */
export function createDisputeManager(repository: ClaimRepository): DisputeManager {
  return new DisputeManager(repository);
}
