/**
 * StellarLend Oracle Service
 * Off-chain oracle integration service that fetches price data from
 * multiple sources (CoinGecko, Binance)
 * @see https://github.com/stellarlend/stellarlend-contracts
 */
import { type OracleServiceConfig } from './config.js';
/**
 * Oracle Service
 */
export declare class OracleService {
    private config;
    private aggregator;
    private contractUpdater;
    private providers;
    private intervalId?;
    private isRunning;
    private lastSuccessfulUpdate;
    constructor(config: OracleServiceConfig);
    /**
     * Start the oracle service
     */
    start(assets?: string[]): Promise<void>;
    /**
     * Stop the oracle service
     */
    stop(): void;
    /**
     * Fetch and update prices for specified assets
     */
    updatePrices(assets: string[]): Promise<void>;
    /**
     * Get current service status (safe for logging — secret key is masked)
     */
    getStatus(): {
        isRunning: boolean;
        network: "testnet" | "mainnet";
        contractId: string;
        adminSecretKey: string;
        providers: {
            name: string;
            enabled: boolean;
            priority: number;
            weight: number;
            apiKey?: string;
            baseUrl: string;
            rateLimit: {
                maxRequests: number;
                windowMs: number;
            };
            concurrencyLimit?: number;
        }[];
        aggregatorStats: {
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
            circuitBreakerMetrics: (import("./services/circuit-breaker.js").CircuitBreakerMetrics & {
                providerName: string;
                state: import("./services/circuit-breaker.js").CircuitState;
            })[];
            circuitBreakers: (import("./services/circuit-breaker.js").CircuitBreakerMetrics & {
                providerName: string;
                state: import("./services/circuit-breaker.js").CircuitState;
            })[];
        };
        circuitBreakers: (import("./services/circuit-breaker.js").CircuitBreakerMetrics & {
            providerName: string;
            state: import("./services/circuit-breaker.js").CircuitState;
        })[];
    };
    /**
     * Manually fetch price for a single asset (for testing)
     */
    fetchPrice(asset: string): Promise<import("./types/index.js").AggregatedPrice | null>;
    private validateConfig;
    private normalizeProviders;
    private createRuntimeProviders;
}
export { loadConfig, maskSecret, getSafeConfig } from './config.js';
export type { OracleServiceConfig } from './config.js';
//# sourceMappingURL=index.d.ts.map