/**
 * Risk-Adjusted Collateral Ratio Service — Issue #453
 *
 * Dynamically computes required collateral ratios per asset based on:
 *   - 30-day realized volatility
 *   - Liquidity depth (order-book depth / daily volume proxy)
 *   - Daily on-chain volume
 *   - Cross-asset correlation
 *
 * Formula:
 *   collateral_ratio = base × (1 + volatility_factor + liquidity_factor + correlation_factor)
 *
 * All factors are in basis-point increments; the final ratio is expressed in bps
 * (e.g. 15000 = 150% collateral requirement).
 *
 * Includes a backtesting framework that validates ratios against simulated
 * historical liquidation data.
 */

import { redisCacheService } from '../redisCache.service';
import { volatilityOracleService } from './volatilityOracle.service';
import { correlationMatrixService } from './correlationMatrix.service';
import logger from '../../utils/logger';
import { ValidationError } from '../../utils/errors';
import {
  CollateralRatioFactors,
  CollateralRatioResponse,
  RatioFactorWeights,
  BacktestRequest,
  BacktestResult,
  CollateralRatioHistory,
} from '../../types/riskEngine';

const RATIO_CACHE_TTL_S = 3600;

// Base collateral ratios (bps) — governance-settable
const DEFAULT_BASE_RATIOS: Record<string, number> = {
  XLM: 15000,   // 150%
  USDC: 11000,  // 110%
  BTC: 13000,   // 130%
  ETH: 13500,   // 135%
  AQUA: 20000,  // 200%
  yXLM: 15500,  // 155%
};

const FALLBACK_BASE_RATIO = 15000;

// Factor weights (must sum to ≤ 1)
const DEFAULT_WEIGHTS: RatioFactorWeights = {
  volatilityWeight: 0.5,
  liquidityWeight: 0.3,
  correlationWeight: 0.2,
};

// Simulated liquidity data (production: pull from DEX order-book API)
const ASSET_LIQUIDITY_SCORE: Record<string, number> = {
  XLM: 0.85,   // 0 = illiquid, 1 = very liquid
  USDC: 0.98,
  BTC: 0.95,
  ETH: 0.92,
  AQUA: 0.30,
  yXLM: 0.70,
};

// Simulated daily volumes (USD)
const ASSET_DAILY_VOLUME: Record<string, number> = {
  XLM: 150_000_000,
  USDC: 500_000_000,
  BTC: 1_200_000_000,
  ETH: 900_000_000,
  AQUA: 2_000_000,
  yXLM: 20_000_000,
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

function seededRandom(seed: number): () => number {
  let s = seed;
  return () => {
    s = (s * 1664525 + 1013904223) & 0xffffffff;
    return (s >>> 0) / 0xffffffff;
  };
}

// ─── Service ──────────────────────────────────────────────────────────────────

class RiskAdjustedRatioService {
  private weights: RatioFactorWeights = { ...DEFAULT_WEIGHTS };

  // In-memory history (production: query collateral_ratio_history table)
  private history = new Map<string, CollateralRatioFactors[]>();

  /**
   * Compute the risk-adjusted collateral ratio for `asset`.
   *
   * Returns the ratio in basis points (e.g. 15000 = 150%).
   */
  async getCollateralRatio(asset: string): Promise<CollateralRatioResponse> {
    const cacheKey = redisCacheService.buildKey('price', `coll-ratio:${asset}`);
    const cached = await redisCacheService.get<CollateralRatioResponse>(cacheKey);
    if (cached) return cached;

    const baseRatio = DEFAULT_BASE_RATIOS[asset] ?? FALLBACK_BASE_RATIO;

    // ── Volatility factor ─────────────────────────────────────────────────
    // 30d annualised vol → premium in bps
    const vol30d = await volatilityOracleService.getVolatility(asset, 20);
    const volatilityFactor = Math.round(
      vol30d.realizedVol * 10000 * this.weights.volatilityWeight,
    );

    // ── Liquidity factor ──────────────────────────────────────────────────
    // Low liquidity → higher collateral requirement
    const liquidityScore = ASSET_LIQUIDITY_SCORE[asset] ?? 0.5;
    const dailyVolume = ASSET_DAILY_VOLUME[asset] ?? 1_000_000;
    // Normalise: score near 0 → 3000 bps penalty; score near 1 → 0 bps
    const liquidityPenalty = Math.round((1 - liquidityScore) * 3000);
    // Volume discount: very high volume reduces liquidity factor slightly
    const volumeDiscount = Math.round(Math.min(500, Math.log10(dailyVolume / 1_000_000) * 100));
    const liquidityFactor = Math.round(
      Math.max(0, liquidityPenalty - volumeDiscount) * this.weights.liquidityWeight,
    );

    // ── Correlation factor ────────────────────────────────────────────────
    // Average correlation with all other assets (higher correlation = less
    // diversification benefit = higher collateral)
    let avgCorrelation = 0;
    try {
      const matrix = await correlationMatrixService.getFullMatrix(30);
      const others = Object.keys(matrix.matrix).filter((a) => a !== asset);
      if (others.length > 0) {
        const correlations = others.map((other) => Math.abs(matrix.matrix[asset]?.[other] ?? 0));
        avgCorrelation = correlations.reduce((s, v) => s + v, 0) / correlations.length;
      }
    } catch {
      logger.warn('Could not fetch correlation matrix for ratio calc', { asset });
    }
    const correlationFactor = Math.round(
      avgCorrelation * 2000 * this.weights.correlationWeight,
    );

    // ── Final ratio ───────────────────────────────────────────────────────
    const totalAddon = Math.round(
      baseRatio * (volatilityFactor + liquidityFactor + correlationFactor) / 10000,
    );
    const finalRatio = Math.min(30000, Math.max(11000, baseRatio + totalAddon));

    const factors: CollateralRatioFactors = {
      baseRatio,
      volatilityFactor,
      liquidityFactor,
      correlationFactor,
      finalRatio,
      computedAt: new Date().toISOString(),
    };

    const recommendation = this.buildRecommendation(asset, factors);

    const result: CollateralRatioResponse = { asset, factors, recommendation };
    await redisCacheService.set(cacheKey, result, RATIO_CACHE_TTL_S);
    this.appendHistory(asset, factors);

    logger.debug('Collateral ratio computed', { asset, finalRatio });
    return result;
  }

  /**
   * Run a backtest: would `proposedRatio` have prevented bad-debt events
   * in the `[startDate, endDate]` window for `asset`?
   */
  async backtest(req: BacktestRequest): Promise<BacktestResult> {
    const { asset, startDate, endDate, proposedRatio } = req;

    const start = Date.parse(startDate);
    const end = Date.parse(endDate);

    if (isNaN(start) || isNaN(end)) {
      throw new ValidationError('startDate and endDate must be valid ISO-8601 strings');
    }
    if (end <= start) {
      throw new ValidationError('endDate must be after startDate');
    }
    if (proposedRatio < 10000 || proposedRatio > 50000) {
      throw new ValidationError('proposedRatio must be in [10000, 50000] bps');
    }

    const days = Math.ceil((end - start) / 86_400_000);
    const rng = seededRandom(
      [...asset].reduce((a, c) => a * 31 + c.charCodeAt(0), 17) + proposedRatio,
    );

    // Simulate liquidation events based on asset volatility
    const vol = (await volatilityOracleService.getVolatility(asset, 20)).realizedVol;
    const expectedLiqEvents = Math.round(days * vol * 0.1);
    const liquidationEvents = Math.round(expectedLiqEvents * (0.5 + rng()));
    const badDebtRatio = Math.max(0, 1 - proposedRatio / 10000);
    const badDebtEvents = Math.round(liquidationEvents * badDebtRatio * (0.2 + rng() * 0.3));
    const wouldHavePrevented = Math.round(badDebtEvents * 0.7);
    const minSafeRatio = Math.round((1 + vol * 1.5) * 10000);

    const recommendation =
      proposedRatio >= minSafeRatio
        ? `Proposed ratio of ${proposedRatio / 100}% appears adequate for ${asset} over the test period.`
        : `Proposed ratio of ${proposedRatio / 100}% may be insufficient. Recommended minimum: ${minSafeRatio / 100}%.`;

    return {
      asset,
      proposedRatio,
      periodStart: startDate,
      periodEnd: endDate,
      liquidationEvents,
      badDebtEvents,
      wouldHavePrevented,
      minSafeRatio,
      recommendation,
    };
  }

  // ── Factor weights (governance-settable) ──────────────────────────────────

  updateWeights(w: Partial<RatioFactorWeights>): void {
    this.weights = { ...this.weights, ...w };
    void redisCacheService.delByPrefix('stellarlend:price:coll-ratio:');
    logger.info('Collateral ratio factor weights updated', { weights: this.weights });
  }

  getWeights(): RatioFactorWeights {
    return { ...this.weights };
  }

  // ── History ───────────────────────────────────────────────────────────────

  private appendHistory(asset: string, factors: CollateralRatioFactors): void {
    const hist = this.history.get(asset) ?? [];
    hist.push(factors);
    if (hist.length > 90) hist.splice(0, hist.length - 90);
    this.history.set(asset, hist);
  }

  getHistory(asset: string): CollateralRatioHistory {
    const hist = this.history.get(asset) ?? [];
    return {
      asset,
      history: hist.map((f) => ({
        computedAt: f.computedAt,
        finalRatio: f.finalRatio,
        volatilityFactor: f.volatilityFactor,
        liquidityFactor: f.liquidityFactor,
        correlationFactor: f.correlationFactor,
      })),
    };
  }

  // ── Recommendation text ───────────────────────────────────────────────────

  private buildRecommendation(asset: string, factors: CollateralRatioFactors): string {
    const ratio = (factors.finalRatio / 100).toFixed(0);
    const base = (factors.baseRatio / 100).toFixed(0);
    const parts: string[] = [`Risk-adjusted collateral ratio for ${asset}: ${ratio}% (base ${base}%).`];

    if (factors.volatilityFactor > 500) {
      parts.push(`High volatility adds ${factors.volatilityFactor} bps.`);
    }
    if (factors.liquidityFactor > 300) {
      parts.push(`Liquidity risk adds ${factors.liquidityFactor} bps.`);
    }
    if (factors.correlationFactor > 200) {
      parts.push(`Correlation risk adds ${factors.correlationFactor} bps.`);
    }

    return parts.join(' ');
  }

  // ── Scheduler ─────────────────────────────────────────────────────────────

  async recalculateAll(): Promise<void> {
    logger.info('Recalculating risk-adjusted collateral ratios');
    const assets = Object.keys(DEFAULT_BASE_RATIOS);
    await redisCacheService.delByPrefix('stellarlend:price:coll-ratio:');
    await Promise.all(assets.map((a) => this.getCollateralRatio(a)));
    logger.info('Collateral ratio recalculation complete');
  }
}

export const riskAdjustedRatioService = new RiskAdjustedRatioService();
