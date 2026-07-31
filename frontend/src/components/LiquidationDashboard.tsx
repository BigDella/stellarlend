import React, { useState, useEffect, useRef, useCallback } from 'react';

interface PositionHealth {
  address: string;
  collateralAsset: string;
  debtAsset: string;
  collateralValue: number;
  debtValue: number;
  healthFactor: number;
  utilizationBps: number;
  liquidationPrice: number;
  estimatedProfit: number;
  gasCostStroops: number;
  netProfit: number;
  riskCategory: 'safe' | 'moderate' | 'danger' | 'critical';
  lastUpdated: number;
}

const HF_COLORS: Record<string, string> = {
  safe: '#10B981',
  moderate: '#F59E0B',
  danger: '#EF4444',
  critical: '#DC2626',
};

export const LiquidationDashboard: React.FC = () => {
  const [positions, setPositions] = useState<PositionHealth[]>([]);
  const [filter, setFilter] = useState<string>('all');
  const [sortBy, setSortBy] = useState<'healthFactor' | 'netProfit' | 'collateralValue'>('healthFactor');
  const [searchAddress, setSearchAddress] = useState('');
  const [selectedPosition, setSelectedPosition] = useState<PositionHealth | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const [alertThreshold, setAlertThreshold] = useState(1.2);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    const ws = new WebSocket(`ws://${window.location.host}/api/ws/health-updates`);
    wsRef.current = ws;

    ws.onopen = () => setConnected(true);
    ws.onclose = () => setConnected(false);

    ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      if (msg.type === 'health_snapshot' || msg.type === 'health_update') {
        setPositions(msg.positions);
      }
    };

    return () => ws.close();
  }, []);

  const filtered = positions
    .filter((p) => {
      if (filter === 'all') return true;
      return p.riskCategory === filter;
    })
    .filter((p) =>
      searchAddress ? p.address.toLowerCase().includes(searchAddress.toLowerCase()) : true
    )
    .sort((a, b) => {
      if (sortBy === 'netProfit') return b.netProfit - a.netProfit;
      if (sortBy === 'collateralValue') return b.collateralValue - a.collateralValue;
      return a.healthFactor - b.healthFactor;
    });

  const getHfColor = (hf: number): string => {
    if (hf >= 1.5) return HF_COLORS.safe;
    if (hf >= 1.2) return HF_COLORS.moderate;
    if (hf >= 1.05) return HF_COLORS.danger;
    return HF_COLORS.critical;
  };

  return (
    <div style={styles.container}>
      <header style={styles.header}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <h2>Liquidation Monitor</h2>
          <span
            style={{
              ...styles.statusDot,
              backgroundColor: connected ? '#10B981' : '#EF4444',
            }}
          />
          <span style={{ fontSize: '12px', color: '#94A3B8' }}>
            {connected ? 'Live' : 'Disconnected'}
          </span>
        </div>
        <p style={{ color: '#94A3B8', margin: 0 }}>
          {filtered.length} positions monitored
        </p>
      </header>

      <div style={styles.controls}>
        <div style={styles.controlGroup}>
          <label>Risk Filter:</label>
          <select value={filter} onChange={(e) => setFilter(e.target.value)} style={styles.select}>
            <option value="all">All Positions</option>
            <option value="critical">Critical (HF {'<'} 1.05)</option>
            <option value="danger">Danger (HF {'<'} 1.2)</option>
            <option value="moderate">Moderate (HF {'<'} 1.5)</option>
            <option value="safe">Safe (HF {'>='} 1.5)</option>
          </select>
        </div>
        <div style={styles.controlGroup}>
          <label>Sort By:</label>
          <select value={sortBy} onChange={(e) => setSortBy(e.target.value as any)} style={styles.select}>
            <option value="healthFactor">Health Factor (lowest)</option>
            <option value="netProfit">Net Profit (highest)</option>
            <option value="collateralValue">Collateral (highest)</option>
          </select>
        </div>
        <div style={styles.controlGroup}>
          <label>Search Address:</label>
          <input
            type="text"
            value={searchAddress}
            onChange={(e) => setSearchAddress(e.target.value)}
            placeholder="G..."
            style={styles.input}
          />
        </div>
        <div style={styles.controlGroup}>
          <label>Alert Threshold (HF):</label>
          <input
            type="number"
            step="0.05"
            min="1.0"
            max="2.0"
            value={alertThreshold}
            onChange={(e) => setAlertThreshold(parseFloat(e.target.value))}
            style={{ ...styles.input, width: '80px' }}
          />
        </div>
      </div>

      <div style={styles.tableWrapper}>
        <table style={styles.table}>
          <thead>
            <tr>
              <th>Address</th>
              <th>Collateral</th>
              <th>Debt</th>
              <th>Health Factor</th>
              <th>Utilization</th>
              <th>Est. Profit</th>
              <th>Net Profit</th>
              <th>Risk</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((p) => (
              <tr
                key={p.address}
                style={{
                  ...styles.row,
                  cursor: 'pointer',
                  backgroundColor:
                    selectedPosition?.address === p.address ? '#1E3A5F' : undefined,
                }}
                onClick={() => setSelectedPosition(p)}
              >
                <td style={{ fontFamily: 'monospace', fontSize: '12px' }}>
                  {p.address.slice(0, 8)}...
                </td>
                <td>
                  {p.collateralValue.toLocaleString()} {p.collateralAsset}
                </td>
                <td>
                  {p.debtValue.toLocaleString()} {p.debtAsset}
                </td>
                <td>
                  <span
                    style={{
                      fontWeight: 700,
                      color: getHfColor(p.healthFactor),
                    }}
                  >
                    {p.healthFactor.toFixed(2)}
                  </span>
                </td>
                <td>{(p.utilizationBps / 100).toFixed(1)}%</td>
                <td style={{ color: '#34D399' }}>
                  {p.estimatedProfit.toLocaleString()} stroops
                </td>
                <td
                  style={{
                    fontWeight: 600,
                    color: p.netProfit > 0 ? '#34D399' : '#EF4444',
                  }}
                >
                  {p.netProfit.toLocaleString()} stroops
                </td>
                <td>
                  <span
                    style={{
                      ...styles.riskBadge,
                      backgroundColor: HF_COLORS[p.riskCategory],
                    }}
                  >
                    {p.riskCategory}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {selectedPosition && (
        <div style={styles.detailPanel}>
          <h3>Position Detail — {selectedPosition.address}</h3>
          <div style={styles.detailGrid}>
            <div style={styles.detailItem}>
              <span style={styles.detailLabel}>Collateral</span>
              <span style={styles.detailValue}>
                {selectedPosition.collateralValue.toLocaleString()}{' '}
                {selectedPosition.collateralAsset}
              </span>
            </div>
            <div style={styles.detailItem}>
              <span style={styles.detailLabel}>Debt</span>
              <span style={styles.detailValue}>
                {selectedPosition.debtValue.toLocaleString()}{' '}
                {selectedPosition.debtAsset}
              </span>
            </div>
            <div style={styles.detailItem}>
              <span style={styles.detailLabel}>Health Factor</span>
              <span
                style={{
                  ...styles.detailValue,
                  color: getHfColor(selectedPosition.healthFactor),
                }}
              >
                {selectedPosition.healthFactor.toFixed(4)}
              </span>
            </div>
            <div style={styles.detailItem}>
              <span style={styles.detailLabel}>Liquidation Price</span>
              <span style={styles.detailValue}>
                {selectedPosition.liquidationPrice} XLM
              </span>
            </div>
            <div style={styles.detailItem}>
              <span style={styles.detailLabel}>Est. Profit</span>
              <span style={{ ...styles.detailValue, color: '#34D399' }}>
                {selectedPosition.estimatedProfit.toLocaleString()} stroops
              </span>
            </div>
            <div style={styles.detailItem}>
              <span style={styles.detailLabel}>Gas Cost</span>
              <span style={styles.detailValue}>
                {selectedPosition.gasCostStroops.toLocaleString()} stroops
              </span>
            </div>
          </div>
        </div>
      )}
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
  statusDot: {
    width: '10px',
    height: '10px',
    borderRadius: '50%',
    display: 'inline-block',
  },
  controls: {
    display: 'flex',
    gap: '16px',
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
  select: {
    padding: '6px 10px',
    borderRadius: '6px',
    backgroundColor: '#1E293B',
    color: '#FFF',
    border: '1px solid #334155',
    fontSize: '13px',
  },
  input: {
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
  detailPanel: {
    marginTop: '20px',
    padding: '16px',
    backgroundColor: '#1E293B',
    borderRadius: '8px',
  },
  detailGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
    gap: '12px',
    marginTop: '12px',
  },
  detailItem: {
    display: 'flex',
    flexDirection: 'column',
    gap: '4px',
  },
  detailLabel: {
    fontSize: '11px',
    color: '#64748B',
    textTransform: 'uppercase',
  },
  detailValue: {
    fontSize: '16px',
    fontWeight: 600,
  },
};

export default LiquidationDashboard;
