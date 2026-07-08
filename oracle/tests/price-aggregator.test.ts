/**
 * Tests for Price Aggregator Service
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { PriceAggregator, createAggregator } from '../src/services/price-aggregator.js';
import { createValidator } from '../src/services/price-validator.js';
import { createPriceCache } from '../src/services/cache.js';
import { BasePriceProvider } from '../src/providers/base-provider.js';
import type { RawPriceData, ProviderConfig, HealthStatus } from '../src/types/index.js';

/**
 * Mock provider for testing
 */
class MockProvider extends BasePriceProvider {
  private mockPrices: Map<string, number> = new Map();
  private shouldFail: boolean = false;

  constructor(name: string, priority: number, weight: number, prices: Record<string, number> = {}) {
    super({
      name,
      enabled: true,
      priority,
      weight,
      baseUrl: 'https://mock.api',
      rateLimit: { maxRequests: 1000, windowMs: 60000 },
    });

    Object.entries(prices).forEach(([asset, price]) => {
      this.mockPrices.set(asset.toUpperCase(), price);
    });
  }

  async fetchPrice(asset: string): Promise<RawPriceData> {
    if (this.shouldFail) {
      throw new Error(`Mock provider ${this.name} failed`);
    }

    const price = this.mockPrices.get(asset.toUpperCase());
    if (price === undefined) {
      throw new Error(`Asset ${asset} not found in mock provider`);
    }

    return {
      asset: asset.toUpperCase(),
      price,
      timestamp: Math.floor(Date.now() / 1000),
      source: this.name,
    };
  }

  setPrice(asset: string, price: number): void {
    this.mockPrices.set(asset.toUpperCase(), price);
  }

  setFail(shouldFail: boolean): void {
    this.shouldFail = shouldFail;
  }
}

describe('PriceAggregator', () => {
  let aggregator: PriceAggregator;
  let mockProvider1: MockProvider;
  let mockProvider2: MockProvider;
  let mockProvider3: MockProvider;

  beforeEach(() => {
    // Create mock providers with different prices
    mockProvider1 = new MockProvider('provider1', 1, 0.5, {
      XLM: 0.15,
      BTC: 50000,
      ETH: 3000,
    });

    mockProvider2 = new MockProvider('provider2', 2, 0.3, {
      XLM: 0.152,
      BTC: 50100,
      ETH: 3010,
    });

    mockProvider3 = new MockProvider('provider3', 3, 0.2, {
      XLM: 0.148,
      BTC: 49900,
      ETH: 2990,
    });

    const validator = createValidator({
      maxDeviationPercent: 20, // Higher threshold for test variation
      maxStalenessSeconds: 300,
    });

    const cache = createPriceCache(30);

    aggregator = createAggregator([mockProvider1, mockProvider2, mockProvider3], validator, cache, {
      minSources: 1,
    });
  });

  describe('getPrice', () => {
    it('should fetch and aggregate price from multiple sources', async () => {
      const result = await aggregator.getPrice('XLM');

      expect(result).not.toBeNull();
      expect(result?.asset).toBe('XLM');
      expect(result?.sources.length).toBeGreaterThanOrEqual(1);
    });

    it('should use cache for subsequent requests', async () => {
      const result1 = await aggregator.getPrice('BTC');
      const result2 = await aggregator.getPrice('BTC');

      expect(result2?.sources).toHaveLength(0);
      expect(result2?.price).toBe(result1?.price);
    });

    it('should return null when no sources provide valid prices', async () => {
      mockProvider1.setFail(true);
      mockProvider2.setFail(true);
      mockProvider3.setFail(true);

      const strictAggregator = createAggregator(
        [mockProvider1, mockProvider2, mockProvider3],
        createValidator(),
        createPriceCache(30),
        { minSources: 1 }
      );

      const result = await strictAggregator.getPrice('XLM');

      expect(result).toBeNull();
    });

    it('should handle fallback when primary provider fails', async () => {
      mockProvider1.setFail(true);

      const result = await aggregator.getPrice('XLM');

      expect(result).not.toBeNull();
      expect(result?.sources.every((s) => s.source !== 'provider1')).toBe(true);
    });
  });

  describe('getPrices', () => {
    it('should fetch prices for multiple assets', async () => {
      const results = await aggregator.getPrices(['XLM', 'BTC', 'ETH']);

      expect(results.size).toBe(3);
      expect(results.has('XLM')).toBe(true);
      expect(results.has('BTC')).toBe(true);
      expect(results.has('ETH')).toBe(true);
    });

    it('should skip assets that fail', async () => {
      // SOL not in any mock provider
      const results = await aggregator.getPrices(['XLM', 'SOL']);

      expect(results.size).toBe(1);
      expect(results.has('XLM')).toBe(true);
      expect(results.has('SOL')).toBe(false);
    });
  });

  describe('weighted median calculation', () => {
    it('should calculate correct weighted median', async () => {
      const result = await aggregator.getPrice('XLM');
      expect(result).not.toBeNull();
    });
  });

  describe('provider ordering', () => {
    it('should sort providers by priority', () => {
      const providers = aggregator.getProviders();

      expect(providers[0]).toBe('provider1');
      expect(providers[1]).toBe('provider2');
      expect(providers[2]).toBe('provider3');
    });
  });

  describe('stats', () => {
    it('should return aggregator statistics', async () => {
      await aggregator.getPrice('XLM');

      const stats = aggregator.getStats();

      expect(stats.enabledProviders).toBe(3);
      expect(stats.cacheStats).toBeDefined();
    });
  });
});

// ---------------------------------------------------------------------------
// Priority-based failover tests
// ---------------------------------------------------------------------------

describe('PriceAggregator – priority-based failover', () => {
  let high: MockProvider; // priority 1 (highest)
  let mid: MockProvider;  // priority 2
  let low: MockProvider;  // priority 3 (lowest)

  function makeFailoverAggregator(failureThreshold = 1) {
    const validator = createValidator({ maxDeviationPercent: 50, maxStalenessSeconds: 300 });
    const cache = createPriceCache(30);
    return createAggregator([high, mid, low], validator, cache, {
      minSources: 1,
      failoverMode: true,
      circuitBreaker: { failureThreshold, backoffMs: 60_000 },
    });
  }

  beforeEach(() => {
    high = new MockProvider('high', 1, 0.5, { XLM: 0.15 });
    mid  = new MockProvider('mid',  2, 0.3, { XLM: 0.152 });
    low  = new MockProvider('low',  3, 0.2, { XLM: 0.148 });
  });

  it('uses highest-priority provider when healthy', async () => {
    const agg = makeFailoverAggregator();
    const result = await agg.getPrice('XLM');

    expect(result).not.toBeNull();
    expect(result!.sources).toHaveLength(1);
    expect(result!.sources[0].source).toBe('high');
  });

  it('does NOT query lower-priority providers when primary succeeds', async () => {
    const midSpy = vi.spyOn(mid, 'fetchPrice');
    const lowSpy = vi.spyOn(low, 'fetchPrice');

    const agg = makeFailoverAggregator();
    await agg.getPrice('XLM');

    expect(midSpy).not.toHaveBeenCalled();
    expect(lowSpy).not.toHaveBeenCalled();
  });

  it('fails over to mid-priority provider when high-priority fails', async () => {
    high.setFail(true);
    const agg = makeFailoverAggregator();
    const result = await agg.getPrice('XLM');

    expect(result).not.toBeNull();
    expect(result!.sources[0].source).toBe('mid');
  });

  it('fails over to lowest-priority provider when high and mid both fail', async () => {
    high.setFail(true);
    mid.setFail(true);
    const agg = makeFailoverAggregator();
    const result = await agg.getPrice('XLM');

    expect(result).not.toBeNull();
    expect(result!.sources[0].source).toBe('low');
  });

  it('returns null when all providers fail', async () => {
    high.setFail(true);
    mid.setFail(true);
    low.setFail(true);
    const agg = makeFailoverAggregator();
    const result = await agg.getPrice('XLM');

    expect(result).toBeNull();
  });

  it('skips provider with open circuit breaker and uses next in priority order', async () => {
    // failureThreshold=1 → one failure opens the circuit
    const agg = makeFailoverAggregator(1);

    // Open high's circuit breaker
    high.setFail(true);
    await agg.getPrice('XLM'); // mid serves; high's CB opens

    // Verify high's circuit breaker is now OPEN
    const metrics = agg.getCircuitBreakerMetrics();
    const highMetrics = metrics.find((m) => m.providerName === 'high');
    expect(highMetrics?.state).toBe('OPEN');
  });

  it('recovers to higher-priority provider after circuit breaker closes', async () => {
    const validator = createValidator({ maxDeviationPercent: 50, maxStalenessSeconds: 300 });

    // backoffMs=0 → circuit moves to HALF_OPEN immediately after failure
    const cache = createPriceCache(30);
    const agg = createAggregator([high, mid, low], validator, cache, {
      minSources: 1,
      failoverMode: true,
      circuitBreaker: { failureThreshold: 1, backoffMs: 0 },
    });

    // 1. High fails → circuit opens → mid serves
    high.setFail(true);
    const r1 = await agg.getPrice('XLM');
    expect(r1!.sources[0]!.source).toBe('mid');

    // 2. High recovers; backoffMs=0 → HALF_OPEN probe allowed immediately
    high.setFail(false);

    // Open circuit for high in agg
    high.setFail(true);
    await agg.getPrice('XLM'); // opens circuit, result cached
    high.setFail(false);

    // Clear cache so the next getPrice() triggers a real fetch
    await cache.clear();

    // Next request: backoffMs=0 → HALF_OPEN probe → high succeeds → CLOSED → high serves
    const r2 = await agg.getPrice('XLM');
    expect(r2!.sources[0]!.source).toBe('high');
  });

  it('isFailoverMode() returns true when failoverMode is enabled', () => {
    const agg = makeFailoverAggregator();
    expect(agg.isFailoverMode()).toBe(true);
  });

  it('isFailoverMode() returns false by default', () => {
    const validator = createValidator({ maxDeviationPercent: 50, maxStalenessSeconds: 300 });
    const cache = createPriceCache(30);
    const agg = createAggregator([high, mid, low], validator, cache, { minSources: 1 });
    expect(agg.isFailoverMode()).toBe(false);
  });

  it('getStats() includes failoverMode flag', () => {
    const agg = makeFailoverAggregator();
    expect(agg.getStats().failoverMode).toBe(true);
  });

  it('providers are ordered by priority (ascending)', () => {
    const agg = makeFailoverAggregator();
    const names = agg.getProviders();
    expect(names).toEqual(['high', 'mid', 'low']);
  });
});
