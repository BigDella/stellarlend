import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AggregatedPrice, OracleServiceConfig } from '../src/types/index.js';
import { OracleService } from '../src/index.js';
import { logger } from '../src/utils/logger.js';
import { createContractUpdater, createAggregator } from '../src/services/index.js';

const aggregatedPrices = new Map<string, AggregatedPrice>([
  [
    'XLM',
    {
      asset: 'XLM',
      price: 150000n,
      timestamp: 1_711_111_111,
      confidence: 0.98,
      sources: [
        {
          asset: 'XLM',
          price: 150000n,
          timestamp: 1_711_111_111,
          source: 'coingecko',
          confidence: 0.99,
        },
      ],
    },
  ],
]);

const mockAggregator = {
  getPrices: vi.fn(async () => aggregatedPrices),
  getProviders: vi.fn(() => ['coingecko', 'binance']),
  getStats: vi.fn(() => ({ hits: 0, misses: 0 })),
  getCircuitBreakerMetrics: vi.fn(() => ({ coingecko: { state: 'closed' } })),
  getPrice: vi.fn(),
};

const mockContractUpdater = {
  updatePrices: vi.fn(),
};

vi.mock('../src/providers/index.js', () => ({
  createCoinGeckoProvider: vi.fn(() => ({ name: 'coingecko' })),
  createBinanceProvider: vi.fn(() => ({ name: 'binance' })),
}));

vi.mock('../src/services/index.js', () => ({
  createValidator: vi.fn(() => ({ validate: vi.fn() })),
  createPriceCache: vi.fn(() => ({ get: vi.fn(), set: vi.fn() })),
  createPriceHistoryService: vi.fn(() => ({
    addAggregatedPrice: vi.fn(),
    getStats: vi.fn(() => ({})),
  })),
  createAggregator: vi.fn(() => mockAggregator),
  createContractUpdater: vi.fn(() => mockContractUpdater),
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

vi.mock('../src/utils/logger.js', () => ({
  logger: {
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  },
  configureLogger: vi.fn(),
  logProviderHealth: vi.fn(),
  logStalenessAlert: vi.fn(),
}));

describe('OracleService dry run mode', () => {
  const baseConfig: OracleServiceConfig = {
    stellarNetwork: 'testnet',
    stellarRpcUrl: 'https://soroban-testnet.stellar.org',
    contractId: 'CTEST123',
    adminSecretKey: 'STEST123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ123456',
    dryRun: true,
    updateIntervalMs: 60000,
    maxPriceDeviationPercent: 10,
    priceStaleThresholdSeconds: 300,
    cacheTtlSeconds: 30,
    logLevel: 'info',
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
    circuitBreaker: {
      failureThreshold: 3,
      backoffMs: 30000,
    },
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockAggregator.getPrices.mockResolvedValue(aggregatedPrices);
    mockContractUpdater.updatePrices.mockResolvedValue([
      {
        success: true,
        asset: 'XLM',
        price: 150000n,
        timestamp: Date.now(),
      },
    ]);
  });

  it('skips contract updates while still fetching aggregated prices', async () => {
    const service = new OracleService(baseConfig);

    await service.updatePrices(['XLM']);

    expect(createAggregator).toHaveBeenCalled();
    expect(mockAggregator.getPrices).toHaveBeenCalledWith(['XLM']);
    expect(createContractUpdater).toHaveBeenCalled();
    expect(mockContractUpdater.updatePrices).not.toHaveBeenCalled();
  });

  it('logs a clearly labeled dry-run message with serialized prices', async () => {
    const service = new OracleService(baseConfig);

    await service.updatePrices(['XLM']);

    expect(logger.info).toHaveBeenCalledWith(
      'DRY RUN: Would update prices on contract',
      expect.objectContaining({
        dryRun: true,
        contractId: 'CTEST123',
        assets: ['XLM'],
        prices: [
          expect.objectContaining({
            asset: 'XLM',
            price: '150000',
            sources: ['coingecko'],
          }),
        ],
      })
    );
  });
});
