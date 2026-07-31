/**
 * Volatility Oracle Service — Issue #451 (supporting service)
 *
 * Computes rolling realized volatility for collateral assets using
 * 5-day and 20-day windows. Results feed into DynamicLiquidationService.
 *
 * Realized volatility formula (annualised):
 *   σ = stdDev(log-returns) × √(trading_days_per_year)
 *
 * Trading days approximation: 365 (crypto markets are 24/7).
 */

import { redisCacheService } from '../redisCache.service';
import logger from '../../utils/logger';
import { AssetVolatility, VolatilityWindow } from '../../types/riskEngine';

const TRADING_DAYS_PER_YEAR = 365;
const VOL_CACHE_TTL_S = 3600; // 1 hour
const VOLATILITY_WINDOWS: VolatilityWindow[] = [5, 20];

// ─── Synthetic price generator (mirrors correlationMatrix for consistency) ───

function seededRandom(seed: number): () => number {
  let s = seed;
  return () => {
    s = (s * 1664525 + 1013904223) & 0xffffffff;
    return (s >>> 0) / 0xffffffff;
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

function generateDailyPrices(asset: string, days: number): number[] {
  const base = ASSET_BASE_PRICES[asset] ?? 1.0;
  const rng = seededRandom([...asset].reduce((a, c) => a * 31 + c.charCodeAt(0), 7));
  const prices: number[] = [];
  let p = base;
  for (let i = 0; i < days + 1; i++) {
    const drift = 0.0002;
    const vol = 0.03 * (rng() + 0.4);
    p = p * Math.exp(drift + vol * (rng() - 0.5));
    prices.push(Math.max(p, base * 0.01));
  }
  return prices;
}

// ─── Math helpers ─────────────────────────────────────────────────────────────

function logReturns(prices: number[]): number[] {
  const returns: number[] = [];
  for (let i = 1; i < prices.length; i++) {
    const prev = prices[i - 1];
    const curr = prices[i];
    if (prev != null && curr != null && prev > 0) {
      returns.push(Math.log(curr / prev));
    }
  }
  return returns;
}

function stdDev(values: number[]): number {
  if (values.length === 0) return 0;
  const mean = values.reduce((s, v) => s + v, 0) / values.length;
  const variance = values.reduce((s, v) => s + (v - mean) ** 2, 0) / values.length;
  return Math.sqrt(variance);
}

function annualisedVol(dailyReturns: number[]): number {
  return stdDev(dailyReturns) * Math.sqrt(TRADING_DAYS_PER_YEAR);
}

// ─── Service ──────────────────────────────────────────────────────────────────

class VolatilityOracleService {
  /**
   * Compute realized annualised volatility for `asset` over `windowDays`.
   */
  async getVolatility(asset: string, windowDays: VolatilityWindow): Promise<AssetVolatility> {
    const cacheKey = redisCacheService.buildKey('price', `vol:${asset}:${windowDays}d`);
    const cached = await redisCacheService.get<AssetVolatility>(cacheKey);
    if (cached) return cached;

    // Production: query asset_price_history for last windowDays daily closing prices
    const prices = generateDailyPrices(asset, windowDays);
    const returns = logReturns(prices);
    const vol = annualisedVol(returns);

    const result: AssetVolatility = {
      asset,
      windowDays,
      realizedVol: Math.round(vol * 100000) / 100000, // 5dp
      computedAt: new Date().toISOString(),
    };

    await redisCacheService.set(cacheKey, result, VOL_CACHE_TTL_S);
    logger.debug('Volatility computed', { asset, windowDays, vol: result.realizedVol });
    return result;
  }

  /**
   * Get volatility for all supported assets and all windows.
   */
  async getAllVolatilities(): Promise<AssetVolatility[]> {
    const assets = Object.keys(ASSET_BASE_PRICES);
    const results: AssetVolatility[] = [];
    for (const asset of assets) {
      for (const w of VOLATILITY_WINDOWS) {
        results.push(await this.getVolatility(asset, w));
      }
    }
    return results;
  }

  /**
   * Hourly recalculation — invalidate caches then recompute.
   */
  async recalculateAll(): Promise<void> {
    logger.info('Recalculating volatilities');
    await redisCacheService.delByPrefix('stellarlend:price:vol:');
    await this.getAllVolatilities();
    logger.info('Volatility recalculation complete');
  }
}

export const volatilityOracleService = new VolatilityOracleService();
