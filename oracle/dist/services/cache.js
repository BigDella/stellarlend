/**
 * Cache Service
 *
 * In-memory caching layer with TTL support and LRU eviction.
 * Supports Redis with fallback to in-memory when Redis unavailable.
 */
import { logger } from '../utils/logger.js';
import Redis from 'ioredis';
/**
 * Default cache configuration
 */
const DEFAULT_CONFIG = {
    defaultTtlSeconds: 30,
    maxEntries: 1000,
    evictBatchFraction: 0.1,
};
/**
 * In-memory LRU cache implementation with Redis support.
 *
 * Access order is maintained by deleting and re-inserting keys into the Map
 * on every read, so the Map's natural insertion order reflects LRU order
 * (oldest = first entry, most-recently-used = last entry).
 */
export class Cache {
    config;
    store = new Map();
    hits = 0;
    misses = 0;
    evictions = 0;
    redis;
    usingRedis = false;
    constructor(config = {}) {
        this.config = { ...DEFAULT_CONFIG, ...config };
        // Initialize Redis if URL is provided
        if (this.config.redisUrl) {
            this.initializeRedis();
        }
        logger.info('Cache initialized', {
            defaultTtlSeconds: this.config.defaultTtlSeconds,
            maxEntries: this.config.maxEntries,
            evictBatchFraction: this.config.evictBatchFraction,
            usingRedis: this.usingRedis,
            redisUrl: this.config.redisUrl ? 'configured' : 'not configured',
        });
    }
    /**
     * Initialize Redis connection
     */
    initializeRedis() {
        try {
            this.redis = new Redis(this.config.redisUrl, {
                maxRetriesPerRequest: 3,
                lazyConnect: true,
            });
            this.redis.on('connect', () => {
                logger.info('Redis connected');
                this.usingRedis = true;
            });
            this.redis.on('error', (error) => {
                logger.warn('Redis connection failed, falling back to in-memory cache', { error });
                this.usingRedis = false;
                this.redis?.disconnect();
                this.redis = undefined;
            });
            // Test connection
            this.redis.connect().catch(() => {
                logger.warn('Redis connection failed during initialization, using in-memory cache');
                this.usingRedis = false;
                this.redis = undefined;
            });
        }
        catch (error) {
            logger.warn('Failed to initialize Redis, using in-memory cache', { error });
            this.usingRedis = false;
            this.redis = undefined;
        }
    }
    /**
     * Get a value from cache.
     * Moves the accessed entry to the "most recently used" position.
     */
    async get(key) {
        // Try Redis first if available
        if (this.usingRedis && this.redis) {
            try {
                const value = await this.redis.get(key);
                if (value !== null) {
                    const parsed = JSON.parse(value);
                    if (Date.now() <= parsed.expiresAt) {
                        this.hits++;
                        return parsed.data;
                    }
                    else {
                        // Expired in Redis, remove it
                        await this.redis.del(key);
                    }
                }
            }
            catch (error) {
                logger.warn('Redis get failed, falling back to in-memory', { error, key });
                this.usingRedis = false;
            }
        }
        // Fallback to in-memory cache
        const entry = this.store.get(key);
        if (!entry) {
            this.misses++;
            return undefined;
        }
        // Check if expired
        if (Date.now() > entry.expiresAt) {
            this.store.delete(key);
            this.misses++;
            return undefined;
        }
        // Refresh LRU position: delete then re-insert moves key to end of Map
        this.store.delete(key);
        this.store.set(key, entry);
        this.hits++;
        return entry.data;
    }
    /**
     * Set a value in cache with optional TTL.
     * Performs LRU batch eviction when at capacity.
     */
    async set(key, value, ttlSeconds) {
        const ttl = ttlSeconds ?? this.config.defaultTtlSeconds;
        const now = Date.now();
        const entry = {
            data: value,
            cachedAt: now,
            expiresAt: now + ttl * 1000,
        };
        // Try Redis first if available
        if (this.usingRedis && this.redis) {
            try {
                await this.redis.setex(key, ttl, JSON.stringify(entry));
            }
            catch (error) {
                logger.warn('Redis set failed, falling back to in-memory', { error, key });
                this.usingRedis = false;
            }
        }
        // Always store in memory as fallback
        // If key already exists, remove it first so it gets a fresh LRU position
        if (this.store.has(key)) {
            this.store.delete(key);
        }
        else if (this.store.size >= this.config.maxEntries) {
            this.evictLRUBatch();
        }
        this.store.set(key, entry);
    }
    /**
     * Delete a specific key
     */
    async delete(key) {
        // Try Redis first if available
        if (this.usingRedis && this.redis) {
            try {
                await this.redis.del(key);
            }
            catch (error) {
                logger.warn('Redis delete failed, using in-memory only', { error, key });
                this.usingRedis = false;
            }
        }
        return this.store.delete(key);
    }
    /**
     * Clear all entries
     */
    async clear() {
        // Try Redis first if available
        if (this.usingRedis && this.redis) {
            try {
                await this.redis.flushdb();
            }
            catch (error) {
                logger.warn('Redis clear failed, using in-memory only', { error });
                this.usingRedis = false;
            }
        }
        this.store.clear();
        logger.info('Cache cleared');
    }
    /**
     * Check if key exists and is not expired
     */
    async has(key) {
        // Try Redis first if available
        if (this.usingRedis && this.redis) {
            try {
                const value = await this.redis.get(key);
                if (value !== null) {
                    const parsed = JSON.parse(value);
                    if (Date.now() <= parsed.expiresAt) {
                        return true;
                    }
                    else {
                        // Expired in Redis, remove it
                        await this.redis.del(key);
                        return false;
                    }
                }
            }
            catch (error) {
                logger.warn('Redis has check failed, using in-memory only', { error, key });
                this.usingRedis = false;
            }
        }
        // Fallback to in-memory cache
        const entry = this.store.get(key);
        if (!entry) {
            return false;
        }
        if (Date.now() > entry.expiresAt) {
            this.store.delete(key);
            return false;
        }
        return true;
    }
    /**
     * Get cache statistics including hit rate and eviction count.
     */
    getStats() {
        const total = this.hits + this.misses;
        const hitRate = total > 0 ? this.hits / total : 0;
        logger.debug('Cache stats', {
            size: this.store.size,
            hits: this.hits,
            misses: this.misses,
            hitRate: hitRate.toFixed(4),
            evictions: this.evictions,
        });
        return {
            size: this.store.size,
            hits: this.hits,
            misses: this.misses,
            hitRate,
            evictions: this.evictions,
        };
    }
    /**
     * Evict a batch of least-recently-used entries.
     *
     * The Map preserves insertion order and we refresh position on every get,
     * so the first N keys are always the least recently used.
     * Batch size = ceil(maxEntries * evictBatchFraction), minimum 1.
     */
    evictLRUBatch() {
        const batchSize = Math.max(1, Math.ceil(this.config.maxEntries * this.config.evictBatchFraction));
        let evicted = 0;
        for (const key of this.store.keys()) {
            if (evicted >= batchSize)
                break;
            this.store.delete(key);
            evicted++;
        }
        this.evictions += evicted;
        logger.debug(`LRU batch eviction: removed ${evicted} entries`, {
            remaining: this.store.size,
            totalEvictions: this.evictions,
        });
    }
    /**
     * Clean up expired entries periodically
     */
    cleanup() {
        const now = Date.now();
        let cleaned = 0;
        for (const [key, entry] of this.store) {
            if (now > entry.expiresAt) {
                this.store.delete(key);
                cleaned++;
            }
        }
        if (cleaned > 0) {
            logger.debug(`Cleaned up ${cleaned} expired cache entries`);
        }
        return cleaned;
    }
}
/**
 * Price-specific cache wrapper
 */
export class PriceCache {
    cache;
    keyPrefix = 'price:';
    constructor(ttlSeconds = 30, redisUrl) {
        this.cache = new Cache({
            defaultTtlSeconds: ttlSeconds,
            maxEntries: 100,
            redisUrl,
        });
    }
    /**
     * Get cached price for an asset
     */
    async getPrice(asset) {
        return await this.cache.get(`${this.keyPrefix}${asset.toUpperCase()}`);
    }
    /**
     * Cache a price for an asset
     */
    async setPrice(asset, price, ttlSeconds) {
        await this.cache.set(`${this.keyPrefix}${asset.toUpperCase()}`, price, ttlSeconds);
    }
    /**
     * Check if we have a cached price
     */
    async hasPrice(asset) {
        return await this.cache.has(`${this.keyPrefix}${asset.toUpperCase()}`);
    }
    /**
     * Get cache statistics
     */
    getStats() {
        return this.cache.getStats();
    }
    /**
     * Clear all cached prices
     */
    async clear() {
        await this.cache.clear();
    }
}
/**
 * Create a new cache instance
 */
export function createCache(config) {
    return new Cache(config);
}
/**
 * Create a price-specific cache
 */
export function createPriceCache(ttlSeconds, redisUrl) {
    return new PriceCache(ttlSeconds, redisUrl);
}
//# sourceMappingURL=cache.js.map