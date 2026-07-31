/**
 * Cache Service
 *
 * In-memory caching layer with TTL support and LRU eviction.
 * Supports Redis with fallback to in-memory when Redis unavailable.
 */
/**
 * Cache config
 */
export interface CacheConfig {
    defaultTtlSeconds: number;
    maxEntries: number;
    /** Fraction of entries to evict in a batch when at capacity (0 < x <= 1) */
    evictBatchFraction: number;
    /** Redis URL (optional) */
    redisUrl?: string;
}
/**
 * In-memory LRU cache implementation with Redis support.
 *
 * Access order is maintained by deleting and re-inserting keys into the Map
 * on every read, so the Map's natural insertion order reflects LRU order
 * (oldest = first entry, most-recently-used = last entry).
 */
export declare class Cache {
    private config;
    private store;
    private hits;
    private misses;
    private evictions;
    private redis?;
    private usingRedis;
    constructor(config?: Partial<CacheConfig>);
    /**
     * Initialize Redis connection
     */
    private initializeRedis;
    /**
     * Get a value from cache.
     * Moves the accessed entry to the "most recently used" position.
     */
    get<T>(key: string): Promise<T | undefined>;
    /**
     * Set a value in cache with optional TTL.
     * Performs LRU batch eviction when at capacity.
     */
    set<T>(key: string, value: T, ttlSeconds?: number): Promise<void>;
    /**
     * Delete a specific key
     */
    delete(key: string): Promise<boolean>;
    /**
     * Clear all entries
     */
    clear(): Promise<void>;
    /**
     * Check if key exists and is not expired
     */
    has(key: string): Promise<boolean>;
    /**
     * Get cache statistics including hit rate and eviction count.
     */
    getStats(): {
        size: number;
        hits: number;
        misses: number;
        hitRate: number;
        evictions: number;
    };
    /**
     * Evict a batch of least-recently-used entries.
     *
     * The Map preserves insertion order and we refresh position on every get,
     * so the first N keys are always the least recently used.
     * Batch size = ceil(maxEntries * evictBatchFraction), minimum 1.
     */
    private evictLRUBatch;
    /**
     * Clean up expired entries periodically
     */
    cleanup(): number;
}
/**
 * Price-specific cache wrapper
 */
export declare class PriceCache {
    private cache;
    private keyPrefix;
    constructor(ttlSeconds?: number, redisUrl?: string);
    /**
     * Get cached price for an asset
     */
    getPrice(asset: string): Promise<bigint | undefined>;
    /**
     * Cache a price for an asset
     */
    setPrice(asset: string, price: bigint, ttlSeconds?: number): Promise<void>;
    /**
     * Check if we have a cached price
     */
    hasPrice(asset: string): Promise<boolean>;
    /**
     * Get cache statistics
     */
    getStats(): {
        size: number;
        hits: number;
        misses: number;
        hitRate: number;
        evictions: number;
    };
    /**
     * Clear all cached prices
     */
    clear(): Promise<void>;
}
/**
 * Create a new cache instance
 */
export declare function createCache(config?: Partial<CacheConfig>): Cache;
/**
 * Create a price-specific cache
 */
export declare function createPriceCache(ttlSeconds?: number, redisUrl?: string): PriceCache;
//# sourceMappingURL=cache.d.ts.map