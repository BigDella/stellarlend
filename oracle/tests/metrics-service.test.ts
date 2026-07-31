/**
 * Metrics Service Tests
 *
 * Tests for the metrics tracking service and HTTP endpoint
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import http from 'http';
import { MetricsService, createMetricsService } from '../src/services/metrics-service.js';

describe('MetricsService', () => {
  let metricsService: MetricsService;
  let testPort: number;

  beforeEach(() => {
    // Use a random port to avoid conflicts
    testPort = 3001 + Math.floor(Math.random() * 1000);
    metricsService = createMetricsService(testPort);
  });

  afterEach(async () => {
    await metricsService.stop();
  });

  describe('constructor and initialization', () => {
    it('should create a metrics service with default port', async () => {
      const defaultService = createMetricsService();
      expect(defaultService).toBeDefined();
      await defaultService.stop();
    });

    it('should create a metrics service with custom port', async () => {
      const customService = createMetricsService(4000);
      expect(customService).toBeDefined();
      await customService.stop();
    });

    it('should initialize with zero counts', () => {
      expect(metricsService.getUpdateCount()).toBe(0);
      expect(metricsService.getErrorCount()).toBe(0);
      expect(metricsService.getUptime()).toBeGreaterThanOrEqual(0);
    });
  });

  describe('metrics tracking', () => {
    it('should record updates', () => {
      metricsService.recordUpdate();
      expect(metricsService.getUpdateCount()).toBe(1);

      metricsService.recordUpdate();
      metricsService.recordUpdate();
      expect(metricsService.getUpdateCount()).toBe(3);
    });

    it('should record errors', () => {
      metricsService.recordError();
      expect(metricsService.getErrorCount()).toBe(1);

      metricsService.recordError();
      metricsService.recordError();
      expect(metricsService.getErrorCount()).toBe(3);
    });

    it('should update provider health', () => {
      metricsService.updateProviderHealth('binance', 'healthy');
      metricsService.updateProviderHealth('coingecko', 'degraded');

      const metrics = metricsService.getMetrics();
      expect(metrics.providers.binance).toBe('healthy');
      expect(metrics.providers.coingecko).toBe('degraded');
    });

    it('should update asset prices', () => {
      metricsService.updateAssetPrice('XLM', 0.12);
      metricsService.updateAssetPrice('BTC', 67000);

      const metrics = metricsService.getMetrics();
      expect(metrics.assets.XLM).toBeDefined();
      expect(metrics.assets.XLM.price).toBe(0.12);
      expect(metrics.assets.XLM.age).toBeGreaterThanOrEqual(0);

      expect(metrics.assets.BTC).toBeDefined();
      expect(metrics.assets.BTC.price).toBe(67000);
    });

    it('should calculate asset age correctly', async () => {
      metricsService.updateAssetPrice('XLM', 0.12);
      
      // Wait a bit
      await new Promise(resolve => setTimeout(resolve, 100));

      const metrics = metricsService.getMetrics();
      expect(metrics.assets.XLM.age).toBeGreaterThanOrEqual(0);
      expect(metrics.assets.XLM.age).toBeLessThan(1); // Should be less than 1 second
    });
  });

  describe('metrics endpoint', () => {
    it('should start and stop the HTTP server', async () => {
      metricsService.start();
      await metricsService.stop();
      // If we get here without error, the test passes
    });

    it('should return metrics on GET /metrics', async () => {
      metricsService.start();

      // Set up some test data
      metricsService.recordUpdate();
      metricsService.recordUpdate();
      metricsService.recordError();
      metricsService.updateProviderHealth('binance', 'healthy');
      metricsService.updateAssetPrice('XLM', 0.12);

      // Make HTTP request
      await new Promise<void>((resolve, reject) => {
        const req = http.get(`http://localhost:${testPort}/metrics`, (res) => {
          let data = '';

          res.on('data', (chunk) => {
            data += chunk;
          });

          res.on('end', () => {
            try {
              expect(res.statusCode).toBe(200);
              expect(res.headers['content-type']).toBe('application/json');

              const metrics = JSON.parse(data);
              expect(metrics.uptime).toBeGreaterThanOrEqual(0);
              expect(metrics.updateCount).toBe(2);
              expect(metrics.errorCount).toBe(1);
              expect(metrics.providers.binance).toBe('healthy');
              expect(metrics.assets.XLM).toBeDefined();
              expect(metrics.assets.XLM.price).toBe(0.12);

              metricsService.stop();
              resolve();
            } catch (e) {
              metricsService.stop();
              reject(e);
            }
          });
        });

        req.on('error', (err) => {
          metricsService.stop();
          reject(err);
        });
      });
    });

    it('should return 404 for non-metrics endpoints', async () => {
      metricsService.start();

      await new Promise<void>((resolve, reject) => {
        const req = http.get(`http://localhost:${testPort}/other`, (res) => {
          let data = '';

          res.on('data', (chunk) => {
            data += chunk;
          });

          res.on('end', () => {
            try {
              expect(res.statusCode).toBe(404);
              const body = JSON.parse(data);
              expect(body.error).toBe('Not found');

              metricsService.stop();
              resolve();
            } catch (e) {
              metricsService.stop();
              reject(e);
            }
          });
        });

        req.on('error', (err) => {
          metricsService.stop();
          reject(err);
        });
      });
    });
  });

  describe('metrics response structure', () => {
    it('should return correctly structured metrics', () => {
      metricsService.recordUpdate();
      metricsService.updateProviderHealth('binance', 'healthy');
      metricsService.updateAssetPrice('XLM', 0.12);

      const metrics = metricsService.getMetrics();

      expect(metrics).toHaveProperty('uptime');
      expect(metrics).toHaveProperty('lastUpdate');
      expect(metrics).toHaveProperty('updateCount');
      expect(metrics).toHaveProperty('errorCount');
      expect(metrics).toHaveProperty('providers');
      expect(metrics).toHaveProperty('assets');

      expect(typeof metrics.uptime).toBe('number');
      expect(typeof metrics.lastUpdate).toBe('string');
      expect(typeof metrics.updateCount).toBe('number');
      expect(typeof metrics.errorCount).toBe('number');
      expect(typeof metrics.providers).toBe('object');
      expect(typeof metrics.assets).toBe('object');
    });

    it('should format lastUpdate as ISO string', () => {
      metricsService.recordUpdate();
      const metrics = metricsService.getMetrics();

      expect(metrics.lastUpdate).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/);
    });

    it('should return epoch time when no updates have occurred', () => {
      const metrics = metricsService.getMetrics();
      expect(metrics.lastUpdate).toBe('1970-01-01T00:00:00.000Z');
    });
  });

  describe('reset functionality', () => {
    it('should reset all metrics', () => {
      metricsService.recordUpdate();
      metricsService.recordUpdate();
      metricsService.recordError();
      metricsService.updateProviderHealth('binance', 'healthy');
      metricsService.updateAssetPrice('XLM', 0.12);

      expect(metricsService.getUpdateCount()).toBe(2);
      expect(metricsService.getErrorCount()).toBe(1);

      metricsService.reset();

      expect(metricsService.getUpdateCount()).toBe(0);
      expect(metricsService.getErrorCount()).toBe(0);

      const metrics = metricsService.getMetrics();
      expect(metrics.providers).toEqual({});
      expect(metrics.assets).toEqual({});
    });
  });

  describe('uptime tracking', () => {
    it('should track uptime correctly', () => {
      vi.useFakeTimers();
      const initialUptime = metricsService.getUptime();
      expect(initialUptime).toBeGreaterThanOrEqual(0);

      vi.advanceTimersByTime(1500);

      const laterUptime = metricsService.getUptime();
      expect(laterUptime).toBeGreaterThan(initialUptime);
      vi.useRealTimers();
    });
  });
});
