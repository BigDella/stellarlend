/**
 * Claim Verifier
 *
 * Validates an insurance claim using live oracle price data.
 * Checks asset support, price freshness, oracle confidence,
 * amount validity, and coverage limits.
 */
import type { PriceAggregator } from '../services/price-aggregator.js';
import type { InsuranceClaim, ClaimVerificationResult } from './types.js';
/**
 * Verifier configuration.
 */
export interface ClaimVerifierConfig {
    /** Max age of oracle price in seconds before it is considered stale. Default: 300 */
    maxPriceAgeSeconds: number;
    /** Oracle confidence below which LOW_ORACLE_CONFIDENCE is flagged. Default: 80 */
    minOracleConfidence: number;
    /** If true, low confidence is an error (not just a warning). Default: false */
    rejectOnLowConfidence: boolean;
}
/**
 * Oracle-based claim verifier.
 */
export declare class ClaimVerifier {
    private aggregator;
    private config;
    constructor(aggregator: PriceAggregator, config?: Partial<ClaimVerifierConfig>);
    /**
     * Verify a claim against oracle data and business rules.
     */
    verify(claim: InsuranceClaim): Promise<ClaimVerificationResult>;
}
/**
 * Factory function.
 */
export declare function createClaimVerifier(aggregator: PriceAggregator, config?: Partial<ClaimVerifierConfig>): ClaimVerifier;
//# sourceMappingURL=claim-verifier.d.ts.map