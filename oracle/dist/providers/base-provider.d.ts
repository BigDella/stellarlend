/**
 * Base Price Provider
 *
 * Abstract base class for all price data providers.
 * Implements common functionality like rate limiting and error handling.
 */
import type { RawPriceData, ProviderConfig, HealthStatus } from '../types/index.js';
/**
 * Abstract base class for price providers
 */
export declare abstract class BasePriceProvider {
    protected config: ProviderConfig;
    protected requestTimestamps: number[];
    private rateLimitChain;
    constructor(config: ProviderConfig);
    /**
     * Get provider name
     */
    get name(): string;
    /**
     * Get provider priority
     */
    get priority(): number;
    /**
     * Get the provider weight for aggregation
     */
    get weight(): number;
    /**
     * Check if the provider is enabled
     */
    get isEnabled(): boolean;
    /**
     * Fetch price for a specific asset
     * Must be implemented by each provider
     */
    abstract fetchPrice(asset: string): Promise<RawPriceData>;
    /**
     * Fetch prices for multiple assets in parallel with a concurrency limit.
     * Failed fetches are logged and skipped without blocking successful ones.
     */
    fetchPrices(assets: string[]): Promise<RawPriceData[]>;
    /**
     * Check provider health
     */
    healthCheck(): Promise<HealthStatus>;
    /**
     * Enforce rate limiting
     */
    protected enforceRateLimit(): Promise<void>;
    private enforceRateLimitInternal;
    /**
     * Sleep util
     */
    protected sleep(ms: number): Promise<void>;
    /**
     * Make HTTP request using axios with IPv4 forced
     */
    protected request<T>(url: string, options?: {
        headers?: Record<string, string>;
    }): Promise<T>;
}
//# sourceMappingURL=base-provider.d.ts.map