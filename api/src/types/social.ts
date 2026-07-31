export interface LeaderboardEntry {
  address: string;
  alias?: string;
  apy: string;
  totalReturns: string;
  riskAdjustedReturns: string;
  totalFollowers: number;
  totalValue: string;
  riskLevel: 'low' | 'medium' | 'high';
  strategy: StrategySummary;
  isOptedOut: boolean;
}

export interface StrategySummary {
  allocation: Record<string, string>;
  rebalanceFrequency: string;
  riskLevel: 'low' | 'medium' | 'high';
  description?: string;
}

export interface FollowRelation {
  followerAddress: string;
  leaderAddress: string;
  investedAmount: string;
  proportionalAllocation: number;
  totalProfit: string;
  leaderProfitShare: string;
  startedAt: string;
  active: boolean;
}

export interface LeaderProfile {
  address: string;
  alias?: string;
  totalPortfolioValue: string;
  strategy: StrategySummary;
  performance: {
    apy: string;
    totalReturns: string;
    weeklyReturns: string;
    monthlyReturns: string;
    riskAdjustedReturns: string;
    volatility: string;
    sharpeRatio: string;
    maxDrawdown: string;
    winRate: string;
  };
  followers: number;
  totalFollowerValue: string;
  createdAt: string;
  isOptedOut: boolean;
}

export interface FollowRequest {
  leaderAddress: string;
  amount: string;
  acknowledgeRisk: boolean;
}

export interface UnfollowRequest {
  leaderAddress: string;
}

export interface LeaderboardQuery {
  sortBy?: 'apy' | 'totalReturns' | 'riskAdjustedReturns' | 'followers';
  limit?: number;
  offset?: number;
  riskLevel?: 'low' | 'medium' | 'high';
}

export interface PrivacySettings {
  optOutCopying: boolean;
}
