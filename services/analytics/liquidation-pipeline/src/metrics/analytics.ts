import type {
  CollateralFrequency,
  LiquidationMetrics,
  ProfitabilityDistribution,
  TimeClusterBucket,
} from '../types.js';

function percentile(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0;
  const idx = Math.min(sorted.length - 1, Math.max(0, Math.ceil((p / 100) * sorted.length) - 1));
  return sorted[idx]!;
}

export function profitabilityDistribution(metrics: LiquidationMetrics[]): ProfitabilityDistribution {
  const profits = metrics.map((m) => m.netProfit).sort((a, b) => a - b);
  const mean =
    profits.length === 0 ? 0 : profits.reduce((a, b) => a + b, 0) / profits.length;
  const profitable = profits.filter((p) => p > 0).length;

  return {
    count: profits.length,
    meanProfit: mean,
    medianProfit: percentile(profits, 50),
    p25: percentile(profits, 25),
    p75: percentile(profits, 75),
    p95: percentile(profits, 95),
    profitableShare: profits.length === 0 ? 0 : profitable / profits.length,
  };
}

export function clusterByHour(metrics: LiquidationMetrics[]): TimeClusterBucket[] {
  const counts = new Array<number>(24).fill(0);
  for (const m of metrics) {
    counts[m.hourOfDay] = (counts[m.hourOfDay] ?? 0) + 1;
  }
  return counts.map((count, hour) => ({ key: `hour-${hour}`, count: count ?? 0 }));
}

const DAY_NAMES = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

export function clusterByDayOfWeek(metrics: LiquidationMetrics[]): TimeClusterBucket[] {
  const counts = new Array<number>(7).fill(0);
  for (const m of metrics) {
    counts[m.dayOfWeek] = (counts[m.dayOfWeek] ?? 0) + 1;
  }
  return counts.map((count, day) => ({
    key: DAY_NAMES[day] ?? `day-${day}`,
    count: count ?? 0,
  }));
}

export function collateralFrequency(metrics: LiquidationMetrics[]): CollateralFrequency[] {
  const map = new Map<string, number>();
  for (const m of metrics) {
    map.set(m.collateralAsset, (map.get(m.collateralAsset) ?? 0) + 1);
  }
  const total = metrics.length || 1;
  return [...map.entries()]
    .map(([asset, count]) => ({ asset, count, share: count / total }))
    .sort((a, b) => b.count - a.count);
}
