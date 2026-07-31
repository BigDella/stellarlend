/**
 * Claims Service
 *
 * Top-level facade for the insurance claims automation pipeline.
 *
 * Flow:
 *   submitClaim()
 *     → fraud detection
 *     → oracle verification  (ClaimVerifier)
 *     → payout calculation   (PayoutCalculator)
 *     → persistence          (ClaimRepository)
 *
 *   processPayout()          — register approved payout on-chain
 *   openDispute()            — delegate to DisputeManager
 *   resolveDispute()         — admin override
 *   getClaimHistory()        — full audit trail
 *   getStats()               — aggregate metrics
 */
import type { PriceAggregator } from '../services/price-aggregator.js';
import type { ClaimSubmissionRequest, InsuranceClaim, ClaimHistoryEntry, DisputeRecord, ClaimsStats, ClaimsServiceConfig } from './types.js';
import { ClaimStatus, DisputeResolution } from './types.js';
import { DisputeError } from './dispute-manager.js';
/**
 * Result of a claim submission.
 */
export interface SubmitClaimResult {
    claim: InsuranceClaim;
    /** true if the claim was auto-approved (oracle verified, no fraud) */
    autoApproved: boolean;
    /** true if the claim was flagged as fraudulent and rejected */
    fraudRejected: boolean;
    /** true if the oracle was unavailable and claim is pending manual review */
    pendingManual: boolean;
}
/**
 * Insurance Claims Service.
 */
export declare class ClaimsService {
    private repository;
    private verifier;
    private calculator;
    private fraudDetector;
    private disputeManager;
    private config;
    /** Tracks verification durations for stats. */
    private verificationDurationsMs;
    constructor(aggregator: PriceAggregator, config?: Partial<ClaimsServiceConfig>);
    /**
     * Submit a new insurance claim and run the automated verification pipeline.
     */
    submitClaim(request: ClaimSubmissionRequest): Promise<SubmitClaimResult>;
    /**
     * Mark an approved claim as PAID_OUT.
     * In a full integration this would call ContractUpdater; here it records
     * the transaction hash supplied by the caller after on-chain dispatch.
     *
     * @param claimId     - ID of the APPROVED claim.
     * @param txHash      - On-chain transaction hash of the payout.
     * @param adminAddress - Admin or system address initiating the payout.
     */
    processPayout(claimId: string, txHash: string, adminAddress: string): InsuranceClaim;
    /**
     * Re-run oracle verification on a PENDING or PENDING_MANUAL claim
     * (e.g. after oracle comes back online).
     */
    verifyClaim(claimId: string): Promise<InsuranceClaim>;
    openDispute(claimId: string, disputantAddress: string, reason: string, evidence?: string[]): DisputeRecord;
    resolveDispute(claimId: string, resolution: DisputeResolution, adminAddress: string, resolutionNotes?: string): {
        claim: InsuranceClaim;
        dispute: DisputeRecord;
    };
    getDispute(claimId: string): DisputeRecord | null;
    getClaim(claimId: string): InsuranceClaim | null;
    getClaimsByAddress(address: string): InsuranceClaim[];
    getClaimHistory(claimId: string): ClaimHistoryEntry[];
    getClaimsByStatus(status: ClaimStatus): InsuranceClaim[];
    getStats(): ClaimsStats;
    private errorCodeToRejectionReason;
}
/**
 * Factory function.
 */
export declare function createClaimsService(aggregator: PriceAggregator, config?: Partial<ClaimsServiceConfig>): ClaimsService;
export { DisputeError };
//# sourceMappingURL=claims-service.d.ts.map