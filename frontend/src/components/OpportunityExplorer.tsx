import React, { useState, useEffect } from 'react';

interface LiquidationOpportunity {
  positionAddress: string;
  collateralAsset: string;
  debtAsset: string;
  collateralValue: number;
  debtValue: number;
  healthFactor: number;
  estimatedProfitStroops: number;
  gasCostStroops: number;
  netProfitStroops: number;
  profitPercent: number;
  collateralRatio: number;
  riskScore: number;
  updatedAt: number;
}

export const OpportunityExplorer: React.FC = () => {
  const [opportunities, setOpportunities] = useState<LiquidationOpportunity[]>([]);
  const [loading, setLoading] = useState(true);
  const [minProfit, setMinProfit] = useState(0);
  const [maxHf, setMaxHf] = useState(2.0);
  const [assetFilter, setAssetFilter] = useState('');
  const [sortBy, setSortBy] = useState('healthFactor');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');
  const [gasEstimate, setGasEstimate] = useState<number>(525000);

  useEffect(() => {
    fetchOpportunities();
  }, [minProfit, maxHf, assetFilter, sortBy, sortDir]);

  const fetchOpportunities = async () => {
    setLoading(true);
    try {
      const params = new URLSearchParams();
      if (minProfit > 0) params.set('minProfit', (minProfit * 1000000).toString());
      if (maxHf < 2.0) params.set('maxHf', maxHf.toString());
      if (assetFilter) params.set('asset', assetFilter);
      params.set('sortBy', sortBy);
      params.set('sortDir', sortDir);

      const res = await fetch(`/api/liquidations/opportunities?${params}`);
      if (res.ok) {
        const data = await res.json();
        setOpportunities(data.opportunities || []);
      }
    } catch (err) {
      console.error('Failed to fetch opportunities:', err);
    } finally {
      setLoading(false);
    }
  };

  const getHfColor = (hf: number): string => {
    if (hf >= 1.5) return '#10B981';
    if (hf >= 1.2) return '#F59E0B';
    if (hf >= 1.05) return '#EF4444';
    return '#DC2626';
  };

  return (
    <div style={styles.container}>
      <header style={styles.header}>
        <h2>Liquidation Opportunity Explorer</h2>
        <p style={{ color: '#94A3B8', margin: 0 }}>
          {opportunities.length} opportunities — Gas est. {(gasEstimate / 1000000).toFixed(2)} XLM
        </p>
      </header>

      <div style={styles.controls}>
        <div style={styles.controlGroup}>
          <label>Min Profit (XLM)</label>
          <input
            type="number"
            min="0"
            step="1"
            value={minProfit}
            onChange={(e) => setMinProfit(parseInt(e.target.value) || 0)}
            style={styles.input}
          />
        </div>
        <div style={styles.controlGroup}>
          <label>Max Health Factor</label>
          <input
            type="number"
            step="0.05"
            min="1.0"
            max="3.0"
            value={maxHf}
            onChange={(e) => setMaxHf(parseFloat(e.target.value) || 2.0)}
            style={styles.input}
          />
        </div>
        <div style={styles.controlGroup}>
          <label>Asset Filter</label>
          <input
            type="text"
            value={assetFilter}
            onChange={(e) => setAssetFilter(e.target.value.toUpperCase())}
            placeholder="XLM, USDC..."
            style={styles.input}
          />
        </div>
        <div style={styles.controlGroup}>
          <label>Sort By</label>
          <select value={sortBy} onChange={(e) => setSortBy(e.target.value)} style={styles.select}>
            <option value="healthFactor">Health Factor</option>
            <option value="netProfitStroops">Net Profit</option>
            <option value="collateralValue">Collateral Value</option>
            <option value="riskScore">Risk Score</option>
          </select>
        </div>
        <div style={styles.controlGroup}>
          <label>Direction</label>
          <select value={sortDir} onChange={(e) => setSortDir(e.target.value as any)} style={styles.select}>
            <option value="asc">Ascending</option>
            <option value="desc">Descending</option>
          </select>
        </div>
      </div>

      <div style={styles.tableWrapper}>
        <table style={styles.table}>
          <thead>
            <tr>
              <th>Position</th>
              <th>Collateral</th>
              <th>Debt</th>
              <th>Health Factor</th>
              <th>Est. Profit</th>
              <th>Gas Cost</th>
              <th>Net Profit</th>
              <th>Profit %</th>
              <th>Risk</th>
              <th>Action</th>
            </tr>
          </thead>
          <tbody>
            {opportunities.map((o) => (
              <tr key={o.positionAddress} style={styles.row}>
                <td style={{ fontFamily: 'monospace', fontSize: '12px' }}>
                  {o.positionAddress.slice(0, 8)}...
                </td>
                <td>
                  {(o.collateralValue / 1000000).toFixed(2)} {o.collateralAsset}
                </td>
                <td>
                  {(o.debtValue / 1000000).toFixed(2)} {o.debtAsset}
                </td>
                <td>
                  <span
                    style={{
                      fontWeight: 700,
                      color: getHfColor(o.healthFactor),
                    }}
                  >
                    {o.healthFactor.toFixed(2)}
                  </span>
                </td>
                <td style={{ color: '#34D399' }}>
                  {(o.estimatedProfitStroops / 1000000).toFixed(2)} XLM
                </td>
                <td style={{ color: '#94A3B8' }}>
                  {(o.gasCostStroops / 1000000).toFixed(2)} XLM
                </td>
                <td
                  style={{
                    fontWeight: 600,
                    color: o.netProfitStroops > 0 ? '#34D399' : '#EF4444',
                  }}
                >
                  {(o.netProfitStroops / 1000000).toFixed(2)} XLM
                </td>
                <td>{(o.profitPercent / 100).toFixed(1)}x</td>
                <td>
                  <span
                    style={{
                      ...styles.riskBadge,
                      backgroundColor:
                        o.riskScore > 80
                          ? '#DC2626'
                          : o.riskScore > 60
                          ? '#EF4444'
                          : o.riskScore > 40
                          ? '#F59E0B'
                          : '#10B981',
                    }}
                  >
                    {o.riskScore}
                  </span>
                </td>
                <td>
                  <button
                    style={styles.liquidateBtn}
                    onClick={() =>
                      window.open(
                        `https://stellarchain.io/tx?to=${o.positionAddress}`,
                        '_blank'
                      )
                    }
                  >
                    Liquidate
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {loading && <p style={{ textAlign: 'center', color: '#94A3B8' }}>Loading...</p>}
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  container: {
    padding: '24px',
    backgroundColor: '#0F172A',
    color: '#F8FAFC',
    borderRadius: '12px',
    fontFamily: 'Inter, system-ui, sans-serif',
  },
  header: {
    marginBottom: '20px',
  },
  controls: {
    display: 'flex',
    gap: '12px',
    marginBottom: '16px',
    flexWrap: 'wrap',
    alignItems: 'flex-end',
  },
  controlGroup: {
    display: 'flex',
    flexDirection: 'column',
    gap: '4px',
    fontSize: '12px',
    color: '#94A3B8',
  },
  input: {
    padding: '6px 10px',
    borderRadius: '6px',
    backgroundColor: '#1E293B',
    color: '#FFF',
    border: '1px solid #334155',
    fontSize: '13px',
    width: '100px',
  },
  select: {
    padding: '6px 10px',
    borderRadius: '6px',
    backgroundColor: '#1E293B',
    color: '#FFF',
    border: '1px solid #334155',
    fontSize: '13px',
  },
  tableWrapper: {
    overflowX: 'auto',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    fontSize: '13px',
  },
  row: {
    borderBottom: '1px solid #1E293B',
  },
  riskBadge: {
    padding: '2px 8px',
    borderRadius: '10px',
    fontSize: '11px',
    fontWeight: 700,
    color: '#FFF',
  },
  liquidateBtn: {
    padding: '4px 12px',
    backgroundColor: '#EF4444',
    color: '#FFF',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontWeight: 600,
    fontSize: '12px',
  },
};

export default OpportunityExplorer;
