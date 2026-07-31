/**
 * Price Aggregator Service
 *
 * Fetches prices from multiple providers and aggregates them
 * using weighted median calculation.
 */
import { PriceHistoryService } from './price-history.js';
import { createCircuitBreaker } from './circuit-breaker.js';
import { logger } from '../utils/logger.js';
/**
 * Default aggregator configuration
 */
const DEFAULT_CONFIG = {
    minSources: 1,
    useWeightedMedian: true,
};
/**
 * Price Aggregator
 */
export class PriceAggregator {
    providers;
    validator;
    cache;
    priceHistory;
    config;
    circuitBreakers;
    constructor(providers, validator, cache, priceHistory, config = {}) {
        this.providers = providers.filter((p) => p.isEnabled).sort((a, b) => a.priority - b.priority);
        this.validator = validator;
        this.cache = cache;
        this.priceHistory = priceHistory;
        this.config = { ...DEFAULT_CONFIG, ...config };
        // Create one circuit breaker per provider
        this.circuitBreakers = new Map(this.providers.map((p) => [
            p.name,
            createCircuitBreaker({
                providerName: p.name,
                ...this.config.circuitBreaker,
            }),
        ]));
        logger.info('Price aggregator initialized', {
            enabledProviders: this.providers.map((p) => p.name),
            minSources: this.config.minSources,
        });
    }
    /**
     * Fetch and aggregate price for a single asset
     */
    async getPrice(asset) {
        const upperAsset = asset.toUpperCase();
        const cachedPrice = await this.cache.getPrice(upperAsset);
        if (cachedPrice !== undefined) {
            logger.debug(`Using cached price for ${upperAsset}`);
            return {
                asset: upperAsset,
                price: cachedPrice,
                sources: [],
                timestamp: Math.floor(Date.now() / 1000),
                confidence: 100,
            };
        }
        const validPrices = await this.fetchWithFallback(upperAsset);
        if (validPrices.length < this.config.minSources) {
            logger.error(`Not enough valid sources for ${upperAsset}`, {
                got: validPrices.length,
                required: this.config.minSources,
            });
            return null;
        }
        const aggregated = this.aggregate(upperAsset, validPrices);
        this.cache.setPrice(upperAsset, aggregated.price);
        // Store in price history
        this.priceHistory.addAggregatedPrice(aggregated);
        return aggregated;
    }
    /**
     * Fetch prices for multiple assets
     */
    async getPrices(assets) {
        const results = new Map();
        const promises = assets.map(async (asset) => {
            const price = await this.getPrice(asset);
            if (price) {
                results.set(asset.toUpperCase(), price);
            }
        });
        await Promise.allSettled(promises);
        return results;
    }
    /**
     * Fetch price from providers with fallback logic
     */
    async fetchWithFallback(asset) {
        const validPrices = [];
        const errors = new Map();
        for (const provider of this.providers) {
            try {
                const circuitBreaker = this.circuitBreakers.get(provider.name);
                // Check circuit breaker state
                if (circuitBreaker && !circuitBreaker.isAllowed()) {
                    logger.warn(`Circuit breaker OPEN for ${provider.name}, skipping`);
                    continue;
                }
                const rawPrice = await provider.fetchPrice(asset);
                const validation = this.validator.validate(rawPrice);
                if (validation.isValid && validation.price) {
                    validPrices.push(validation.price);
                    // Record success for circuit breaker
                    if (circuitBreaker) {
                        circuitBreaker.recordSuccess();
                    }
                    logger.debug(`Got valid price from ${provider.name} for ${asset}`, {
                        price: validation.price.price.toString(),
                    });
                }
                else {
                    // Record failure for circuit breaker
                    if (circuitBreaker) {
                        circuitBreaker.recordFailure();
                    }
                    logger.warn(`Invalid price from ${provider.name} for ${asset}`, {
                        errors: validation.errors,
                    });
                }
            }
            catch (error) {
                // Record failure for circuit breaker
                const circuitBreaker = this.circuitBreakers.get(provider.name);
                if (circuitBreaker) {
                    circuitBreaker.recordFailure();
                }
                errors.set(provider.name, error instanceof Error ? error : new Error(String(error)));
                logger.warn(`Provider ${provider.name} failed for ${asset}`, { error });
            }
        }
        if (validPrices.length === 0 && errors.size > 0) {
            logger.error(`All providers failed for ${asset}`, {
                providers: Array.from(errors.keys()),
            });
        }
        return validPrices;
    }
    /**
     * Aggregate prices from multiple sources
     */
    aggregate(asset, prices) {
        const now = Math.floor(Date.now() / 1000);
        if (prices.length === 1) {
            return {
                asset,
                price: prices[0].price,
                sources: prices,
                timestamp: now,
                confidence: prices[0].confidence,
            };
        }
        const aggregatedPrice = this.config.useWeightedMedian
            ? this.weightedMedian(prices)
            : this.simpleMedian(prices);
        const totalWeight = this.providers
            .filter((p) => prices.some((pr) => pr.source === p.name))
            .reduce((sum, p) => sum + p.weight, 0);
        const weightedConfidence = prices.reduce((sum, p) => {
            const provider = this.providers.find((pr) => pr.name === p.source);
            const weight = provider?.weight ?? 0.1;
            return sum + p.confidence * weight;
        }, 0) / totalWeight;
        return {
            asset,
            price: aggregatedPrice,
            sources: prices,
            timestamp: now,
            confidence: Math.round(weightedConfidence),
        };
    }
    /**
     * Calculate weighted median of prices
     */
    weightedMedian(prices) {
        const sorted = [...prices].sort((a, b) => (a.price < b.price ? -1 : a.price > b.price ? 1 : 0));
        const weights = sorted.map((p) => {
            const provider = this.providers.find((pr) => pr.name === p.source);
            return provider?.weight ?? 0.1;
        });
        const totalWeight = weights.reduce((a, b) => a + b, 0);
        const halfWeight = totalWeight / 2;
        let cumWeight = 0;
        for (let i = 0; i < sorted.length; i++) {
            cumWeight += weights[i];
            if (cumWeight >= halfWeight) {
                return sorted[i].price;
            }
        }
        return sorted[sorted.length - 1].price;
    }
    /**
     * Calculate simple median of prices
     */
    simpleMedian(prices) {
        const sorted = [...prices].sort((a, b) => (a.price < b.price ? -1 : a.price > b.price ? 1 : 0));
        const mid = Math.floor(sorted.length / 2);
        if (sorted.length % 2 === 0) {
            const avg = (sorted[mid - 1].price + sorted[mid].price) / 2n;
            return avg;
        }
        return sorted[mid].price;
    }
    /**
     * Get price history service
     */
    getPriceHistory() {
        return this.priceHistory;
    }
    /**
     * Get circuit breaker metrics for all providers
     */
    getCircuitBreakerMetrics() {
        const metrics = [];
        for (const breaker of this.circuitBreakers.values()) {
            metrics.push(breaker.getMetrics());
        }
        return metrics;
    }
    /**
     * Get list of enabled providers
     */
    getProviders() {
        return this.providers.map((p) => p.name);
    }
    /**
     * Get aggregator statistics
     */
    getStats() {
        return {
            enabledProviders: this.providers.length,
            cacheStats: this.cache.getStats(),
            priceHistoryStats: this.priceHistory.getStats(),
            circuitBreakerMetrics: this.getCircuitBreakerMetrics(),
            circuitBreakers: this.getCircuitBreakerMetrics(),
        };
    }
}
function isAggregatorConfig(value) {
    if (!value || typeof value !== 'object') {
        return false;
    }
    return 'minSources' in value || 'useWeightedMedian' in value || 'circuitBreaker' in value;
}
function isPriceHistoryService(value) {
    return value instanceof PriceHistoryService;
}
/**
 * Create a price aggregator
 */
export function createAggregator(providers, validator, cache, priceHistoryOrConfig, config) {
    const priceHistory = isPriceHistoryService(priceHistoryOrConfig)
        ? priceHistoryOrConfig
        : new PriceHistoryService();
    const resolvedConfig = isPriceHistoryService(priceHistoryOrConfig)
        ? config
        : isAggregatorConfig(priceHistoryOrConfig)
            ? priceHistoryOrConfig
            : config;
    return new PriceAggregator(providers, validator, cache, priceHistory, resolvedConfig);
}
//# sourceMappingURL=price-aggregator.js.map