/**
 * Concentration Monitor Service — Issue #452
 *
 * Tracks position concentration across each asset pool and enforces limits
 * when thresholds are exceeded.
 *
 * Metrics computed:
 *   HHI  = Σ(share_i²) × 10000   [0 = perfectly distributed, 10000 = monopoly]
 *   Top-N = sum of N largest position shares
 *
 * Enforcement:
 *   soft cap → excess positions pay a fee multiplier (default 2×)
 *   hard cap → new deposits blocked for addresses exceeding the cap
 *
 * Graduated enforcement: hard cap threshold scales with TVL (larger TVL → tighter).
 */

import { redisCacheService } from '../redisCache.service';
import logger from '../../utils/logger';
import { v4 as uuidv4 } from 'uuid';
import {
  ConcentrationMetrics,
  ConcentrationAlert,
  ConcentrationConfig,
  ConcentrationHistoryPoint,
  ConcentrationDashboard,
} from '../../types/riskEngine';

const CONC_CACHE_TTL_S = 3600;

// ─── Synthetic on-chain position data ────────────────────────────────────────
// Production: query indexed on-chain event data per pool.

interface Position {
  address: string;
  value: number; // USD value
}

function seededRandom(seed: number): () => number {
  let s = seed;
  return () => {
    s = (s * 1664525 + 1013904223) & 0xffffffff;
    return (s >>> 0) / 0xffffffff;
  };
}

function generatePositions(asset: string, tvl: number): Position[] {
  const rng = seededRandom([...asset].reduce((a, c) => a * 31 + c.charCodeAt(0), 13));
  const count = Math.floor(50 + rng() * 150);
  const positions: Position[] = [];

  // One dominant whale
  positions.push({ address: `G${asset}WHALE0001`, value: tvl * (0.08 + rng() * 0.06) });

  // Several large holders
  for (let i = 1; i < 10; i++) {
    positions.push({ address: `G${asset}LARGE${i.toString().padStart(4, '0')}`, value: tvl * (0.01 + rng() * 0.04) });
  }

  // Many small holders
  for (let i = 10; i < count; i++) {
    positions.push({ address: `G${asset}SMALL${i.toString().padStart(4, '0')}`, value: tvl * (0.001 + rng() * 0.005) });
  }

  return positions;
}

const ASSET_TVL: Record<string, number> = {
  XLM: 5_000_000,
  USDC: 8_000_000,
  BTC: 12_000_000,
  ETH: 10_000_000,
  AQUA: 500_000,
  yXLM: 2_000_000,
};

// ─── HHI computation ─────────────────────────────────────────────────────────

/**
 * Compute HHI (scaled 0-10000) from an array of position values.
 */
function computeHHI(values: number[]): number {
  const total = values.reduce((s, v) => s + v, 0);
  if (total === 0) return 0;
  const hhi = values.reduce((s, v) => {
    const share = v / total;
    return s + share * share;
  }, 0);
  return Math.round(hhi * 10000);
}

// ─── Service ──────────────────────────────────────────────────────────────────

class ConcentrationMonitorService {
  private config: ConcentrationConfig = {
    maxSinglePositionPct: 10,
    softCapMultiplier: 2.0,
    hardCapEnabled: true,
  };

  // In-memory history (production: query concentration_snapshots table)
  private history = new Map<string, ConcentrationHistoryPoint[]>();

  // Active alerts (production: query concentration_alerts table)
  private activeAlerts: ConcentrationAlert[] = [];

  // ── Metrics ───────────────────────────────────────────────────────────────

  /**
   * Compute concentration metrics for a single asset pool.
   */
  async getConcentrationMetrics(asset: string): Promise<ConcentrationMetrics> {
    const cacheKey = redisCacheService.buildKey('pool', `conc:${asset}`);
    const cached = await redisCacheService.get<ConcentrationMetrics>(cacheKey);
    if (cached) return cached;

    const tvl = ASSET_TVL[asset] ?? 1_000_000;
    const positions = generatePositions(asset, tvl);
    const values = positions.map((p) => p.value).sort((a, b) => b - a);
    const total = values.reduce((s, v) => s + v, 0);

    const hhi = computeHHI(values);
    const top5Pct = values.slice(0, 5).reduce((s, v) => s + v, 0) / total * 100;
    const top10Pct = values.slice(0, 10).reduce((s, v) => s + v, 0) / total * 100;
    const largestPositionPct = ((values[0] ?? 0) / total) * 100;

    // Graduated hard cap: tighter for larger pools
    const hardCapThreshold = total > 5_000_000 ? this.config.maxSinglePositionPct * 0.8
      : this.config.maxSinglePositionPct;

    const metrics: ConcentrationMetrics = {
      asset,
      hhi,
      top5Pct: Math.round(top5Pct * 100) / 100,
      top10Pct: Math.round(top10Pct * 100) / 100,
      totalPositions: positions.length,
      tvl: total,
      largestPositionPct: Math.round(largestPositionPct * 100) / 100,
      snapshotAt: new Date().toISOString(),
    };

    await redisCacheService.set(cacheKey, metrics, CONC_CACHE_TTL_S);
    this.appendHistory(asset, metrics);
    this.checkAndEmitAlerts(asset, positions, total, hardCapThreshold);

    logger.debug('Concentration metrics computed', { asset, hhi, top5Pct });
    return metrics;
  }

  /**
   * Concentration metrics for all supported assets.
   */
  async getAllConcentrationMetrics(): Promise<ConcentrationMetrics[]> {
    const assets = Object.keys(ASSET_TVL);
    return Promise.all(assets.map((a) => this.getConcentrationMetrics(a)));
  }

  // ── Enforcement helpers ───────────────────────────────────────────────────

  /**
   * Returns the fee multiplier for a deposit.
   * 1.0 = normal fee; >1.0 = soft-cap surcharge.
   */
  async getDepositFeeMultiplier(asset: string, address: string, depositValue: number): Promise<number> {
    const tvl = ASSET_TVL[asset] ?? 1_000_000;
    const positions = generatePositions(asset, tvl);
    const existingPosition = positions.find((p) => p.address === address);
    const existingValue = existingPosition?.value ?? 0;
    const newValue = existingValue + depositValue;
    const total = positions.reduce((s, p) => s + p.value, 0) + depositValue;
    const sharePct = (newValue / total) * 100;

    if (sharePct > this.config.maxSinglePositionPct) {
      return this.config.softCapMultiplier;
    }
    return 1.0;
  }

  /**
   * Returns true if a new deposit should be blocked (hard cap).
   * Hard cap applies when hardCapEnabled AND the resulting position
   * would exceed the threshold.
   */
  async isDepositBlocked(asset: string, address: string, depositValue: number): Promise<boolean> {
    if (!this.config.hardCapEnabled) return false;
    const tvl = ASSET_TVL[asset] ?? 1_000_000;
    const positions = generatePositions(asset, tvl);
    const existingPosition = positions.find((p) => p.address === address);
    const existingValue = existingPosition?.value ?? 0;
    const newValue = existingValue + depositValue;
    const total = positions.reduce((s, p) => s + p.value, 0) + depositValue;

    // Graduated threshold
    const hardCapThreshold = total > 5_000_000
      ? this.config.maxSinglePositionPct * 0.8
      : this.config.maxSinglePositionPct;

    const sharePct = (newValue / total) * 100;
    return sharePct > hardCapThreshold * 1.5; // hard block at 1.5× the cap
  }

  // ── Alerts ────────────────────────────────────────────────────────────────

  private checkAndEmitAlerts(
    asset: string,
    positions: Position[],
    total: number,
    threshold: number,
  ): void {
    for (const pos of positions) {
      const pct = (pos.value / total) * 100;
      if (pct > threshold) {
        const enforcement: 'soft' | 'hard' = pct > threshold * 1.5 ? 'hard' : 'soft';
        const existing = this.activeAlerts.find(
          (a) => a.asset === asset && a.address === pos.address && !a.resolvedAt,
        );
        if (!existing) {
          this.activeAlerts.push({
            id: uuidv4(),
            asset,
            address: pos.address,
            positionPct: Math.round(pct * 100) / 100,
            thresholdPct: threshold,
            enforcement,
            alertedAt: new Date().toISOString(),
          });
          logger.warn('Concentration threshold exceeded', { asset, address: pos.address, pct, enforcement });
        }
      }
    }

    // Auto-resolve alerts for positions that are now below threshold
    for (const alert of this.activeAlerts) {
      if (alert.asset !== asset || alert.resolvedAt) continue;
      const pos = positions.find((p) => p.address === alert.address);
      if (!pos) {
        alert.resolvedAt = new Date().toISOString();
        continue;
      }
      const pct = (pos.value / total) * 100;
      if (pct <= threshold) {
        alert.resolvedAt = new Date().toISOString();
      }
    }
  }

  getActiveAlerts(asset?: string): ConcentrationAlert[] {
    const open = this.activeAlerts.filter((a) => !a.resolvedAt);
    return asset ? open.filter((a) => a.asset === asset) : open;
  }

  getAlertHistory(asset?: string): ConcentrationAlert[] {
    return asset ? this.activeAlerts.filter((a) => a.asset === asset) : [...this.activeAlerts];
  }

  // ── History ───────────────────────────────────────────────────────────────

  private appendHistory(asset: string, metrics: ConcentrationMetrics): void {
    const hist = this.history.get(asset) ?? [];
    hist.push({
      snapshotAt: metrics.snapshotAt,
      hhi: metrics.hhi,
      top5Pct: metrics.top5Pct,
      top10Pct: metrics.top10Pct,
      tvl: metrics.tvl,
    });
    if (hist.length > 90) hist.splice(0, hist.length - 90);
    this.history.set(asset, hist);
  }

  getHistory(asset: string): ConcentrationHistoryPoint[] {
    return this.history.get(asset) ?? [];
  }

  // ── Dashboard ─────────────────────────────────────────────────────────────

  async getDashboard(): Promise<ConcentrationDashboard> {
    const assets = await this.getAllConcentrationMetrics();
    const globalHhi = Math.round(assets.reduce((s, a) => s + a.hhi, 0) / assets.length);
    const recentAlerts = this.activeAlerts.slice(-20);
    const history = Array.from(this.history.values()).flat().slice(-20);

    return {
      assets,
      globalHhi,
      totalAlerts: this.activeAlerts.filter((a) => !a.resolvedAt).length,
      recentAlerts,
      history,
    };
  }

  // ── Config ────────────────────────────────────────────────────────────────

  updateConfig(cfg: Partial<ConcentrationConfig>): void {
    this.config = { ...this.config, ...cfg };
    void redisCacheService.delByPrefix('stellarlend:pool:conc:');
    logger.info('Concentration config updated', { config: this.config });
  }

  getConfig(): ConcentrationConfig {
    return { ...this.config };
  }

  // ── Scheduler ─────────────────────────────────────────────────────────────

  async recalculateAll(): Promise<void> {
    logger.info('Recalculating concentration metrics');
    await redisCacheService.delByPrefix('stellarlend:pool:conc:');
    await this.getAllConcentrationMetrics();
    logger.info('Concentration recalculation complete');
  }
}

export const concentrationMonitorService = new ConcentrationMonitorService();
