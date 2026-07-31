/**
 * Payout Calculator
 *
 * Calculates insurance claim payouts using the oracle-verified
 * asset price. Applies coverage cap, deductible, and a
 * confidence-weighted discount for low-certainty oracle data.
 *
 * All amounts use 7-decimal fixed-point arithmetic (Stellar standard).
 * SCALE = 10_000_000n represents 1.0.
 */
import type { InsuranceClaim, PayoutResult, OracleVerificationData } from './types.js';
/** 7-decimal fixed-point scale factor (Stellar standard). */
export declare const SCALE = 10000000n;
/**
 * Payout calculator configuration.
 */
export interface PayoutCalculatorConfig {
    /**
     * Deductible as a percentage (0–100).
     * e.g. 5 → 5 % of the capped amount is subtracted.
     * Default: 5
     */
    deductiblePercent: number;
    /**
     * Oracle confidence threshold below which a discount is applied.
     * Default: 80
     */
    minOracleConfidence: number;
    /**
     * Confidence discount rate.
     * For each percentage point below minOracleConfidence the payout is
     * reduced by (confidenceDiscountRate / 100).
     * Default: 0.5  → 0.5 % per point below threshold
     */
    confidenceDiscountRate: number;
}
/**
 * Calculates insurance payouts.
 */
export declare class PayoutCalculator {
    private config;
    constructor(config?: Partial<PayoutCalculatorConfig>);
    /**
     * Calculate the payout for an approved claim.
     *
     * @param claim    - The verified claim.
     * @param oracle   - Oracle data captured during verification.
     * @returns        PayoutResult with full breakdown.
     */
    calculate(claim: InsuranceClaim, oracle: OracleVerificationData): PayoutResult;
    private computeDeductible;
    /**
     * Returns a discount multiplier [0.0, 1.0].
     * 0 → no discount; 1 → full discount (zero payout).
     */
    private computeConfidenceDiscount;
}
/**
 * Factory function.
 */
export declare function createPayoutCalculator(config?: Partial<PayoutCalculatorConfig>): PayoutCalculator;
//# sourceMappingURL=payout-calculator.d.ts.map