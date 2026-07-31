import type { AssetMetricSample, ProtocolMetricSample } from '../types.js';

export interface ProtocolStatsSource {
  fetchProtocolStats(): Promise<{
    tvl: number;
    totalBorrows: number;
    utilizationRate: number;
    liquidations?: number;
    totalDeposits: number;
    activeUsers?: number;
    assets?: Array<{
      asset: string;
      supply: number;
      borrow: number;
      availableLiquidity: number;
      price?: number;
      volatility?: number;
      apy?: number;
    }>;
  }>;
}

/**
 * HTTP source that samples the existing StellarLend protocol stats API.
 */
export class HttpProtocolStatsSource implements ProtocolStatsSource {
  constructor(private readonly url: string) {}

  async fetchProtocolStats() {
    const response = await fetch(this.url);
    if (!response.ok) {
      throw new Error(`Protocol stats request failed: ${response.status} ${response.statusText}`);
    }
    const body = (await response.json()) as Record<string, unknown>;
    const data = (body.data as Record<string, unknown> | undefined) ?? body;

    return {
      tvl: Number(data.tvl ?? data.TVL ?? 0),
      totalBorrows: Number(data.total_borrows ?? data.totalBorrows ?? 0),
      utilizationRate: Number(data.utilization_rate ?? data.utilizationRate ?? 0),
      liquidations: Number(data.liquidations ?? data.liquidation_count ?? 0),
      totalDeposits: Number(data.total_deposits ?? data.totalDeposits ?? 0),
      activeUsers: Number(data.active_users ?? data.activeUsers ?? data.users ?? 0),
      assets: Array.isArray(data.assets)
        ? (data.assets as Array<Record<string, unknown>>).map((asset) => ({
            asset: String(asset.asset ?? asset.symbol ?? 'UNKNOWN'),
            supply: Number(asset.supply ?? 0),
            borrow: Number(asset.borrow ?? asset.total_borrows ?? 0),
            availableLiquidity: Number(asset.available_liquidity ?? asset.availableLiquidity ?? 0),
            price: asset.price !== undefined ? Number(asset.price) : undefined,
            volatility: asset.volatility !== undefined ? Number(asset.volatility) : undefined,
            apy: asset.apy !== undefined ? Number(asset.apy) : undefined,
          }))
        : undefined,
    };
  }
}

export function toProtocolSample(
  stats: Awaited<ReturnType<ProtocolStatsSource['fetchProtocolStats']>>,
  time: Date = new Date()
): ProtocolMetricSample {
  return {
    time,
    tvl: stats.tvl,
    totalBorrows: stats.totalBorrows,
    utilizationRate: stats.utilizationRate,
    liquidations: stats.liquidations ?? 0,
    totalDeposits: stats.totalDeposits,
    activeUsers: stats.activeUsers ?? 0,
  };
}

export function toAssetSamples(
  stats: Awaited<ReturnType<ProtocolStatsSource['fetchProtocolStats']>>,
  time: Date = new Date()
): AssetMetricSample[] {
  return (stats.assets ?? []).map((asset) => ({
    time,
    asset: asset.asset,
    supply: asset.supply,
    borrow: asset.borrow,
    availableLiquidity: asset.availableLiquidity,
    price: asset.price ?? null,
    volatility: asset.volatility ?? null,
    apy: asset.apy ?? null,
  }));
}
