/**
 * Claim Repository
 *
 * In-memory store for insurance claims with LRU eviction and an
 * append-only audit trail per claim.
 */
import type { InsuranceClaim, ClaimHistoryEntry, ClaimSubmissionRequest } from './types.js';
import { ClaimStatus } from './types.js';
/**
 * Configuration for the repository.
 */
export interface ClaimRepositoryConfig {
    /** Maximum number of claims to hold before evicting oldest. Default: 10_000 */
    maxEntries: number;
}
/**
 * In-memory insurance claim store.
 *
 * Uses a Map (insertion-order preserved) to track LRU position.
 * The first key in the Map is always the oldest / least-recently touched.
 */
export declare class ClaimRepository {
    private store;
    private config;
    constructor(config?: Partial<ClaimRepositoryConfig>);
    /**
     * Create a new claim from a submission request.
     * Returns the persisted claim.
     */
    create(request: ClaimSubmissionRequest): InsuranceClaim;
    /**
     * Persist (insert or update) a claim.
     */
    save(claim: InsuranceClaim): void;
    /**
     * Append a history entry and persist.
     */
    appendHistory(claimId: string, entry: ClaimHistoryEntry): InsuranceClaim | null;
    /**
     * Transition a claim to a new status, recording the history entry.
     */
    transition(claimId: string, toStatus: ClaimStatus, actor: string, description: string, metadata?: Record<string, unknown>): InsuranceClaim | null;
    findById(id: string): InsuranceClaim | null;
    findByAddress(address: string): InsuranceClaim[];
    findByStatus(status: ClaimStatus): InsuranceClaim[];
    /**
     * Return all claims submitted since a given Unix timestamp.
     */
    findSince(sinceTimestamp: number): InsuranceClaim[];
    /**
     * Return the full audit history for a claim.
     */
    getHistory(claimId: string): ClaimHistoryEntry[];
    count(): number;
    getAll(): InsuranceClaim[];
    private persist;
}
/**
 * Factory function.
 */
export declare function createClaimRepository(config?: Partial<ClaimRepositoryConfig>): ClaimRepository;
//# sourceMappingURL=claim-repository.d.ts.map