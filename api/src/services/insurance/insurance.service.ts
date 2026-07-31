import { randomUUID } from 'crypto';

export type ClaimStatus = 'submitted' | 'approved' | 'denied' | 'disputed';
export type Trigger = 'oracle_failure' | 'contract_compromise';

export interface InsuranceProvider {
  id: string;
  address: string;
  name: string;
  kycStatus: 'pending' | 'approved' | 'rejected';
  collateral: number;
  availableCollateral: number;
  rating: number;
}

export interface InsurancePolicy {
  id: string;
  providerId: string;
  coverageAmount: number;
  premiumBps: number;
  durationDays: number;
  terms: string;
  coveredTriggers: Trigger[];
  exclusions: string[];
  active: boolean;
}

export interface PurchasedCoverage {
  id: string;
  policyId: string;
  lender: string;
  positionId: string;
  coverageAmount: number;
  premiumPaid: number;
  startsAt: number;
  expiresAt: number;
}

export interface InsuranceClaim {
  id: string;
  coverageId: string;
  trigger: Trigger;
  evidence: string;
  amount: number;
  status: ClaimStatus;
  submittedAt: number;
  resolvedAt?: number;
}

class InsuranceService {
  private providers = new Map<string, InsuranceProvider>();
  private policies = new Map<string, InsurancePolicy>();
  private coverages = new Map<string, PurchasedCoverage>();
  private claims = new Map<string, InsuranceClaim>();

  onboardProvider(input: Omit<InsuranceProvider, 'id' | 'availableCollateral' | 'rating'>) {
    if (input.collateral <= 0) throw new Error('Provider collateral must be positive');
    const provider: InsuranceProvider = {
      ...input,
      id: randomUUID(),
      availableCollateral: input.collateral,
      rating: 0,
    };
    this.providers.set(provider.id, provider);
    return provider;
  }

  createPolicy(input: Omit<InsurancePolicy, 'id' | 'active'>) {
    const provider = this.providers.get(input.providerId);
    if (!provider || provider.kycStatus !== 'approved')
      throw new Error('Approved provider required');
    if (input.coverageAmount <= 0 || input.coverageAmount > provider.availableCollateral) {
      throw new Error('Coverage exceeds provider collateral');
    }
    const policy = { ...input, id: randomUUID(), active: true };
    this.policies.set(policy.id, policy);
    return policy;
  }

  listPolicies() {
    return [...this.policies.values()].filter((policy) => policy.active);
  }

  purchase(policyId: string, lender: string, positionId: string, requestedCoverage?: number) {
    const policy = this.policies.get(policyId);
    if (!policy?.active) throw new Error('Policy is unavailable');
    const provider = this.providers.get(policy.providerId)!;
    const coverageAmount = Math.min(
      requestedCoverage ?? policy.coverageAmount,
      policy.coverageAmount
    );
    if (coverageAmount <= 0 || coverageAmount > provider.availableCollateral)
      throw new Error('Insurer is insolvent');
    const now = Date.now();
    const coverage: PurchasedCoverage = {
      id: randomUUID(),
      policyId,
      lender,
      positionId,
      coverageAmount,
      premiumPaid: (coverageAmount * policy.premiumBps) / 10_000,
      startsAt: now,
      expiresAt: now + policy.durationDays * 86_400_000,
    };
    provider.availableCollateral -= coverageAmount;
    this.coverages.set(coverage.id, coverage);
    return coverage;
  }

  submitClaim(coverageId: string, trigger: Trigger, evidence: string, amount: number) {
    const coverage = this.coverages.get(coverageId);
    if (!coverage) throw new Error('Coverage not found');
    const policy = this.policies.get(coverage.policyId)!;
    const eligible = Date.now() <= coverage.expiresAt && policy.coveredTriggers.includes(trigger);
    const claim: InsuranceClaim = {
      id: randomUUID(),
      coverageId,
      trigger,
      evidence,
      amount: Math.min(amount, coverage.coverageAmount),
      status: eligible ? 'approved' : 'denied',
      submittedAt: Date.now(),
      resolvedAt: Date.now(),
    };
    this.claims.set(claim.id, claim);
    return claim;
  }

  disputeClaim(claimId: string) {
    const claim = this.claims.get(claimId);
    if (!claim || claim.status !== 'denied') throw new Error('Only denied claims can be disputed');
    claim.status = 'disputed';
    claim.resolvedAt = undefined;
    return claim;
  }

  dashboard(providerId?: string) {
    const claims = [...this.claims.values()].filter((claim) => {
      if (!providerId) return true;
      const coverage = this.coverages.get(claim.coverageId);
      return coverage && this.policies.get(coverage.policyId)?.providerId === providerId;
    });
    return {
      claims,
      totals: claims.reduce<Record<ClaimStatus, number>>(
        (a, c) => ({ ...a, [c.status]: a[c.status] + 1 }),
        { submitted: 0, approved: 0, denied: 0, disputed: 0 }
      ),
    };
  }
}

export const insuranceService = new InsuranceService();
