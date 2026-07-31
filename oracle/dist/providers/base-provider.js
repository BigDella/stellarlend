/**
 * Base Price Provider
 *
 * Abstract base class for all price data providers.
 * Implements common functionality like rate limiting and error handling.
 */
import axios from 'axios';
import https from 'https';
import { logger } from '../utils/logger.js';
/**
 * HTTPS Agent
 */
const httpsAgent = new https.Agent({
    family: 4,
    keepAlive: true,
    timeout: 30000,
});
/**
 * Abstract base class for price providers
 */
export class BasePriceProvider {
    config;
    // Sliding-window rate limit state: store timestamps (ms) for recent requests.
    // We keep timestamps for requests that are still within the active window.
    requestTimestamps = [];
    rateLimitChain = Promise.resolve();
    constructor(config) {
        this.config = config;
    }
    /**
     * Get provider name
     */
    get name() {
        return this.config.name;
    }
    /**
     * Get provider priority
     */
    get priority() {
        return this.config.priority;
    }
    /**
     * Get the provider weight for aggregation
     */
    get weight() {
        return this.config.weight;
    }
    /**
     * Check if the provider is enabled
     */
    get isEnabled() {
        return this.config.enabled;
    }
    /**
     * Fetch prices for multiple assets in parallel with a concurrency limit.
     * Failed fetches are logged and skipped without blocking successful ones.
     */
    async fetchPrices(assets) {
        const concurrency = this.config.concurrencyLimit ?? 5;
        const results = [];
        for (let i = 0; i < assets.length; i += concurrency) {
            const batch = assets.slice(i, i + concurrency);
            const settled = await Promise.allSettled(batch.map(async (asset) => {
                await this.enforceRateLimit();
                return this.fetchPrice(asset);
            }));
            for (const outcome of settled) {
                if (outcome.status === 'fulfilled') {
                    results.push(outcome.value);
                }
                else {
                    logger.error(`Failed to fetch price from ${this.name}`, { error: outcome.reason });
                }
            }
        }
        return results;
    }
    /**
     * Check provider health
     */
    async healthCheck() {
        const startTime = Date.now();
        try {
            await this.fetchPrice('XLM');
            return {
                provider: this.name,
                healthy: true,
                lastCheck: Date.now(),
                latencyMs: Date.now() - startTime,
            };
        }
        catch (error) {
            return {
                provider: this.name,
                healthy: false,
                lastCheck: Date.now(),
                latencyMs: Date.now() - startTime,
                error: error instanceof Error ? error.message : 'Unknown error',
            };
        }
    }
    /**
     * Enforce rate limiting
     */
    async enforceRateLimit() {
        // Serialize rate-limit state updates so concurrent requests cannot
        // all pass the same counter check in parallel.
        const run = this.rateLimitChain.then(() => this.enforceRateLimitInternal());
        this.rateLimitChain = run.catch(() => undefined);
        await run;
    }
    async enforceRateLimitInternal() {
        const { maxRequests, windowMs } = this.config.rateLimit;
        // Loop until the request can be accepted under a moving window.
        // We use an inclusive window definition (requests at exactly `windowMs`
        // age are still counted) to prevent boundary bursts.
        // eslint-disable-next-line no-constant-condition
        while (true) {
            const now = Date.now();
            // Keep requests that are within the last windowMs (inclusive).
            // That means remove only those strictly older than windowMs.
            this.requestTimestamps = this.requestTimestamps.filter((t) => now - t <= windowMs);
            if (this.requestTimestamps.length < maxRequests) {
                this.requestTimestamps.push(now);
                return;
            }
            const earliest = this.requestTimestamps[0];
            const waitTime = Math.max(0, earliest + windowMs - now + 1);
            logger.warn(`Rate limit reached for ${this.name}, waiting ${waitTime}ms`);
            await this.sleep(waitTime);
        }
    }
    /**
     * Sleep util
     */
    sleep(ms) {
        return new Promise((resolve) => setTimeout(resolve, ms));
    }
    /**
     * Make HTTP request using axios with IPv4 forced
     */
    async request(url, options = {}) {
        const response = await axios.get(url, {
            headers: {
                'Content-Type': 'application/json',
                ...options.headers,
            },
            timeout: 30000,
            httpsAgent,
        });
        return response.data;
    }
}
//# sourceMappingURL=base-provider.js.map