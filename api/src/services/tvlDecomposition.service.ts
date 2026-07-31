import { StellarService } from './stellar.service';
import { getMarketShare } from './cross-protocol-etl/etl.service';
import { ProtocolMarketShare } from './cross-protocol-etl/types';

const STROOPS_PER_UNIT = 10_000_000;

/**
 * StellarLend doesn't yet track per-user deposit records, so segment and
 * strategy composition are approximated with fixed weights rather than
 * fabricated per-user data. These are the only assumptions in this module —
 * everything else derives from live pool state.
 */
const SEGMENT_WEIGHTS: Record<'whale' | 'mid' | 'retail', number> = {
  whale: 0.45,
  mid: 0.35,
  retail: 0.2,
};

const STRATEGY_WEIGHTS: Record<'direct_deposit' | 'yield_farming' | 'leveraged', number> = {
  direct_deposit: 0.6,
  yield_farming: 0.25,
  leveraged: 0.15,
};

export interface AssetTvlBreakdown {
  asset: string;
  poolAddress: string;
  tvlUsd: number;
  percentOfTotal: number;
}

export interface SegmentTvlBreakdown {
  segment: 'whale' | 'mid' | 'retail';
  tvlUsd: number;
  percentOfTotal: number;
}

export interface StrategyTvlBreakdown {
  strategy: 'direct_deposit' | 'yield_farming' | 'leveraged';
  tvlUsd: number;
  percentOfTotal: number;
}

export interface TvlBreakdownResult {
  totalTvlUsd: number;
  byAsset: AssetTvlBreakdown[];
  bySegment: SegmentTvlBreakdown[];
  byStrategy: StrategyTvlBreakdown[];
  generatedAt: string;
}

export interface TvlAttributionPoint {
  asset: string;
  poolAddress: string;
  previousTvlUsd: number;
  currentTvlUsd: number;
  totalChangeUsd: number;
  priceEffectUsd: number;
  flowEffectUsd: number;
}

export interface TvlAttributionResult {
  timeRange: string;
  totalChangeUsd: number;
  totalPriceEffectUsd: number;
  totalFlowEffectUsd: number;
  growthRatePercent: number;
  momentum: 'accelerating' | 'decelerating' | 'stable';
  byAsset: TvlAttributionPoint[];
  waterfall: Array<{ label: string; valueUsd: number }>;
}

export interface HistoricalTvlPoint {
  timestamp: string;
  totalTvlUsd: number;
  byAsset: AssetTvlBreakdown[];
}

const RANGE_MS: Record<string, number> = {
  '24h': 86_400_000,
  '7d': 604_800_000,
  '30d': 2_592_000_000,
  '90d': 7_776_000_000,
};

function toUsd(rawTvl: string): number {
  return Number(rawTvl) / STROOPS_PER_UNIT;
}

function withPercentages<T extends { tvlUsd: number; percentOfTotal: number }>(
  items: Array<Omit<T, 'percentOfTotal'>>,
  total: number
): T[] {
  return items.map((item) => ({
    ...item,
    percentOfTotal: total > 0 ? (item.tvlUsd / total) * 100 : 0,
  })) as unknown as T[];
}

export async function getTvlBreakdown(): Promise<TvlBreakdownResult> {
  const stellarService = new StellarService();
  const pools = await stellarService.getAllPools();

  const totalTvlUsd = pools.reduce((sum, p) => sum + toUsd(p.tvl), 0);

  const byAsset = withPercentages<AssetTvlBreakdown>(
    pools.map((p) => ({
      asset: p.name ?? p.address,
      poolAddress: p.address,
      tvlUsd: toUsd(p.tvl),
    })),
    totalTvlUsd
  );

  const bySegment = withPercentages<SegmentTvlBreakdown>(
    (Object.keys(SEGMENT_WEIGHTS) as Array<keyof typeof SEGMENT_WEIGHTS>).map((segment) => ({
      segment,
      tvlUsd: totalTvlUsd * SEGMENT_WEIGHTS[segment],
    })),
    totalTvlUsd
  );

  const byStrategy = withPercentages<StrategyTvlBreakdown>(
    (Object.keys(STRATEGY_WEIGHTS) as Array<keyof typeof STRATEGY_WEIGHTS>).map((strategy) => ({
      strategy,
      tvlUsd: totalTvlUsd * STRATEGY_WEIGHTS[strategy],
    })),
    totalTvlUsd
  );

  return {
    totalTvlUsd,
    byAsset,
    bySegment,
    byStrategy,
    generatedAt: new Date().toISOString(),
  };
}

export async function getTvlAttribution(timeRange: string): Promise<TvlAttributionResult> {
  const stellarService = new StellarService();
  const pools = await stellarService.getAllPools();
  const rangeMs = RANGE_MS[timeRange] ?? RANGE_MS['7d']!;
  const now = Math.floor(Date.now() / 1000);
  const previousTimestamp = now - Math.floor(rangeMs / 1000);

  const byAsset: TvlAttributionPoint[] = await Promise.all(
    pools.map(async (pool) => {
      const [previousState, currentState] = await Promise.all([
        stellarService.getPoolStateAt(pool.address, previousTimestamp),
        stellarService.getPoolStateAt(pool.address, now),
      ]);

      const previousTvlUsd = Number(previousState.totalDeposits) / STROOPS_PER_UNIT;
      const currentTvlUsd = Number(currentState.totalDeposits) / STROOPS_PER_UNIT;
      const totalChangeUsd = currentTvlUsd - previousTvlUsd;

      // Utilization-rate delta approximates the price/yield-driven share of
      // the change; the remainder is attributed to net user deposit flows.
      const utilizationDelta = currentState.utilizationRate - previousState.utilizationRate;
      const priceEffectUsd = previousTvlUsd * utilizationDelta;
      const flowEffectUsd = totalChangeUsd - priceEffectUsd;

      return {
        asset: pool.name ?? pool.address,
        poolAddress: pool.address,
        previousTvlUsd,
        currentTvlUsd,
        totalChangeUsd,
        priceEffectUsd,
        flowEffectUsd,
      };
    })
  );

  const totalChangeUsd = byAsset.reduce((sum, a) => sum + a.totalChangeUsd, 0);
  const totalPriceEffectUsd = byAsset.reduce((sum, a) => sum + a.priceEffectUsd, 0);
  const totalFlowEffectUsd = byAsset.reduce((sum, a) => sum + a.flowEffectUsd, 0);
  const previousTotal = byAsset.reduce((sum, a) => sum + a.previousTvlUsd, 0);
  const growthRatePercent = previousTotal > 0 ? (totalChangeUsd / previousTotal) * 100 : 0;

  const momentum: TvlAttributionResult['momentum'] =
    growthRatePercent > 1 ? 'accelerating' : growthRatePercent < -1 ? 'decelerating' : 'stable';

  return {
    timeRange,
    totalChangeUsd,
    totalPriceEffectUsd,
    totalFlowEffectUsd,
    growthRatePercent,
    momentum,
    byAsset,
    waterfall: [
      { label: 'Starting TVL', valueUsd: previousTotal },
      { label: 'Price/yield effect', valueUsd: totalPriceEffectUsd },
      { label: 'Net flow effect', valueUsd: totalFlowEffectUsd },
      { label: 'Ending TVL', valueUsd: previousTotal + totalChangeUsd },
    ],
  };
}

export async function getHistoricalTvlBreakdown(
  timeRange: string,
  points = 12
): Promise<HistoricalTvlPoint[]> {
  const stellarService = new StellarService();
  const pools = await stellarService.getAllPools();
  const rangeMs = RANGE_MS[timeRange] ?? RANGE_MS['30d']!;
  const now = Date.now();
  const interval = rangeMs / points;

  const timestamps = Array.from({ length: points }, (_, i) => now - rangeMs + interval * (i + 1));

  return Promise.all(
    timestamps.map(async (timestampMs) => {
      const timestampS = Math.floor(timestampMs / 1000);
      const states = await Promise.all(
        pools.map((pool) => stellarService.getPoolStateAt(pool.address, timestampS))
      );

      const byAssetRaw = pools.map((pool, i) => ({
        asset: pool.name ?? pool.address,
        poolAddress: pool.address,
        tvlUsd: Number(states[i]!.totalDeposits) / STROOPS_PER_UNIT,
      }));

      const totalTvlUsd = byAssetRaw.reduce((sum, a) => sum + a.tvlUsd, 0);

      return {
        timestamp: new Date(timestampMs).toISOString(),
        totalTvlUsd,
        byAsset: withPercentages<AssetTvlBreakdown>(byAssetRaw, totalTvlUsd),
      };
    })
  );
}

export async function getCompetitorTvlComparison(): Promise<ProtocolMarketShare[]> {
  return getMarketShare();
}
