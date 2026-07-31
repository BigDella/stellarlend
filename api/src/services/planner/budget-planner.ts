export interface PoolAllocation {
  poolId: string;
  weightBps: number;
  apyBps: number;
  riskGrade: string;
  capacity: number;
  actualReturn?: number;
}
export interface BudgetPlanInput {
  capital: number;
  horizonDays: number;
  goalAmount?: number;
  rebalanceThresholdBps: number;
  maxRiskExposureBps: Record<string, number>;
  pools: PoolAllocation[];
}

export const budgetPlanner = {
  build(input: BudgetPlanInput) {
    if (input.capital <= 0 || input.horizonDays <= 0)
      throw new Error('Capital and horizon must be positive');
    const weight = input.pools.reduce((sum, pool) => sum + pool.weightBps, 0);
    if (weight !== 10_000) throw new Error('Pool weights must total 10000 bps');
    const riskTotals: Record<string, number> = {};
    const allocations = input.pools.map((pool) => {
      const requested = (input.capital * pool.weightBps) / 10_000;
      const amount = Math.min(requested, pool.capacity);
      riskTotals[pool.riskGrade] = (riskTotals[pool.riskGrade] ?? 0) + amount;
      const projectedReturn = amount * (pool.apyBps / 10_000) * (input.horizonDays / 365);
      return {
        ...pool,
        amount,
        projectedReturn,
        variance: pool.actualReturn == null ? null : pool.actualReturn - projectedReturn,
      };
    });
    for (const [grade, amount] of Object.entries(riskTotals)) {
      const exposureBps = (amount / input.capital) * 10_000;
      if (exposureBps > (input.maxRiskExposureBps[grade] ?? 10_000))
        throw new Error(`Risk budget exceeded for grade ${grade}`);
    }
    const highest = [...allocations].sort((a, b) => b.apyBps - a.apyBps)[0];
    const rebalance = allocations
      .filter((pool) => highest && highest.apyBps - pool.apyBps > input.rebalanceThresholdBps)
      .map((pool) => ({
        from: pool.poolId,
        to: highest!.poolId,
        reason: `APY spread ${highest!.apyBps - pool.apyBps} bps`,
      }));
    const totalProjectedReturn = allocations.reduce((sum, pool) => sum + pool.projectedReturn, 0);
    return {
      allocations,
      totalProjectedReturn,
      projectedBalance: input.capital + totalProjectedReturn,
      goalOnTrack:
        input.goalAmount == null || input.capital + totalProjectedReturn >= input.goalAmount,
      rebalance,
      steps: allocations
        .filter((p) => p.amount > 0)
        .map((p, index) => ({
          order: index + 1,
          action: 'deposit',
          poolId: p.poolId,
          amount: p.amount,
        })),
    };
  },
};
