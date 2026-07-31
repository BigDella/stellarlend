/**
 * StellarLend Oracle Service
 * Off-chain oracle integration service that fetches price data from
 * multiple sources (CoinGecko, Binance)
 * @see https://github.com/stellarlend/stellarlend-contracts
 */

import { fileURLToPath } from 'node:url';
import { loadConfig, getSafeConfig, type OracleServiceConfig } from './config.js';
import { configureLogger, logger, logProviderHealth, logStalenessAlert } from './utils/logger.js';
import {
  createCoinGeckoProvider,
  createBinanceProvider,
  type BasePriceProvider,
} from './providers/index.js';
import {
  createValidator,
  createPriceCache,
  createPriceHistoryService,
  createAggregator,
  createContractUpdater,
  createMetricsService,
  createTWAPService,
  type PriceAggregator,
  type ContractUpdater,
  type MetricsService,
  type TWAPService,
} from './services/index.js';
import type { ProviderConfig } from './types/index.js';

/**
 * Default assets to fetch prices for
 */
const DEFAULT_ASSETS = ['XLM', 'USDC', 'BTC', 'ETH', 'SOL'];

function serializePricesForLog(
  prices: {
    asset: string;
    price: bigint;
    timestamp: number;
    confidence: number;
    sources: { source: string }[];
  }[]
) {
  return prices.map((price) => ({
    asset: price.asset,
    price: price.price.toString(),
    timestamp: price.timestamp,
    confidence: price.confidence,
    sources: price.sources.map((source) => source.source),
  }));
}

/**
 * Oracle Service
 */
export class OracleService {
  private config: OracleServiceConfig;
  private aggregator: PriceAggregator;
  private contractUpdater: ContractUpdater;
  private twapService: TWAPService;
  private providers: ProviderConfig[];
  private intervalId?: ReturnType<typeof setInterval>;
  private isRunning: boolean = false;
  private lastSuccessfulUpdate: number | null = null;
  private metricsService: MetricsService;

  constructor(config: OracleServiceConfig) {
    this.validateConfig(config);

    // Store config but never log adminSecretKey directly
    this.config = config;
    this.providers = this.normalizeProviders(config.providers);

    // Configure logging
    configureLogger(config.logLevel);

    // Create runtime providers for supported integrations only.
    const providers = this.createRuntimeProviders(this.providers);

    // Create services
    const validator = createValidator({
      maxDeviationPercent: config.maxPriceDeviationPercent,
      maxStalenessSeconds: config.priceStaleThresholdSeconds,
    });

    const cache = createPriceCache(config.cacheTtlSeconds);
    const priceHistory = createPriceHistoryService();

    this.twapService = createTWAPService(priceHistory, {
      windowSeconds: 1800,    // 30-minute TWAP window
      maxDeviationBps: 500,   // 5% deviation triggers fallback
      minDataPoints: 3,
      fallbackToMedian: true,
    });

    this.aggregator = createAggregator(providers, validator, cache, priceHistory, {
      circuitBreaker: config.circuitBreaker,
    });

    this.contractUpdater = createContractUpdater({
      network: config.stellarNetwork,
      rpcUrl: config.stellarRpcUrl,
      contractId: config.contractId,
      adminSecretKey: config.adminSecretKey,
      baseFee: config.baseFee,
      maxFee: config.maxFee,
      maxRetries: 3,
      retryDelayMs: 1000,
    });

    // Create metrics service
    this.metricsService = createMetricsService(config.metricsPort ?? 3001);

    logger.info('Oracle service initialized', {
      network: config.stellarNetwork,
      contractId: config.contractId,
      dryRun: !!config.dryRun,
      updateInterval: config.updateIntervalMs,
      providers: this.providers.map((provider) => provider.name),
    });
  }

  /**
   * Start the oracle service
   */
  async start(assets: string[] = DEFAULT_ASSETS): Promise<void> {
    if (this.isRunning) {
      logger.warn('Oracle service is already running');
      return;
    }

    this.isRunning = true;
    logger.info('Starting oracle service', { assets });

    // Start metrics server
    this.metricsService.start();

    // Run immediately on start
    await this.updatePrices(assets);

    // Schedule periodic updates
    this.intervalId = setInterval(async () => {
      await this.updatePrices(assets);
    }, this.config.updateIntervalMs);

    logger.info('Oracle service started', {
      intervalMs: this.config.updateIntervalMs,
    });
  }

  /**
   * Stop the oracle service
   */
  async stop(): Promise<void> {
    if (!this.isRunning) {
      logger.warn('Oracle service is not running');
      return;
    }

    this.isRunning = false;

    if (this.intervalId) {
      clearInterval(this.intervalId);
      this.intervalId = undefined;
    }

    // Stop metrics server
    await this.metricsService.stop();

    logger.info('Oracle service stopped');
  }

  /**
   * Fetch and update prices for specified assets
   */
  async updatePrices(assets: string[]): Promise<void> {
    const startTime = Date.now();

    logger.info('Starting price update cycle', { assets });

    // Check for staleness
    if (this.lastSuccessfulUpdate) {
      const ageSeconds = (Date.now() - this.lastSuccessfulUpdate) / 1000;
      const thresholdSeconds = this.config.priceStaleThresholdSeconds;

      if (ageSeconds > thresholdSeconds) {
        logStalenessAlert(ageSeconds, thresholdSeconds, this.lastSuccessfulUpdate);
      }
    }

    try {
      // Fetch aggregated prices
      const prices = await this.aggregator.getPrices(assets);

      if (prices.size === 0) {
        logger.error('No prices fetched from any provider');
        return;
      }

      logger.info(`Fetched ${prices.size} prices`, {
        assets: Array.from(prices.keys()),
      });

      const priceArray = Array.from(prices.values());

      // Record spot-price observations for TWAP accumulation, then compute
      // TWAP-smoothed prices to send to the contract.  This resists flash-loan
      // or low-liquidity spot-price manipulation.
      const twapPrices = priceArray.map((p) => {
        this.twapService.recordObservation(p.asset, p.price);
        const twapStatus = this.twapService.getTWAPStatus(p.asset, p.price);
        if (twapStatus?.manipulationDetected) {
          logger.warn('Spot-price manipulation detected, using TWAP for contract update', {
            asset: p.asset,
            spotPrice: p.price.toString(),
            twapPrice: twapStatus.twap.toString(),
            deviationBps: twapStatus.deviationBps,
          });
        }
        return { ...p, price: twapStatus?.twap ?? p.price };
      });

      const serializedPrices = serializePricesForLog(twapPrices);

      if (this.config.dryRun) {
        this.lastSuccessfulUpdate = Date.now();

        logger.info('DRY RUN: Would update prices on contract', {
          assets: serializedPrices.map((price) => price.asset),
          prices: serializedPrices,
          durationMs: Date.now() - startTime,
          contractId: this.config.contractId,
          dryRun: true,
        });

        return;
      }

      // Update contract with TWAP-smoothed prices
      const results = await this.contractUpdater.updatePrices(twapPrices);

      // Log results
      const successful = results.filter((r) => r.success);
      const failed = results.filter((r) => !r.success);

      logger.info('Price update cycle complete', {
        successful: successful.length,
        failed: failed.length,
        durationMs: Date.now() - startTime,
      });

      if (successful.length > 0) {
        this.lastSuccessfulUpdate = Date.now();
        this.metricsService.recordUpdate();

        // Update asset prices in metrics
        for (const result of successful) {
          const price = Number(result.price) / 1_000_000; // Convert from stroops
          this.metricsService.updateAssetPrice(result.asset, price);
        }
      }

      if (failed.length > 0) {
        this.metricsService.recordError();
        logger.warn('Some price updates failed', {
          failedAssets: failed.map((f) => f.asset),
        });
      }

      // Update provider health based on circuit breaker metrics
      const circuitBreakerMetrics = this.aggregator.getCircuitBreakerMetrics();
      for (const metric of circuitBreakerMetrics) {
        const health: 'healthy' | 'degraded' | 'unhealthy' =
          metric.state === 'CLOSED' ? 'healthy' : metric.state === 'HALF_OPEN' ? 'degraded' : 'unhealthy';
        this.metricsService.updateProviderHealth(metric.providerName, health);
      }
    } catch (error) {
      this.metricsService.recordError();
      logger.error('Price update cycle failed', { error });
    }
  }

  /**
   * Get current service status (safe for logging — secret key is masked)
   */
  getStatus() {
    const safe = getSafeConfig(this.config);
    return {
      isRunning: this.isRunning,
      network: safe.stellarNetwork,
      contractId: safe.contractId,
      adminSecretKey: safe.adminSecretKey, // masked value
      providers: this.providers.map((provider) => ({ ...provider })),
      aggregatorStats: this.aggregator.getStats(),
      circuitBreakers: this.aggregator.getCircuitBreakerMetrics?.() ?? [],
    };
  }

  /**
   * Manually fetch price for a single asset (for testing)
   */
  async fetchPrice(asset: string) {
    return this.aggregator.getPrice(asset);
  }

  private validateConfig(config: OracleServiceConfig): void {
    if (config.stellarNetwork !== 'testnet' && config.stellarNetwork !== 'mainnet') {
      throw new Error(`Invalid stellar network: ${String(config.stellarNetwork)}`);
    }

    try {
      new URL(config.stellarRpcUrl);
    } catch {
      throw new Error('Invalid stellar RPC URL');
    }

    if (!config.contractId?.trim()) {
      throw new Error('Contract ID is required');
    }
  }

  private normalizeProviders(providers: ProviderConfig[]): ProviderConfig[] {
    if (providers.length === 1) {
      return [{ ...providers[0], weight: 1 } as ProviderConfig];
    }

    return providers.map((provider) => ({ ...provider } as ProviderConfig));
  }

  private createRuntimeProviders(configuredProviders: ProviderConfig[]): BasePriceProvider[] {
    const runtimeProviders: BasePriceProvider[] = [];

    for (const provider of configuredProviders) {
      if (!provider.enabled) {
        continue;
      }

      if (provider.name === 'coingecko') {
        runtimeProviders.push(createCoinGeckoProvider(provider.apiKey));
        continue;
      }

      if (provider.name === 'binance') {
        runtimeProviders.push(createBinanceProvider());
      }
    }

    return runtimeProviders;
  }
}

/**
 * Main entry point
 */
async function main(): Promise<void> {
  logger.info('Starting StellarLend Oracle Service');

  try {
    // Load configuration
    const config = loadConfig();

    // Create and start service
    const service = new OracleService(config);

    // Handle shutdown
    process.on('SIGINT', () => {
      logger.info('Received SIGINT, shutting down...');
      service.stop();
      process.exit(0);
    });

    process.on('SIGTERM', () => {
      logger.info('Received SIGTERM, shutting down...');
      service.stop();
      process.exit(0);
    });

    // Start service
    await service.start();
  } catch (error) {
    logger.error('Failed to start oracle service', { error });
    process.exit(1);
  }
}

function isExecutedDirectly(): boolean {
  const entryFile = process.argv[1];

  if (!entryFile) {
    return false;
  }

  return fileURLToPath(import.meta.url) === entryFile;
}

// Run only when executed as the entrypoint, not when imported by tests/modules
if (isExecutedDirectly()) {
  main().catch((error) => {
    logger.error('Unhandled oracle service error', { error });
    process.exit(1);
  });
}

// Export for programmatic use
export { loadConfig, maskSecret, getSafeConfig } from './config.js';
export type { OracleServiceConfig } from './config.js';
