import { describe, it, expect, beforeEach } from 'vitest';
import { createPayoutCalculator, PayoutCalculator, SCALE } from '../src/claims/payout-calculator.js';
import { ClaimStatus } from '../src/claims/types.js';

describe('PayoutCalculator', () => {
  let calculator: PayoutCalculator;
  const now = Math.floor(Date.now() / 1000);

  beforeEach(() => {
    calculator = createPayoutCalculator({
      deductiblePercent: 5,
      minOracleConfidence: 80,
      confidenceDiscountRate: 0.5,
    });
  });

  const baseClaim: any = {
    id: 'claim-1',
    claimedAmount: 1000_0000000n, // 1000 units
    coverageLimit: 2000_0000000n, // 2000 units
  };

  const baseOracle: any = {
    priceAtVerification: 1_5000000n, // 1.5 USD
    confidence: 100,
    verifiedAt: now,
  };

  it('should calculate basic payout correctly', () => {
    // 1000 units, 5% deductible = 950 units. 1.5 USD/unit = 1425 USD
    const result = calculator.calculate(baseClaim, baseOracle);

    expect(result.grossAmount).toBe(1000_0000000n);
    expect(result.netPayoutAmount).toBe(950_0000000n);
    expect(result.usdValue).toBe(1425_0000000n);
    expect(result.deductibleAmount).toBe(50_0000000n);
  });

  it('should enforce coverage limit', () => {
    const claim = { ...baseClaim, claimedAmount: 5000_0000000n, coverageLimit: 2000_0000000n };
    // Capped at 2000. 5% deductible of 2000 = 100. Net = 1900.
    const result = calculator.calculate(claim, baseOracle);

    expect(result.cappedAmount).toBe(2000_0000000n);
    expect(result.netPayoutAmount).toBe(1900_0000000n);
  });

  it('should apply confidence discount', () => {
    const lowConfidenceOracle = { ...baseOracle, confidence: 60 };
    // 80 - 60 = 20 points below threshold.
    // 20 * 0.5% = 10% discount.
    // Base payout after deductible: 950.
    // 950 * 0.9 = 855.
    const result = calculator.calculate(baseClaim, lowConfidenceOracle);

    expect(result.confidenceDiscount).toBe(0.1);
    expect(result.netPayoutAmount).toBe(855_0000000n);
  });

  it('should clamp confidence discount to 50%', () => {
    const zeroConfidenceOracle = { ...baseOracle, confidence: 0 };
    // 80 - 0 = 80 points below.
    // 80 * 0.5% = 40% discount. (Actually 40 is less than 50, let's try a higher rate)
    
    const highRateCalculator = createPayoutCalculator({
        minOracleConfidence: 80,
        confidenceDiscountRate: 1.0, // 1% per point
    });
    
    // 80 points * 1% = 80% discount -> should clamp to 50%
    const result = highRateCalculator.calculate(baseClaim, zeroConfidenceOracle);
    
    expect(result.confidenceDiscount).toBe(0.5);
    expect(result.netPayoutAmount).toBe(475_0000000n); // 950 * 0.5
  });

  it('should handle zero deductible', () => {
    const noDeductibleCalculator = createPayoutCalculator({ deductiblePercent: 0 });
    const result = noDeductibleCalculator.calculate(baseClaim, baseOracle);

    expect(result.deductibleAmount).toBe(0n);
    expect(result.netPayoutAmount).toBe(1000_0000000n);
  });
});
