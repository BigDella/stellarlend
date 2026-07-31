/**
 * Price History Service
 *
 * Stores historical price data for trend analysis, TWAP calculations, and debugging.
 * Uses a circular buffer to maintain memory-bounded storage.
 */
import { logger } from '../utils/logger.js';
/**
 * Default configuration
 */
const DEFAULT_CONFIG = {
    maxEntries: 100,
};
/**
 * Price History Service
 */
export class PriceHistoryService {
    histories;
    config;
    constructor(config = {}) {
        this.config = { ...DEFAULT_CONFIG, ...config };
        this.histories = new Map();
        logger.info('Price history service initialized', {
            maxEntries: this.config.maxEntries,
        });
    }
    /**
     * Add a price entry to history
     */
    addPriceEntry(asset, price, timestamp) {
        const upperAsset = asset.toUpperCase();
        const history = this.getOrCreateHistory(upperAsset);
        // Add entry at current index (circular buffer behavior)
        history.entries[history.currentIndex] = {
            price,
            timestamp,
        };
        // Move to next position
        history.currentIndex = (history.currentIndex + 1) % history.maxEntries;
        // Mark as full after we've wrapped around
        if (history.currentIndex === 0) {
            history.isFull = true;
        }
        logger.debug(`Added price history entry for ${upperAsset}`, {
            price: price.toString(),
            timestamp,
            currentIndex: history.currentIndex,
            isFull: history.isFull,
        });
    }
    /**
     * Add aggregated price to history
     */
    addAggregatedPrice(price) {
        this.addPriceEntry(price.asset, price.price, price.timestamp);
    }
    /**
     * Get price history for an asset
     */
    getPriceHistory(asset, limit) {
        const upperAsset = asset.toUpperCase();
        const history = this.histories.get(upperAsset);
        if (!history) {
            return [];
        }
        const entries = [];
        if (history.isFull) {
            // Buffer is full, return entries starting from current index
            const startIndex = history.currentIndex;
            for (let i = 0; i < history.maxEntries; i++) {
                const index = (startIndex + i) % history.maxEntries;
                const entry = history.entries[index];
                if (entry) {
                    entries.push(entry);
                    if (limit && entries.length >= limit) {
                        break;
                    }
                }
            }
        }
        else {
            // Buffer not full, return entries from start
            for (let i = 0; i < history.currentIndex; i++) {
                const entry = history.entries[i];
                if (entry) {
                    entries.push(entry);
                    if (limit && entries.length >= limit) {
                        break;
                    }
                }
            }
        }
        return entries;
    }
    /**
     * Calculate Time-Weighted Average Price (TWAP)
     */
    calculateTWAP(asset, periodSeconds) {
        const upperAsset = asset.toUpperCase();
        const entries = this.getPriceHistory(upperAsset);
        if (entries.length < 2) {
            logger.warn(`Insufficient data for TWAP calculation for ${upperAsset}`, {
                availableEntries: entries.length,
                required: 2,
            });
            return null;
        }
        const now = Math.floor(Date.now() / 1000);
        const startTime = now - periodSeconds;
        // Filter entries within the time period
        const periodEntries = entries.filter((entry) => entry.timestamp >= startTime);
        if (periodEntries.length < 2) {
            logger.warn(`Insufficient data within time period for TWAP calculation for ${upperAsset}`, {
                periodSeconds,
                availableEntries: periodEntries.length,
                required: 2,
            });
            return null;
        }
        // Calculate TWAP using time-weighted average
        let totalTime = 0;
        let weightedSum = 0n;
        for (let i = 0; i < periodEntries.length - 1; i++) {
            const current = periodEntries[i];
            const next = periodEntries[i + 1];
            const timeDiff = next.timestamp - current.timestamp;
            totalTime += timeDiff;
            weightedSum += current.price * BigInt(timeDiff);
        }
        // Add the last entry's contribution (assume it lasts until now)
        const lastEntry = periodEntries[periodEntries.length - 1];
        const lastTimeDiff = now - lastEntry.timestamp;
        totalTime += lastTimeDiff;
        weightedSum += lastEntry.price * BigInt(lastTimeDiff);
        if (totalTime === 0) {
            logger.warn(`Zero time duration for TWAP calculation for ${upperAsset}`);
            return null;
        }
        const twap = weightedSum / BigInt(totalTime);
        const result = {
            asset: upperAsset,
            twap,
            periodSeconds,
            dataPoints: periodEntries.length,
            startTime: periodEntries[0].timestamp,
            endTime: lastEntry.timestamp,
        };
        logger.info(`Calculated TWAP for ${upperAsset}`, {
            twap: twap.toString(),
            periodSeconds,
            dataPoints: periodEntries.length,
        });
        return result;
    }
    /**
     * Get the latest price for an asset
     */
    getLatestPrice(asset) {
        const upperAsset = asset.toUpperCase();
        const history = this.histories.get(upperAsset);
        if (!history || (history.currentIndex === 0 && !history.isFull)) {
            return null;
        }
        // Get the most recent entry
        const latestIndex = history.currentIndex === 0 ? history.maxEntries - 1 : history.currentIndex - 1;
        return history.entries[latestIndex] || null;
    }
    /**
     * Get statistics for an asset
     */
    getAssetStats(asset) {
        const upperAsset = asset.toUpperCase();
        const entries = this.getPriceHistory(upperAsset);
        if (entries.length === 0) {
            return { totalEntries: 0 };
        }
        const timestamps = entries.map((e) => e.timestamp);
        const prices = entries.map((e) => e.price);
        return {
            totalEntries: entries.length,
            oldestTimestamp: Math.min(...timestamps),
            newestTimestamp: Math.max(...timestamps),
            priceRange: {
                min: prices.reduce((a, b) => (a < b ? a : b)),
                max: prices.reduce((a, b) => (a > b ? a : b)),
            },
        };
    }
    /**
     * Clear history for an asset
     */
    clearHistory(asset) {
        const upperAsset = asset.toUpperCase();
        this.histories.delete(upperAsset);
        logger.info(`Cleared price history for ${upperAsset}`);
    }
    /**
     * Clear all history
     */
    clearAllHistory() {
        this.histories.clear();
        logger.info('Cleared all price history');
    }
    /**
     * Get list of assets with history
     */
    getAssets() {
        return Array.from(this.histories.keys());
    }
    /**
     * Get service statistics
     */
    getStats() {
        const assets = this.getAssets();
        const totalEntries = assets.reduce((sum, asset) => {
            const history = this.histories.get(asset);
            if (history) {
                return sum + (history.isFull ? history.maxEntries : history.currentIndex);
            }
            return sum;
        }, 0);
        return {
            trackedAssets: assets.length,
            totalEntries,
            maxEntriesPerAsset: this.config.maxEntries,
            assets,
        };
    }
    /**
     * Get or create history for an asset
     */
    getOrCreateHistory(asset) {
        let history = this.histories.get(asset);
        if (!history) {
            history = {
                entries: new Array(this.config.maxEntries),
                maxEntries: this.config.maxEntries,
                currentIndex: 0,
                isFull: false,
            };
            this.histories.set(asset, history);
        }
        return history;
    }
}
/**
 * Create a price history service
 */
export function createPriceHistoryService(config) {
    return new PriceHistoryService(config);
}
//# sourceMappingURL=price-history.js.map