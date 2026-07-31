import { LeaderboardEntry, LeaderProfile, LeaderboardQuery } from '../../types/social';

interface LeaderMetrics {
  address: string;
  alias?: string;
  totalDeposits: bigint;
  totalBorrows: bigint;
  totalReturns: bigint;
  apy: number;
  riskAdjustedReturns: number;
  volatility: number;
  sharpeRatio: number;
  winRate: number;
  totalValue: bigint;
  riskLevel: 'low' | 'medium' | 'high';
  followers: number;
}

class LeaderboardStore {
  private leaders: Map<string, LeaderMetrics> = new Map();
  private leaderFollowers: Map<string, Set<string>> = new Map();
  private optedOut: Set<string> = new Set();

  registerLeader(address: string, metrics: LeaderMetrics): void {
    this.leaders.set(address, metrics);
  }

  addFollower(leaderAddress: string, followerAddress: string): void {
    const followers = this.leaderFollowers.get(leaderAddress) || new Set();
    followers.add(followerAddress);
    this.leaderFollowers.set(leaderAddress, followers);
    const leader = this.leaders.get(leaderAddress);
    if (leader) {
      this.leaders.set(leaderAddress, { ...leader, followers: followers.size });
    }
  }

  removeFollower(leaderAddress: string, followerAddress: string): void {
    const followers = this.leaderFollowers.get(leaderAddress);
    if (followers) {
      followers.delete(followerAddress);
      this.leaderFollowers.set(leaderAddress, followers);
      const leader = this.leaders.get(leaderAddress);
      if (leader) {
        this.leaders.set(leaderAddress, { ...leader, followers: followers.size });
      }
    }
  }

  setOptOut(address: string, optedOut: boolean): void {
    if (optedOut) {
      this.optedOut.add(address);
    } else {
      this.optedOut.delete(address);
    }
  }

  getLeaderboard(query: Record<string, unknown>): LeaderboardEntry[] {
    const entries: LeaderboardEntry[] = [];
    for (const [address, metrics] of this.leaders) {
      if (this.optedOut.has(address)) continue;
      const riskLevelFilter = query.riskLevel as string | undefined;
      if (riskLevelFilter && metrics.riskLevel !== riskLevelFilter) continue;

      entries.push({
        address,
        alias: metrics.alias,
        apy: metrics.apy.toFixed(4),
        totalReturns: metrics.totalReturns.toString(),
        riskAdjustedReturns: metrics.riskAdjustedReturns.toFixed(4),
        totalFollowers: metrics.followers,
        totalValue: metrics.totalValue.toString(),
        riskLevel: metrics.riskLevel,
        strategy: {
          allocation: {},
          rebalanceFrequency: 'manual',
          riskLevel: metrics.riskLevel,
        },
        isOptedOut: false,
      });
    }

    const sortBy = (query.sortBy as string) || 'apy';
    entries.sort((a, b) => {
      switch (sortBy) {
        case 'totalReturns':
          return compareBigInt(b.totalReturns, a.totalReturns);
        case 'riskAdjustedReturns':
          return parseFloat(b.riskAdjustedReturns) - parseFloat(a.riskAdjustedReturns);
        case 'followers':
          return b.totalFollowers - a.totalFollowers;
        default:
          return parseFloat(b.apy) - parseFloat(a.apy);
      }
    });

    const offset = (query.offset as number) || 0;
    const limit = (query.limit as number) || 20;
    return entries.slice(offset, offset + limit);
  }

  getLeaderProfile(address: string): LeaderProfile | null {
    const metrics = this.leaders.get(address);
    if (!metrics || this.optedOut.has(address)) return null;

    return {
      address: metrics.address,
      alias: metrics.alias,
      totalPortfolioValue: metrics.totalValue.toString(),
      strategy: {
        allocation: {},
        rebalanceFrequency: 'manual',
        riskLevel: metrics.riskLevel,
      },
      performance: {
        apy: metrics.apy.toFixed(4),
        totalReturns: metrics.totalReturns.toString(),
        weeklyReturns: '0',
        monthlyReturns: '0',
        riskAdjustedReturns: metrics.riskAdjustedReturns.toFixed(4),
        volatility: metrics.volatility.toFixed(4),
        sharpeRatio: metrics.sharpeRatio.toFixed(4),
        maxDrawdown: '0',
        winRate: (metrics.winRate * 100).toFixed(2) + '%',
      },
      followers: metrics.followers,
      totalFollowerValue: '0',
      createdAt: new Date().toISOString(),
      isOptedOut: false,
    };
  }
}

function compareBigInt(a: string, b: string): number {
  const diff = BigInt(a) - BigInt(b);
  if (diff > 0n) return 1;
  if (diff < 0n) return -1;
  return 0;
}

export const leaderboardService = new LeaderboardStore();
