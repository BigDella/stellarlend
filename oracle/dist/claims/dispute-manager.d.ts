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
import { DisputeResolution } from './types.js';
import type { ClaimRepository } from './claim-repository.js';
/**
 * Errors raised by DisputeManager.
 */
export declare class DisputeError extends Error {
    readonly code: 'CLAIM_NOT_FOUND' | 'INVALID_STATUS' | 'DISPUTE_NOT_FOUND' | 'ALREADY_RESOLVED';
    constructor(message: string, code: 'CLAIM_NOT_FOUND' | 'INVALID_STATUS' | 'DISPUTE_NOT_FOUND' | 'ALREADY_RESOLVED');
}
/**
 * Dispute Manager.
 */
export declare class DisputeManager {
    /** claimId → DisputeRecord */
    private disputes;
    private repository;
    constructor(repository: ClaimRepository);
    /**
     * Open a dispute on an existing claim.
     *
     * @param claimId           - ID of the claim to dispute.
     * @param disputantAddress  - Stellar address opening the dispute.
     * @param reason            - Human-readable reason.
     * @param evidence          - List of evidence strings (URLs, hashes, descriptions).
     */
    openDispute(claimId: string, disputantAddress: string, reason: string, evidence?: string[]): DisputeRecord;
    /**
     * Admin resolves a dispute.
     *
     * @param claimId          - The disputed claim ID.
     * @param resolution       - APPROVED, REJECTED, or ESCALATED.
     * @param adminAddress     - Admin Stellar address performing resolution.
     * @param resolutionNotes  - Optional notes.
     */
    resolveDispute(claimId: string, resolution: DisputeResolution, adminAddress: string, resolutionNotes?: string): {
        claim: InsuranceClaim;
        dispute: DisputeRecord;
    };
    getDispute(claimId: string): DisputeRecord | null;
    getAllDisputes(): DisputeRecord[];
    getOpenDisputes(): DisputeRecord[];
    private mapResolutionToStatus;
}
/**
 * Factory function.
 */
export declare function createDisputeManager(repository: ClaimRepository): DisputeManager;
//# sourceMappingURL=dispute-manager.d.ts.map