/**
 * Analytics Dashboard Service  — Issue #795
 *
 * Provides the data-fetching layer for the real-time protocol analytics
 * dashboard. Aggregates metrics from the StellarService (contract calls)
 * and local time-series stores, with Redis caching for hot paths.
 *
 * All functions return typed dashboard DTOs defined in types/analytics.ts.
 */

import { StellarService } from './stellar.service';
import { redisCacheService } from './redisCache.service';
import {
  ProtocolMetricsSnapshot,
  UserMetricsDashboard,
  ActivityFeedEntry,
  MetricsHistoryPoint,
  TvlForecastPoint,
  MetricAlertConfig,
  TriggeredAlertRecord,
  CollateralRatioSnapshot,
  DashboardView,
} from '../types/analytics';

const CACHE_TTL_S = 15; // 15-second TTL for real-time panels
const HISTORY_CACHE_TTL_S = 60;

function ck(segment: string): string {
  return redisCacheService.buildKey('dashboard', segment);
}

// ─── Protocol Metrics ─────────────────────────────────────────────────────────

export async function getProtocolMetrics(): Promise<ProtocolMetricsSnapshot> {
  const cacheKey = ck('protocol-metrics');
  const cached = await redisCacheService.get<ProtocolMetricsSnapshot>(cacheKey);
  if (cached) return cached;

  const svc = new StellarService();
  const stats = await svc.getProtocolStats();

  const snapshot: ProtocolMetricsSnapshot = {
    totalValueLocked: String(stats.totalValueLocked ?? '0'),
    totalDeposits: String(stats.totalDeposits ?? '0'),
    totalBorrows: String(stats.totalBorrows ?? '0'),
    utilizationRateBps: Number(stats.utilizationRate ?? 0),
    averageBorrowRateBps: Number(stats.averageBorrowRate ?? 0),
    totalUsers: Number(stats.totalUsers ?? 0),
    totalTransactions: Number(stats.totalTransactions ?? 0),
    timestamp: new Date().toISOString(),
  };

  await redisCacheService.set(cacheKey, snapshot, CACHE_TTL_S);
  return snapshot;
}

// ─── User Metrics ─────────────────────────────────────────────────────────────

export async function getUserMetrics(userAddress: string): Promise<UserMetricsDashboard> {
  const cacheKey = ck(`user-metrics:${userAddress}`);
  const cached = await redisCacheService.get<UserMetricsDashboard>(cacheKey);
  if (cached) return cached;

  const svc = new StellarService();
  const [position, healthFactor] = await Promise.all([
    svc.getUserPosition(userAddress).catch(() => null),
    svc.getHealthFactor(userAddress).catch(() => null),
  ]);

  const metrics: UserMetricsDashboard = {
    userAddress,
    collateral: String(position?.collateral ?? '0'),
    debt: String(position?.debt ?? '0'),
    healthFactor: Number(healthFactor ?? 0),
    totalDeposits: String(position?.totalDeposits ?? '0'),
    totalBorrows: String(position?.totalBorrows ?? '0'),
    totalWithdrawals: String(position?.totalWithdrawals ?? '0'),
    totalRepayments: String(position?.totalRepayments ?? '0'),
    activityScore: Number(position?.activityScore ?? 0),
    riskLevel: Number(position?.riskLevel ?? 0),
    transactionCount: Number(position?.transactionCount ?? 0),
  };

  await redisCacheService.set(cacheKey, metrics, CACHE_TTL_S);
  return metrics;
}

// ─── Activity Feed ────────────────────────────────────────────────────────────

export async function getActivityFeed(
  limit = 20,
  offset = 0,
  userAddress?: string
): Promise<ActivityFeedEntry[]> {
  const cacheKey = ck(`activity-feed:${userAddress ?? 'all'}:${limit}:${offset}`);
  const cached = await redisCacheService.get<ActivityFeedEntry[]>(cacheKey);
  if (cached) return cached;

  const svc = new StellarService();
  const raw = userAddress
    ? await svc.getUserActivityFeed(userAddress, limit, offset).catch(() => [])
    : await svc.getRecentActivity(limit, offset).catch(() => []);

  const feed: ActivityFeedEntry[] = raw.map((entry: any) => ({
    userAddress: String(entry.user ?? ''),
    activityType: String(entry.activityType ?? entry.activity_type ?? ''),
    amount: String(entry.amount ?? '0'),
    asset: entry.asset ? String(entry.asset) : null,
    timestamp: entry.timestamp
      ? new Date(Number(entry.timestamp) * 1000).toISOString()
      : new Date().toISOString(),
  }));

  await redisCacheService.set(cacheKey, feed, CACHE_TTL_S);
  return feed;
}

// ─── Metrics History ──────────────────────────────────────────────────────────

export async function getMetricsHistory(): Promise<MetricsHistoryPoint[]> {
  const cacheKey = ck('metrics-history');
  const cached = await redisCacheService.get<MetricsHistoryPoint[]>(cacheKey);
  if (cached) return cached;

  const svc = new StellarService();
  const raw = await svc.getMetricsHistory().catch(() => []);

  const history: MetricsHistoryPoint[] = raw.map((snap: any) => ({
    timestamp: snap.timestamp
      ? new Date(Number(snap.timestamp) * 1000).toISOString()
      : new Date().toISOString(),
    totalValueLocked: String(snap.totalValueLocked ?? snap.total_value_locked ?? '0'),
    utilizationRateBps: Number(snap.utilizationRate ?? snap.utilization_rate ?? 0),
    averageBorrowRateBps: Number(snap.averageBorrowRate ?? snap.average_borrow_rate ?? 0),
  }));

  await redisCacheService.set(cacheKey, history, HISTORY_CACHE_TTL_S);
  return history;
}

// ─── TVL Forecast ─────────────────────────────────────────────────────────────

export async function getTvlForecast(periodsAhead: number): Promise<TvlForecastPoint[]> {
  const cacheKey = ck(`tvl-forecast:${periodsAhead}`);
  const cached = await redisCacheService.get<TvlForecastPoint[]>(cacheKey);
  if (cached) return cached;

  const svc = new StellarService();

  const results: TvlForecastPoint[] = [];
  for (let p = 1; p <= periodsAhead; p++) {
    const raw = await svc.forecastTvl(p).catch(() => null);
    results.push({
      periodIndex: p,
      forecastedTvl: raw !== null ? String(raw) : '0',
      periodsAhead: p,
    });
  }

  await redisCacheService.set(cacheKey, results, HISTORY_CACHE_TTL_S);
  return results;
}

// ─── Collateral Ratios ────────────────────────────────────────────────────────

export async function getCollateralRatioSnapshots(): Promise<CollateralRatioSnapshot[]> {
  const cacheKey = ck('collateral-ratios');
  const cached = await redisCacheService.get<CollateralRatioSnapshot[]>(cacheKey);
  if (cached) return cached;

  const svc = new StellarService();
  const raw = await svc.getCollateralRatioSnapshots().catch(() => []);

  const snapshots: CollateralRatioSnapshot[] = raw.map((snap: any) => ({
    asset: String(snap.asset ?? ''),
    currentRatioBps: Number(snap.currentRatio ?? snap.current_ratio ?? 0),
    requiredRatioBps: Number(snap.requiredRatio ?? snap.required_ratio ?? 0),
    healthFactor: Number(snap.healthFactor ?? snap.health_factor ?? 0),
    riskLevel: String(snap.riskLevel ?? snap.risk_level ?? 'safe') as CollateralRatioSnapshot['riskLevel'],
    collateralValue: String(snap.collateralValue ?? snap.collateral_value ?? '0'),
    debtValue: String(snap.debtValue ?? snap.debt_value ?? '0'),
    timestamp: snap.timestamp
      ? new Date(Number(snap.timestamp) * 1000).toISOString()
      : new Date().toISOString(),
  }));

  await redisCacheService.set(cacheKey, snapshots, CACHE_TTL_S);
  return snapshots;
}

// ─── Alerts ───────────────────────────────────────────────────────────────────

export async function getTriggeredAlerts(): Promise<TriggeredAlertRecord[]> {
  const cacheKey = ck('triggered-alerts');
  const cached = await redisCacheService.get<TriggeredAlertRecord[]>(cacheKey);
  if (cached) return cached;

  const svc = new StellarService();
  const raw = await svc.getTriggeredAlerts().catch(() => []);

  const alerts: TriggeredAlertRecord[] = raw.map((alert: any) => ({
    metric: String(alert.metric ?? ''),
    value: String(alert.value ?? '0'),
    threshold: String(alert.threshold ?? '0'),
    timestamp: alert.timestamp
      ? new Date(Number(alert.timestamp) * 1000).toISOString()
      : new Date().toISOString(),
  }));

  await redisCacheService.set(cacheKey, alerts, CACHE_TTL_S);
  return alerts;
}

// ─── Full Dashboard View ──────────────────────────────────────────────────────

/**
 * Aggregate all real-time dashboard panels in parallel.
 * This is the primary data source for the dashboard's initial page load.
 * Partial failures are handled gracefully — failed sub-requests degrade
 * to empty collections rather than breaking the whole response.
 */
export async function getDashboardView(): Promise<DashboardView> {
  const cacheKey = ck('dashboard-view');
  const cached = await redisCacheService.get<DashboardView>(cacheKey);
  if (cached) return cached;

  const [protocol, activityFeed, collateralRatios, alerts] = await Promise.all([
    getProtocolMetrics().catch(() => ({
      totalValueLocked: '0',
      totalDeposits: '0',
      totalBorrows: '0',
      utilizationRateBps: 0,
      averageBorrowRateBps: 0,
      totalUsers: 0,
      totalTransactions: 0,
      timestamp: new Date().toISOString(),
    } as ProtocolMetricsSnapshot)),
    getActivityFeed(20).catch(() => [] as ActivityFeedEntry[]),
    getCollateralRatioSnapshots().catch(() => [] as CollateralRatioSnapshot[]),
    getTriggeredAlerts().catch(() => [] as TriggeredAlertRecord[]),
  ]);

  const view: DashboardView = {
    protocol,
    activityFeed,
    collateralRatios,
    alerts,
    generatedAt: new Date().toISOString(),
  };

  await redisCacheService.set(cacheKey, view, CACHE_TTL_S);
  return view;
}

// ─── Risk Distribution ────────────────────────────────────────────────────────

export interface RiskDistribution {
  usersSampled: number;
  level1: number;
  level2: number;
  level3: number;
  level4: number;
  level5: number;
}

export async function getRiskDistribution(): Promise<RiskDistribution> {
  const cacheKey = ck('risk-distribution');
  const cached = await redisCacheService.get<RiskDistribution>(cacheKey);
  if (cached) return cached;

  const svc = new StellarService();
  const raw = await svc.getRiskDistribution().catch(() => null);

  const dist: RiskDistribution = {
    usersSampled: Number(raw?.usersSampled ?? raw?.users_sampled ?? 0),
    level1: Number(raw?.level1 ?? raw?.level_1 ?? 0),
    level2: Number(raw?.level2 ?? raw?.level_2 ?? 0),
    level3: Number(raw?.level3 ?? raw?.level_3 ?? 0),
    level4: Number(raw?.level4 ?? raw?.level_4 ?? 0),
    level5: Number(raw?.level5 ?? raw?.level_5 ?? 0),
  };

  await redisCacheService.set(cacheKey, dist, CACHE_TTL_S);
  return dist;
}

// ─── Volume Summary ───────────────────────────────────────────────────────────

export interface VolumeSummary {
  totalDepositVolume: string;
  totalBorrowVolume: string;
  totalWithdrawalVolume: string;
  totalRepaymentVolume: string;
  totalLiquidationVolume: string;
  entryCount: number;
}

export async function getVolumeSummary(): Promise<VolumeSummary> {
  const cacheKey = ck('volume-summary');
  const cached = await redisCacheService.get<VolumeSummary>(cacheKey);
  if (cached) return cached;

  const svc = new StellarService();
  const raw = await svc.getVolumeSummary().catch(() => null);

  const summary: VolumeSummary = {
    totalDepositVolume: String(raw?.totalDepositVolume ?? raw?.total_deposit_volume ?? '0'),
    totalBorrowVolume: String(raw?.totalBorrowVolume ?? raw?.total_borrow_volume ?? '0'),
    totalWithdrawalVolume: String(raw?.totalWithdrawalVolume ?? raw?.total_withdrawal_volume ?? '0'),
    totalRepaymentVolume: String(raw?.totalRepaymentVolume ?? raw?.total_repayment_volume ?? '0'),
    totalLiquidationVolume: String(raw?.totalLiquidationVolume ?? raw?.total_liquidation_volume ?? '0'),
    entryCount: Number(raw?.entryCount ?? raw?.entry_count ?? 0),
  };

  await redisCacheService.set(cacheKey, summary, CACHE_TTL_S);
  return summary;
}
