process.env.DRY_RUN = 'true';
process.env.STELLAR_NETWORK = 'testnet';
process.env.MIN_PROFIT_THRESHOLD_XLM = '0.5';
process.env.POLL_INTERVAL_MS = '60000';
process.env.LOG_LEVEL = 'error';

import { LiquidationBotService } from '../liquidationBot.service';
import { loadConfig } from '../config';

describe('LiquidationBotService', () => {
  let bot: LiquidationBotService;

  beforeEach(() => {
    bot = new LiquidationBotService();
  });

  afterEach(() => {
    bot.stop();
  });

  describe('config', () => {
    it('should load configuration from environment', () => {
      const config = loadConfig();
      expect(config.dryRun).toBe(true);
      expect(config.stellarNetwork).toBe('testnet');
      expect(config.minProfitThresholdXlm).toBe(0.5);
    });
  });

  describe('stats', () => {
    it('should return zero stats on initialization', () => {
      const stats = bot.getStats();
      expect(stats.totalAttempted).toBe(0);
      expect(stats.totalSuccessful).toBe(0);
      expect(stats.totalProfit).toBe(0);
    });
  });

  describe('start and stop', () => {
    it('should start and stop without errors', () => {
      expect(() => bot.start()).not.toThrow();
      expect(() => bot.stop()).not.toThrow();
    });
  });
});
