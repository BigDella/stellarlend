/**
 * Memory Stability Stress Test for Oracle Service
 *
 * Uses fake timers to fast-forward through extended operation and verifies:
 * - Cache Map size stays bounded (no unbounded growth)
 * - setInterval is properly cleared on stop()
 * - No event listener accumulation on the process object
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { OracleService } from '../src/index.js';
import type { OracleServiceConfig } from '../src/config.js';

vi.mock('../src/services/contract-updater.js', () => ({
  createContractUpdater: vi.fn(() => ({
    updatePrices: vi
      .fn()
      .mockResolvedValue([{ success: true, asset: 'XLM', price: 150000n, timestamp: Date.now() }]),
    healthCheck: vi.fn().mockResolvedValue(true),
    getAdminPublicKey: vi.fn().mockReturnValue('GTEST123'),
  })),
  ContractUpdater: vi.fn(),
}));

vi.mock('../src/providers/coingecko.js', () => ({
  createCoinGeckoProvider: vi.fn(() => ({
    name: 'coingecko',
    isEnabled: true,
    priority: 1,
    weight: 0.6,
    getSupportedAssets: () => ['XLM', 'BTC', 'ETH'],
    fetchPrice: vi.fn().mockResolvedValue({
      asset: 'XLM',
      price: 0.15,
      timestamp: Math.floor(Date.now() / 1000),
      source: 'coingecko',
    }),
  })),
}));

vi.mock('../src/providers/binance.js', () => ({
  createBinanceProvider: vi.fn(() => ({
    name: 'binance',
    isEnabled: true,
    priority: 2,
    weight: 0.4,
    getSupportedAssets: () => ['XLM', 'BTC', 'ETH'],
    fetchPrice: vi.fn().mockResolvedValue({
      asset: 'XLM',
      price: 0.152,
      timestamp: Math.floor(Date.now() / 1000),
      source: 'binance',
    }),
  })),
}));

const BASE_CONFIG: OracleServiceConfig = {
  stellarNetwork: 'testnet',
  stellarRpcUrl: 'https://soroban-testnet.stellar.org',
  contractId: 'CTEST123',
  adminSecretKey: 'STEST123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ123456',
  updateIntervalMs: 1000,
  maxPriceDeviationPercent: 10,
  priceStaleThresholdSeconds: 300,
  cacheTtlSeconds: 30,
  logLevel: 'error',
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

describe('OracleService Memory Stability', () => {
  let service: OracleService;

  beforeEach(() => {
    vi.useFakeTimers();
    service = new OracleService({ ...BASE_CONFIG });
  });

  afterEach(async () => {
    await service.stop();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it('cache size stays bounded after many update cycles', async () => {
    const assets = ['XLM', 'BTC', 'ETH'];

    // Run 500 update cycles directly (no real time needed)
    for (let i = 0; i < 500; i++) {
      await service.updatePrices(assets);
    }

    const stats = service.getStatus().aggregatorStats;
    // Cache has maxEntries=100 by default; it must never exceed that
    expect(stats.cacheStats.size).toBeLessThanOrEqual(100);
    // With only 3 assets the cache should hold at most 3 entries
    expect(stats.cacheStats.size).toBeLessThanOrEqual(assets.length);
  });

  it('interval is cleared after stop()', async () => {
    const clearIntervalSpy = vi.spyOn(globalThis, 'clearInterval');

    await service.start(['XLM']);
    expect(service.getStatus().isRunning).toBe(true);

    await service.stop();

    expect(clearIntervalSpy).toHaveBeenCalledTimes(1);
    expect(service.getStatus().isRunning).toBe(false);

    clearIntervalSpy.mockRestore();
  });

  it('no new process event listeners accumulate across start/stop cycles', async () => {
    const listenersBefore = process.listenerCount('uncaughtException');

    for (let i = 0; i < 10; i++) {
      await service.start(['XLM']);
      await service.stop();
      // Re-create service to simulate repeated instantiation
      service = new OracleService({ ...BASE_CONFIG });
    }

    const listenersAfter = process.listenerCount('uncaughtException');
    expect(listenersAfter).toBeLessThanOrEqual(listenersBefore + 1);
  });

  it('fast-forwarded timer triggers updates without growing circuit breaker Maps', async () => {
    await service.start(['XLM', 'BTC']);

    const tickCount = 50;
    for (let i = 0; i < tickCount; i++) {
      await vi.advanceTimersByTimeAsync(BASE_CONFIG.updateIntervalMs);
    }

    const metrics = service.getStatus().circuitBreakers;
    // Circuit breaker Map is fixed at provider count — must not grow
    expect(metrics.length).toBe(2); // coingecko + binance
  });

  it('service is fully stopped and interval does not fire after stop()', async () => {
    const updateSpy = vi.spyOn(service, 'updatePrices');

    await service.start(['XLM']);
    const callsAfterStart = updateSpy.mock.calls.length;

    await service.stop();

    // Advance time well past the interval — no additional calls expected
    await vi.advanceTimersByTimeAsync(BASE_CONFIG.updateIntervalMs * 10);

    expect(updateSpy.mock.calls.length).toBe(callsAfterStart);
  });
});
