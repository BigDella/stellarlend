/**
 * Manipulation Detection Service
 *
 * Monitors oracle price feeds for signs of manipulation and triggers
 * protective responses including alerts, circuit breaker activation,
 * and fallback to median pricing across sources.
 *
 * Detection strategies:
 * 1. Source deviation monitoring — compares individual source prices
 *    against the aggregate median
 * 2. TWAP vs spot deviation — flags when spot price diverges
 *    significantly from the time-weighted average
 * 3. Volatility spike detection — identifies rapid price movements
 *    within short time windows
 * 4. Source availability tracking — monitors source health for
 *    early warning of feed manipulation attacks
 */

import { logger } from '../utils/logger.js';
import type { PriceData } from '../types/index.js';
import { calculateDeviationBps, deviationBpsToNumber } from '../utils/price-math.js';

/**
 * Manipulation alert severity levels
 */
export enum AlertSeverity {
  INFO = 'info',
  WARNING = 'warning',
  CRITICAL = 'critical',
}

/**
 * A single manipulation alert
 */
export interface ManipulationAlert {
  id: string;
  asset: string;
  severity: AlertSeverity;
  type: AlertType;
  message: string;
  deviationBps: number;
  thresholdBps: number;
  referencePrice: string;
  observedPrice: string;
  timestamp: number;
  sources: string[];
}

/**
 * Types of manipulation detection alerts
 */
export enum AlertType {
  SOURCE_DEVIATION = 'source_deviation',
  TWAP_SPOT_DEVIATION = 'twap_spot_deviation',
  VOLATILITY_SPIKE = 'volatility_spike',
  SOURCE_UNAVAILABILITY = 'source_unavailability',
  LOW_LIQUIDITY_PERIOD = 'low_liquidity_period',
}

/**
 * Detector configuration
 */
export interface ManipulationDetectorConfig {
  /** Alert deviation threshold in bps (source vs median) */
  sourceAlertBps: number;
  /** Pause deviation threshold in bps (source vs median) */
  sourcePauseBps: number;
  /** TWAP vs spot deviation alert threshold in bps */
  twapSpotAlertBps: number;
  /** TWAP vs spot deviation pause threshold in bps */
  twapSpotPauseBps: number;
  /** Short-window volatility threshold in bps */
  volatilityBps: number;
  /** Short window for volatility check in seconds */
  volatilityWindowSeconds: number;
  /** Minimum sources required for safe pricing */
  minSourcesForSafety: number;
  /** Max alerts stored in memory */
  maxAlerts: number;
}

const DEFAULT_CONFIG: ManipulationDetectorConfig = {
  sourceAlertBps: 200, // 2%
  sourcePauseBps: 1000, // 10%
  twapSpotAlertBps: 500, // 5%
  twapSpotPauseBps: 2500, // 25%
  volatilityBps: 2000, // 20%
  volatilityWindowSeconds: 600, // 10 minutes
  minSourcesForSafety: 2,
  maxAlerts: 100,
};

/**
 * Snapshot of a source's state for deviation checking and drift detection.
 * Tracks consecutive deviations to identify persistent manipulation from a single source.
 */
interface SourceState {
  name: string;
  lastPrice: bigint;
  lastTimestamp: number;
  /** Consecutive times this source has deviated from median */
  consecutiveDeviations: number;
  /** Whether this source is currently flagged as suspicious */
  isSuspicious: boolean;
}

/**
 * Manipulation Detection Service
 */
export class ManipulationDetector {
  private config: ManipulationDetectorConfig;
  private alerts: ManipulationAlert[] = [];
  private sourceStates: Map<string, SourceState[]> = new Map();
  private alertCounter: number = 0;

  constructor(config: Partial<ManipulationDetectorConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    logger.info('Manipulation detector initialized', this.config);
  }

  /**
   * Check source prices for deviation from the median.
   * Returns alerts when individual sources diverge significantly.
   * Tracks consecutive deviations to detect persistent manipulation from a single source.
   */
  checkSourceDeviations(
    asset: string,
    prices: PriceData[],
    medianPrice: bigint
  ): ManipulationAlert[] {
    const alerts: ManipulationAlert[] = [];

    for (const price of prices) {
      if (price.price <= 0n || medianPrice <= 0n) continue;

      const deviationBpsBig = calculateDeviationBps(medianPrice, price.price);
      const deviationBps = deviationBpsToNumber(deviationBpsBig);
      const sourceState = this.getOrCreateSourceState(asset, price.source);

      if (deviationBps > this.config.sourcePauseBps) {
        sourceState.consecutiveDeviations++;
        sourceState.isSuspicious = true;

        const alert = this.createAlert(
          asset,
          AlertSeverity.CRITICAL,
          AlertType.SOURCE_DEVIATION,
          `Source ${price.source} deviates ${deviationBps} bps from median (${sourceState.consecutiveDeviations} consecutive) — exceeding pause threshold`,
          deviationBps,
          this.config.sourcePauseBps,
          medianPrice.toString(),
          price.price.toString(),
          prices.map((p) => p.source)
        );
        alerts.push(alert);
        this.storeAlert(alert);
      } else if (deviationBps > this.config.sourceAlertBps) {
        sourceState.consecutiveDeviations++;

        if (sourceState.consecutiveDeviations >= 3) {
          sourceState.isSuspicious = true;
        }

        const alert = this.createAlert(
          asset,
          AlertSeverity.WARNING,
          AlertType.SOURCE_DEVIATION,
          `Source ${price.source} deviates ${deviationBps} bps from median (${sourceState.consecutiveDeviations} consecutive)`,
          deviationBps,
          this.config.sourceAlertBps,
          medianPrice.toString(),
          price.price.toString(),
          prices.map((p) => p.source)
        );
        alerts.push(alert);
        this.storeAlert(alert);
      } else {
        // Source is aligned — reset deviation counter
        sourceState.consecutiveDeviations = 0;
        sourceState.isSuspicious = false;
      }

      // Update source state with latest price
      sourceState.lastPrice = price.price;
      sourceState.lastTimestamp = Math.floor(Date.now() / 1000);
      this.updateSourceState(asset, sourceState);
    }

    return alerts;
  }

  /**
   * Get suspicious sources for an asset (sources flagged as consistently deviating).
   */
  getSuspiciousSources(asset: string): string[] {
    const upperAsset = asset.toUpperCase();
    const states = this.sourceStates.get(upperAsset) ?? [];
    return states.filter((s) => s.isSuspicious).map((s) => s.name);
  }

  /**
   * Get the consecutive deviation count for a specific source.
   * Useful for monitoring dashboards.
   */
  getSourceDeviationCount(asset: string, source: string): number {
    const state = this.getOrCreateSourceState(asset, source);
    return state.consecutiveDeviations;
  }

  /**
   * Check TWAP vs spot price deviation.
   * Large deviations may indicate short-term manipulation of the spot market.
   */
  checkTWAPSpotDeviation(
    asset: string,
    twap: bigint,
    spotPrice: bigint,
    sources: string[]
  ): ManipulationAlert | null {
    if (twap <= 0n || spotPrice <= 0n) return null;

    const deviationBps = deviationBpsToNumber(calculateDeviationBps(twap, spotPrice));

    if (deviationBps > this.config.twapSpotPauseBps) {
      const alert = this.createAlert(
        asset,
        AlertSeverity.CRITICAL,
        AlertType.TWAP_SPOT_DEVIATION,
        `Spot price deviates ${deviationBps} bps from TWAP — possible manipulation`,
        deviationBps,
        this.config.twapSpotPauseBps,
        twap.toString(),
        spotPrice.toString(),
        sources
      );
      this.storeAlert(alert);
      return alert;
    }

    if (deviationBps > this.config.twapSpotAlertBps) {
      const alert = this.createAlert(
        asset,
        AlertSeverity.WARNING,
        AlertType.TWAP_SPOT_DEVIATION,
        `Spot price deviates ${deviationBps} bps from TWAP`,
        deviationBps,
        this.config.twapSpotAlertBps,
        twap.toString(),
        spotPrice.toString(),
        sources
      );
      this.storeAlert(alert);
      return alert;
    }

    return null;
  }

  /**
   * Check for volatility spikes within a short time window.
   */
  checkVolatilitySpike(
    asset: string,
    currentPrice: bigint,
    historicalPrices: Array<{ price: bigint; timestamp: number }>
  ): ManipulationAlert | null {
    if (currentPrice <= 0n) return null;

    const now = Math.floor(Date.now() / 1000);
    const windowStart = now - this.config.volatilityWindowSeconds;
    let maxDeviationBps = 0;
    let referencePrice = 0n;

    for (const hist of historicalPrices) {
      if (hist.timestamp < windowStart || hist.price <= 0n) continue;
      const bps = this.calculateDeviationBps(hist.price, currentPrice);
      if (bps > maxDeviationBps) {
        maxDeviationBps = bps;
        referencePrice = hist.price;
      }
    }

    if (maxDeviationBps > this.config.volatilityBps) {
      const alert = this.createAlert(
        asset,
        AlertSeverity.CRITICAL,
        AlertType.VOLATILITY_SPIKE,
        `Price moved ${maxDeviationBps} bps in ${this.config.volatilityWindowSeconds}s — volatility spike detected`,
        maxDeviationBps,
        this.config.volatilityBps,
        referencePrice.toString(),
        currentPrice.toString(),
        []
      );
      this.storeAlert(alert);
      return alert;
    }

    return null;
  }

  /**
   * Check source availability. Triggers alerts when too few sources are
   * available, which could indicate a feed manipulation attack.
   */
  checkSourceAvailability(asset: string, availableSources: string[]): ManipulationAlert | null {
    if (availableSources.length < this.config.minSourcesForSafety) {
      const alert = this.createAlert(
        asset,
        AlertSeverity.WARNING,
        AlertType.SOURCE_UNAVAILABILITY,
        `Only ${availableSources.length} sources available (minimum: ${this.config.minSourcesForSafety}) — potential manipulation risk`,
        0,
        0,
        '0',
        '0',
        availableSources
      );
      this.storeAlert(alert);
      return alert;
    }
    return null;
  }

  /**
   * Determine if manipulation requires fallback to median pricing.
   * Returns true when any critical alert has been raised.
   */
  shouldFallbackToMedian(asset: string): boolean {
    const upperAsset = asset.toUpperCase();
    const assetAlerts = this.alerts.filter(
      (a) => a.asset === upperAsset && a.severity === AlertSeverity.CRITICAL
    );
    return assetAlerts.length > 0;
  }

  /**
   * Get all stored alerts
   */
  getAlerts(asset?: string): ManipulationAlert[] {
    if (asset) {
      return this.alerts.filter((a) => a.asset === asset.toUpperCase());
    }
    return [...this.alerts];
  }

  /**
   * Get recent alerts (within the last N seconds)
   */
  getRecentAlerts(seconds: number = 300, asset?: string): ManipulationAlert[] {
    const now = Math.floor(Date.now() / 1000);
    const threshold = now - seconds;
    return this.getAlerts(asset).filter((a) => a.timestamp >= threshold);
  }

  /**
   * Get detector configuration
   */
  getConfig(): ManipulationDetectorConfig {
    return { ...this.config };
  }

  /**
   * Update configuration at runtime
   */
  updateConfig(config: Partial<ManipulationDetectorConfig>): void {
    this.config = { ...this.config, ...config };
    logger.info('Manipulation detector config updated', this.config);
  }

  /**
   * Clear stored alerts
   */
  clearAlerts(asset?: string): void {
    if (asset) {
      this.alerts = this.alerts.filter((a) => a.asset !== asset.toUpperCase());
    } else {
      this.alerts = [];
    }
  }

  // ── Private helpers ─────────────────────────────────────────────────────

  private calculateDeviationBps(reference: bigint, observed: bigint): number {
    return deviationBpsToNumber(calculateDeviationBps(reference, observed));
  }

  private createAlert(
    asset: string,
    severity: AlertSeverity,
    type: AlertType,
    message: string,
    deviationBps: number,
    thresholdBps: number,
    referencePrice: string,
    observedPrice: string,
    sources: string[]
  ): ManipulationAlert {
    this.alertCounter++;
    return {
      id: `alert_${this.alertCounter}_${Date.now()}`,
      asset: asset.toUpperCase(),
      severity,
      type,
      message,
      deviationBps,
      thresholdBps,
      referencePrice,
      observedPrice,
      timestamp: Math.floor(Date.now() / 1000),
      sources,
    };
  }

  private storeAlert(alert: ManipulationAlert): void {
    this.alerts.push(alert);
    // Trim old alerts if exceeding max
    while (this.alerts.length > this.config.maxAlerts) {
      this.alerts.shift();
    }

    logger.warn(`[${alert.severity.toUpperCase()}] ${alert.type}: ${alert.message}`);
  }

  private getOrCreateSourceState(asset: string, source: string): SourceState {
    const existing = this.getSourceState(asset, source);
    if (existing) return existing;

    return {
      name: source,
      lastPrice: 0n,
      lastTimestamp: 0,
      consecutiveDeviations: 0,
      isSuspicious: false,
    };
  }

  private getSourceState(asset: string, source: string): SourceState | undefined {
    const states = this.sourceStates.get(asset.toUpperCase());
    return states?.find((s) => s.name === source);
  }

  private updateSourceState(asset: string, state: SourceState): void {
    const upperAsset = asset.toUpperCase();
    const states = this.sourceStates.get(upperAsset) ?? [];
    const idx = states.findIndex((s) => s.name === state.name);
    if (idx >= 0) {
      states[idx] = state;
    } else {
      states.push(state);
    }
    this.sourceStates.set(upperAsset, states);
  }
}

/**
 * Create a manipulation detector instance
 */
export function createManipulationDetector(
  config?: Partial<ManipulationDetectorConfig>
): ManipulationDetector {
  return new ManipulationDetector(config);
}
