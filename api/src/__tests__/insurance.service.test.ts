import { insuranceService } from '../services/insurance/insurance.service';

describe('InsuranceService', () => {
  let providerId: string;
  let policyId: string;

  describe('onboardProvider', () => {
    it('creates a provider with positive collateral', () => {
      const provider = insuranceService.onboardProvider({
        address: 'GPROVIDER1',
        name: 'InsureCo',
        kycStatus: 'approved',
        collateral: 100_000,
      });
      expect(provider.id).toBeDefined();
      expect(provider.availableCollateral).toBe(100_000);
      expect(provider.rating).toBe(0);
      providerId = provider.id;
    });

    it('rejects zero or negative collateral', () => {
      expect(() =>
        insuranceService.onboardProvider({
          address: 'GPROVIDER2',
          name: 'BadCo',
          kycStatus: 'approved',
          collateral: 0,
        })
      ).toThrow('Provider collateral must be positive');
    });
  });

  describe('createPolicy', () => {
    it('creates a policy for an approved provider', () => {
      const policy = insuranceService.createPolicy({
        providerId,
        coverageAmount: 50_000,
        premiumBps: 200,
        durationDays: 30,
        terms: 'Covers oracle failures',
        coveredTriggers: ['oracle_failure'],
        exclusions: ['self-inflicted'],
      });
      expect(policy.id).toBeDefined();
      expect(policy.active).toBe(true);
      policyId = policy.id;
    });

    it('rejects policy when coverage exceeds collateral', () => {
      expect(() =>
        insuranceService.createPolicy({
          providerId,
          coverageAmount: 999_999,
          premiumBps: 100,
          durationDays: 30,
          terms: 'Too much',
          coveredTriggers: ['oracle_failure'],
          exclusions: [],
        })
      ).toThrow('Coverage exceeds provider collateral');
    });
  });

  describe('listPolicies', () => {
    it('returns only active policies', () => {
      const policies = insuranceService.listPolicies();
      expect(policies.length).toBeGreaterThan(0);
      expect(policies.every((p) => p.active)).toBe(true);
    });
  });

  describe('purchase', () => {
    it('purchases coverage and deducts collateral', () => {
      const coverage = insuranceService.purchase(policyId, 'GLENDER1', 'POS1', 10_000);
      expect(coverage.coverageAmount).toBe(10_000);
      expect(coverage.premiumPaid).toBe((10_000 * 200) / 10_000);
      expect(coverage.expiresAt).toBeGreaterThan(coverage.startsAt);
    });

    it('rejects purchase on inactive policy', () => {
      expect(() => insuranceService.purchase('nonexistent', 'GLENDER2', 'POS2')).toThrow(
        'Policy is unavailable'
      );
    });
  });

  describe('submitClaim', () => {
    let coverageId: string;

    beforeAll(() => {
      const coverage = insuranceService.purchase(policyId, 'GLENDER3', 'POS3', 5_000);
      coverageId = coverage.id;
    });

    it('approves a valid claim with covered trigger', () => {
      const claim = insuranceService.submitClaim(coverageId, 'oracle_failure', 'tx proof', 3_000);
      expect(claim.status).toBe('approved');
      expect(claim.amount).toBe(3_000);
    });

    it('denies claim with uncovered trigger', () => {
      const claim = insuranceService.submitClaim(
        coverageId,
        'contract_compromise',
        'no evidence',
        1_000
      );
      expect(claim.status).toBe('denied');
    });

    it('caps claim amount at coverage amount', () => {
      const claim = insuranceService.submitClaim(coverageId, 'oracle_failure', 'proof', 99_999);
      expect(claim.amount).toBeLessThanOrEqual(5_000);
    });
  });

  describe('disputeClaim', () => {
    it('disputes a denied claim', () => {
      const coverage = insuranceService.purchase(policyId, 'GLENDER4', 'POS4', 2_000);
      const claim = insuranceService.submitClaim(
        coverage.id,
        'contract_compromise',
        'evidence',
        1_000
      );
      expect(claim.status).toBe('denied');

      const disputed = insuranceService.disputeClaim(claim.id);
      expect(disputed.status).toBe('disputed');
      expect(disputed.resolvedAt).toBeUndefined();
    });

    it('rejects dispute on approved claim', () => {
      const coverage = insuranceService.purchase(policyId, 'GLENDER5', 'POS5', 2_000);
      const claim = insuranceService.submitClaim(
        coverage.id,
        'oracle_failure',
        'evidence',
        1_000
      );
      expect(claim.status).toBe('approved');
      expect(() => insuranceService.disputeClaim(claim.id)).toThrow(
        'Only denied claims can be disputed'
      );
    });
  });

  describe('dashboard', () => {
    it('returns claims and totals', () => {
      const dash = insuranceService.dashboard();
      expect(dash.claims.length).toBeGreaterThan(0);
      expect(dash.totals).toHaveProperty('approved');
      expect(dash.totals).toHaveProperty('denied');
    });
  });
});
