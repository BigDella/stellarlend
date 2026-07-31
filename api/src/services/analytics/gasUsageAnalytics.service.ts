/**
 * Gas Usage Analytics Service — Issue #483
 *
 * Tracks observed gas usage per contract function over time and derives
 * optimization-focused analytics from it: percentile stats, anomaly
 * detection, function-to-function comparison, and calldata-size
 * correlation. Complements (does not replace) the pre-transaction cost
 * estimator in `services/gas/estimator.ts`, which forecasts cost *before*
 * a transaction runs; this service analyzes what functions *actually* cost
 * once executed.
 *
 * No live chain indexer exists in this API service, so — consistent with
 * `concentrationMonitor.service.ts` and `gas/estimator.ts` — the store is
 * seeded with realistic synthetic history on boot and grows from there as
 * real samples are recorded via `recordSample` (wired from
 * `gasEstimatorService.recordActualCost`).
 */

import logger from '../../utils/logger';
import {
  GasUsageSample,
  FunctionGasStats,
  GasAnomaly,
  GasUsageTrend,
  GasTrendPoint,
  FunctionComparison,
  CalldataCorrelation,
  FunctionGasReport,
} from '../../types/gasUsageAnalytics';

const MAX_SAMPLES_PER_FUNCTION = 2000;
const DEFAULT_ANOMALY_STD_DEV_THRESHOLD = 3;

const PERIOD_MS: Record<string, number> = {
  '24h': 24 * 60 * 60 * 1000,
  '7d': 7 * 24 * 60 * 60 * 1000,
  '30d': 30 * 24 * 60 * 60 * 1000,
};

/** Baseline observed gas usage (resource units) used to seed synthetic history. */
const SEED_BASELINE_GAS: Record<string, number> = {
  deposit: 354765,
  withdraw: 144093,
  borrow: 244830,
  repay: 430316,
  liquidation: 394438,
  flash_loan: 70030,
};

/** Baseline calldata size (bytes) used to seed a plausible size/gas correlation. */
const SEED_BASELINE_CALLDATA: Record<string, number> = {
  deposit: 220,
  withdraw: 180,
  borrow: 260,
  repay: 300,
  liquidation: 340,
  flash_loan: 160,
};

function seededRandom(seed: number): () => number {
  let s = seed;
  return () => {
    s = (s * 1664525 + 1013904223) & 0xffffffff;
    return (s >>> 0) / 0xffffffff;
  };
}

function mean(values: number[]): number {
  if (values.length === 0) return 0;
  return values.reduce((s, v) => s + v, 0) / values.length;
}

function median(values: number[]): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0 ? (sorted[mid - 1]! + sorted[mid]!) / 2 : sorted[mid]!;
}

function percentile(values: number[], p: number): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const index = Math.min(sorted.length - 1, Math.max(0, Math.ceil(p * sorted.length) - 1));
  return sorted[index]!;
}

function stdDeviation(values: number[]): number {
  if (values.length === 0) return 0;
  const m = mean(values);
  const variance = values.reduce((s, v) => s + (v - m) ** 2, 0) / values.length;
  return Math.sqrt(variance);
}

function pearsonCorrelation(xs: number[], ys: number[]): number {
  if (xs.length < 2 || xs.length !== ys.length) return 0;
  const mx = mean(xs);
  const my = mean(ys);
  let cov = 0;
  let varX = 0;
  let varY = 0;
  for (let i = 0; i < xs.length; i++) {
    const dx = xs[i]! - mx;
    const dy = ys[i]! - my;
    cov += dx * dy;
    varX += dx * dx;
    varY += dy * dy;
  }
  if (varX === 0 || varY === 0) return 0;
  return cov / Math.sqrt(varX * varY);
}

class GasUsageAnalyticsService {
  private samples = new Map<string, GasUsageSample[]>();

  constructor() {
    this.seedSyntheticHistory();
  }

  /** Seeds ~30 days of hourly synthetic samples per known lending function. */
  private seedSyntheticHistory(): void {
    const now = Date.now();
    const hourMs = 60 * 60 * 1000;

    for (const functionName of Object.keys(SEED_BASELINE_GAS)) {
      const rng = seededRandom([...functionName].reduce((a, c) => a * 31 + c.charCodeAt(0), 7));
      const baselineGas = SEED_BASELINE_GAS[functionName]!;
      const baselineCalldata = SEED_BASELINE_CALLDATA[functionName]!;
      const history: GasUsageSample[] = [];

      for (let i = 30 * 24; i >= 0; i--) {
        const variance = (rng() - 0.5) * 0.16; // +/-8%
        const gasUsed = Math.round(baselineGas * (1 + variance));
        // Calldata scales sub-linearly with gas usage plus noise, giving a
        // meaningful (but imperfect) positive correlation to detect.
        const calldataSize = Math.round(
          baselineCalldata * (1 + variance * 0.5) + (rng() - 0.5) * 20
        );
        history.push({
          functionName,
          gasUsed,
          calldataSize,
          timestamp: now - i * hourMs,
        });
      }

      this.samples.set(functionName, history);
    }
  }

  /** Records an observed gas usage sample for a contract function. */
  recordSample(sample: GasUsageSample): void {
    const list = this.samples.get(sample.functionName) ?? [];
    list.push(sample);
    if (list.length > MAX_SAMPLES_PER_FUNCTION) {
      list.splice(0, list.length - MAX_SAMPLES_PER_FUNCTION);
    }
    this.samples.set(sample.functionName, list);
    logger.debug('Gas usage sample recorded', {
      functionName: sample.functionName,
      gasUsed: sample.gasUsed,
    });
  }

  listFunctions(): string[] {
    return Array.from(this.samples.keys());
  }

  private samplesInPeriod(functionName: string, period: string): GasUsageSample[] {
    const windowMs = PERIOD_MS[period] ?? PERIOD_MS['30d']!;
    const cutoff = Date.now() - windowMs;
    return (this.samples.get(functionName) ?? []).filter((s) => s.timestamp >= cutoff);
  }

  getStats(functionName: string, period: string = '30d'): FunctionGasStats {
    const samples = this.samplesInPeriod(functionName, period);
    const values = samples.map((s) => s.gasUsed);

    return {
      functionName,
      sampleCount: values.length,
      average: Math.round(mean(values)),
      median: Math.round(median(values)),
      p95: Math.round(percentile(values, 0.95)),
      p99: Math.round(percentile(values, 0.99)),
      min: values.length ? Math.min(...values) : 0,
      max: values.length ? Math.max(...values) : 0,
      stdDeviation: Math.round(stdDeviation(values)),
      period,
    };
  }

  getAllStats(period: string = '30d'): FunctionGasStats[] {
    return this.listFunctions().map((fn) => this.getStats(fn, period));
  }

  /** Buckets samples into daily/weekly averages for trend charting. */
  getTrend(functionName: string, granularity: 'daily' | 'weekly' = 'daily', period: string = '30d'): GasUsageTrend {
    const samples = this.samplesInPeriod(functionName, period);
    const bucketMs = granularity === 'daily' ? 24 * 60 * 60 * 1000 : 7 * 24 * 60 * 60 * 1000;

    const buckets = new Map<number, number[]>();
    for (const sample of samples) {
      const bucketKey = Math.floor(sample.timestamp / bucketMs) * bucketMs;
      const bucket = buckets.get(bucketKey) ?? [];
      bucket.push(sample.gasUsed);
      buckets.set(bucketKey, bucket);
    }

    const points: GasTrendPoint[] = Array.from(buckets.entries())
      .sort(([a], [b]) => a - b)
      .map(([bucketKey, values]) => ({
        timestamp: new Date(bucketKey).toISOString(),
        average: Math.round(mean(values)),
        sampleCount: values.length,
      }));

    return { functionName, granularity, points };
  }

  /**
   * Flags samples whose gas usage deviates from the function's mean by more
   * than `stdDevThreshold` standard deviations (default 3σ).
   */
  detectAnomalies(period: string = '30d', stdDevThreshold: number = DEFAULT_ANOMALY_STD_DEV_THRESHOLD): GasAnomaly[] {
    const anomalies: GasAnomaly[] = [];

    for (const functionName of this.listFunctions()) {
      const samples = this.samplesInPeriod(functionName, period);
      const values = samples.map((s) => s.gasUsed);
      if (values.length < 2) continue;

      const m = mean(values);
      const sd = stdDeviation(values);
      if (sd === 0) continue;

      for (const sample of samples) {
        const deviations = Math.abs(sample.gasUsed - m) / sd;
        if (deviations > stdDevThreshold) {
          anomalies.push({
            functionName,
            gasUsed: sample.gasUsed,
            timestamp: sample.timestamp,
            txHash: sample.txHash,
            deviations: Math.round(deviations * 100) / 100,
            mean: Math.round(m),
            stdDeviation: Math.round(sd),
          });
        }
      }
    }

    return anomalies.sort((a, b) => b.deviations - a.deviations);
  }

  compareFunctions(functionA: string, functionB: string, period: string = '30d'): FunctionComparison {
    const statsA = this.getStats(functionA, period);
    const statsB = this.getStats(functionB, period);
    const averageDeltaPct = statsA.average === 0 ? 0 : ((statsB.average - statsA.average) / statsA.average) * 100;

    return {
      functionA: statsA,
      functionB: statsB,
      averageDeltaPct: Math.round(averageDeltaPct * 100) / 100,
    };
  }

  getCalldataCorrelation(functionName: string, period: string = '30d'): CalldataCorrelation {
    const samples = this.samplesInPeriod(functionName, period).filter(
      (s) => s.calldataSize !== undefined
    );
    const xs = samples.map((s) => s.calldataSize as number);
    const ys = samples.map((s) => s.gasUsed);

    return {
      functionName,
      sampleCount: samples.length,
      correlationCoefficient: Math.round(pearsonCorrelation(xs, ys) * 1000) / 1000,
    };
  }

  getFunctionReport(functionName: string, period: string = '30d'): FunctionGasReport {
    const stats = this.getStats(functionName, period);
    const trend = this.getTrend(functionName, 'daily', period);
    const anomalies = this.detectAnomalies(period).filter((a) => a.functionName === functionName);
    const calldataCorrelation = this.getCalldataCorrelation(functionName, period);

    let recommendation = 'Gas usage is stable; no optimization action needed.';
    if (anomalies.length > 0) {
      recommendation = `${anomalies.length} anomalous call(s) detected in this period — investigate for regressions or edge-case input sizes.`;
    } else if (calldataCorrelation.correlationCoefficient > 0.7) {
      recommendation = 'Gas usage strongly tracks calldata size — consider batching or payload compression to reduce cost.';
    } else if (trend.points.length >= 2) {
      const first = trend.points[0]!.average;
      const last = trend.points[trend.points.length - 1]!.average;
      if (first > 0 && (last - first) / first > 0.1) {
        recommendation = 'Gas usage trending upward over the period — worth profiling for regressions.';
      }
    }

    return { functionName, stats, trend, anomalies, calldataCorrelation, recommendation };
  }
}

export const gasUsageAnalyticsService = new GasUsageAnalyticsService();
