/**
 * Price Aggregator Service
 *
 * Fetches prices from multiple providers and aggregates them
 * using weighted median calculation.
 */
import type { AggregatedPrice } from '../types/index.js';
import { BasePriceProvider } from '../providers/base-provider.js';
import { PriceValidator } from './price-validator.js';
import { PriceCache } from './cache.js';
import { PriceHistoryService } from './price-history.js';
import { CircuitState } from './circuit-breaker.js';
import type { CircuitBreakerConfig, CircuitBreakerMetrics } from './circuit-breaker.js';
/**
 * Aggregator configuration
 */
export interface AggregatorConfig {
    minSources: number;
    useWeightedMedian: boolean;
    circuitBreaker?: Partial<Omit<CircuitBreakerConfig, 'providerName'>>;
}
/**
 * Price Aggregator
 */
export declare class PriceAggregator {
    private providers;
    private validator;
    private cache;
    private priceHistory;
    private config;
    private circuitBreakers;
    constructor(providers: BasePriceProvider[], validator: PriceValidator, cache: PriceCache, priceHistory: PriceHistoryService, config?: Partial<AggregatorConfig>);
    /**
     * Fetch and aggregate price for a single asset
     */
    getPrice(asset: string): Promise<AggregatedPrice | null>;
    /**
     * Fetch prices for multiple assets
     */
    getPrices(assets: string[]): Promise<Map<string, AggregatedPrice>>;
    /**
     * Fetch price from providers with fallback logic
     */
    private fetchWithFallback;
    /**
     * Aggregate prices from multiple sources
     */
    private aggregate;
    /**
     * Calculate weighted median of prices
     */
    private weightedMedian;
    /**
     * Calculate simple median of prices
     */
    private simpleMedian;
    /**
     * Get price history service
     */
    getPriceHistory(): PriceHistoryService;
    /**
     * Get circuit breaker metrics for all providers
     */
    getCircuitBreakerMetrics(): Array<CircuitBreakerMetrics & {
        providerName: string;
        state: CircuitState;
    }>;
    /**
     * Get list of enabled providers
     */
    getProviders(): string[];
    /**
     * Get aggregator statistics
     */
    getStats(): {
        enabledProviders: number;
        cacheStats: {
            size: number;
            hits: number;
            misses: number;
            hitRate: number;
            evictions: number;
        };
        priceHistoryStats: {
            trackedAssets: number;
            totalEntries: number;
            maxEntriesPerAsset: number;
            assets: string[];
        };
        circuitBreakerMetrics: (CircuitBreakerMetrics & {
            providerName: string;
            state: CircuitState;
        })[];
        circuitBreakers: (CircuitBreakerMetrics & {
            providerName: string;
            state: CircuitState;
        })[];
    };
}
/**
 * Create a price aggregator
 */
export declare function createAggregator(providers: BasePriceProvider[], validator: PriceValidator, cache: PriceCache, priceHistoryOrConfig?: PriceHistoryService | Partial<AggregatorConfig>, config?: Partial<AggregatorConfig>): PriceAggregator;
//# sourceMappingURL=price-aggregator.d.ts.map