/**
 * TWAP (Time-Weighted Average Price) Service
 *
 * Accumulates prices over time and computes TWAP values resistant to
 * short-term manipulation. Integrates with the PriceHistoryService for
 * observation storage and the ManipulationDetector for deviation alerts.
 *
 * Key features:
 * - Configurable TWAP window (default 30 minutes)
 * - Gas-efficient single-update-per-period accumulation
 * - Manipulation resistance via deviation checks
 * - Fallback to median across sources on detected manipulation
 * - Liquidation pricing endpoint for lending protocol integration
 */

import type { AggregatedPrice } from '../types/index.js';
import { PriceHistoryService, type TWAPResult } from './price-history.js';
import { logger } from '../utils/logger.js';
import { calculateDeviationBps, deviationBpsToNumber } from '../utils/price-math.js';

/**
 * TWAP service configuration
 */
export interface TWAPConfig {
  /** TWAP time window in seconds (default: 1800 = 30 minutes) */
  windowSeconds: number;
  /** Maximum allowed deviation between TWAP and spot price (in basis points) */
  maxDeviationBps: number;
  /** Minimum number of data points required for valid TWAP */
  minDataPoints: number;
  /** Whether to fall back to median on manipulation detection */
  fallbackToMedian: boolean;
  /** Maximum staleness of TWAP before recomputation (seconds) */
  maxStalenessSeconds: number;
  /** Whether to use single-update-per-period for gas efficiency */
  singleUpdatePerPeriod: boolean;
}

/**
 * Default TWAP configuration
 */
const DEFAULT_TWAP_CONFIG: TWAPConfig = {
  windowSeconds: 1800, // 30 minutes
  maxDeviationBps: 500, // 5%
  minDataPoints: 3,
  fallbackToMedian: true,
  maxStalenessSeconds: 60,
  singleUpdatePerPeriod: true,
};

/**
 * TWAP computation status
 */
export interface TWAPStatus {
  twap: bigint;
  spotPrice: bigint;
  deviationBps: number;
  manipulationDetected: boolean;
  usedFallback: boolean;
  dataPoints: number;
  windowSeconds: number;
  lastComputedAt: number;
}

/**
 * Cached TWAP entry to avoid recomputation within the same period
 */
interface CachedTWAP {
  twap: bigint;
  computedAt: number;
  spotAnchor: bigint;
}

/**
 * TWAP Oracle Service
 */
export class TWAPService {
  private priceHistory: PriceHistoryService;
  private config: TWAPConfig;
  private twapCache: Map<string, CachedTWAP> = new Map();
  private lastPeriodUpdate: Map<string, number> = new Map();

  constructor(priceHistory: PriceHistoryService, config: Partial<TWAPConfig> = {}) {
    this.priceHistory = priceHistory;
    this.config = { ...DEFAULT_TWAP_CONFIG, ...config };

    logger.info('TWAP service initialized', {
      windowSeconds: this.config.windowSeconds,
      maxDeviationBps: this.config.maxDeviationBps,
      fallbackToMedian: this.config.fallbackToMedian,
      singleUpdatePerPeriod: this.config.singleUpdatePerPeriod,
    });
  }

  /**
   * Compute TWAP for an asset using accumulated price observations.
   *
   * Implements gas-efficient single-update-per-period: if a TWAP was
   * already computed within the staleness window, returns the cached value.
   */
  computeTWAP(asset: string, spotPrice: bigint, now: number = Math.floor(Date.now() / 1000)): TWAPStatus | null {
    const upperAsset = asset.toUpperCase();

    // Check cache for recent computation (gas-efficient path)
    const cached = this.twapCache.get(upperAsset);
    if (cached && this.config.singleUpdatePerPeriod) {
      const age = now - cached.computedAt;
      if (age < this.config.maxStalenessSeconds) {
        const deviationBps = this.calculateDeviationBps(cached.spotAnchor, spotPrice);

        return {
          twap: cached.twap,
          spotPrice,
          deviationBps,
          manipulationDetected: deviationBps > this.config.maxDeviationBps,
          usedFallback: false,
          dataPoints: this.priceHistory.getPriceHistory(upperAsset).length,
          windowSeconds: this.config.windowSeconds,
          lastComputedAt: cached.computedAt,
        };
      }
    }

    // Compute fresh TWAP from price history
    const twapResult = this.priceHistory.calculateTWAP(upperAsset, this.config.windowSeconds);

    if (!twapResult || twapResult.dataPoints < this.config.minDataPoints) {
      logger.warn(`Insufficient data for TWAP for ${upperAsset}`, {
        dataPoints: twapResult?.dataPoints ?? 0,
        required: this.config.minDataPoints,
      });

      // Fall back to spot price when insufficient history
      return {
        twap: spotPrice,
        spotPrice,
        deviationBps: 0,
        manipulationDetected: false,
        usedFallback: true,
        dataPoints: twapResult?.dataPoints ?? 0,
        windowSeconds: this.config.windowSeconds,
        lastComputedAt: now,
      };
    }

    const twap = twapResult.twap;
    const deviationBps = this.calculateDeviationBps(twap, spotPrice);
    const manipulationDetected = deviationBps > this.config.maxDeviationBps;

    let finalTWAP = twap;
    let usedFallback = false;

    if (manipulationDetected && this.config.fallbackToMedian) {
      logger.warn(`Manipulation detected for ${upperAsset}`, {
        twap: twap.toString(),
        spotPrice: spotPrice.toString(),
        deviationBps,
        maxDeviationBps: this.config.maxDeviationBps,
        action: 'Falling back to spot price (median across sources)',
      });

      // Fall back to the spot price which is already a median across sources
      finalTWAP = spotPrice;
      usedFallback = true;
    }

    // Cache the result
    this.twapCache.set(upperAsset, {
      twap: finalTWAP,
      computedAt: now,
      spotAnchor: spotPrice,
    });

    return {
      twap: finalTWAP,
      spotPrice,
      deviationBps,
      manipulationDetected,
      usedFallback,
      dataPoints: twapResult.dataPoints,
      windowSeconds: this.config.windowSeconds,
      lastComputedAt: now,
    };
  }

  /**
   * Record a new price observation for TWAP accumulation.
   * Called after each successful price fetch.
   *
   * In single-update-per-period mode, observations are recorded at most
   * once per period to save gas on-chain.
   */
  recordObservation(asset: string, price: bigint, now: number = Math.floor(Date.now() / 1000)): void {
    const upperAsset = asset.toUpperCase();

    if (this.config.singleUpdatePerPeriod) {
      const lastUpdate = this.lastPeriodUpdate.get(upperAsset) ?? 0;
      const periodSeconds = Math.max(60, this.config.windowSeconds / 30); // At most one update per period/30

      if (now - lastUpdate < periodSeconds) {
        return; // Skip this observation — already updated within this period
      }
    }

    this.priceHistory.addPriceEntry(upperAsset, price, now);
    this.lastPeriodUpdate.set(upperAsset, now);
  }

  /**
   * Get the TWAP-based liquidation price for an asset.
   *
   * This is the primary integration point for the lending protocol.
   * Uses TWAP when sufficient history exists; falls back to median
   * spot price across sources when manipulation is detected.
   */
  getLiquidationPrice(
    asset: string,
    spotPrice: bigint,
    now: number = Math.floor(Date.now() / 1000)
  ): bigint {
    const status = this.computeTWAP(asset, spotPrice, now);

    if (!status) {
      logger.warn(`Failed to compute TWAP for ${asset}, using spot price for liquidation`);
      return spotPrice;
    }

    if (status.manipulationDetected) {
      logger.warn(
        `TWAP manipulation detected for ${asset} (deviation: ${status.deviationBps} bps), ` +
        `using fallback price for liquidation`
      );
    }

    return status.twap;
  }

  /**
   * Get full TWAP status including manipulation detection info
   */
  getTWAPStatus(asset: string, spotPrice: bigint): TWAPStatus | null {
    return this.computeTWAP(asset, spotPrice);
  }

  /**
   * Force recompute TWAP (bypasses cache)
   */
  forceRecomputeTWAP(asset: string, spotPrice: bigint): TWAPStatus | null {
    this.twapCache.delete(asset.toUpperCase());
    return this.computeTWAP(asset, spotPrice);
  }

  /**
   * Get the current configuration
   */
  getConfig(): TWAPConfig {
    return { ...this.config };
  }

  /**
   * Update TWAP configuration at runtime
   */
  updateConfig(config: Partial<TWAPConfig>): void {
    this.config = { ...this.config, ...config };
    this.twapCache.clear(); // Invalidate cache on config change
    logger.info('TWAP configuration updated', this.config);
  }

  /**
   * Calculate deviation between two prices in basis points.
   * Uses shared price-math utility for precision-safe calculation.
   */
  private calculateDeviationBps(reference: bigint, observed: bigint): number {
    return deviationBpsToNumber(calculateDeviationBps(reference, observed));
  }

  /**
   * Clear all cached data
   */
  clearCache(asset?: string): void {
    if (asset) {
      const upperAsset = asset.toUpperCase();
      this.twapCache.delete(upperAsset);
      this.lastPeriodUpdate.delete(upperAsset);
    } else {
      this.twapCache.clear();
      this.lastPeriodUpdate.clear();
    }
  }
}

/**
 * Create a TWAP service instance
 */
export function createTWAPService(
  priceHistory: PriceHistoryService,
  config?: Partial<TWAPConfig>
): TWAPService {
  return new TWAPService(priceHistory, config);
}
