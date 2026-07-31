import { ProtocolAdapter, StandardizedProtocolMetrics } from '../types';
import { getRateComparison } from '../../analytics.service';

/**
 * Wraps StellarLend's own pool metrics as a `ProtocolAdapter` so it can be
 * compared against peer protocols through the same pipeline. Reuses
 * `getRateComparison` (the existing simulated pool-metrics source) rather
 * than duplicating pool-reading logic.
 */
export class StellarLendAdapter implements ProtocolAdapter {
  readonly protocolId = 'stellarlend';
  readonly displayName = 'StellarLend';

  async fetchMetrics(): Promise<StandardizedProtocolMetrics[]> {
    const pools = await getRateComparison();
    const fetchedAt = new Date().toISOString();

    return pools.map((pool) => ({
      protocol: this.protocolId,
      displayName: this.displayName,
      chain: 'stellar',
      asset: pool.poolName ?? pool.poolAddress,
      supplyApy: pool.depositApy,
      borrowApy: pool.borrowApy,
      // `tvl` is already treated as a USD-equivalent notional value
      // elsewhere in this analytics layer (see `getAnalyticsSummary`'s
      // `totalValueLocked`), so this reuses the same convention rather
      // than introducing a new one.
      tvlUsd: Number(pool.tvl) || 0,
      utilizationRate: pool.utilizationRate,
      fetchedAt,
      source: 'stellarlend-internal',
    }));
  }
}
