/**
 * Correlation Matrix Service — Issue #450
 *
 * Computes rolling 30/60/90-day Pearson and Spearman rank correlations between
 * all collateral asset pairs. Results are cached in Redis and recalculated hourly.
 *
 * Key responsibilities:
 *   - Maintain in-memory simulated price history (real impl: pull from DB / oracle)
 *   - Pearson and Spearman calculations (no external stats library required)
 *   - Health-factor adjustment coefficient for correlated collateral baskets
 *   - Alert emission when |pearson| exceeds configurable threshold (default 0.8)
 */

import { redisCacheService } from '../redisCache.service';
import logger from '../../utils/logger';
import {
  AssetPairCorrelation,
  CorrelationMatrix,
  CorrelationWindow,
  CorrelationHistoryPoint,
  PositionCorrelationRisk,
  CorrelationAlertConfig,
  PricePoint,
} from '../../types/riskEngine';

// Cache TTL for a full matrix recompute (1 hour per spec)
const MATRIX_CACHE_TTL_S = 3600;
// Cache TTL for a single pair
const PAIR_CACHE_TTL_S = 3600;

const CORRELATION_WINDOWS: CorrelationWindow[] = [30, 60, 90];

// Default alert threshold
const DEFAULT_CORRELATION_THRESHOLD = 0.8;

// Minimum required price data points to compute a meaningful correlation.
// If an asset has fewer than this many points we use implied correlation = 0.5.
const MIN_SAMPLES = 10;

// ─── Math helpers ────────────────────────────────────────────────────────────

/**
 * Pearson product-moment correlation coefficient.
 * Returns NaN when stdDev of either series is 0.
 */
function pearson(x: number[], y: number[]): number {
  const n = x.length;
  if (n < 2) return 0;

  const meanX = x.reduce((s, v) => s + v, 0) / n;
  const meanY = y.reduce((s, v) => s + v, 0) / n;

  let num = 0;
  let denX = 0;
  let denY = 0;

  for (let i = 0; i < n; i++) {
    const dx = (x[i] as number) - meanX;
    const dy = (y[i] as number) - meanY;
    num += dx * dy;
    denX += dx * dx;
    denY += dy * dy;
  }

  const den = Math.sqrt(denX * denY);
  if (den === 0) return 0;
  return Math.max(-1, Math.min(1, num / den));
}

/**
 * Spearman rank correlation: convert each series to ranks then apply Pearson.
 */
function rankArray(arr: number[]): number[] {
  const sorted = [...arr].map((v, i) => ({ v, i })).sort((a, b) => a.v - b.v);
  const ranks = new Array<number>(arr.length);
  for (let i = 0; i < sorted.length; ) {
    let j = i;
    // handle ties: average rank
    while (j < sorted.length && (sorted[j]?.v ?? 0) === (sorted[i]?.v ?? 0)) j++;
    const avgRank = (i + j - 1) / 2 + 1;
    for (let k = i; k < j; k++) {
      ranks[(sorted[k] as { v: number; i: number }).i] = avgRank;
    }
    i = j;
  }
  return ranks;
}

function spearman(x: number[], y: number[]): number {
  return pearson(rankArray(x), rankArray(y));
}

/**
 * Align two price series by timestamp, returning arrays of matching prices.
 * Uses the closest prior price for any gap (forward-fill).
 */
function alignPriceSeries(
  seriesA: PricePoint[],
  seriesB: PricePoint[],
): [number[], number[]] {
  const aligned: [number, number][] = [];
  let bIdx = 0;

  for (const a of seriesA) {
    while (bIdx + 1 < seriesB.length && (seriesB[bIdx + 1]?.timestamp ?? 0) <= a.timestamp) {
      bIdx++;
    }
    const b = seriesB[bIdx];
    if (b) {
      aligned.push([a.price, b.price]);
    }
  }

  const xs = aligned.map(([x]) => x);
  const ys = aligned.map(([, y]) => y);
  return [xs, ys];
}

// ─── Price history provider ──────────────────────────────────────────────────
// In production this would query asset_price_history via a DB client.
// Here we generate realistic synthetic data seeded per asset so results are
// deterministic and the math is exercisable in tests.

function seededRandom(seed: number): () => number {
  let s = seed;
  return () => {
    s = (s * 1664525 + 1013904223) & 0xffffffff;
    return ((s >>> 0) / 0xffffffff);
  };
}

const ASSET_BASE_PRICES: Record<string, number> = {
  XLM: 0.12,
  USDC: 1.0,
  BTC: 45000,
  ETH: 2500,
  AQUA: 0.001,
  yXLM: 0.13,
};

function getAssetSeed(asset: string): number {
  return [...asset].reduce((acc, c) => acc * 31 + c.charCodeAt(0), 1);
}

function generatePriceHistory(asset: string, windowDays: number): PricePoint[] {
  const basePrice = ASSET_BASE_PRICES[asset] ?? 1.0;
  const rng = seededRandom(getAssetSeed(asset));
  const points: PricePoint[] = [];
  const now = Date.now();
  const intervalMs = 60 * 60 * 1000; // hourly
  const count = windowDays * 24;
  let price = basePrice;

  for (let i = count; i >= 0; i--) {
    // Geometric Brownian Motion step (simplified)
    const drift = 0.0001;
    const vol = 0.02 * (rng() + 0.5);
    price = price * Math.exp(drift + vol * (rng() - 0.5));
    price = Math.max(price, basePrice * 0.01); // floor
    points.push({ timestamp: now - i * intervalMs, price });
  }
  return points;
}

// ─── Service ─────────────────────────────────────────────────────────────────

class CorrelationMatrixService {
  private alertConfig: CorrelationAlertConfig = {
    threshold: DEFAULT_CORRELATION_THRESHOLD,
    windowDays: 30,
  };

  // In-memory store of supported assets (real impl: pull from contract/DB)
  private supportedAssets: string[] = Object.keys(ASSET_BASE_PRICES);

  // ── Price data access ──────────────────────────────────────────────────────

  /**
   * Returns price history for `asset` over `windowDays`.
   * Production: replace with SELECT ... FROM asset_price_history.
   */
  private getPriceHistory(asset: string, windowDays: number): PricePoint[] {
    return generatePriceHistory(asset, windowDays);
  }

  // ── Core computation ───────────────────────────────────────────────────────

  /**
   * Compute Pearson + Spearman for a single asset pair over a given window.
   */
  async computePairCorrelation(
    assetA: string,
    assetB: string,
    windowDays: CorrelationWindow,
  ): Promise<AssetPairCorrelation> {
    const cacheKey = redisCacheService.buildKey(
      'price',
      `corr:${[assetA, assetB].sort().join(':')}:${windowDays}d`,
    );

    const cached = await redisCacheService.get<AssetPairCorrelation>(cacheKey);
    if (cached) return cached;

    const seriesA = this.getPriceHistory(assetA, windowDays);
    const seriesB = this.getPriceHistory(assetB, windowDays);

    let pearsonVal: number;
    let spearmanVal: number;
    let sampleCount: number;

    if (seriesA.length < MIN_SAMPLES || seriesB.length < MIN_SAMPLES) {
      // Implied correlation for new assets with limited history
      logger.warn('Insufficient price history, using implied correlation', { assetA, assetB, windowDays });
      pearsonVal = 0.5;
      spearmanVal = 0.5;
      sampleCount = Math.min(seriesA.length, seriesB.length);
    } else {
      const [xs, ys] = alignPriceSeries(seriesA, seriesB);
      pearsonVal = xs.length >= 2 ? pearson(xs, ys) : 0.5;
      spearmanVal = xs.length >= 2 ? spearman(xs, ys) : 0.5;
      sampleCount = xs.length;
    }

    const result: AssetPairCorrelation = {
      assetA,
      assetB,
      windowDays,
      pearson: Math.round(pearsonVal * 10000) / 10000,
      spearman: Math.round(spearmanVal * 10000) / 10000,
      sampleCount,
      computedAt: new Date().toISOString(),
      isHighlyCorrelated: Math.abs(pearsonVal) > this.alertConfig.threshold,
    };

    await redisCacheService.set(cacheKey, result, PAIR_CACHE_TTL_S);
    return result;
  }

  /**
   * Build the full N×N correlation matrix for all supported assets.
   */
  async getFullMatrix(windowDays: CorrelationWindow = 30): Promise<CorrelationMatrix> {
    const cacheKey = redisCacheService.buildKey('price', `corr-matrix:${windowDays}d`);
    const cached = await redisCacheService.get<CorrelationMatrix>(cacheKey);
    if (cached) return cached;

    const assets = this.supportedAssets;
    const matrix: Record<string, Record<string, number>> = {};
    const spearmanMatrix: Record<string, Record<string, number>> = {};
    const highCorrelationPairs: Array<{ assetA: string; assetB: string; pearson: number }> = [];

    for (const a of assets) {
      matrix[a] = {};
      spearmanMatrix[a] = {};
      for (const b of assets) {
        if (a === b) {
          matrix[a]![b] = 1;
          spearmanMatrix[a]![b] = 1;
          continue;
        }
        if ((matrix[b]?.[a] ?? undefined) !== undefined) {
          // Already computed, reuse symmetric value
          matrix[a]![b] = matrix[b]![a] as number;
          spearmanMatrix[a]![b] = spearmanMatrix[b]![a] as number;
          continue;
        }
        const pair = await this.computePairCorrelation(a, b, windowDays);
        matrix[a]![b] = pair.pearson;
        spearmanMatrix[a]![b] = pair.spearman;
        if (pair.isHighlyCorrelated) {
          highCorrelationPairs.push({ assetA: a, assetB: b, pearson: pair.pearson });
        }
      }
    }

    const result: CorrelationMatrix = {
      assets,
      windowDays,
      matrix,
      spearmanMatrix,
      computedAt: new Date().toISOString(),
      highCorrelationPairs,
    };

    await redisCacheService.set(cacheKey, result, MATRIX_CACHE_TTL_S);
    return result;
  }

  /**
   * Get all three windows for a specific pair with historical trend.
   */
  async getPairCorrelationAllWindows(
    assetA: string,
    assetB: string,
  ): Promise<{
    pair: string;
    windows: AssetPairCorrelation[];
    trend: CorrelationHistoryPoint[];
  }> {
    const windows = await Promise.all(
      CORRELATION_WINDOWS.map((w) => this.computePairCorrelation(assetA, assetB, w)),
    );

    // Simulated historical trend (12 monthly snapshots)
    const trend: CorrelationHistoryPoint[] = Array.from({ length: 12 }, (_, i) => {
      const rng = seededRandom(getAssetSeed(assetA + assetB) + i);
      return {
        computedAt: new Date(Date.now() - i * 30 * 24 * 60 * 60 * 1000).toISOString(),
        pearson: Math.round((0.3 + rng() * 0.6) * 10000) / 10000,
        spearman: Math.round((0.3 + rng() * 0.6) * 10000) / 10000,
      };
    }).reverse();

    return { pair: `${assetA}/${assetB}`, windows, trend };
  }

  /**
   * Compute correlation-based health factor adjustment for a multi-collateral position.
   *
   * If a basket has highly correlated assets the health factor is penalised:
   *   adjustedHF = baseHF × (1 - avgHighCorrelationPenalty)
   *
   * The adjustment returned is a multiplier in [0.7, 1.0].
   */
  async getPositionCorrelationRisk(
    userAddress: string,
    collateralAssets: string[],
    windowDays: CorrelationWindow = 30,
  ): Promise<PositionCorrelationRisk> {
    if (collateralAssets.length < 2) {
      return {
        userAddress,
        collateralAssets,
        averageCorrelation: 0,
        maxPairCorrelation: 0,
        healthFactorAdjustment: 1.0,
        warnings: [],
      };
    }

    const pairs: number[] = [];
    const warnings: string[] = [];

    for (let i = 0; i < collateralAssets.length; i++) {
      for (let j = i + 1; j < collateralAssets.length; j++) {
        const a = collateralAssets[i] as string;
        const b = collateralAssets[j] as string;
        const pair = await this.computePairCorrelation(a, b, windowDays);
        pairs.push(pair.pearson);
        if (pair.isHighlyCorrelated) {
          warnings.push(
            `${a} and ${b} are highly correlated (ρ = ${pair.pearson.toFixed(3)}). ` +
            `Adding ${b} provides limited diversification benefit.`,
          );
        }
      }
    }

    const avgCorrelation = pairs.reduce((s, v) => s + v, 0) / pairs.length;
    const maxCorrelation = Math.max(...pairs);

    // Penalty: 5% per 0.1 above threshold, capped at 30%
    const excess = Math.max(0, avgCorrelation - this.alertConfig.threshold);
    const penalty = Math.min(0.3, excess * 0.5);
    const healthFactorAdjustment = Math.max(0.7, 1 - penalty);

    return {
      userAddress,
      collateralAssets,
      averageCorrelation: Math.round(avgCorrelation * 10000) / 10000,
      maxPairCorrelation: Math.round(maxCorrelation * 10000) / 10000,
      healthFactorAdjustment: Math.round(healthFactorAdjustment * 10000) / 10000,
      warnings,
    };
  }

  /**
   * Update alert threshold configuration.
   */
  updateAlertConfig(config: Partial<CorrelationAlertConfig>): void {
    this.alertConfig = { ...this.alertConfig, ...config };
    // Invalidate cached matrices so they are recomputed with new threshold
    void redisCacheService.delByPrefix('stellarlend:price:corr');
    logger.info('Correlation alert config updated', { config: this.alertConfig });
  }

  getAlertConfig(): CorrelationAlertConfig {
    return { ...this.alertConfig };
  }

  /**
   * Trigger hourly recalculation of the full matrix for all windows.
   * Called by the scheduled job in the index / cron setup.
   */
  async recalculateAll(): Promise<void> {
    logger.info('Starting correlation matrix recalculation');
    await Promise.all(
      CORRELATION_WINDOWS.map(async (w) => {
        await redisCacheService.del(
          redisCacheService.buildKey('price', `corr-matrix:${w}d`),
        );
        await this.getFullMatrix(w);
      }),
    );
    logger.info('Correlation matrix recalculation complete');
  }
}

export const correlationMatrixService = new CorrelationMatrixService();
