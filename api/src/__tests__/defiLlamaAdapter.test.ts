import axios from 'axios';
import { DefiLlamaAdapter } from '../services/cross-protocol-etl/adapters/defiLlamaAdapter';

jest.mock('axios');
const mockedAxios = axios as jest.Mocked<typeof axios>;

describe('DefiLlamaAdapter', () => {
  it('normalizes tracked-project pools into StandardizedProtocolMetrics', async () => {
    mockedAxios.get.mockResolvedValueOnce({
      data: {
        status: 'success',
        data: [
          { chain: 'Ethereum', project: 'aave-v3', symbol: 'USDC', tvlUsd: 500_000_000, apy: 3.2 },
          {
            chain: 'Ethereum',
            project: 'compound-v3',
            symbol: 'USDC',
            tvlUsd: 200_000_000,
            apy: 2.8,
          },
          // Untracked project must be filtered out.
          { chain: 'Ethereum', project: 'some-random-farm', symbol: 'USDC', tvlUsd: 10, apy: 999 },
        ],
      },
    });

    const adapter = new DefiLlamaAdapter();
    const metrics = await adapter.fetchMetrics();

    expect(metrics).toHaveLength(2);
    expect(metrics.map((m) => m.protocol).sort()).toEqual(['aave-v3', 'compound-v3']);

    const aave = metrics.find((m) => m.protocol === 'aave-v3')!;
    expect(aave.supplyApy).toBeCloseTo(0.032);
    expect(aave.tvlUsd).toBe(500_000_000);
    expect(aave.chain).toBe('Ethereum');
    expect(aave.source).toBe('defillama');
    // Honestly zeroed, not fabricated — see adapter doc comment.
    expect(aave.borrowApy).toBe(0);
    expect(aave.utilizationRate).toBe(0);
  });

  it('handles a null apy without throwing', async () => {
    mockedAxios.get.mockResolvedValueOnce({
      data: {
        status: 'success',
        data: [{ chain: 'Ethereum', project: 'spark', symbol: 'DAI', tvlUsd: 1000, apy: null }],
      },
    });

    const adapter = new DefiLlamaAdapter();
    const metrics = await adapter.fetchMetrics();
    expect(metrics[0]!.supplyApy).toBe(0);
  });

  it('propagates upstream errors so the ETL orchestrator can isolate them', async () => {
    mockedAxios.get.mockRejectedValueOnce(new Error('network error'));
    const adapter = new DefiLlamaAdapter();
    await expect(adapter.fetchMetrics()).rejects.toThrow('network error');
  });
});
