/**
 * Protocol Health Score Service — Issue #484
 *
 * Computes a single composite 0-100 health score from six weighted
 * component metrics, reusing existing simulated on-chain/oracle/governance
 * data sources rather than duplicating them:
 *
 *   - capitalEfficiency  → utilization rate vs. the optimal band (analytics.service)
 *   - liquidity          → available liquidity vs. total deposits per pool (stellar.service)
 *   - badDebt            → bad debt / total borrows across pools (stellar.service)
 *   - concentration      → HHI-based inverse measure (concentrationMonitor.service, #452)
 *   - oracleHealth       → price deviation, staleness, source diversity (riskMonitoring.service)
 *   - governanceHealth   → staker participation + voting power distribution (staking.service)
 */

import { redisCacheService } from '../redisCache.service';
import { StellarService } from '../stellar.service';
import { getAnalyticsSummary } from '../analytics.service';
import { concentrationMonitorService } from '../risk-engine/concentrationMonitor.service';
import { riskMonitoringService } from '../riskMonitoring.service';
import { stakingService } from '../staking.service';
import { ValidationError } from '../../utils/errors';
import logger from '../../utils/logger';
import {
  HealthScoreComponents,
  HealthScoreWeights,
  ProtocolHealthScore,
  HealthScoreHistoryPoint,
  HealthScoreAlert,
} from '../../types/protocolHealth';

const HEALTH_SCORE_CACHE_TTL_S = 300;
const MAX_HISTORY_POINTS = 90;

// Utilization band considered "efficient" — below it capital sits idle,
// above it the protocol has little buffer for withdrawals. Mirrors the
// optimal-utilization concept used by kinked interest-rate models.
const OPTIMAL_UTILIZATION_MIN = 0.7;
const OPTIMAL_UTILIZATION_MAX = 0.9;

// Below this average available-liquidity ratio, the liquidity score starts
// dropping from 100; 0% available liquidity scores 0.
const HEALTHY_LIQUIDITY_RATIO = 0.3;

// A 1% (100bps) protocol-wide bad-debt-to-borrows ratio is treated as the
// worst tolerable case (score 0); 0% scores 100.
const MAX_TOLERABLE_BAD_DEBT_RATIO = 0.01;

// Oracle thresholds, aligned with `riskMonitoringService`'s own alert config.
const ORACLE_STALENESS_ZERO_SCORE_S = 300;
const ORACLE_DEVIATION_ZERO_SCORE_BPS = 200;

// Number of independently-operated price sources this API's oracle
// currently ingests from (see `oracle/src/providers`: CoinGecko, Binance).
// Static, not live-fetched — provider count changes rarely and isn't
// exposed by the oracle service today.
const ORACLE_SOURCE_COUNT = 2;

// Soft target for "healthy" governance participation — reaching this many
// distinct stakers scores 100; fewer scales down linearly. Not derived from
// on-chain total supply (no such query exists in this API), so this is a
// deliberately conservative proxy rather than a literal participation rate.
const GOVERNANCE_PARTICIPATION_TARGET_STAKERS = 50;

const DEFAULT_WEIGHTS: HealthScoreWeights = {
  capitalEfficiency: 0.15,
  liquidity: 0.2,
  badDebt: 0.25,
  concentration: 0.15,
  oracleHealth: 0.15,
  governanceHealth: 0.1,
};

const DEFAULT_ALERT_THRESHOLD = 60;

function clampScore(value: number): number {
  return Math.max(0, Math.min(100, Math.round(value * 100) / 100));
}

/** Herfindahl-Hirschman-style inverse: HHI 0 → 100, HHI 10000 (monopoly) → 0. */
function hhiToScore(hhi: number): number {
  return clampScore(100 * (1 - hhi / 10000));
}

function computeHHI(shares: number[]): number {
  const total = shares.reduce((s, v) => s + v, 0);
  if (total === 0) return 0;
  return Math.round(shares.reduce((s, v) => s + (v / total) ** 2, 0) * 10000);
}

class ProtocolHealthScoreService {
  private stellarService = new StellarService();
  private weights: HealthScoreWeights = { ...DEFAULT_WEIGHTS };
  private alertThreshold = DEFAULT_ALERT_THRESHOLD;
  private history: HealthScoreHistoryPoint[] = [];

  // ── Component scores ────────────────────────────────────────────────────

  private async getCapitalEfficiencyScore(): Promise<number> {
    const summary = await getAnalyticsSummary();
    const utilization = summary.averageUtilizationRate;

    if (utilization >= OPTIMAL_UTILIZATION_MIN && utilization <= OPTIMAL_UTILIZATION_MAX) {
      return 100;
    }
    const distance =
      utilization < OPTIMAL_UTILIZATION_MIN
        ? OPTIMAL_UTILIZATION_MIN - utilization
        : utilization - OPTIMAL_UTILIZATION_MAX;
    // Fully out of band by 30 points of utilization (e.g. 40% or 120%) scores 0.
    return clampScore(100 - (distance / 0.3) * 100);
  }

  private async getLiquidityScore(): Promise<number> {
    const pools = await this.stellarService.getAllPools();
    if (pools.length === 0) return 100;
    const avgAvailableRatio = pools.reduce((sum, p) => sum + (1 - p.utilizationRate), 0) / pools.length;
    return clampScore((avgAvailableRatio / HEALTHY_LIQUIDITY_RATIO) * 100);
  }

  private async getBadDebtScore(): Promise<{ score: number; badDebtRatio: number }> {
    const pools = await this.stellarService.getAllPools();
    if (pools.length === 0) return { score: 100, badDebtRatio: 0 };

    let totalBadDebt = 0;
    let totalBorrows = 0;
    for (const pool of pools) {
      const deposits = Number(pool.tvl);
      const borrows = deposits * pool.utilizationRate;
      // Same 0.1%-of-deposits synthetic bad-debt convention used in
      // `poolPerformance.service.ts`, applied here consistently rather than
      // introducing a second, differing assumption.
      totalBadDebt += deposits * 0.001;
      totalBorrows += borrows;
    }

    const badDebtRatio = totalBorrows > 0 ? totalBadDebt / totalBorrows : 0;
    const score = clampScore(100 - (badDebtRatio / MAX_TOLERABLE_BAD_DEBT_RATIO) * 100);
    return { score, badDebtRatio };
  }

  private async getConcentrationScore(): Promise<number> {
    const dashboard = await concentrationMonitorService.getDashboard();
    return hhiToScore(dashboard.globalHhi);
  }

  private async getOracleHealthScore(): Promise<number> {
    const statuses = await riskMonitoringService.getOracleHealthStatus();
    if (statuses.length === 0) return 100;

    const perAssetScores = statuses.map((s) => {
      const freshness = clampScore(100 - (s.stalenessSeconds / ORACLE_STALENESS_ZERO_SCORE_S) * 100);
      const deviation = clampScore(100 - (s.deviationFromTwap / ORACLE_DEVIATION_ZERO_SCORE_BPS) * 100);
      return (freshness + deviation) / 2;
    });
    const avgAssetScore = perAssetScores.reduce((s, v) => s + v, 0) / perAssetScores.length;

    const diversityScore = clampScore((Math.min(ORACLE_SOURCE_COUNT, 3) / 3) * 100);
    return clampScore(avgAssetScore * 0.6 + diversityScore * 0.4);
  }

  private getGovernanceHealthScore(): number {
    const positions = stakingService.getAllPositions();
    if (positions.length === 0) {
      // Insufficient data rather than "bad" — neutral midpoint.
      return 50;
    }

    const participationScore = clampScore(
      (positions.length / GOVERNANCE_PARTICIPATION_TARGET_STAKERS) * 100
    );

    const votingPowerShares = positions.map((p) => Number(p.votingPower));
    const votingPowerHhi = computeHHI(votingPowerShares);
    const distributionScore = hhiToScore(votingPowerHhi);

    return clampScore((participationScore + distributionScore) / 2);
  }

  // ── Composite ────────────────────────────────────────────────────────────

  async getHealthScore(): Promise<ProtocolHealthScore> {
    const cacheKey = redisCacheService.buildKey('protocol', 'health-score');
    const cached = await redisCacheService.get<ProtocolHealthScore>(cacheKey);
    if (cached) return cached;

    const [capitalEfficiency, liquidity, badDebtResult, concentration, oracleHealth] = await Promise.all([
      this.getCapitalEfficiencyScore(),
      this.getLiquidityScore(),
      this.getBadDebtScore(),
      this.getConcentrationScore(),
      this.getOracleHealthScore(),
    ]);
    const governanceHealth = this.getGovernanceHealthScore();

    const components: HealthScoreComponents = {
      capitalEfficiency,
      liquidity,
      badDebt: badDebtResult.score,
      concentration,
      oracleHealth,
      governanceHealth,
    };

    const overallScore = clampScore(
      components.capitalEfficiency * this.weights.capitalEfficiency +
        components.liquidity * this.weights.liquidity +
        components.badDebt * this.weights.badDebt +
        components.concentration * this.weights.concentration +
        components.oracleHealth * this.weights.oracleHealth +
        components.governanceHealth * this.weights.governanceHealth
    );

    const result: ProtocolHealthScore = {
      overallScore,
      components,
      weights: { ...this.weights },
      timestamp: new Date().toISOString(),
    };

    await redisCacheService.set(cacheKey, result, HEALTH_SCORE_CACHE_TTL_S);
    this.appendHistory(result);
    logger.debug('Protocol health score computed', { overallScore });
    return result;
  }

  private appendHistory(result: ProtocolHealthScore): void {
    this.history.push({
      timestamp: result.timestamp,
      overallScore: result.overallScore,
      components: result.components,
    });
    if (this.history.length > MAX_HISTORY_POINTS) {
      this.history.splice(0, this.history.length - MAX_HISTORY_POINTS);
    }
  }

  getHistory(limit?: number): HealthScoreHistoryPoint[] {
    return limit ? this.history.slice(-limit) : [...this.history];
  }

  // ── Governance-controlled configuration ─────────────────────────────────

  getWeights(): HealthScoreWeights {
    return { ...this.weights };
  }

  updateWeights(partial: Partial<HealthScoreWeights>): HealthScoreWeights {
    const next = { ...this.weights, ...partial };
    const sum = Object.values(next).reduce((s, v) => s + v, 0);
    if (sum <= 0) {
      throw new ValidationError('Weights must sum to a positive value');
    }
    // Normalize so weights always sum to 1, regardless of what the caller passed in.
    const normalized = Object.fromEntries(
      Object.entries(next).map(([k, v]) => [k, v / sum])
    ) as unknown as HealthScoreWeights;
    this.weights = normalized;
    void redisCacheService.delByPrefix('stellarlend:protocol:health-score');
    logger.info('Protocol health score weights updated', { weights: this.weights });
    return this.getWeights();
  }

  getAlertThreshold(): number {
    return this.alertThreshold;
  }

  setAlertThreshold(threshold: number): void {
    if (threshold < 0 || threshold > 100) {
      throw new ValidationError('threshold must be between 0 and 100');
    }
    this.alertThreshold = threshold;
  }

  async getAlerts(): Promise<HealthScoreAlert[]> {
    const current = await this.getHealthScore();
    if (current.overallScore >= this.alertThreshold) return [];
    return [
      {
        threshold: this.alertThreshold,
        overallScore: current.overallScore,
        triggeredAt: current.timestamp,
        message: `Protocol health score ${current.overallScore} is below the configured threshold of ${this.alertThreshold}`,
      },
    ];
  }
}

export const protocolHealthScoreService = new ProtocolHealthScoreService();
