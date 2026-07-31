import { FollowRelation } from '../../types/social';
import { leaderboardService } from './leaderboard.service';

interface MirrorPosition {
  followerAddress: string;
  leaderAddress: string;
  assetAddress: string;
  proportionalAmount: string;
  allocatedAt: string;
}

const MIN_FOLLOWER_INVESTMENT = 1000000n; // 1 XLM in stroops

class CopyTradingService {
  private follows: Map<string, FollowRelation> = new Map();
  private mirrorPositions: MirrorPosition[] = [];

  follow(followerAddress: string, leaderAddress: string, amount: string): FollowRelation {
    const investAmount = BigInt(amount);
    if (investAmount < MIN_FOLLOWER_INVESTMENT) {
      throw new Error(`Minimum investment is ${MIN_FOLLOWER_INVESTMENT} stroops`);
    }

    const key = `${followerAddress}:${leaderAddress}`;
    if (this.follows.has(key)) {
      throw new Error('Already following this leader');
    }

    const relation: FollowRelation = {
      followerAddress,
      leaderAddress,
      investedAmount: amount,
      proportionalAllocation: 0,
      totalProfit: '0',
      leaderProfitShare: '0',
      startedAt: new Date().toISOString(),
      active: true,
    };

    this.follows.set(key, relation);
    leaderboardService.addFollower(leaderAddress, followerAddress);
    return relation;
  }

  unfollow(followerAddress: string, leaderAddress: string): FollowRelation | null {
    const key = `${followerAddress}:${leaderAddress}`;
    const relation = this.follows.get(key);
    if (!relation) return null;

    this.follows.set(key, { ...relation, active: false });
    leaderboardService.removeFollower(leaderAddress, followerAddress);
    return { ...relation, active: false };
  }

  getFollowRelation(followerAddress: string, leaderAddress: string): FollowRelation | null {
    const key = `${followerAddress}:${leaderAddress}`;
    return this.follows.get(key) || null;
  }

  getFollowers(leaderAddress: string): FollowRelation[] {
    const result: FollowRelation[] = [];
    for (const [, relation] of this.follows) {
      if (relation.leaderAddress === leaderAddress && relation.active) {
        result.push(relation);
      }
    }
    return result;
  }

  getFollowing(followerAddress: string): FollowRelation[] {
    const result: FollowRelation[] = [];
    for (const [, relation] of this.follows) {
      if (relation.followerAddress === followerAddress) {
        result.push(relation);
      }
    }
    return result;
  }

  mirrorLeaderPosition(
    followerAddress: string,
    leaderAddress: string,
    assetAddress: string,
    leaderAmount: string,
    leaderTotalValue: string,
    followerInvestment: string
  ): MirrorPosition {
    const proportion = Number(BigInt(followerInvestment)) / Number(BigInt(leaderTotalValue));
    const proportionalAmount = BigInt(Math.floor(Number(BigInt(leaderAmount)) * proportion)).toString();

    const position: MirrorPosition = {
      followerAddress,
      leaderAddress,
      assetAddress,
      proportionalAmount,
      allocatedAt: new Date().toISOString(),
    };

    this.mirrorPositions.push(position);
    return position;
  }

  calculateProfitShare(followerProfit: string, leaderSharePercent: number = 10): string {
    const profit = BigInt(followerProfit);
    if (profit <= 0n) return '0';
    return (profit * BigInt(leaderSharePercent) / 100n).toString();
  }
}

export const copyTradingService = new CopyTradingService();
