import { protocolHealthScoreService } from '../services/protocol-health/healthScore.service';
import { redisCacheService } from '../services/redisCache.service';
import { stakingService } from '../services/staking.service';
import { StellarService } from '../services/stellar.service';

describe('protocolHealthScoreService', () => {
  let getProtocolStatsSpy: jest.SpyInstance;

  beforeEach(() => {
    redisCacheService.clearAllForTests();
    // getAnalyticsSummary (used for the capital-efficiency component) calls
    // through to StellarService.getProtocolStats, which performs a real
    // Soroban simulation call — mock it the same way analytics.service.test.ts
    // mocks getPoolRateAt, so this suite runs offline and deterministically.
    getProtocolStatsSpy = jest.spyOn(StellarService.prototype, 'getProtocolStats').mockResolvedValue({
      totalDeposits: '10000000',
      totalBorrows: '6000000',
      utilizationRate: 0.6,
      numberOfUsers: 42,
      tvl: '10000000',
    });
    // Reset weights/threshold to defaults between tests since the service is a singleton.
    protocolHealthScoreService.updateWeights({
      capitalEfficiency: 0.15,
      liquidity: 0.2,
      badDebt: 0.25,
      concentration: 0.15,
      oracleHealth: 0.15,
      governanceHealth: 0.1,
    });
    protocolHealthScoreService.setAlertThreshold(60);
  });

  afterEach(() => {
    getProtocolStatsSpy.mockRestore();
  });

  describe('getHealthScore', () => {
    it('returns an overall score and components all within [0, 100]', async () => {
      const result = await protocolHealthScoreService.getHealthScore();

      expect(result.overallScore).toBeGreaterThanOrEqual(0);
      expect(result.overallScore).toBeLessThanOrEqual(100);
      for (const value of Object.values(result.components)) {
        expect(value).toBeGreaterThanOrEqual(0);
        expect(value).toBeLessThanOrEqual(100);
      }
    });

    it('caches the result so repeated calls are stable within the TTL', async () => {
      const first = await protocolHealthScoreService.getHealthScore();
      const second = await protocolHealthScoreService.getHealthScore();
      expect(second.timestamp).toBe(first.timestamp);
    });

    it('appends to history on each fresh computation', async () => {
      redisCacheService.clearAllForTests();
      const before = protocolHealthScoreService.getHistory().length;
      await protocolHealthScoreService.getHealthScore();
      const after = protocolHealthScoreService.getHistory().length;
      expect(after).toBe(before + 1);
    });
  });

  describe('weights', () => {
    it('normalizes weights to sum to 1', () => {
      const updated = protocolHealthScoreService.updateWeights({ badDebt: 1 });
      const sum = Object.values(updated).reduce((s, v) => s + v, 0);
      expect(sum).toBeCloseTo(1, 5);
    });

    it('rejects weights that sum to zero or less', () => {
      expect(() =>
        protocolHealthScoreService.updateWeights({
          capitalEfficiency: 0,
          liquidity: 0,
          badDebt: 0,
          concentration: 0,
          oracleHealth: 0,
          governanceHealth: 0,
        })
      ).toThrow();
    });
  });

  describe('alerts', () => {
    it('produces no alert when the threshold is very low', async () => {
      protocolHealthScoreService.setAlertThreshold(0);
      redisCacheService.clearAllForTests();
      const alerts = await protocolHealthScoreService.getAlerts();
      expect(alerts).toHaveLength(0);
    });

    it('produces an alert when the threshold exceeds the current score', async () => {
      protocolHealthScoreService.setAlertThreshold(100);
      redisCacheService.clearAllForTests();
      const alerts = await protocolHealthScoreService.getAlerts();
      expect(alerts.length).toBeGreaterThan(0);
      expect(alerts[0]!.threshold).toBe(100);
    });

    it('rejects an out-of-range threshold', () => {
      expect(() => protocolHealthScoreService.setAlertThreshold(150)).toThrow();
    });
  });

  describe('governance health component', () => {
    it('falls back to a neutral score with no staking positions', async () => {
      redisCacheService.clearAllForTests();
      const result = await protocolHealthScoreService.getHealthScore();
      expect(result.components.governanceHealth).toBe(50);
    });

    it('moves off the neutral fallback once a staking position exists', async () => {
      stakingService.stake({ userAddress: 'GTESTUSERHEALTHSCORE1', amount: '1000000000' });
      redisCacheService.clearAllForTests();
      const result = await protocolHealthScoreService.getHealthScore();
      expect(result.components.governanceHealth).not.toBe(50);
      expect(result.components.governanceHealth).toBeGreaterThanOrEqual(0);
      expect(result.components.governanceHealth).toBeLessThanOrEqual(100);
    });
  });
});
