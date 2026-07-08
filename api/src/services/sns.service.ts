import logger from '../utils/logger';

interface SNSRecord {
  name: string;
  address: string;
  registeredAt: number;
  expiresAt: number;
  owner: string;
}

interface SNSCache {
  name: string;
  address: string;
  cachedAt: number;
  ttl: number;
}

interface SNSAnalytics {
  totalNamesResolved: number;
  cacheHitRate: number;
  resolutionLatencyMs: number;
  topResolvedNames: Array<{ name: string; count: number }>;
}

const CACHE_TTL_SECONDS = 3600; // 1 hour
const NAME_EXPIRY_DAYS = 365;

const snsRecords = new Map<string, SNSRecord>(); // name -> record
const snsCache = new Map<string, SNSCache>(); // name -> cached record
const resolutionMetrics = new Map<string, number>(); // name -> resolution count
let totalResolutions = 0;
let cacheHits = 0;

export const snsService = {
  registerName(name: string, address: string, owner: string): SNSRecord {
    if (!name || name.length === 0) {
      throw new Error('Invalid SNS name');
    }

    if (snsRecords.has(name) && snsRecords.get(name)!.owner !== owner) {
      throw new Error('Name already registered by another user');
    }

    const now = Date.now();
    const expiresAt = now + NAME_EXPIRY_DAYS * 24 * 60 * 60 * 1000;

    const record: SNSRecord = {
      name,
      address,
      registeredAt: now,
      expiresAt,
      owner,
    };

    snsRecords.set(name, record);
    // Invalidate cache
    snsCache.delete(name);

    logger.info(`SNS name registered: ${name} -> ${address}`);
    return record;
  },

  resolveName(name: string): Address {
    const startTime = Date.now();

    // Check cache first
    const cached = snsCache.get(name);
    if (cached && startTime < cached.cachedAt + cached.ttl * 1000) {
      cacheHits++;
      logger.debug(`SNS cache hit for: ${name}`);
      this.updateMetrics(name);
      return cached.address;
    }

    // Fetch from records
    const record = snsRecords.get(name);
    if (!record) {
      throw new Error(`SNS name not found: ${name}`);
    }

    if (Date.now() > record.expiresAt) {
      throw new Error(`SNS name expired: ${name}`);
    }

    // Cache the resolution
    snsCache.set(name, {
      name,
      address: record.address,
      cachedAt: Date.now(),
      ttl: CACHE_TTL_SECONDS,
    });

    totalResolutions++;
    this.updateMetrics(name);

    const latency = Date.now() - startTime;
    logger.debug(`SNS resolution latency: ${latency}ms for ${name}`);

    return record.address;
  },

  resolveNameBatch(names: string[]): Map<string, string | null> {
    const results = new Map<string, string | null>();

    for (const name of names) {
      try {
        results.set(name, this.resolveName(name));
      } catch (err) {
        results.set(name, null);
      }
    }

    return results;
  },

  validateName(name: string): boolean {
    try {
      this.resolveName(name);
      return true;
    } catch {
      return false;
    }
  },

  isNameExpired(name: string): boolean {
    const record = snsRecords.get(name);
    if (!record) throw new Error(`SNS name not found: ${name}`);
    return Date.now() > record.expiresAt;
  },

  renewName(name: string, owner: string): SNSRecord {
    const record = snsRecords.get(name);
    if (!record) {
      throw new Error(`SNS name not found: ${name}`);
    }

    if (record.owner !== owner) {
      throw new Error('Unauthorized: not the name owner');
    }

    const now = Date.now();
    record.expiresAt = now + NAME_EXPIRY_DAYS * 24 * 60 * 60 * 1000;

    // Invalidate cache
    snsCache.delete(name);

    logger.info(`SNS name renewed: ${name}`);
    return record;
  },

  getAnalytics(): SNSAnalytics {
    const totalMetrics = Array.from(resolutionMetrics.entries());
    const topResolved = totalMetrics
      .sort((a, b) => b[1] - a[1])
      .slice(0, 10)
      .map(([name, count]) => ({ name, count }));

    const cacheHitRate = totalResolutions > 0 ? (cacheHits / totalResolutions) * 100 : 0;

    return {
      totalNamesResolved: totalResolutions,
      cacheHitRate: Math.round(cacheHitRate * 100) / 100,
      resolutionLatencyMs: 5, // Simplified average
      topResolvedNames: topResolved,
    };
  },

  getRecordByName(name: string): SNSRecord | null {
    return snsRecords.get(name) ?? null;
  },

  getAllNames(): string[] {
    return Array.from(snsRecords.keys());
  },

  invalidateCache(name: string): void {
    snsCache.delete(name);
    logger.debug(`Cache invalidated for SNS name: ${name}`);
  },

  clearAllCache(): void {
    snsCache.clear();
    logger.info('SNS cache cleared');
  },

  updateMetrics(name: string): void {
    const current = resolutionMetrics.get(name) ?? 0;
    resolutionMetrics.set(name, current + 1);
  },
};

// Type for resolution result
type Address = string;
