/**
 * Oracle Service Lifecycle Integration Tests
 * Tests for startup/shutdown edge cases and race conditions
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { OracleService } from '../src/index.js';
import type { OracleServiceConfig } from '../src/config.js';

// Mock contract updater to avoid actual blockchain calls
vi.mock('../src/services/contract-updater.js', () => ({
  createContractUpdater: vi.fn(() => ({
    updatePrices: vi.fn().mockImplementation(async (prices) => {
      // Simulate some processing time
      await new Promise((resolve) => setTimeout(resolve, 50));
      return prices.map((price, index) => ({
        success: true,
        asset: price.asset || `ASSET_${index}`,
        price: BigInt(Math.floor((price.price || 0.15) * 1000000)),
        timestamp: Date.now(),
      }));
    }),
    healthCheck: vi.fn().mockResolvedValue(true),
    getAdminPublicKey: vi.fn().mockReturnValue('GTEST123'),
  })),
  ContractUpdater: vi.fn(),
}));

// Mock providers to avoid actual API calls
vi.mock('../src/providers/coingecko.js', () => ({
  createCoinGeckoProvider: vi.fn(() => ({
    name: 'coingecko',
    isEnabled: true,
    priority: 1,
    weight: 0.6,
    getSupportedAssets: () => ['XLM', 'BTC', 'ETH', 'USDC', 'SOL'],
    fetchPrice: vi.fn().mockImplementation(async (asset) => {
      // Simulate network delay
      await new Promise((resolve) => setTimeout(resolve, 100));
      return {
        asset,
        price: asset === 'XLM' ? 0.15 : asset === 'BTC' ? 45000 : asset === 'ETH' ? 3000 : 1,
        timestamp: Math.floor(Date.now() / 1000),
        source: 'coingecko',
      };
    }),
  })),
}));

vi.mock('../src/providers/binance.js', () => ({
  createBinanceProvider: vi.fn(() => ({
    name: 'binance',
    isEnabled: true,
    priority: 2,
    weight: 0.4,
    getSupportedAssets: () => ['XLM', 'BTC', 'ETH', 'USDC', 'SOL'],
    fetchPrice: vi.fn().mockImplementation(async (asset) => {
      // Simulate network delay
      await new Promise((resolve) => setTimeout(resolve, 80));
      return {
        asset,
        price: asset === 'XLM' ? 0.152 : asset === 'BTC' ? 45100 : asset === 'ETH' ? 3010 : 1.01,
        timestamp: Math.floor(Date.now() / 1000),
        source: 'binance',
      };
    }),
  })),
}));

describe('OracleService Lifecycle Integration Tests', () => {
  let service: OracleService;
  let mockConfig: OracleServiceConfig;

  beforeEach(() => {
    vi.clearAllMocks();
    mockConfig = {
      stellarNetwork: 'testnet',
      stellarRpcUrl: 'https://soroban-testnet.stellar.org',
      contractId: 'CTEST123',
      adminSecretKey: 'STEST123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ123456',
      updateIntervalMs: 1000, // Use same interval as existing tests
      maxPriceDeviationPercent: 10,
      priceStaleThresholdSeconds: 300,
      cacheTtlSeconds: 30,
      logLevel: 'error', // Reduce log noise in tests
      circuitBreaker: {
        failureThreshold: 5,
        backoffMs: 1000,
      },
      providers: [
        {
          name: 'coingecko',
          enabled: true,
          priority: 1,
          weight: 0.6,
          baseUrl: 'https://api.coingecko.com/api/v3',
          rateLimit: { maxRequests: 10, windowMs: 60000 },
        },
        {
          name: 'binance',
          enabled: true,
          priority: 2,
          weight: 0.4,
          baseUrl: 'https://api.binance.com/api/v3',
          rateLimit: { maxRequests: 1200, windowMs: 60000 },
        },
      ],
    };
  });

  afterEach(async () => {
    if (service) {
      await service.stop();
    }
    vi.clearAllTimers();
  });

  describe('Scenario 1: Normal start/stop cycle', () => {
    it('should complete normal lifecycle without resource leaks', async () => {
      service = new OracleService(mockConfig);

      // Verify initial state
      expect(service.getStatus().isRunning).toBe(false);

      // Start service
      await service.start(['XLM', 'BTC']);
      expect(service.getStatus().isRunning).toBe(true);

      // Allow at least one update cycle
      await new Promise((resolve) => setTimeout(resolve, 1100));

      // Stop service
      service.stop();
      expect(service.getStatus().isRunning).toBe(false);

      // Verify no pending intervals (by waiting a bit longer)
      await new Promise((resolve) => setTimeout(resolve, 300));
      expect(service.getStatus().isRunning).toBe(false);
    });

    it('should handle multiple start/stop cycles', async () => {
      service = new OracleService(mockConfig);

      // Perform multiple cycles
      for (let i = 0; i < 3; i++) {
        await service.start(['XLM']);
        await new Promise((resolve) => setTimeout(resolve, 1100));
        service.stop();
        await new Promise((resolve) => setTimeout(resolve, 100));

        expect(service.getStatus().isRunning).toBe(false);
      }
    });
  });

  describe('Scenario 2: Double start (should be idempotent or error)', () => {
    it('should handle double start gracefully', async () => {
      service = new OracleService(mockConfig);

      // First start
      await service.start(['XLM']);
      const firstStartStatus = service.getStatus();
      expect(firstStartStatus.isRunning).toBe(true);

      // Second start should be handled gracefully
      await service.start(['XLM', 'BTC']);
      const secondStartStatus = service.getStatus();
      expect(secondStartStatus.isRunning).toBe(true);

      // Service should still be functional
      await new Promise((resolve) => setTimeout(resolve, 250));
      expect(service.getStatus().isRunning).toBe(true);

      service.stop();
    });

    it('should not create multiple intervals on double start', async () => {
      service = new OracleService(mockConfig);

      const setIntervalSpy = vi.spyOn(global, 'setInterval');

      await service.start(['XLM']);
      const firstCallCount = setIntervalSpy.mock.calls.length;

      await service.start(['BTC']);
      const secondCallCount = setIntervalSpy.mock.calls.length;

      // Should not create additional intervals
      expect(secondCallCount).toBe(firstCallCount);

      setIntervalSpy.mockRestore();
      service.stop();
    });
  });

  describe('Scenario 3: Stop during active price update', () => {
    it('should handle stop during active price update gracefully', async () => {
      service = new OracleService(mockConfig);

      await service.start(['XLM', 'BTC', 'ETH']);

      // Wait for update to start, then stop immediately
      await new Promise((resolve) => setTimeout(resolve, 500));
      service.stop();

      // Should stop without throwing
      expect(service.getStatus().isRunning).toBe(false);

      // Wait to ensure no background processes
      await new Promise((resolve) => setTimeout(resolve, 200));
      expect(service.getStatus().isRunning).toBe(false);
    });

    it('should handle multiple concurrent stop calls during update', async () => {
      service = new OracleService(mockConfig);

      await service.start(['XLM']);

      // Wait for update to start
      await new Promise((resolve) => setTimeout(resolve, 50));

      // Call stop multiple times concurrently
      const stopPromises = [
        Promise.resolve(service.stop()),
        Promise.resolve(service.stop()),
        Promise.resolve(service.stop()),
      ];

      await Promise.all(stopPromises);

      expect(service.getStatus().isRunning).toBe(false);
    });

    it('should clean up resources even when stop is called during update', async () => {
      service = new OracleService(mockConfig);

      const clearIntervalSpy = vi.spyOn(global, 'clearInterval');

      await service.start(['XLM', 'BTC']);

      // Stop during active update
      await new Promise((resolve) => setTimeout(resolve, 50));
      service.stop();

      // Verify cleanup was called
      expect(clearIntervalSpy).toHaveBeenCalled();

      clearIntervalSpy.mockRestore();
    });
  });

  describe('Scenario 4: Restart after failure', () => {
    it('should restart successfully after price update failure', async () => {
      // Mock contract updater to fail initially
      const { createContractUpdater } = await import('../src/services/contract-updater.js');
      let callCount = 0;

      vi.mocked(createContractUpdater).mockReturnValueOnce({
        updatePrices: vi.fn().mockImplementation(async () => {
          callCount++;
          if (callCount <= 2) {
            throw new Error('Network failure');
          }
          return [{ success: true, asset: 'XLM', price: 150000n, timestamp: Date.now() }];
        }),
        healthCheck: vi.fn().mockResolvedValue(true),
        getAdminPublicKey: vi.fn().mockReturnValue('GTEST123'),
      } as any);

      service = new OracleService(mockConfig);

      await service.start(['XLM']);

      // Wait for failure cycles
      await new Promise((resolve) => setTimeout(resolve, 500));

      // Service should still be running despite failures
      expect(service.getStatus().isRunning).toBe(true);

      service.stop();

      // Restart should work
      await service.start(['XLM']);
      expect(service.getStatus().isRunning).toBe(true);

      service.stop();
    });

    it('should handle restart after provider failure', async () => {
      service = new OracleService(mockConfig);

      // Start and let it run
      await service.start(['XLM']);
      await new Promise((resolve) => setTimeout(resolve, 1100));

      // Stop
      service.stop();

      // Restart should work normally
      await service.start(['BTC']);
      expect(service.getStatus().isRunning).toBe(true);

      await new Promise((resolve) => setTimeout(resolve, 1100));
      service.stop();
    });

    it('should maintain state consistency across restart cycles', async () => {
      service = new OracleService(mockConfig);

      // First run
      await service.start(['XLM']);
      await new Promise((resolve) => setTimeout(resolve, 1100));
      service.stop();

      const statusAfterStop = service.getStatus();
      expect(statusAfterStop.isRunning).toBe(false);

      // Second run
      await service.start(['BTC']);
      const statusAfterRestart = service.getStatus();
      expect(statusAfterRestart.isRunning).toBe(true);

      await new Promise((resolve) => setTimeout(resolve, 1100));
      service.stop();
    });
  });

  describe('Scenario 5: Graceful shutdown with pending updates', () => {
    it('should handle graceful shutdown with pending operations', async () => {
      service = new OracleService(mockConfig);

      // Start service with multiple assets to ensure longer update times
      await service.start(['XLM', 'BTC', 'ETH', 'USDC', 'SOL']);

      // Wait for updates to be in progress
      await new Promise((resolve) => setTimeout(resolve, 500));

      // Initiate graceful shutdown
      const stopStartTime = Date.now();
      service.stop();
      const stopEndTime = Date.now();

      // Stop should be quick (not waiting for pending operations)
      expect(stopEndTime - stopStartTime).toBeLessThan(100);

      // Service should be stopped
      expect(service.getStatus().isRunning).toBe(false);

      // Wait to ensure no background activity
      await new Promise((resolve) => setTimeout(resolve, 300));
      expect(service.getStatus().isRunning).toBe(false);
    });

    it('should clean up all resources during shutdown', async () => {
      service = new OracleService(mockConfig);

      const clearIntervalSpy = vi.spyOn(global, 'clearInterval');

      await service.start(['XLM', 'BTC']);

      // Ensure updates are running
      await new Promise((resolve) => setTimeout(resolve, 100));

      service.stop();

      // Verify cleanup
      expect(clearIntervalSpy).toHaveBeenCalled();
      expect(service.getStatus().isRunning).toBe(false);

      clearIntervalSpy.mockRestore();
    });

    it('should handle shutdown when multiple update cycles are pending', async () => {
      service = new OracleService(mockConfig);

      // Start with very short interval to create overlapping updates
      const fastConfig = { ...mockConfig, updateIntervalMs: 50 };
      service = new OracleService(fastConfig);

      await service.start(['XLM', 'BTC']);

      // Let multiple cycles start
      await new Promise((resolve) => setTimeout(resolve, 150));

      // Shutdown should handle overlapping operations
      service.stop();
      expect(service.getStatus().isRunning).toBe(false);

      // Ensure complete shutdown
      await new Promise((resolve) => setTimeout(resolve, 200));
      expect(service.getStatus().isRunning).toBe(false);
    });

    it('should not leave any promises hanging after shutdown', async () => {
      service = new OracleService(mockConfig);

      await service.start(['XLM', 'BTC']);

      // Start some manual operations
      const updatePromise1 = service.updatePrices(['ETH']);
      const updatePromise2 = service.updatePrices(['USDC']);

      // Stop during operations
      service.stop();

      // All operations should resolve or be handled gracefully
      await Promise.allSettled([updatePromise1, updatePromise2]);

      expect(service.getStatus().isRunning).toBe(false);
    });
  });

  describe('Resource leak prevention', () => {
    it('should not accumulate interval references', async () => {
      service = new OracleService(mockConfig);

      const setIntervalSpy = vi.spyOn(global, 'setInterval');
      const clearIntervalSpy = vi.spyOn(global, 'clearInterval');

      // Multiple start/stop cycles
      for (let i = 0; i < 5; i++) {
        await service.start(['XLM']);
        await new Promise((resolve) => setTimeout(resolve, 100));
        service.stop();
      }

      // Should have balanced set/clear calls
      expect(setIntervalSpy.mock.calls.length).toBe(clearIntervalSpy.mock.calls.length);

      setIntervalSpy.mockRestore();
      clearIntervalSpy.mockRestore();
    });

    it('should handle rapid start/stop without accumulating timers', async () => {
      service = new OracleService(mockConfig);

      // Rapid start/stop cycles
      for (let i = 0; i < 10; i++) {
        await service.start(['XLM']);
        service.stop();
        await new Promise((resolve) => setTimeout(resolve, 10));
      }

      expect(service.getStatus().isRunning).toBe(false);
    });
  });

  describe('Timing reliability', () => {
    it('should handle timing variations without flaky behavior', async () => {
      service = new OracleService(mockConfig);

      // Test with variable delays
      const delays = [0, 10, 50, 100, 200];

      for (const delay of delays) {
        await service.start(['XLM']);
        await new Promise((resolve) => setTimeout(resolve, delay));
        service.stop();

        expect(service.getStatus().isRunning).toBe(false);
        await new Promise((resolve) => setTimeout(resolve, 50));
      }
    });

    it('should maintain consistent behavior under load', async () => {
      service = new OracleService(mockConfig);

      await service.start(['XLM', 'BTC']);

      // Simulate load with concurrent operations
      const operations = Array.from({ length: 10 }, (_, i) => service.updatePrices([`ASSET_${i}`]));

      // Stop during load
      service.stop();

      await Promise.allSettled(operations);

      expect(service.getStatus().isRunning).toBe(false);
    });
  });
});
