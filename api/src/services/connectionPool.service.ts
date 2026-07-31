import axios, { AxiosInstance, AxiosRequestConfig } from 'axios';
import http from 'http';
import https from 'https';
import logger from '../utils/logger';

interface PoolMetrics {
  activeConnections: number;
  idleConnections: number;
  waitingRequests: number;
  totalRequests: number;
  failedRequests: number;
  avgLatencyMs: number;
}

interface ConnectionPoolConfig {
  maxConnections: number;
  maxConcurrentRequests: number;
  keepAliveTimeout: number;
  healthCheckInterval: number;
  requestTimeout: number;
}

class ConnectionPool {
  private httpAgent: http.Agent;
  private httpsAgent: https.Agent;
  private axiosInstance: AxiosInstance;
  private metrics: PoolMetrics;
  private latencies: number[];
  private healthCheckTimer?: NodeJS.Timeout;
  private config: ConnectionPoolConfig;

  constructor(config: Partial<ConnectionPoolConfig> = {}) {
    this.config = {
      maxConnections: config.maxConnections || 50,
      maxConcurrentRequests: config.maxConcurrentRequests || 100,
      keepAliveTimeout: config.keepAliveTimeout || 60000,
      healthCheckInterval: config.healthCheckInterval || 30000,
      requestTimeout: config.requestTimeout || 30000,
    };

    this.httpAgent = new http.Agent({
      keepAlive: true,
      keepAliveMsecs: this.config.keepAliveTimeout,
      maxSockets: this.config.maxConnections,
      maxFreeSockets: Math.floor(this.config.maxConnections / 2),
      timeout: this.config.keepAliveTimeout,
    });

    this.httpsAgent = new https.Agent({
      keepAlive: true,
      keepAliveMsecs: this.config.keepAliveTimeout,
      maxSockets: this.config.maxConnections,
      maxFreeSockets: Math.floor(this.config.maxConnections / 2),
      timeout: this.config.keepAliveTimeout,
    });

    this.axiosInstance = axios.create({
      httpAgent: this.httpAgent,
      httpsAgent: this.httpsAgent,
      timeout: this.config.requestTimeout,
      headers: {
        Connection: 'keep-alive',
      },
    });

    this.metrics = {
      activeConnections: 0,
      idleConnections: 0,
      waitingRequests: 0,
      totalRequests: 0,
      failedRequests: 0,
      avgLatencyMs: 0,
    };

    this.latencies = [];

    this.startHealthCheck();
    logger.info('Connection pool initialized', { config: this.config });
  }

  async request<T = any>(config: AxiosRequestConfig): Promise<T> {
    const startTime = Date.now();
    this.metrics.totalRequests++;
    this.metrics.activeConnections++;

    try {
      const response = await this.axiosInstance.request<T>(config);
      const latency = Date.now() - startTime;
      this.recordLatency(latency);
      return response.data;
    } catch (error) {
      this.metrics.failedRequests++;

      // Fallback: create new connection on pool exhaustion
      if (this.isPoolExhausted(error)) {
        logger.warn('Connection pool exhausted, creating new connection');
        try {
          const fallbackResponse = await axios.request<T>({
            ...config,
            timeout: this.config.requestTimeout,
          });
          const latency = Date.now() - startTime;
          this.recordLatency(latency);
          return fallbackResponse.data;
        } catch (fallbackError) {
          logger.error('Fallback request failed', { error: fallbackError });
          throw fallbackError;
        }
      }

      throw error;
    } finally {
      this.metrics.activeConnections--;
    }
  }

  async get<T = any>(url: string, config?: AxiosRequestConfig): Promise<T> {
    return this.request<T>({ ...config, method: 'GET', url });
  }

  async post<T = any>(url: string, data?: any, config?: AxiosRequestConfig): Promise<T> {
    return this.request<T>({ ...config, method: 'POST', url, data });
  }

  private recordLatency(latency: number): void {
    this.latencies.push(latency);

    // Keep only last 1000 latencies
    if (this.latencies.length > 1000) {
      this.latencies.shift();
    }

    // Update average
    const sum = this.latencies.reduce((a, b) => a + b, 0);
    this.metrics.avgLatencyMs = Math.round(sum / this.latencies.length);
  }

  private isPoolExhausted(error: any): boolean {
    return (
      error?.code === 'ECONNREFUSED' ||
      error?.code === 'ETIMEDOUT' ||
      error?.message?.includes('socket hang up') ||
      error?.message?.includes('ECONNRESET')
    );
  }

  private startHealthCheck(): void {
    this.healthCheckTimer = setInterval(() => {
      this.performHealthCheck();
    }, this.config.healthCheckInterval);
  }

  private performHealthCheck(): void {
    // Get socket stats from agents
    const httpSockets = (this.httpAgent as any).sockets || {};
    const httpFreeSockets = (this.httpAgent as any).freeSockets || {};
    const httpsSockets = (this.httpsAgent as any).sockets || {};
    const httpsFreeSockets = (this.httpsAgent as any).freeSockets || {};

    const activeSockets =
      Object.values(httpSockets).flat().length + Object.values(httpsSockets).flat().length;
    const idleSockets =
      Object.values(httpFreeSockets).flat().length + Object.values(httpsFreeSockets).flat().length;

    this.metrics.idleConnections = idleSockets;

    // Log health metrics
    if (this.metrics.totalRequests > 0) {
      const successRate = (
        ((this.metrics.totalRequests - this.metrics.failedRequests) / this.metrics.totalRequests) *
        100
      ).toFixed(2);

      logger.debug('Connection pool health', {
        active: activeSockets,
        idle: idleSockets,
        totalRequests: this.metrics.totalRequests,
        failedRequests: this.metrics.failedRequests,
        successRate: `${successRate}%`,
        avgLatencyMs: this.metrics.avgLatencyMs,
      });
    }
  }

  getMetrics(): PoolMetrics {
    return { ...this.metrics };
  }

  destroy(): void {
    if (this.healthCheckTimer) {
      clearInterval(this.healthCheckTimer);
    }
    this.httpAgent.destroy();
    this.httpsAgent.destroy();
    logger.info('Connection pool destroyed');
  }
}

// Singleton instances for different endpoints
const horizonPool = new ConnectionPool({
  maxConnections: 30,
  maxConcurrentRequests: 60,
});

const sorobanPool = new ConnectionPool({
  maxConnections: 20,
  maxConcurrentRequests: 40,
});

export const connectionPoolService = {
  horizon: horizonPool,
  soroban: sorobanPool,

  getMetrics: () => ({
    horizon: horizonPool.getMetrics(),
    soroban: sorobanPool.getMetrics(),
  }),

  destroy: () => {
    horizonPool.destroy();
    sorobanPool.destroy();
  },
};
