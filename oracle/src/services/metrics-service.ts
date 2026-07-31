/**
 * Metrics Service
 *
 * Tracks Oracle service metrics for monitoring including:
 * - Uptime
 * - Update frequency and error rates
 * - Provider health status
 * - Asset price ages
 */

import http from 'http';
import { logger } from '../utils/logger.js';

/**
 * Provider health status
 */
export type ProviderHealth = 'healthy' | 'degraded' | 'unhealthy';

/**
 * Asset price info with age
 */
export interface AssetPriceInfo {
  price: number;
  age: number; // seconds since last update
}

/**
 * Metrics response structure
 */
export interface MetricsResponse {
  uptime: number;
  lastUpdate: string;
  updateCount: number;
  errorCount: number;
  providers: Record<string, ProviderHealth>;
  assets: Record<string, AssetPriceInfo>;
}

/**
 * Metrics Service
 */
export class MetricsService {
  private startTime: number;
  private updateCount: number = 0;
  private errorCount: number = 0;
  private lastUpdate: number | null = null;
  private providerHealth: Map<string, ProviderHealth> = new Map();
  private assetPrices: Map<string, { price: number; timestamp: number }> = new Map();
  private server?: http.Server;
  private port: number;

  constructor(port: number = 3001) {
    this.startTime = Date.now();
    this.port = port;
  }

  /**
   * Start the metrics HTTP server
   */
  start(): void {
    this.server = http.createServer((req, res) => {
      if (req.url === '/metrics' && req.method === 'GET') {
        const metrics = this.getMetrics();
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify(metrics, null, 2));
      } else {
        res.writeHead(404, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Not found' }));
      }
    });

    this.server.on('error', (err: NodeJS.ErrnoException) => {
      if (err.code === 'EADDRINUSE') {
        logger.warn(`Metrics server could not bind to port ${this.port} (already in use)`);
      } else {
        logger.error('Metrics server error', { error: err.message });
      }
    });

    this.server.listen(this.port, () => {
      logger.info(`Metrics server listening on port ${this.port}`);
    });
  }

  /**
   * Stop the metrics HTTP server
   */
  stop(): Promise<void> {
    return new Promise((resolve) => {
      if (this.server) {
        this.server.close(() => {
          this.server = undefined;
          logger.info('Metrics server stopped');
          resolve();
        });
      } else {
        resolve();
      }
    });
  }

  /**
   * Record a successful price update
   */
  recordUpdate(): void {
    this.updateCount++;
    this.lastUpdate = Date.now();
  }

  /**
   * Record an error
   */
  recordError(): void {
    this.errorCount++;
  }

  /**
   * Update provider health status
   */
  updateProviderHealth(provider: string, health: ProviderHealth): void {
    this.providerHealth.set(provider, health);
  }

  /**
   * Update asset price info
   */
  updateAssetPrice(asset: string, price: number): void {
    this.assetPrices.set(asset, {
      price,
      timestamp: Date.now(),
    });
  }

  /**
   * Get current metrics
   */
  getMetrics(): MetricsResponse {
    const uptime = Math.floor((Date.now() - this.startTime) / 1000);
    const lastUpdateIso = this.lastUpdate ? new Date(this.lastUpdate).toISOString() : new Date(0).toISOString();

    // Build providers object
    const providers: Record<string, ProviderHealth> = {};
    for (const [provider, health] of this.providerHealth.entries()) {
      providers[provider] = health;
    }

    // Build assets object with age calculation
    const assets: Record<string, AssetPriceInfo> = {};
    for (const [asset, data] of this.assetPrices.entries()) {
      const age = Math.floor((Date.now() - data.timestamp) / 1000);
      assets[asset] = {
        price: data.price,
        age,
      };
    }

    return {
      uptime,
      lastUpdate: lastUpdateIso,
      updateCount: this.updateCount,
      errorCount: this.errorCount,
      providers,
      assets,
    };
  }

  /**
   * Get uptime in seconds
   */
  getUptime(): number {
    return Math.floor((Date.now() - this.startTime) / 1000);
  }

  /**
   * Get update count
   */
  getUpdateCount(): number {
    return this.updateCount;
  }

  /**
   * Get error count
   */
  getErrorCount(): number {
    return this.errorCount;
  }

  /**
   * Reset metrics (useful for testing)
   */
  reset(): void {
    this.updateCount = 0;
    this.errorCount = 0;
    this.lastUpdate = null;
    this.providerHealth.clear();
    this.assetPrices.clear();
    this.startTime = Date.now();
  }
}

/**
 * Create a metrics service
 */
export function createMetricsService(port?: number): MetricsService {
  return new MetricsService(port);
}
