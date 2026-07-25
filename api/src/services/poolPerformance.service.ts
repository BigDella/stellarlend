import { StellarService } from './stellar.service';
import { redisCacheService } from './redisCache.service';

const CACHE_TTL_S = 60;

interface PoolSnapshot {
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

interface PoolPerformanceMetrics {
  poolAddress: string;
  period: '7d' | '30d' | '90d' | '1y';
  avgSupplyApy: number;
  avgBorrowApy: number;
  avgUtilization: number;
  volatility: number;
  cumulativeReturn: number;
  maxDrawdown: number;
  sharpeRatio: number;
}

interface PoolComparison {
  poolAddress: string;
  poolName: string;
  currentApy: number;
  tvl: number;
  utilization: number;
  riskScore: number;
  rank: number;
}

interface PerformanceExportData {
  exportedAt: string;
  snapshots: PoolSnapshot[];
  metrics: PoolPerformanceMetrics[];
  comparison: PoolComparison[];
}

const RANGES_MS: Record<string, number> = {
  '7d': 604_800_000,
  '30d': 2_592_000_000,
  '90d': 7_776_000_000,
  '1y': 31_536_000_000,
};

function standardDeviation(values: number[]): number {
  if (values.length === 0) return 0;
  const mean = values.reduce((sum, v) => sum + v, 0) / values.length;
  const variance = values.reduce((sum, v) => sum + (v - mean) ** 2, 0) / values.length;
  return Math.sqrt(variance);
}

function calculateSharpeRatio(returns: number[], riskFreeRate: number): number {
  if (returns.length === 0) return 0;
  const avgReturn = returns.reduce((sum, r) => sum + r, 0) / returns.length;
  const stdDev = standardDeviation(returns);
  if (stdDev === 0) return 0;
  return (avgReturn - riskFreeRate) / stdDev;
}

function calculateMaxDrawdown(returns: number[]): number {
  if (returns.length === 0) return 0;
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

function generateTimePoints(timeRange: string, count: number): number[] {
  const now = Date.now();
  const range = RANGES_MS[timeRange] || 86_400_000;
  const interval = range / count;
  return Array.from({ length: count }, (_, i) => now - range + interval * i);
}

function toPeriodEnum(timeRange: string): '7d' | '30d' | '90d' | '1y' {
  if (timeRange === '7d' || timeRange === '30d' || timeRange === '90d' || timeRange === '1y')
    return timeRange;
  return '30d';
}

function computeRiskScore(snapshots: PoolSnapshot[]): number {
  const utilizations = snapshots.map((s) => s.utilizationRate);
  const sd = standardDeviation(utilizations);
  const avgUtil = utilizations.length > 0 ? utilizations.reduce((a, b) => a + b, 0) / utilizations.length : 0;
  const badDebtRisk = snapshots.length > 0 ? snapshots[snapshots.length - 1]!.badDebt / 1_000_000 : 0;
  const raw = sd * 0.5 + avgUtil * 0.3 + badDebtRisk * 100_000 * 0.2;
  return Math.min(100, Math.round(raw * 100) / 100);
}

export async function getPoolSnapshots(
  poolAddress: string,
  timeRange: string
): Promise<PoolSnapshot[]> {
  const cacheKey = redisCacheService.buildKey(
    'pool-performance',
    `snapshots:${poolAddress}:${timeRange}`
  );

  const cached = await redisCacheService.get<PoolSnapshot[]>(cacheKey);
  if (cached) return cached;

  const stellarService = new StellarService();
  const count = timeRange === '7d' ? 168 : timeRange === '30d' ? 720 : timeRange === '90d' ? 2160 : 8760;
  const sampleCount = Math.min(count, 200);
  const timePoints = generateTimePoints(timeRange, sampleCount);

  const snapshots: PoolSnapshot[] = await Promise.all(
    timePoints.map(async (timestamp) => {
      const poolData = await stellarService.getPoolStateAt(
        poolAddress,
        Math.floor(timestamp / 1000)
      );
      return {
        poolAddress,
        timestamp: new Date(timestamp).toISOString(),
        tvl: poolData.totalDeposits - (poolData.totalDeposits * 0.1),
        utilizationRate: poolData.utilizationRate,
        borrowApy: poolData.borrowApy,
        supplyApy: poolData.depositApy,
        badDebt: poolData.totalDeposits * 0.001,
        totalDeposits: poolData.totalDeposits,
        totalBorrows: poolData.totalBorrows,
      };
    })
  );

  await redisCacheService.set(cacheKey, snapshots, CACHE_TTL_S);
  return snapshots;
}

export async function capturePoolSnapshot(poolAddress: string): Promise<PoolSnapshot> {
  const cacheKey = redisCacheService.buildKey(
    'pool-performance',
    `snapshot:latest:${poolAddress}`
  );

  const cached = await redisCacheService.get<PoolSnapshot>(cacheKey);
  if (cached) return cached;

  const stellarService = new StellarService();
  const now = Math.floor(Date.now() / 1000);
  const poolData = await stellarService.getPoolStateAt(poolAddress, now);

  const snapshot: PoolSnapshot = {
    poolAddress,
    timestamp: new Date().toISOString(),
    tvl: poolData.totalDeposits - (poolData.totalDeposits * 0.1),
    utilizationRate: poolData.utilizationRate,
    borrowApy: poolData.borrowApy,
    supplyApy: poolData.depositApy,
    badDebt: poolData.totalDeposits * 0.001,
    totalDeposits: poolData.totalDeposits,
    totalBorrows: poolData.totalBorrows,
  };

  await redisCacheService.set(cacheKey, snapshot, CACHE_TTL_S);
  return snapshot;
}

export async function getPoolMetrics(
  poolAddress: string,
  period: string
): Promise<PoolPerformanceMetrics> {
  const cacheKey = redisCacheService.buildKey(
    'pool-performance',
    `metrics:${poolAddress}:${period}`
  );

  const cached = await redisCacheService.get<PoolPerformanceMetrics>(cacheKey);
  if (cached) return cached;

  const snapshots = await getPoolSnapshots(poolAddress, period);

  const avgSupplyApy =
    snapshots.length > 0
      ? snapshots.reduce((sum, s) => sum + s.supplyApy, 0) / snapshots.length
      : 0;
  const avgBorrowApy =
    snapshots.length > 0
      ? snapshots.reduce((sum, s) => sum + s.borrowApy, 0) / snapshots.length
      : 0;
  const avgUtilization =
    snapshots.length > 0
      ? snapshots.reduce((sum, s) => sum + s.utilizationRate, 0) / snapshots.length
      : 0;

  const supplyApyValues = snapshots.map((s) => s.supplyApy);
  const volatility = standardDeviation(supplyApyValues);

  const returns: number[] = [];
  for (let i = 1; i < snapshots.length; i++) {
    const prev = snapshots[i - 1]!.supplyApy;
    const curr = snapshots[i]!.supplyApy;
    if (prev > 0) {
      returns.push((curr - prev) / prev);
    }
  }

  const cumulativeReturn =
    returns.length > 0 ? returns.reduce((prod, r) => prod * (1 + r), 1) - 1 : 0;
  const maxDrawdown = calculateMaxDrawdown(returns);
  const sharpeRatio = calculateSharpeRatio(returns, 0.05 / 365);

  const metrics: PoolPerformanceMetrics = {
    poolAddress,
    period: toPeriodEnum(period),
    avgSupplyApy: Math.round(avgSupplyApy * 1_000_000) / 1_000_000,
    avgBorrowApy: Math.round(avgBorrowApy * 1_000_000) / 1_000_000,
    avgUtilization: Math.round(avgUtilization * 1_000_000) / 1_000_000,
    volatility: Math.round(volatility * 1_000_000) / 1_000_000,
    cumulativeReturn: Math.round(cumulativeReturn * 1_000_000) / 1_000_000,
    maxDrawdown: Math.round(maxDrawdown * 1_000_000) / 1_000_000,
    sharpeRatio: Math.round(sharpeRatio * 1_000_000) / 1_000_000,
  };

  await redisCacheService.set(cacheKey, metrics, CACHE_TTL_S);
  return metrics;
}

export async function comparePools(timeRange: string): Promise<PoolComparison[]> {
  const cacheKey = redisCacheService.buildKey(
    'pool-performance',
    `compare:${timeRange}`
  );

  const cached = await redisCacheService.get<PoolComparison[]>(cacheKey);
  if (cached) return cached;

  const stellarService = new StellarService();
  const pools = await stellarService.getAllPools();

  const comparisons: PoolComparison[] = await Promise.all(
    pools.map(async (pool) => {
      const snapshots = await getPoolSnapshots(pool.address, timeRange);
      const riskScore = computeRiskScore(snapshots);
      return {
        poolAddress: pool.address,
        poolName: pool.name,
        currentApy: pool.depositApy,
        tvl: pool.tvl,
        utilization: pool.utilizationRate,
        riskScore,
        rank: 0,
      };
    })
  );

  comparisons.sort((a, b) => {
    const scoreA = a.currentApy * 0.4 + a.tvl * 0.3 - a.riskScore * 0.3;
    const scoreB = b.currentApy * 0.4 + b.tvl * 0.3 - b.riskScore * 0.3;
    return scoreB - scoreA;
  });

  comparisons.forEach((c, i) => {
    c.rank = i + 1;
  });

  await redisCacheService.set(cacheKey, comparisons, CACHE_TTL_S);
  return comparisons;
}

export async function exportPerformanceData(
  poolAddress: string,
  format: 'csv' | 'json'
): Promise<PerformanceExportData | string> {
  const [snapshots, metrics, comparison] = await Promise.all([
    getPoolSnapshots(poolAddress, '30d'),
    getPoolMetrics(poolAddress, '30d'),
    comparePools('30d'),
  ]);

  const data: PerformanceExportData = {
    exportedAt: new Date().toISOString(),
    snapshots,
    metrics: [metrics],
    comparison,
  };

  if (format === 'csv') {
    return toCSV(data);
  }

  return data;
}

function toCSV(data: PerformanceExportData): string {
  const snapshotHeader = 'poolAddress,timestamp,tvl,utilizationRate,borrowApy,supplyApy,badDebt,totalDeposits,totalBorrows';
  const snapshotRows = data.snapshots.map(
    (s) =>
      `${s.poolAddress},${s.timestamp},${s.tvl},${s.utilizationRate},${s.borrowApy},${s.supplyApy},${s.badDebt},${s.totalDeposits},${s.totalBorrows}`
  );

  const metricsHeader = 'poolAddress,period,avgSupplyApy,avgBorrowApy,avgUtilization,volatility,cumulativeReturn,maxDrawdown,sharpeRatio';
  const metricsRows = data.metrics.map(
    (m) =>
      `${m.poolAddress},${m.period},${m.avgSupplyApy},${m.avgBorrowApy},${m.avgUtilization},${m.volatility},${m.cumulativeReturn},${m.maxDrawdown},${m.sharpeRatio}`
  );

  const compHeader = 'poolAddress,poolName,currentApy,tvl,utilization,riskScore,rank';
  const compRows = data.comparison.map(
    (c) =>
      `${c.poolAddress},${c.poolName},${c.currentApy},${c.tvl},${c.utilization},${c.riskScore},${c.rank}`
  );

  return [
    '# Snapshots',
    snapshotHeader,
    ...snapshotRows,
    '',
    '# Metrics',
    metricsHeader,
    ...metricsRows,
    '',
    '# Comparison',
    compHeader,
    ...compRows,
  ].join('\n');
}

export async function getPerformanceSummary(): Promise<{
  totalPoolsTracked: number;
  avgGlobalApy: number;
  totalTvl: number;
}> {
  const cacheKey = redisCacheService.buildKey('pool-performance', 'summary');

  const cached = await redisCacheService.get<{
    totalPoolsTracked: number;
    avgGlobalApy: number;
    totalTvl: number;
  }>(cacheKey);
  if (cached) return cached;

  const stellarService = new StellarService();
  const [protocolStats, pools] = await Promise.all([
    stellarService.getProtocolStats(),
    stellarService.getAllPools(),
  ]);

  const avgGlobalApy =
    pools.length > 0
      ? pools.reduce((sum, p) => sum + p.depositApy, 0) / pools.length
      : 0;

  const summary = {
    totalPoolsTracked: pools.length,
    avgGlobalApy: Math.round(avgGlobalApy * 1_000_000) / 1_000_000,
    totalTvl: protocolStats.tvl,
  };

  await redisCacheService.set(cacheKey, summary, CACHE_TTL_S);
  return summary;
}
