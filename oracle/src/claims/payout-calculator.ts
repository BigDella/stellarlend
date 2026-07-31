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
import { logger } from '../utils/logger.js';

/** 7-decimal fixed-point scale factor (Stellar standard). */
export const SCALE = 10_000_000n;

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

const DEFAULT_CONFIG: PayoutCalculatorConfig = {
  deductiblePercent: 5,
  minOracleConfidence: 80,
  confidenceDiscountRate: 0.5,
};

/**
 * Calculates insurance payouts.
 */
export class PayoutCalculator {
  private config: PayoutCalculatorConfig;

  constructor(config: Partial<PayoutCalculatorConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /**
   * Calculate the payout for an approved claim.
   *
   * @param claim    - The verified claim.
   * @param oracle   - Oracle data captured during verification.
   * @returns        PayoutResult with full breakdown.
   */
  calculate(claim: InsuranceClaim, oracle: OracleVerificationData): PayoutResult {
    const now = Math.floor(Date.now() / 1000);

    // 1. Gross amount — what was claimed
    const grossAmount = claim.claimedAmount;

    // 2. Apply coverage cap
    const cappedAmount =
      grossAmount > claim.coverageLimit ? claim.coverageLimit : grossAmount;

    // 3. Apply deductible
    const deductibleAmount = this.computeDeductible(cappedAmount);
    const afterDeductible = cappedAmount > deductibleAmount
      ? cappedAmount - deductibleAmount
      : 0n;

    // 4. Apply confidence discount
    const confidenceDiscount = this.computeConfidenceDiscount(oracle.confidence);
    const discountFactor = BigInt(Math.round((1 - confidenceDiscount) * 1_000_000));
    const netPayoutAmount = (afterDeductible * discountFactor) / 1_000_000n;

    // 5. Convert to USD value (oracle price is 7-decimal fixed-point)
    // usdValue = netPayoutAmount * oraclePrice / SCALE
    const usdValue = (netPayoutAmount * oracle.priceAtVerification) / SCALE;

    const result: PayoutResult = {
      grossAmount,
      cappedAmount,
      deductibleAmount,
      netPayoutAmount,
      oraclePriceUsed: oracle.priceAtVerification,
      usdValue,
      confidenceDiscount,
      calculatedAt: now,
    };

    logger.info('Payout calculated', {
      claimId: claim.id,
      grossAmount: grossAmount.toString(),
      cappedAmount: cappedAmount.toString(),
      deductibleAmount: deductibleAmount.toString(),
      netPayoutAmount: netPayoutAmount.toString(),
      usdValue: usdValue.toString(),
      confidenceDiscount,
      oracleConfidence: oracle.confidence,
    });

    return result;
  }

  // ── Private Helpers ─────────────────────────────────────────────────────────

  private computeDeductible(cappedAmount: bigint): bigint {
    if (this.config.deductiblePercent <= 0) return 0n;
    return (cappedAmount * BigInt(Math.round(this.config.deductiblePercent * 1_000))) / 100_000n;
  }

  /**
   * Returns a discount multiplier [0.0, 1.0].
   * 0 → no discount; 1 → full discount (zero payout).
   */
  private computeConfidenceDiscount(confidence: number): number {
    if (confidence >= this.config.minOracleConfidence) return 0;

    const pointsBelow = this.config.minOracleConfidence - confidence;
    const discount = (pointsBelow * this.config.confidenceDiscountRate) / 100;
    // Clamp to [0, 0.5] — never wipe out more than half the payout automatically
    return Math.min(0.5, Math.max(0, discount));
  }
}

/**
 * Factory function.
 */
export function createPayoutCalculator(
  config?: Partial<PayoutCalculatorConfig>
): PayoutCalculator {
  return new PayoutCalculator(config);
}
