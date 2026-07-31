/**
 * Price History Service
 *
 * Stores historical price data for trend analysis, TWAP calculations, and debugging.
 * Uses a circular buffer to maintain memory-bounded storage.
 */
import type { AggregatedPrice } from '../types/index.js';
/**
 * Price history entry
 */
export interface PriceHistoryEntry {
    price: bigint;
    timestamp: number;
}
/**
 * Price history interface
 */
export interface PriceHistory {
    entries: PriceHistoryEntry[];
    maxEntries: number;
    currentIndex: number;
    isFull: boolean;
}
/**
 * TWAP calculation result
 */
export interface TWAPResult {
    asset: string;
    twap: bigint;
    periodSeconds: number;
    dataPoints: number;
    startTime: number;
    endTime: number;
}
/**
 * Price history configuration
 */
export interface PriceHistoryConfig {
    maxEntries: number;
}
/**
 * Price History Service
 */
export declare class PriceHistoryService {
    private histories;
    private config;
    constructor(config?: Partial<PriceHistoryConfig>);
    /**
     * Add a price entry to history
     */
    addPriceEntry(asset: string, price: bigint, timestamp: number): void;
    /**
     * Add aggregated price to history
     */
    addAggregatedPrice(price: AggregatedPrice): void;
    /**
     * Get price history for an asset
     */
    getPriceHistory(asset: string, limit?: number): PriceHistoryEntry[];
    /**
     * Calculate Time-Weighted Average Price (TWAP)
     */
    calculateTWAP(asset: string, periodSeconds: number): TWAPResult | null;
    /**
     * Get the latest price for an asset
     */
    getLatestPrice(asset: string): PriceHistoryEntry | null;
    /**
     * Get statistics for an asset
     */
    getAssetStats(asset: string): {
        totalEntries: number;
        oldestTimestamp?: number;
        newestTimestamp?: number;
        priceRange?: {
            min: bigint;
            max: bigint;
        };
    };
    /**
     * Clear history for an asset
     */
    clearHistory(asset: string): void;
    /**
     * Clear all history
     */
    clearAllHistory(): void;
    /**
     * Get list of assets with history
     */
    getAssets(): string[];
    /**
     * Get service statistics
     */
    getStats(): {
        trackedAssets: number;
        totalEntries: number;
        maxEntriesPerAsset: number;
        assets: string[];
    };
    /**
     * Get or create history for an asset
     */
    private getOrCreateHistory;
}
/**
 * Create a price history service
 */
export declare function createPriceHistoryService(config?: Partial<PriceHistoryConfig>): PriceHistoryService;
//# sourceMappingURL=price-history.d.ts.map