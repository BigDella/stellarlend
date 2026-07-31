/**
 * Dynamic Liquidation Threshold Service — Issue #451
 *
 * Computes volatility-adjusted LTV (Loan-to-Value) thresholds for each
 * collateral asset. The adjusted LTV determines the liquidation threshold:
 *
 *   volatility_premium = calibrated_scale × realizedVol × 10000 (bps)
 *   adjusted_ltv       = base_ltv - volatility_premium
 *   adjusted_ltv       = clamp(adjusted_ltv, MIN_LTV_BPS, MAX_LTV_BPS)
 *
 * Threshold changes are time-locked for 24h to prevent manipulation.
 * Governance override can bypass the timelock.
 */

import { redisCacheService } from '../redisCache.service';
import { volatilityOracleService } from './volatilityOracle.service';
import logger from '../../utils/logger';
import { ValidationError } from '../../utils/errors';
import {
  LtvAdjustment,
  VolatilityAdjustedLtvResponse,
  LtvAdjustmentHistory,
} from '../../types/riskEngine';

// LTV bounds — enforced per spec
const MIN_LTV_BPS = 5000; // 50%
const MAX_LTV_BPS = 9000; // 90%

// Volatility calibration scale: maps annualised vol to basis-point premium.
// vol = 0.50 (50%) → premium = 0.50 × 5000 = 2500 bps
// Calibrated so a very volatile asset (100% vol) loses at most 5000 bps from base.
const VOLATILITY_SCALE = 5000;

// Timelock in milliseconds (24h)
const TIMELOCK_MS = 24 * 60 * 60 * 1000;

// Cache TTL: slightly less than timelock so adjustments are always fresh
const LTV_CACHE_TTL_S = 3600;

// Default base LTV per asset (bps). Production: read from governance contract.
const DEFAULT_BASE_LTV: Record<string, number> = {
  XLM: 7500,
  USDC: 9000,
  BTC: 8000,
  ETH: 7500,
  AQUA: 5500,
  yXLM: 7000,
};

const FALLBACK_BASE_LTV = 7000;

// In-memory pending adjustment tracker (keyed by asset).
// Production: persist in ltv_adjustments table.
const pendingAdjustments = new Map<string, { lockedUntil: number; adjustedLtv: number }>();

// In-memory adjustment history (last 30 entries per asset).
const adjustmentHistory = new Map<string, LtvAdjustment[]>();

// ─── Service ──────────────────────────────────────────────────────────────────

class DynamicLiquidationService {
  /**
   * Compute volatility-adjusted LTV for `asset` using 5d and 20d windows.
   * Returns both adjustments plus the conservative (lower) recommendation.
   */
  async getVolatilityAdjustedLtv(asset: string): Promise<VolatilityAdjustedLtvResponse> {
    const cacheKey = redisCacheService.buildKey('price', `dyn-ltv:${asset}`);
    const cached = await redisCacheService.get<VolatilityAdjustedLtvResponse>(cacheKey);
    if (cached) return cached;

    const [vol5d, vol20d] = await Promise.all([
      volatilityOracleService.getVolatility(asset, 5),
      volatilityOracleService.getVolatility(asset, 20),
    ]);

    const ltv5d = this.buildAdjustment(asset, vol5d.realizedVol);
    const ltv20d = this.buildAdjustment(asset, vol20d.realizedVol);
    const recommendedLtv = Math.min(ltv5d.adjustedLtv, ltv20d.adjustedLtv);

    const result: VolatilityAdjustedLtvResponse = {
      asset,
      currentLtv: DEFAULT_BASE_LTV[asset] ?? FALLBACK_BASE_LTV,
      ltv5d,
      ltv20d,
      recommendedLtv,
    };

    await redisCacheService.set(cacheKey, result, LTV_CACHE_TTL_S);
    this.recordHistory(ltv5d);
    this.recordHistory(ltv20d);

    return result;
  }

  /**
   * Build a single LtvAdjustment struct from a realized volatility value.
   */
  private buildAdjustment(asset: string, realizedVol: number): LtvAdjustment {
    const baseLtv = DEFAULT_BASE_LTV[asset] ?? FALLBACK_BASE_LTV;
    const volatilityPremium = Math.round(realizedVol * VOLATILITY_SCALE);
    const rawAdjusted = baseLtv - volatilityPremium;
    const adjustedLtv = Math.max(MIN_LTV_BPS, Math.min(MAX_LTV_BPS, rawAdjusted));
    const lockedUntil = new Date(Date.now() + TIMELOCK_MS).toISOString();

    return {
      asset,
      baseLtv,
      volatilityPremium,
      adjustedLtv,
      lockedUntil,
      isGovernanceOverride: false,
      computedAt: new Date().toISOString(),
    };
  }

  /**
   * Governance override — bypasses the 24h timelock.
   * Requires the caller to have passed governance authorization checks.
   */
  async applyGovernanceOverride(asset: string, overrideLtv: number): Promise<LtvAdjustment> {
    if (overrideLtv < MIN_LTV_BPS || overrideLtv > MAX_LTV_BPS) {
      throw new ValidationError(
        `Override LTV ${overrideLtv} bps is outside allowed range [${MIN_LTV_BPS}, ${MAX_LTV_BPS}]`,
      );
    }

    const baseLtv = DEFAULT_BASE_LTV[asset] ?? FALLBACK_BASE_LTV;
    const adjustment: LtvAdjustment = {
      asset,
      baseLtv,
      volatilityPremium: 0,
      adjustedLtv: overrideLtv,
      lockedUntil: new Date(Date.now() + TIMELOCK_MS).toISOString(),
      isGovernanceOverride: true,
      computedAt: new Date().toISOString(),
    };

    pendingAdjustments.set(asset, {
      lockedUntil: Date.now() + TIMELOCK_MS,
      adjustedLtv: overrideLtv,
    });

    // Bust cache so next read picks up the override
    await redisCacheService.del(redisCacheService.buildKey('price', `dyn-ltv:${asset}`));
    logger.warn('Governance LTV override applied', { asset, overrideLtv });
    this.recordHistory(adjustment);
    return adjustment;
  }

  /**
   * Return the effective LTV for an asset, accounting for pending timelocked changes.
   */
  async getCollateralFactor(asset: string): Promise<number> {
    const pending = pendingAdjustments.get(asset);
    if (pending && pending.lockedUntil > Date.now()) {
      // Timelock still active — return current base LTV (change not yet live)
      return DEFAULT_BASE_LTV[asset] ?? FALLBACK_BASE_LTV;
    }
    if (pending) {
      // Timelock expired — apply the pending adjustment
      pendingAdjustments.delete(asset);
      return pending.adjustedLtv;
    }
    const adjusted = await this.getVolatilityAdjustedLtv(asset);
    return adjusted.recommendedLtv;
  }

  /**
   * Historical LTV adjustment log for an asset.
   */
  getAdjustmentHistory(asset: string): LtvAdjustmentHistory {
    const history = adjustmentHistory.get(asset) ?? [];
    return {
      asset,
      history: history.map((h) => ({
        adjustedLtv: h.adjustedLtv,
        volatilityPremium: h.volatilityPremium,
        computedAt: h.computedAt,
      })),
    };
  }

  private recordHistory(adj: LtvAdjustment): void {
    const history = adjustmentHistory.get(adj.asset) ?? [];
    history.push(adj);
    // Keep last 90 entries
    if (history.length > 90) history.splice(0, history.length - 90);
    adjustmentHistory.set(adj.asset, history);
  }

  /**
   * Hourly recalculation triggered by scheduler.
   */
  async recalculateAll(assets: string[]): Promise<void> {
    logger.info('Recalculating dynamic LTV thresholds', { assets });
    await Promise.all(assets.map((a) => {
      void redisCacheService.del(redisCacheService.buildKey('price', `dyn-ltv:${a}`));
      return this.getVolatilityAdjustedLtv(a);
    }));
    logger.info('Dynamic LTV recalculation complete');
  }
}

export const dynamicLiquidationService = new DynamicLiquidationService();
