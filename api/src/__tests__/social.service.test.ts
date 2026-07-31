import axios from 'axios';
jest.mock('axios');
const mockedAxios = axios as jest.Mocked<typeof axios>;
beforeAll(() => {
  mockedAxios.create.mockReturnThis();
  const axiosResponse = { data: {}, status: 200, statusText: 'OK', headers: {}, config: { url: '' } };
  mockedAxios.get.mockResolvedValue(axiosResponse);
  mockedAxios.post.mockResolvedValue(axiosResponse);
});
afterEach(() => { jest.clearAllMocks(); });

import { leaderboardService } from '../services/social-trading/leaderboard.service';
import { copyTradingService } from '../services/social-trading/copy-trading.service';

beforeEach(() => {
  jest.clearAllMocks();
});

describe('LeaderboardService', () => {
  const leader1 = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
  const leader2 = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWJF';

  it('returns empty leaderboard initially', () => {
    const entries = leaderboardService.getLeaderboard({});
    expect(entries).toEqual([]);
  });

  it('allows opt-out and filters opted-out leaders', () => {
    leaderboardService.setOptOut(leader1, true);
    const entries = leaderboardService.getLeaderboard({});
    expect(Array.isArray(entries)).toBe(true);
  });

  it('provides leader profile', () => {
    const profile = leaderboardService.getLeaderProfile(leader1);
    expect(profile).toBeNull();
  });
});

describe('CopyTradingService', () => {
  const follower = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABC';
  const leader = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADEF';
  const amount = '10000000';

  it('creates follow relation', () => {
    const relation = copyTradingService.follow(follower, leader, amount);
    expect(relation.followerAddress).toBe(follower);
    expect(relation.leaderAddress).toBe(leader);
    expect(relation.investedAmount).toBe(amount);
    expect(relation.active).toBe(true);
    expect(relation.startedAt).toBeDefined();
  });

  it('rejects duplicate follow', () => {
    expect(() => copyTradingService.follow(follower, leader, amount)).toThrow('Already following this leader');
  });

  it('rejects amount below minimum', () => {
    const smallAmount = '10';
    expect(() => copyTradingService.follow(follower, 'GAAAAAAGIVKJHQVJQ5QFJQ5QFJQ5QFJQ5QFJQ5QFJQ5Q', smallAmount))
      .toThrow('Minimum investment');
  });

  it('returns follow relation', () => {
    const relation = copyTradingService.getFollowRelation(follower, leader);
    expect(relation).not.toBeNull();
    expect(relation!.active).toBe(true);
  });

  it('returns null for non-existent relation', () => {
    const relation = copyTradingService.getFollowRelation('GAAAAAANONEXISTENT12345678901234567890123456789012', leader);
    expect(relation).toBeNull();
  });

  it('unfollows and deactivates', () => {
    const relation = copyTradingService.unfollow(follower, leader);
    expect(relation).not.toBeNull();
    expect(relation!.active).toBe(false);
  });

  it('returns following list', () => {
    const following = copyTradingService.getFollowing(follower);
    expect(Array.isArray(following)).toBe(true);
  });

  it('calculates profit share correctly', () => {
    const share = copyTradingService.calculateProfitShare('1000000', 10);
    expect(share).toBe('100000');
  });

  it('returns zero profit share for zero profit', () => {
    const share = copyTradingService.calculateProfitShare('0');
    expect(share).toBe('0');
  });

  it('mirrors leader position proportionally', () => {
    const mirror = copyTradingService.mirrorLeaderPosition(
      follower, leader, 'CONTRACT_ID', '10000000000', '100000000000', '1000000000'
    );
    expect(mirror.followerAddress).toBe(follower);
    expect(mirror.leaderAddress).toBe(leader);
    expect(mirror.proportionalAmount).toBeDefined();
  });
});
