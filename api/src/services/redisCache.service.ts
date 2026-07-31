import Redis from 'ioredis';
import { config } from '../config';
import logger from '../utils/logger';

export type HotCacheKeyKind = 'price' | 'position' | 'pool' | 'protocol' | 'gas' | 'mev' | 'nonce';

interface CacheMetrics {
  hits: number;
  misses: number;
  errors: number;
}

class RedisCacheService {
  private readonly redis: Redis | null;
  private readonly memoryFallback = new Map<string, { value: string; expiresAt: number }>();
  private readonly metrics: CacheMetrics = { hits: 0, misses: 0, errors: 0 };

  constructor() {
    if (!config.cache.redisEnabled) {
      this.redis = null;
      return;
    }

    this.redis = new Redis(config.cache.redisUrl, {
      lazyConnect: true,
      maxRetriesPerRequest: 1,
      enableAutoPipelining: true,
    });
  }

  buildKey(kind: HotCacheKeyKind, id: string): string {
    return `stellarlend:${kind}:${id}`;
  }

  async warmup(prefetch?: () => Promise<void>): Promise<void> {
    if (!this.redis) return;
    try {
      await this.redis.connect();
      await this.redis.ping();
      logger.info('Redis cache warmed');
      if (prefetch) {
        try {
          await prefetch();
          logger.info('Redis cache pre-fetched');
        } catch (prefetchError) {
          logger.warn('Redis cache prefetch failed', { error: prefetchError });
        }
      }
    } catch (error) {
      this.metrics.errors += 1;
      logger.warn('Redis warmup failed, memory fallback remains active', { error });
    }
  }

  async get<T>(key: string): Promise<T | null> {
    try {
      if (this.redis?.status === 'ready') {
        const value = await this.redis.get(key);
        if (!value) {
          this.metrics.misses += 1;
          return null;
        }
        this.metrics.hits += 1;
        return JSON.parse(value) as T;
      }

      const entry = this.memoryFallback.get(key);
      if (!entry || entry.expiresAt < Date.now()) {
        this.memoryFallback.delete(key);
        this.metrics.misses += 1;
        return null;
      }
      this.metrics.hits += 1;
      return JSON.parse(entry.value) as T;
    } catch (error) {
      this.metrics.errors += 1;
      logger.warn('Cache get failed, treating as cache miss', { key, error });
      return null;
    }
  }

  async set<T>(key: string, value: T, ttlSeconds: number): Promise<void> {
    const serialized = JSON.stringify(value);
    try {
      if (this.redis?.status === 'ready') {
        await this.redis.set(key, serialized, 'EX', ttlSeconds);
        return;
      }

      this.memoryFallback.set(key, {
        value: serialized,
        expiresAt: Date.now() + ttlSeconds * 1000,
      });
    } catch (error) {
      this.metrics.errors += 1;
      logger.warn('Cache set failed', { key, error });
    }
  }

  async del(key: string): Promise<void> {
    try {
      if (this.redis?.status === 'ready') {
        await this.redis.del(key);
        return;
      }
      this.memoryFallback.delete(key);
    } catch (error) {
      this.metrics.errors += 1;
      logger.warn('Cache del failed', { key, error });
    }
  }

  async delByPrefix(prefix: string): Promise<void> {
    try {
      if (this.redis?.status === 'ready') {
        const stream = this.redis.scanStream({ match: `${prefix}*`, count: 100 });
        stream.on('data', (keys: string[]) => {
          if (keys.length > 0) {
            void this.redis?.del(...keys);
          }
        });
        return;
      }

      for (const key of this.memoryFallback.keys()) {
        if (key.startsWith(prefix)) this.memoryFallback.delete(key);
      }
    } catch (error) {
      this.metrics.errors += 1;
      logger.warn('Cache invalidation failed', { prefix, error });
    }
  }

  getMetrics(): CacheMetrics {
    return { ...this.metrics };
  }

  clearAllForTests(): void {
    this.memoryFallback.clear();
    this.metrics.hits = 0;
    this.metrics.misses = 0;
    this.metrics.errors = 0;
  }
}

export const redisCacheService = new RedisCacheService();
