export interface TierRule {
  name: string;
  minDeposits: number;
  minBorrowVolume: number;
  minAccountDays: number;
  minLoyalDays: number;
  discountBps: number;
  loyaltyBonusBps: number;
}
export interface UserFeeMetrics {
  totalDeposits: number;
  borrowingVolume: number;
  accountAgeDays: number;
  daysSinceWithdrawal: number;
}

const DEFAULT_TIERS: TierRule[] = [
  {
    name: 'Base',
    minDeposits: 0,
    minBorrowVolume: 0,
    minAccountDays: 0,
    minLoyalDays: 0,
    discountBps: 0,
    loyaltyBonusBps: 0,
  },
  {
    name: 'Silver',
    minDeposits: 10_000,
    minBorrowVolume: 5_000,
    minAccountDays: 30,
    minLoyalDays: 14,
    discountBps: 1_000,
    loyaltyBonusBps: 100,
  },
  {
    name: 'Gold',
    minDeposits: 50_000,
    minBorrowVolume: 25_000,
    minAccountDays: 90,
    minLoyalDays: 30,
    discountBps: 2_500,
    loyaltyBonusBps: 250,
  },
  {
    name: 'Platinum',
    minDeposits: 250_000,
    minBorrowVolume: 100_000,
    minAccountDays: 180,
    minLoyalDays: 90,
    discountBps: 5_000,
    loyaltyBonusBps: 500,
  },
];

class FeeTierService {
  private tiers = DEFAULT_TIERS;
  private savings = new Map<string, number>();
  configure(tiers: TierRule[]) {
    if (!tiers.length || tiers.some((tier) => tier.discountBps < 0 || tier.discountBps > 5_000))
      throw new Error('Discount must be between 0 and 5000 bps');
    this.tiers = [...tiers].sort((a, b) => a.discountBps - b.discountBps);
    return this.tiers;
  }
  status(userAddress: string, metrics: UserFeeMetrics) {
    const eligible = this.tiers.filter(
      (tier) =>
        metrics.totalDeposits >= tier.minDeposits &&
        metrics.borrowingVolume >= tier.minBorrowVolume &&
        metrics.accountAgeDays >= tier.minAccountDays &&
        metrics.daysSinceWithdrawal >= tier.minLoyalDays
    );
    const current = eligible[eligible.length - 1] ?? this.tiers[0]!;
    const next = this.tiers.find((tier) => tier.discountBps > current.discountBps);
    const progress = (value: number, target: number) =>
      target === 0 ? 1 : Math.min(1, value / target);
    return {
      userAddress,
      current,
      next,
      progress: next
        ? {
            deposits: progress(metrics.totalDeposits, next.minDeposits),
            borrowing: progress(metrics.borrowingVolume, next.minBorrowVolume),
            accountAge: progress(metrics.accountAgeDays, next.minAccountDays),
            loyalty: progress(metrics.daysSinceWithdrawal, next.minLoyalDays),
          }
        : null,
      totalSavings: this.savings.get(userAddress) ?? 0,
      evaluatedAt: Date.now(),
      effectiveAt: Date.now() + 7 * 86_400_000,
    };
  }
  apply(userAddress: string, baseFee: number, metrics: UserFeeMetrics, minimumFee: number) {
    const status = this.status(userAddress, metrics);
    const discounted = Math.max(minimumFee, baseFee * (1 - status.current.discountBps / 10_000));
    const saved = Math.max(0, baseFee - discounted);
    this.savings.set(userAddress, (this.savings.get(userAddress) ?? 0) + saved);
    return {
      fee: discounted,
      saved,
      discountBps: status.current.discountBps,
      tier: status.current.name,
    };
  }
}

export const feeTierService = new FeeTierService();
