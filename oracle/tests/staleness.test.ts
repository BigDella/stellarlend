/**
 * Tests for Oracle Price Staleness Detection
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { OracleService } from '../src/index.js';
import { logger, logStalenessAlert } from '../src/utils/logger.js';

// Mock logger to verify calls
vi.mock('../src/utils/logger.js', async () => {
  const actual = await vi.importActual('../src/utils/logger.js');
  return {
    ...actual,
    logger: {
      info: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
      debug: vi.fn(),
    },
    logStalenessAlert: vi.fn(),
  };
});

// Mock contract updater
vi.mock('../src/services/contract-updater.js', () => ({
  createContractUpdater: vi.fn(() => ({
    updatePrices: vi
      .fn()
      .mockResolvedValue([{ success: true, asset: 'XLM', price: 100n, timestamp: Date.now() }]),
    healthCheck: vi.fn().mockResolvedValue(true),
    getAdminPublicKey: vi.fn().mockReturnValue('GTEST123'),
  })),
  ContractUpdater: vi.fn(),
}));

const mockAggregator = {
  getPrices: vi
    .fn()
    .mockResolvedValue(new Map([['XLM', { asset: 'XLM', price: 100n, timestamp: Date.now() }]])),
  getPrice: vi.fn(),
  getProviders: vi.fn().mockReturnValue([]),
  getStats: vi.fn().mockReturnValue({}),
  getCircuitBreakerMetrics: vi.fn().mockReturnValue([]),
};

vi.mock('../src/services/index.js', () => ({
  createValidator: vi.fn(() => ({ validate: vi.fn() })),
  createPriceCache: vi.fn(() => ({ get: vi.fn(), set: vi.fn(), getStats: vi.fn(() => ({})) })),
  createPriceHistoryService: vi.fn(() => ({
    addAggregatedPrice: vi.fn(),
    getStats: vi.fn(() => ({})),
  })),
  createAggregator: vi.fn(() => mockAggregator),
  createContractUpdater: vi.fn(() => ({
    updatePrices: vi
      .fn()
      .mockResolvedValue([{ success: true, asset: 'XLM', price: 100n, timestamp: Date.now() }]),
    healthCheck: vi.fn().mockResolvedValue(true),
    getAdminPublicKey: vi.fn().mockReturnValue('GTEST123'),
  })),
  createTWAPService: vi.fn(() => ({
    recordObservation: vi.fn(),
    getTWAPStatus: vi.fn(() => ({ twap: 150000n, manipulationDetected: false, deviationBps: 0n })),
    getStatus: vi.fn(),
  })),
  createMetricsService: vi.fn(() => ({
    start: vi.fn(),
    stop: vi.fn(),
    recordUpdate: vi.fn(),
    recordError: vi.fn(),
    updateAssetPrice: vi.fn(),
    updateProviderHealth: vi.fn(),
    getUptime: vi.fn(),
  })),
}));

describe('Oracle Price Staleness Detection', () => {
  let service: OracleService;
  const STALE_THRESHOLD = 300; // 5 minutes

  const mockConfig: any = {
    stellarNetwork: 'testnet',
    stellarRpcUrl: 'http://localhost:8000',
    contractId: 'CTEST123',
    adminSecretKey: 'S123',
    updateIntervalMs: 60000,
    maxPriceDeviationPercent: 10,
    priceStaleThresholdSeconds: STALE_THRESHOLD,
    cacheTtlSeconds: 30,
    logLevel: 'info',
    providers: [],
  };

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-03-24T12:00:00Z'));
    service = new OracleService(mockConfig);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('should not log staleness alert on first update', async () => {
    await service.updatePrices(['XLM']);
    expect(logStalenessAlert).not.toHaveBeenCalled();
  });

  it('should not log staleness alert if update happens within threshold', async () => {
    // First successful update
    await service.updatePrices(['XLM']);

    // Advance time by 4 minutes (less than 5m threshold)
    vi.advanceTimersByTime(4 * 60 * 1000);

    await service.updatePrices(['XLM']);
    expect(logStalenessAlert).not.toHaveBeenCalled();
  });

  it('should log staleness alert if update age exceeds threshold', async () => {
    // First successful update sets lastSuccessfulUpdate
    await service.updatePrices(['XLM']);

    // Set last update far in the past (relative to mocked time)
    // vi.setSystemTime sets Date.now() to 2026-03-24T12:00:00Z = 1743321600000
    // Set lastSuccessfulUpdate to 1 hour before that
    (service as any).lastSuccessfulUpdate = new Date('2026-03-24T11:00:00Z').getTime();

    await service.updatePrices(['XLM']);
    expect(logStalenessAlert).toHaveBeenCalled();
  });

  it('should update lastSuccessfulUpdate after a successful cycle', async () => {
    // First update
    await service.updatePrices(['XLM']);

    // Advance time by 4 minutes
    vi.advanceTimersByTime(4 * 60 * 1000);
    await service.updatePrices(['XLM']);

    // Advance another 4 minutes (total 8 from start, but only 4 from last update)
    vi.advanceTimersByTime(4 * 60 * 1000);
    await service.updatePrices(['XLM']);

    expect(logStalenessAlert).not.toHaveBeenCalled();
  });

  it('should log alert even if price fetching fails but cycle starts', async () => {
    // First success
    await service.updatePrices(['XLM']);
    const firstUpdate = (service as any).lastSuccessfulUpdate;

    // Advance beyond threshold
    vi.advanceTimersByTime(6 * 60 * 1000);

    // Mock failure for the NEXT getPrices call
    mockAggregator.getPrices.mockRejectedValueOnce(new Error('API Down'));

    await service.updatePrices(['XLM']);

    expect((service as any).lastSuccessfulUpdate).toBe(firstUpdate);
    expect(logger.error).toHaveBeenCalledWith(
      'Price update cycle failed',
      expect.objectContaining({
        error: expect.any(Error),
      })
    );
  });
});
