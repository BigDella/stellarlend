/**
 * Lending pool performance tracking (Issue #611).
 *
 * Captures hourly snapshots (TVL, utilization, borrow/supply APY, bad debt),
 * aggregates 7d/30d/90d/1y averages and volatility, tracks protocol events,
 * fills data gaps, ranks pools, benchmarks against Compound/Aave, and exports CSV.
 */

export type Period = '7d' | '30d' | '90d' | '1y';

export type PoolEventType = 'liquidation' | 'bad_debt' | 'parameter_change';

export interface PoolSnapshot {
  poolAddress: string;
  timestamp: string;
  tvl: number;
  utilizationRate: number;
  borrowApy: number;
  supplyApy: number;
  badDebt: number;
  totalDeposits: number;
  totalBorrows: number;
}

export interface PoolPerformanceMetrics {
  poolAddress: string;
  period: Period;
  avgSupplyApy: number;
  avgBorrowApy: number;
  avgUtilization: number;
  volatility: number;
  cumulativeReturn: number;
  maxDrawdown: number;
  sharpeRatio: number;
  sampleCount: number;
  gapFilledPoints: number;
  historyTooShort: boolean;
}

export interface PoolComparison {
  poolAddress: string;
  poolName: string;
  currentApy: number;
  tvl: number;
  utilization: number;
  riskScore: number;
  rank: number;
}

export interface PoolPerformanceEvent {
  id: string;
  poolAddress: string;
  type: PoolEventType;
  timestamp: string;
  payload: Record<string, number | string>;
}

export interface DefiBenchmark {
  name: 'compound' | 'aave';
  supplyApy: number;
  borrowApy: number;
  utilization: number;
}

export interface BenchmarkComparison {
  poolAddress: string;
  period: Period;
  poolSupplyApy: number;
  poolBorrowApy: number;
  benchmarks: Array<DefiBenchmark & { supplyApyDelta: number; borrowApyDelta: number }>;
}

export interface ChartSeries {
  timestamp: string;
  cumulativeReturn: number;
  supplyApy: number;
  borrowApy: number;
  utilization: number;
  tvl: number;
}

export interface UtilizationHeatmapCell {
  day: number;
  hour: number;
  utilization: number;
}

const RANGES_MS: Record<Period, number> = {
  '7d': 604_800_000,
  '30d': 2_592_000_000,
  '90d': 7_776_000_000,
  '1y': 31_536_000_000,
};

const HOUR_MS = 3_600_000;

const COMPOUND_BENCHMARK: DefiBenchmark = {
  name: 'compound',
  supplyApy: 0.038,
  borrowApy: 0.055,
  utilization: 0.72,
};

const AAVE_BENCHMARK: DefiBenchmark = {
  name: 'aave',
  supplyApy: 0.041,
  borrowApy: 0.058,
  utilization: 0.68,
};

let snapshotStore: PoolSnapshot[] = [];
let eventStore: PoolPerformanceEvent[] = [];
let eventSeq = 0;

export function resetPoolPerformanceStore(): void {
  snapshotStore = [];
  eventStore = [];
  eventSeq = 0;
}

function round6(n: number): number {
  return Math.round(n * 1_000_000) / 1_000_000;
}

function toPeriod(value: string): Period {
  if (value === '7d' || value === '30d' || value === '90d' || value === '1y') return value;
  return '30d';
}

function standardDeviation(values: number[]): number {
  if (values.length === 0) return 0;
  const mean = values.reduce((sum, v) => sum + v, 0) / values.length;
  const variance = values.reduce((sum, v) => sum + (v - mean) ** 2, 0) / values.length;
  return Math.sqrt(variance);
}

function maxDrawdown(returns: number[]): number {
  let peak = 0;
  let maxDd = 0;
  let cumulative = 0;
  for (const r of returns) {
    cumulative += r;
    if (cumulative > peak) peak = cumulative;
    const drawdown = peak - cumulative;
    if (drawdown > maxDd) maxDd = drawdown;
  }
  return maxDd;
}

function sharpe(returns: number[], riskFreePerPeriod: number): number {
  if (returns.length === 0) return 0;
  const avg = returns.reduce((s, r) => s + r, 0) / returns.length;
  const sd = standardDeviation(returns);
  if (sd === 0) return 0;
  return (avg - riskFreePerPeriod) / sd;
}

function parseNumeric(value: number | string | undefined, fallback = 0): number {
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  if (typeof value === 'string') {
    const n = Number(value);
    return Number.isFinite(n) ? n : fallback;
  }
  return fallback;
}

export function recordSnapshot(snapshot: PoolSnapshot): PoolSnapshot {
  snapshotStore.push(snapshot);
  snapshotStore.sort((a, b) => a.timestamp.localeCompare(b.timestamp));
  return snapshot;
}

export function recordEvent(
  poolAddress: string,
  type: PoolEventType,
  payload: Record<string, number | string> = {}
): PoolPerformanceEvent {
  eventSeq += 1;
  const event: PoolPerformanceEvent = {
    id: `evt_${eventSeq}`,
    poolAddress,
    type,
    timestamp: new Date().toISOString(),
    payload,
  };
  eventStore.push(event);
  return event;
}

export function listEvents(poolAddress?: string): PoolPerformanceEvent[] {
  return eventStore.filter((e) => !poolAddress || e.poolAddress === poolAddress);
}

/**
 * Linear-interpolate missing hourly points caused by indexer/network downtime.
 * New pools with a single observation are left as-is (historyTooShort).
 */
export function fillSnapshotGaps(snapshots: PoolSnapshot[]): { filled: PoolSnapshot[]; gapFilledPoints: number } {
  if (snapshots.length < 2) {
    return { filled: [...snapshots], gapFilledPoints: 0 };
  }
  const sorted = [...snapshots].sort((a, b) => a.timestamp.localeCompare(b.timestamp));
  const filled: PoolSnapshot[] = [sorted[0]!];
  let gapFilledPoints = 0;

  for (let i = 1; i < sorted.length; i++) {
    const prev = sorted[i - 1]!;
    const next = sorted[i]!;
    const prevTs = Date.parse(prev.timestamp);
    const nextTs = Date.parse(next.timestamp);
    const gaps = Math.floor((nextTs - prevTs) / HOUR_MS) - 1;
    const interpolations = Math.min(Math.max(gaps, 0), 48);
    for (let g = 1; g <= interpolations; g++) {
      const t = g / (interpolations + 1);
      filled.push({
        poolAddress: prev.poolAddress,
        timestamp: new Date(prevTs + g * HOUR_MS).toISOString(),
        tvl: prev.tvl + (next.tvl - prev.tvl) * t,
        utilizationRate: prev.utilizationRate + (next.utilizationRate - prev.utilizationRate) * t,
        borrowApy: prev.borrowApy + (next.borrowApy - prev.borrowApy) * t,
        supplyApy: prev.supplyApy + (next.supplyApy - prev.supplyApy) * t,
        badDebt: prev.badDebt + (next.badDebt - prev.badDebt) * t,
        totalDeposits: prev.totalDeposits + (next.totalDeposits - prev.totalDeposits) * t,
        totalBorrows: prev.totalBorrows + (next.totalBorrows - prev.totalBorrows) * t,
      });
      gapFilledPoints += 1;
    }
    filled.push(next);
  }

  return { filled, gapFilledPoints };
}

export function snapshotsInPeriod(poolAddress: string, period: Period): PoolSnapshot[] {
  const cutoff = Date.now() - RANGES_MS[period];
  return snapshotStore.filter(
    (s) => s.poolAddress === poolAddress && Date.parse(s.timestamp) >= cutoff
  );
}

export function computeMetrics(poolAddress: string, periodInput: string): PoolPerformanceMetrics {
  const period = toPeriod(periodInput);
  const raw = snapshotsInPeriod(poolAddress, period);
  const { filled, gapFilledPoints } = fillSnapshotGaps(raw);
  const historyTooShort = filled.length < 24;

  const avg = (sel: (s: PoolSnapshot) => number) =>
    filled.length === 0 ? 0 : filled.reduce((sum, s) => sum + sel(s), 0) / filled.length;

  const avgSupplyApy = avg((s) => s.supplyApy);
  const avgBorrowApy = avg((s) => s.borrowApy);
  const avgUtilization = avg((s) => s.utilizationRate);
  const volatility = standardDeviation(filled.map((s) => s.supplyApy));

  const returns: number[] = [];
  for (let i = 1; i < filled.length; i++) {
    const hours = Math.max(
      (Date.parse(filled[i]!.timestamp) - Date.parse(filled[i - 1]!.timestamp)) / HOUR_MS,
      1
    );
    returns.push((filled[i - 1]!.supplyApy * hours) / (24 * 365));
  }
  const cumulativeReturn = returns.reduce((prod, r) => prod * (1 + r), 1) - 1;

  return {
    poolAddress,
    period,
    avgSupplyApy: round6(avgSupplyApy),
    avgBorrowApy: round6(avgBorrowApy),
    avgUtilization: round6(avgUtilization),
    volatility: round6(volatility),
    cumulativeReturn: round6(cumulativeReturn),
    maxDrawdown: round6(maxDrawdown(returns)),
    sharpeRatio: round6(sharpe(returns, 0.05 / (24 * 365))),
    sampleCount: filled.length,
    gapFilledPoints,
    historyTooShort,
  };
}

export function buildChartSeries(poolAddress: string, periodInput: string): ChartSeries[] {
  const period = toPeriod(periodInput);
  const { filled } = fillSnapshotGaps(snapshotsInPeriod(poolAddress, period));
  let cumulative = 1;
  const series: ChartSeries[] = [];
  for (let i = 0; i < filled.length; i++) {
    const s = filled[i]!;
    if (i > 0) {
      const hours = Math.max(
        (Date.parse(s.timestamp) - Date.parse(filled[i - 1]!.timestamp)) / HOUR_MS,
        1
      );
      cumulative *= 1 + (filled[i - 1]!.supplyApy * hours) / (24 * 365);
    }
    series.push({
      timestamp: s.timestamp,
      cumulativeReturn: round6(cumulative - 1),
      supplyApy: s.supplyApy,
      borrowApy: s.borrowApy,
      utilization: s.utilizationRate,
      tvl: s.tvl,
    });
  }
  return series;
}

export function buildUtilizationHeatmap(poolAddress: string, periodInput: string): UtilizationHeatmapCell[] {
  const { filled } = fillSnapshotGaps(snapshotsInPeriod(poolAddress, toPeriod(periodInput)));
  const buckets = new Map<string, { sum: number; n: number }>();
  for (const s of filled) {
    const d = new Date(s.timestamp);
    const key = `${d.getUTCDay()}:${d.getUTCHours()}`;
    const cur = buckets.get(key) ?? { sum: 0, n: 0 };
    cur.sum += s.utilizationRate;
    cur.n += 1;
    buckets.set(key, cur);
  }
  const cells: UtilizationHeatmapCell[] = [];
  for (let day = 0; day < 7; day++) {
    for (let hour = 0; hour < 24; hour++) {
      const cur = buckets.get(`${day}:${hour}`);
      cells.push({
        day,
        hour,
        utilization: cur ? round6(cur.sum / cur.n) : 0,
      });
    }
  }
  return cells;
}

export function computeRiskScore(snapshots: PoolSnapshot[]): number {
  const utilizations = snapshots.map((s) => s.utilizationRate);
  const sd = standardDeviation(utilizations);
  const avgUtil = utilizations.length > 0 ? utilizations.reduce((a, b) => a + b, 0) / utilizations.length : 0;
  const last = snapshots[snapshots.length - 1];
  const badDebtRisk = last && last.tvl > 0 ? last.badDebt / last.tvl : 0;
  return Math.min(100, Math.round((sd * 50 + avgUtil * 30 + badDebtRisk * 20) * 100) / 100);
}

export function rankPools(comparisons: PoolComparison[]): PoolComparison[] {
  const ranked = [...comparisons].sort((a, b) => {
    const scoreA = a.currentApy * 0.4 + Math.log10(Math.max(a.tvl, 1)) * 0.3 - a.riskScore * 0.3;
    const scoreB = b.currentApy * 0.4 + Math.log10(Math.max(b.tvl, 1)) * 0.3 - b.riskScore * 0.3;
    return scoreB - scoreA;
  });
  ranked.forEach((c, i) => {
    c.rank = i + 1;
  });
  return ranked;
}

export function benchmarkPool(poolAddress: string, periodInput: string): BenchmarkComparison {
  const metrics = computeMetrics(poolAddress, periodInput);
  const benchmarks = [COMPOUND_BENCHMARK, AAVE_BENCHMARK].map((b) => ({
    ...b,
    supplyApyDelta: round6(metrics.avgSupplyApy - b.supplyApy),
    borrowApyDelta: round6(metrics.avgBorrowApy - b.borrowApy),
  }));
  return {
    poolAddress,
    period: toPeriod(periodInput),
    poolSupplyApy: metrics.avgSupplyApy,
    poolBorrowApy: metrics.avgBorrowApy,
    benchmarks,
  };
}

export function snapshotsToCsv(snapshots: PoolSnapshot[]): string {
  const header = 'poolAddress,timestamp,tvl,utilizationRate,borrowApy,supplyApy,badDebt,totalDeposits,totalBorrows';
  const rows = snapshots.map(
    (s) =>
      `${s.poolAddress},${s.timestamp},${s.tvl},${s.utilizationRate},${s.borrowApy},${s.supplyApy},${s.badDebt},${s.totalDeposits},${s.totalBorrows}`
  );
  return [header, ...rows].join('\n');
}

export function snapshotFromPoolState(
  poolAddress: string,
  state: {
    utilizationRate?: number;
    totalDeposits?: number | string;
    totalBorrows?: number | string;
    borrowApy?: number;
    depositApy?: number;
    supplyApy?: number;
    badDebt?: number | string;
  }
): PoolSnapshot {
  const totalDeposits = parseNumeric(state.totalDeposits);
  const totalBorrows = parseNumeric(state.totalBorrows);
  const utilizationRate =
    typeof state.utilizationRate === 'number'
      ? state.utilizationRate
      : totalDeposits > 0
        ? totalBorrows / totalDeposits
        : 0;
  const borrowApy = state.borrowApy ?? 0.05;
  const supplyApy = state.supplyApy ?? state.depositApy ?? borrowApy * utilizationRate * 0.9;
  return {
    poolAddress,
    timestamp: new Date().toISOString(),
    tvl: totalDeposits,
    utilizationRate,
    borrowApy,
    supplyApy,
    badDebt: parseNumeric(state.badDebt),
    totalDeposits,
    totalBorrows,
  };
}

export function getAllSnapshots(poolAddress?: string): PoolSnapshot[] {
  return snapshotStore.filter((s) => !poolAddress || s.poolAddress === poolAddress);
}
