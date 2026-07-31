import { describe, it, expect, beforeEach } from 'vitest';
import { createFraudDetector, FraudDetector } from '../src/claims/fraud-detector.js';
import { ClaimStatus, FraudSeverity, FraudSignalType } from '../src/claims/types.js';

describe('FraudDetector', () => {
  let detector: FraudDetector;
  const now = Math.floor(Date.now() / 1000);

  beforeEach(() => {
    detector = createFraudDetector({
      velocityThreshold: 2,
      velocityWindowSeconds: 3600,
      minCoverageAgeSeconds: 300,
      amountAnomalyMultiplier: 3,
      fraudRiskThreshold: 60,
    });
  });

  const baseClaim: any = {
    id: 'claim-new',
    claimantAddress: 'GADDRESS1',
    asset: 'XLM',
    claimedAmount: 100n,
    submittedAt: now,
    coveragePurchasedAt: now - 3600,
    lossTimestamp: now - 1800,
  };

  it('should pass a clean claim with no signals', () => {
    const result = detector.detect(baseClaim, []);

    expect(result.isFraudulent).toBe(false);
    expect(result.signals).toHaveLength(0);
    expect(result.riskScore).toBe(0);
  });

  it('should detect velocity fraud', () => {
    const existingClaims: any[] = [
      { id: 'c1', claimantAddress: 'GADDRESS1', submittedAt: now - 100 },
      { id: 'c2', claimantAddress: 'GADDRESS1', submittedAt: now - 200 },
    ];

    const result = detector.detect(baseClaim, existingClaims);

    expect(result.signals.some(s => s.type === FraudSignalType.VELOCITY)).toBe(true);
    expect(result.maxSeverity).toBe(FraudSeverity.HIGH);
  });

  it('should detect suspicious timing', () => {
    const freshClaim = { ...baseClaim, coveragePurchasedAt: now - 10 };
    const result = detector.detect(freshClaim, []);

    expect(result.signals.some(s => s.type === FraudSignalType.SUSPICIOUS_TIMING)).toBe(true);
    expect(result.signals.find(s => s.type === FraudSignalType.SUSPICIOUS_TIMING)?.severity).toBe(FraudSeverity.MEDIUM);
  });

  it('should detect amount anomaly', () => {
    const existingClaims: any[] = [
      { id: 'c1', asset: 'XLM', claimedAmount: 10n },
      { id: 'c2', asset: 'XLM', claimedAmount: 10n },
      { id: 'c3', asset: 'XLM', claimedAmount: 10n },
    ];
    // Avg = 10. Multiplier = 3. Threshold = 30.
    const largeClaim = { ...baseClaim, claimedAmount: 100n };
    
    const result = detector.detect(largeClaim, existingClaims);

    expect(result.signals.some(s => s.type === FraudSignalType.AMOUNT_ANOMALY)).toBe(true);
  });

  it('should detect duplicate claims', () => {
    const existingClaims: any[] = [
      { 
        id: 'c1', 
        claimantAddress: 'GADDRESS1', 
        asset: 'XLM', 
        lossTimestamp: baseClaim.lossTimestamp 
      },
    ];

    const result = detector.detect(baseClaim, existingClaims);

    expect(result.signals.some(s => s.type === FraudSignalType.DUPLICATE_CLAIM)).toBe(true);
    expect(result.maxSeverity).toBe(FraudSeverity.CRITICAL);
    expect(result.isFraudulent).toBe(true);
  });

  it('should accumulate risk scores correctly', () => {
    const existingClaims: any[] = [
      { id: 'c1', claimantAddress: 'GADDRESS1', submittedAt: now - 100 },
      { id: 'c2', claimantAddress: 'GADDRESS1', submittedAt: now - 200 },
    ];
    const suspiciousClaim = { ...baseClaim, coveragePurchasedAt: now - 10 };
    
    const result = detector.detect(suspiciousClaim, existingClaims);

    // VELOCITY (HIGH: 50) + SUSPICIOUS_TIMING (MEDIUM: 25) = 75
    expect(result.riskScore).toBe(75);
    expect(result.isFraudulent).toBe(true);
  });
});
