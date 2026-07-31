import { insuranceService } from '../services/insurance/insurance.service';
import { budgetPlanner } from '../services/planner/budget-planner';
import { feeTierService } from '../services/fees/tier.service';
import { GasEstimatorService } from '../services/gas/estimator';

describe('insurance marketplace', () => {
  it('purchases collateral-backed coverage and automatically approves a covered trigger', () => {
    const provider = insuranceService.onboardProvider({
      address: 'GPROVIDER',
      name: 'SafeCover',
      kycStatus: 'approved',
      collateral: 10_000,
    });
    const policy = insuranceService.createPolicy({
      providerId: provider.id,
      coverageAmount: 5_000,
      premiumBps: 100,
      durationDays: 30,
      terms: 'Oracle and contract risk',
      coveredTriggers: ['oracle_failure'],
      exclusions: ['user error'],
    });
    const coverage = insuranceService.purchase(policy.id, 'GLENDER', 'position-1');
    expect(coverage.premiumPaid).toBe(50);
    expect(
      insuranceService.submitClaim(coverage.id, 'oracle_failure', 'event:123', 5_000).status
    ).toBe('approved');
  });

  it('rejects insolvent coverage and allows denied claims to be disputed', () => {
    const provider = insuranceService.onboardProvider({
      address: 'GPROVIDER2',
      name: 'LimitedCover',
      kycStatus: 'approved',
      collateral: 1_000,
    });
    const policy = insuranceService.createPolicy({
      providerId: provider.id,
      coverageAmount: 1_000,
      premiumBps: 200,
      durationDays: 30,
      terms: 'Oracle risk',
      coveredTriggers: ['oracle_failure'],
      exclusions: ['contract compromise'],
    });
    const coverage = insuranceService.purchase(policy.id, 'GLENDER2', 'position-2');

    expect(() => insuranceService.purchase(policy.id, 'GLENDER3', 'position-3')).toThrow(
      'Insurer is insolvent'
    );
    const denied = insuranceService.submitClaim(
      coverage.id,
      'contract_compromise',
      'event:456',
      1_000
    );
    expect(denied.status).toBe('denied');
    expect(insuranceService.disputeClaim(denied.id).status).toBe('disputed');
  });
});

describe('budget planner', () => {
  it('projects returns and exports sequential actions', () => {
    const plan = budgetPlanner.build({
      capital: 10_000,
      horizonDays: 365,
      rebalanceThresholdBps: 100,
      maxRiskExposureBps: { A: 10_000 },
      pools: [
        { poolId: 'pool-a', weightBps: 5_000, apyBps: 500, riskGrade: 'A', capacity: 10_000 },
        { poolId: 'pool-b', weightBps: 5_000, apyBps: 300, riskGrade: 'A', capacity: 10_000 },
      ],
    });
    expect(plan.totalProjectedReturn).toBe(400);
    expect(plan.steps).toHaveLength(2);
    expect(plan.rebalance).toHaveLength(1);
  });

  it('caps allocations at pool capacity and enforces risk budgets', () => {
    const capped = budgetPlanner.build({
      capital: 10_000,
      horizonDays: 365,
      rebalanceThresholdBps: 100,
      maxRiskExposureBps: { A: 10_000 },
      pools: [
        {
          poolId: 'pool-capped',
          weightBps: 10_000,
          apyBps: 500,
          riskGrade: 'A',
          capacity: 4_000,
        },
      ],
    });
    expect(capped.allocations[0]?.amount).toBe(4_000);

    expect(() =>
      budgetPlanner.build({
        capital: 10_000,
        horizonDays: 365,
        rebalanceThresholdBps: 100,
        maxRiskExposureBps: { C: 2_500 },
        pools: [
          {
            poolId: 'pool-risky',
            weightBps: 10_000,
            apyBps: 900,
            riskGrade: 'C',
            capacity: 10_000,
          },
        ],
      })
    ).toThrow('Risk budget exceeded for grade C');
  });
});

describe('fee tiers', () => {
  it('caps discounts and enforces the minimum fee', () => {
    const result = feeTierService.apply(
      'GUSER',
      100,
      {
        totalDeposits: 300_000,
        borrowingVolume: 150_000,
        accountAgeDays: 365,
        daysSinceWithdrawal: 120,
      },
      60
    );
    expect(result.tier).toBe('Platinum');
    expect(result.fee).toBe(60);
    expect(result.saved).toBe(40);
  });

  it('rejects governance tiers above the fifty-percent discount cap', () => {
    expect(() =>
      feeTierService.configure([
        {
          name: 'Invalid',
          minDeposits: 0,
          minBorrowVolume: 0,
          minAccountDays: 0,
          minLoyalDays: 0,
          discountBps: 5_001,
          loyaltyBonusBps: 0,
        },
      ])
    ).toThrow('Discount must be between 0 and 5000 bps');
  });
});

describe('gas estimator', () => {
  it('compares every supported operation from cheapest to most expensive', async () => {
    const estimator = new GasEstimatorService();
    const costs = {
      deposit: 10,
      withdraw: 20,
      borrow: 30,
      repay: 40,
      liquidation: 60,
      flash_loan: 50,
    } as const;

    jest.spyOn(estimator, 'getHistoricalData').mockImplementation(async (operation, period) => ({
      operation,
      averageCost: String(costs[operation]),
      minCost: String(costs[operation]),
      maxCost: String(costs[operation]),
      stdDeviation: '0',
      sampleCount: 1,
      period,
    }));

    const comparison = await estimator.compareOperations();

    expect(comparison.operations).toHaveLength(6);
    expect(comparison.operations.map(({ operation }) => operation)).toEqual([
      'deposit',
      'withdraw',
      'borrow',
      'repay',
      'flash_loan',
      'liquidation',
    ]);
    expect(comparison.operations.map(({ rank }) => rank)).toEqual([1, 2, 3, 4, 5, 6]);
  });
});
