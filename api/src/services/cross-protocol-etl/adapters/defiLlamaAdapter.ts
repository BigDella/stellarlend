import axios from 'axios';
import { ProtocolAdapter, StandardizedProtocolMetrics } from '../types';

const DEFAULT_DEFILLAMA_YIELDS_URL = 'https://yields.llama.fi/pools';

interface DefiLlamaPool {
  chain: string;
  project: string;
  symbol: string;
  tvlUsd: number;
  apy: number | null;
}

interface DefiLlamaPoolsResponse {
  status: string;
  data: DefiLlamaPool[];
}

/** DefiLlama `project` slugs this adapter tracks, mapped to a display name. */
const TRACKED_PROJECTS: Record<string, string> = {
  'aave-v3': 'Aave V3',
  'compound-v3': 'Compound V3',
  'morpho-blue': 'Morpho Blue',
  spark: 'Spark',
  exactly: 'Exactly',
};

/**
 * Ingests peer-protocol lending metrics from DefiLlama's public yields API
 * (`GET /pools`, no API key required — see
 * https://defillama.com/docs/api). Filtered to `TRACKED_PROJECTS`
 * (Aave, Compound, Morpho, Spark) per the cross-protocol comparison scope.
 *
 * The public `/pools` endpoint only reports supply-side APY and TVL — it
 * does not expose per-pool borrow APY or utilization. Rather than fabricate
 * those fields, this adapter honestly reports `borrowApy: 0` and
 * `utilizationRate: 0` for DefiLlama-sourced rows; a future iteration could
 * layer in a borrow-APY-capable source (e.g. each protocol's own subgraph)
 * behind the same `ProtocolAdapter` interface without touching callers.
 */
export class DefiLlamaAdapter implements ProtocolAdapter {
  readonly protocolId = 'defillama';
  readonly displayName = 'DefiLlama-tracked peer protocols';

  constructor(private readonly baseUrl: string = DEFAULT_DEFILLAMA_YIELDS_URL) {}

  async fetchMetrics(): Promise<StandardizedProtocolMetrics[]> {
    const response = await axios.get<DefiLlamaPoolsResponse>(this.baseUrl, {
      timeout: 10_000,
    });

    const pools = response.data?.data ?? [];
    const fetchedAt = new Date().toISOString();

    return pools
      .filter((pool) => pool.project in TRACKED_PROJECTS)
      .map((pool) => ({
        protocol: pool.project,
        displayName: TRACKED_PROJECTS[pool.project]!,
        chain: pool.chain,
        asset: pool.symbol,
        supplyApy: (pool.apy ?? 0) / 100,
        borrowApy: 0,
        tvlUsd: pool.tvlUsd ?? 0,
        utilizationRate: 0,
        fetchedAt,
        source: 'defillama',
      }));
  }
}
