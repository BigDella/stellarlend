import React, { useState, useEffect, useCallback } from 'react';

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
  period: string;
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

interface PerformanceState {
  selectedPool: string;
  period: string;
  snapshots: PoolSnapshot[];
  metrics: PoolPerformanceMetrics | null;
  comparison: PoolComparison[];
  chartSeries: { timestamp: string; cumulativeReturn: number; supplyApy: number; utilization: number }[];
  heatmap: { day: number; hour: number; utilization: number }[];
  benchmarks: { name: string; supplyApy: number; supplyApyDelta: number }[];
  summary: { totalPoolsTracked: number; avgGlobalApy: number; totalTvl: number } | null;
  isLoading: boolean;
  view: 'overview' | 'charts' | 'comparison' | 'heatmap' | 'benchmarks';
}

export const PerformanceDashboard: React.FC = () => {
  const [state, setState] = useState<PerformanceState>({
    selectedPool: '',
    period: '30d',
    snapshots: [],
    metrics: null,
    comparison: [],
    chartSeries: [],
    heatmap: [],
    benchmarks: [],
    summary: null,
    isLoading: true,
    view: 'overview',
  });

  const loadOverview = useCallback(async () => {
    setState(prev => ({ ...prev, isLoading: true }));
    try {
      const [summaryRes, comparisonRes] = await Promise.all([
        fetch('/api/pool-performance/summary'),
        fetch('/api/pool-performance/compare?period=30d'),
      ]);

      const summary = await summaryRes.json();
      const comparisonData = await comparisonRes.json();

      setState(prev => ({
        ...prev,
        summary,
        comparison: comparisonData.comparison || [],
        isLoading: false,
      }));
    } catch (err) {
      setState(prev => ({ ...prev, isLoading: false }));
    }
  }, []);

  const loadPoolMetrics = useCallback(async () => {
    if (!state.selectedPool) return;
    try {
      const [snapshotsRes, metricsRes, chartRes, heatRes, benchRes] = await Promise.all([
        fetch(`/api/pool-performance/snapshots/${state.selectedPool}?period=${state.period}`),
        fetch(`/api/pool-performance/metrics/${state.selectedPool}?period=${state.period}`),
        fetch(`/api/pool-performance/charts/${state.selectedPool}?period=${state.period}`),
        fetch(`/api/pool-performance/heatmap/${state.selectedPool}?period=${state.period}`),
        fetch(`/api/pool-performance/benchmarks/${state.selectedPool}?period=${state.period}`),
      ]);

      const snapshotsData = await snapshotsRes.json();
      const metricsData = await metricsRes.json();
      const chartData = await chartRes.json();
      const heatData = await heatRes.json();
      const benchData = await benchRes.json();

      setState(prev => ({
        ...prev,
        snapshots: snapshotsData.snapshots || snapshotsData || [],
        metrics: metricsData,
        chartSeries: Array.isArray(chartData) ? chartData : [],
        heatmap: Array.isArray(heatData) ? heatData : [],
        benchmarks: benchData.benchmarks || [],
      }));
    } catch (err) {
      console.error('Failed to load pool metrics:', err);
    }
  }, [state.selectedPool, state.period]);

  useEffect(() => {
    loadOverview();
  }, [loadOverview]);

  useEffect(() => {
    if (state.selectedPool) {
      loadPoolMetrics();
    }
  }, [loadPoolMetrics, state.selectedPool]);

  const handleExport = async () => {
    if (!state.selectedPool) return;
    try {
      const res = await fetch(
        `/api/pool-performance/export/${state.selectedPool}?format=csv`
      );
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `pool-performance-${state.selectedPool.slice(0, 8)}.csv`;
      a.click();
    } catch (err) {
      console.error('Export failed:', err);
    }
  };

  return (
    <div style={styles.container}>
      <h2 style={styles.title}>Pool Performance Tracking</h2>

      {state.summary && (
        <div style={styles.summaryGrid}>
          <div style={styles.summaryCard}>
            <div style={styles.summaryLabel}>Pools Tracked</div>
            <div style={styles.summaryValue}>{state.summary.totalPoolsTracked}</div>
          </div>
          <div style={styles.summaryCard}>
            <div style={styles.summaryLabel}>Avg Global APY</div>
            <div style={styles.summaryValue}>{state.summary.avgGlobalApy.toFixed(2)}%</div>
          </div>
          <div style={styles.summaryCard}>
            <div style={styles.summaryLabel}>Total TVL</div>
            <div style={styles.summaryValue}>
              ${(state.summary.totalTvl / 1_000_000).toFixed(2)}M
            </div>
          </div>
        </div>
      )}

      <div style={styles.tabBar}>
        {(['overview', 'charts', 'comparison', 'heatmap', 'benchmarks'] as const).map(v => (
          <button
            key={v}
            onClick={() => setState(prev => ({ ...prev, view: v }))}
            style={state.view === v ? styles.tabActive : styles.tab}
          >
            {v.charAt(0).toUpperCase() + v.slice(1)}
          </button>
        ))}
      </div>

      {state.view === 'overview' && (
        <div style={styles.section}>
          <h3 style={styles.sectionTitle}>Pool Comparison</h3>
          <table style={styles.table}>
            <thead>
              <tr>
                <th style={styles.th}>Rank</th>
                <th style={styles.th}>Pool</th>
                <th style={styles.th}>APY</th>
                <th style={styles.th}>TVL</th>
                <th style={styles.th}>Utilization</th>
                <th style={styles.th}>Risk Score</th>
              </tr>
            </thead>
            <tbody>
              {state.comparison.map(pool => (
                <tr
                  key={pool.poolAddress}
                  style={{
                    ...styles.tr,
                    cursor: 'pointer',
                    backgroundColor:
                      state.selectedPool === pool.poolAddress ? '#f0f7ff' : undefined,
                  }}
                  onClick={() =>
                    setState(prev => ({ ...prev, selectedPool: pool.poolAddress, view: 'charts' }))
                  }
                >
                  <td style={styles.td}>#{pool.rank}</td>
                  <td style={styles.td}>{pool.poolName}</td>
                  <td style={{ ...styles.td, color: '#28a745', fontWeight: 600 }}>
                    {pool.currentApy.toFixed(2)}%
                  </td>
                  <td style={styles.td}>${(pool.tvl / 1_000_000).toFixed(2)}M</td>
                  <td style={styles.td}>{(pool.utilization * 100).toFixed(1)}%</td>
                  <td style={styles.td}>
                    <span
                      style={{
                        ...styles.riskBadge,
                        backgroundColor:
                          pool.riskScore < 30
                            ? '#d4edda'
                            : pool.riskScore < 60
                            ? '#fff3cd'
                            : '#f8d7da',
                        color:
                          pool.riskScore < 30
                            ? '#155724'
                            : pool.riskScore < 60
                            ? '#856404'
                            : '#721c24',
                      }}
                    >
                      {pool.riskScore}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {state.view === 'charts' && (
        <div style={styles.section}>
          <div style={styles.chartHeader}>
            <div>
              <h3 style={styles.sectionTitle}>Pool Metrics</h3>
              {state.selectedPool && (
                <span style={styles.poolAddress}>{state.selectedPool.slice(0, 12)}...</span>
              )}
            </div>
            <div style={styles.chartControls}>
              <select
                value={state.period}
                onChange={e => setState(prev => ({ ...prev, period: e.target.value }))}
                style={styles.select}
              >
                <option value="7d">7 Days</option>
                <option value="30d">30 Days</option>
                <option value="90d">90 Days</option>
                <option value="1y">1 Year</option>
              </select>
              <button onClick={handleExport} style={styles.exportButton}>
                Export CSV
              </button>
            </div>
          </div>

          {state.metrics && (
            <div style={styles.metricsGrid}>
              <div style={styles.metricCard}>
                <div style={styles.metricLabel}>Avg Supply APY</div>
                <div style={styles.metricValue}>{state.metrics.avgSupplyApy.toFixed(2)}%</div>
              </div>
              <div style={styles.metricCard}>
                <div style={styles.metricLabel}>Avg Borrow APY</div>
                <div style={styles.metricValue}>{state.metrics.avgBorrowApy.toFixed(2)}%</div>
              </div>
              <div style={styles.metricCard}>
                <div style={styles.metricLabel}>Avg Utilization</div>
                <div style={styles.metricValue}>{(state.metrics.avgUtilization * 100).toFixed(1)}%</div>
              </div>
              <div style={styles.metricCard}>
                <div style={styles.metricLabel}>Volatility</div>
                <div style={styles.metricValue}>{(state.metrics.volatility * 100).toFixed(2)}%</div>
              </div>
              <div style={styles.metricCard}>
                <div style={styles.metricLabel}>Cumulative Return</div>
                <div style={{ ...styles.metricValue, color: '#28a745' }}>
                  {state.metrics.cumulativeReturn > 0 ? '+' : ''}
                  {(state.metrics.cumulativeReturn * 100).toFixed(2)}%
                </div>
              </div>
              <div style={styles.metricCard}>
                <div style={styles.metricLabel}>Max Drawdown</div>
                <div style={{ ...styles.metricValue, color: '#dc3545' }}>
                  {(state.metrics.maxDrawdown * 100).toFixed(2)}%
                </div>
              </div>
              <div style={styles.metricCard}>
                <div style={styles.metricLabel}>Sharpe Ratio</div>
                <div style={styles.metricValue}>{state.metrics.sharpeRatio.toFixed(2)}</div>
              </div>
            </div>
          )}

          {state.snapshots.length > 0 && (
            <div style={styles.snapshotTable}>
              <h4>Recent Snapshots</h4>
              <table style={styles.table}>
                <thead>
                  <tr>
                    <th style={styles.th}>Time</th>
                    <th style={styles.th}>TVL</th>
                    <th style={styles.th}>Utilization</th>
                    <th style={styles.th}>Borrow APY</th>
                    <th style={styles.th}>Supply APY</th>
                    <th style={styles.th}>Bad Debt</th>
                  </tr>
                </thead>
                <tbody>
                  {state.snapshots.slice(0, 10).map((snap, i) => (
                    <tr key={i} style={styles.tr}>
                      <td style={styles.td}>{new Date(snap.timestamp).toLocaleString()}</td>
                      <td style={styles.td}>${(snap.tvl / 1_000_000).toFixed(2)}M</td>
                      <td style={styles.td}>{(snap.utilizationRate * 100).toFixed(1)}%</td>
                      <td style={styles.td}>{snap.borrowApy.toFixed(2)}%</td>
                      <td style={styles.td}>{snap.supplyApy.toFixed(2)}%</td>
                      <td style={styles.td}>${snap.badDebt.toLocaleString()}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {state.view === 'comparison' && (
        <div style={styles.section}>
          <h3 style={styles.sectionTitle}>Performance Comparison</h3>
          <div style={styles.comparisonGrid}>
            {state.comparison.map(pool => (
              <div key={pool.poolAddress} style={styles.comparisonCard}>
                <h4 style={styles.comparisonTitle}>{pool.poolName}</h4>
                <div style={styles.comparisonStat}>
                  <span>APY</span>
                  <span style={{ fontWeight: 600 }}>{pool.currentApy.toFixed(2)}%</span>
                </div>
                <div style={styles.comparisonStat}>
                  <span>TVL</span>
                  <span>${(pool.tvl / 1_000_000).toFixed(2)}M</span>
                </div>
                <div style={styles.comparisonStat}>
                  <span>Utilization</span>
                  <span>{(pool.utilization * 100).toFixed(1)}%</span>
                </div>
                <div style={styles.comparisonStat}>
                  <span>Risk</span>
                  <span>{pool.riskScore}/100</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {state.view === 'heatmap' && (
        <div style={styles.section}>
          <h3 style={styles.sectionTitle}>Utilization Heatmap (UTC day × hour)</h3>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(24, 1fr)', gap: 2 }}>
            {state.heatmap.map((cell, i) => {
              const intensity = Math.min(1, Math.max(0, cell.utilization));
              return (
                <div
                  key={i}
                  title={`D${cell.day} H${cell.hour}: ${(intensity * 100).toFixed(0)}%`}
                  style={{
                    height: 12,
                    backgroundColor: `rgba(0, 123, 255, ${0.1 + intensity * 0.9})`,
                  }}
                />
              );
            })}
          </div>
        </div>
      )}

      {state.view === 'benchmarks' && (
        <div style={styles.section}>
          <h3 style={styles.sectionTitle}>DeFi Benchmarks (Compound / Aave)</h3>
          <table style={styles.table}>
            <thead>
              <tr>
                <th style={styles.th}>Protocol</th>
                <th style={styles.th}>Supply APY</th>
                <th style={styles.th}>Delta vs Pool</th>
              </tr>
            </thead>
            <tbody>
              {state.benchmarks.map((b) => (
                <tr key={b.name} style={styles.tr}>
                  <td style={styles.td}>{b.name}</td>
                  <td style={styles.td}>{(b.supplyApy * 100).toFixed(2)}%</td>
                  <td style={styles.td}>
                    {b.supplyApyDelta >= 0 ? '+' : ''}
                    {(b.supplyApyDelta * 100).toFixed(2)}%
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {state.chartSeries.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <h4>Cumulative return</h4>
              <div style={{ display: 'flex', alignItems: 'flex-end', gap: 2, height: 80 }}>
                {state.chartSeries.slice(-48).map((p, i) => (
                  <div
                    key={i}
                    title={`${p.timestamp}: ${(p.cumulativeReturn * 100).toFixed(3)}%`}
                    style={{
                      width: 6,
                      height: `${Math.max(4, Math.abs(p.cumulativeReturn) * 400)}%`,
                      backgroundColor: p.cumulativeReturn >= 0 ? '#28a745' : '#dc3545',
                    }}
                  />
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  container: {
    padding: '24px',
    maxWidth: '1100px',
    margin: '0 auto',
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
  },
  title: {
    fontSize: '24px',
    fontWeight: 600,
    marginBottom: '24px',
    color: '#1a1a2e',
  },
  summaryGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    gap: '16px',
    marginBottom: '24px',
  },
  summaryCard: {
    padding: '20px',
    backgroundColor: '#fff',
    borderRadius: '12px',
    border: '1px solid #e0e0e0',
    textAlign: 'center',
  },
  summaryLabel: {
    fontSize: '12px',
    color: '#666',
    marginBottom: '4px',
    textTransform: 'uppercase',
    fontWeight: 600,
  },
  summaryValue: {
    fontSize: '24px',
    fontWeight: 700,
    color: '#1a1a2e',
  },
  tabBar: {
    display: 'flex',
    gap: '4px',
    marginBottom: '20px',
    borderBottom: '1px solid #e0e0e0',
    paddingBottom: '4px',
  },
  tab: {
    padding: '8px 20px',
    backgroundColor: 'transparent',
    border: 'none',
    borderRadius: '6px 6px 0 0',
    fontSize: '14px',
    cursor: 'pointer',
    color: '#666',
  },
  tabActive: {
    padding: '8px 20px',
    backgroundColor: '#f0f7ff',
    border: 'none',
    borderBottom: '2px solid #0066ff',
    borderRadius: '6px 6px 0 0',
    fontSize: '14px',
    cursor: 'pointer',
    color: '#0066ff',
    fontWeight: 600,
  },
  section: {
    backgroundColor: '#fff',
    borderRadius: '12px',
    border: '1px solid #e0e0e0',
    padding: '20px',
  },
  sectionTitle: {
    fontSize: '16px',
    fontWeight: 600,
    marginBottom: '16px',
  },
  chartHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '16px',
  },
  chartControls: {
    display: 'flex',
    gap: '12px',
    alignItems: 'center',
  },
  select: {
    padding: '6px 12px',
    borderRadius: '6px',
    border: '1px solid #ddd',
    fontSize: '13px',
  },
  exportButton: {
    padding: '6px 16px',
    backgroundColor: '#f5f5f5',
    border: '1px solid #ddd',
    borderRadius: '6px',
    fontSize: '13px',
    cursor: 'pointer',
  },
  poolAddress: {
    fontSize: '13px',
    color: '#888',
  },
  metricsGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(4, 1fr)',
    gap: '12px',
    marginBottom: '20px',
  },
  metricCard: {
    padding: '14px',
    backgroundColor: '#f8f9fa',
    borderRadius: '8px',
    textAlign: 'center',
  },
  metricLabel: {
    fontSize: '11px',
    color: '#666',
    marginBottom: '4px',
    textTransform: 'uppercase',
    fontWeight: 600,
  },
  metricValue: {
    fontSize: '18px',
    fontWeight: 700,
    color: '#1a1a2e',
  },
  snapshotTable: {
    marginTop: '16px',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    fontSize: '13px',
  },
  th: {
    textAlign: 'left',
    padding: '10px 16px',
    borderBottom: '2px solid #e0e0e0',
    color: '#666',
    fontSize: '11px',
    fontWeight: 600,
    textTransform: 'uppercase',
  },
  tr: {
    borderBottom: '1px solid #f0f0f0',
  },
  td: {
    padding: '10px 16px',
  },
  riskBadge: {
    padding: '2px 8px',
    borderRadius: '4px',
    fontSize: '12px',
    fontWeight: 600,
  },
  comparisonGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))',
    gap: '16px',
  },
  comparisonCard: {
    padding: '16px',
    backgroundColor: '#f8f9fa',
    borderRadius: '10px',
    border: '1px solid #e0e0e0',
  },
  comparisonTitle: {
    fontSize: '14px',
    fontWeight: 600,
    marginBottom: '12px',
  },
  comparisonStat: {
    display: 'flex',
    justifyContent: 'space-between',
    padding: '6px 0',
    borderBottom: '1px solid #eee',
    fontSize: '13px',
  },
};
