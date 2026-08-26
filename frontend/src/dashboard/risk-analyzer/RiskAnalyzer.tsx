import React, { useState, useEffect } from 'react';

interface CollateralEntry {
  asset: string;
  amount: number;
  price: number;
}

interface BorrowEntry {
  asset: string;
  amount: number;
  price: number;
}

interface SimulationResult {
  scenario: { name: string; description: string; priceChanges: { asset: string; changePercent: number }[] };
  currentHealthFactor: number;
  afterScenarioHealthFactor: number;
  currentLtv: number;
  afterScenarioLtv: number;
  isLiquidatable: boolean;
  liquidationPrice: Record<string, number>;
  safetyMargin: number;
  collateralValueChange: number;
  debtValueChange: number;
  recommendation: string;
}

interface CollateralRatioSnapshot {
  asset: string;
  currentRatio: number;
  requiredRatio: number;
  healthFactor: number;
  riskLevel: 'safe' | 'warning' | 'danger' | 'critical';
  collateralValue: string;
  debtValue: string;
  timestamp: number;
}

interface RiskAlert {
  id: string;
  type: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  address: string;
  asset: string;
  message: string;
  currentValue: number;
  thresholdValue: number;
  timestamp: number;
  acknowledged: boolean;
}

const COLORS = {
  safe: '#28a745',
  warning: '#ffc107',
  danger: '#fd7e14',
  critical: '#dc3545',
};

const getHealthColor = (health: number): string => {
  if (health >= 2) return COLORS.safe;
  if (health >= 1.5) return COLORS.warning;
  if (health >= 1.0) return COLORS.danger;
  return COLORS.critical;
};

export const RiskAnalyzer: React.FC = () => {
  const [collateral, setCollateral] = useState<CollateralEntry[]>([
    { asset: 'XLM', amount: 10000, price: 0.12 },
  ]);
  const [borrow, setBorrow] = useState<BorrowEntry[]>([
    { asset: 'USDC', amount: 800, price: 1.0 },
  ]);
  const [results, setResults] = useState<SimulationResult[]>([]);
  const [scenarios, setScenarios] = useState<any[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  
  // Real-time collateral ratio monitoring state
  const [collateralSnapshots, setCollateralSnapshots] = useState<CollateralRatioSnapshot[]>([]);
  const [riskAlerts, setRiskAlerts] = useState<RiskAlert[]>([]);
  const [wsConnected, setWsConnected] = useState(false);
  const [showCollateralMonitoring, setShowCollateralMonitoring] = useState(true);

  useEffect(() => {
    fetchCollateralSnapshots();
    connectWebSocket();
    return () => disconnectWebSocket();
  }, []);

  const fetchCollateralSnapshots = async () => {
    try {
      const res = await fetch('/api/risk/collateral-ratio/snapshots');
      const data = await res.json();
      setCollateralSnapshots(data);
    } catch (e) {
      console.error('Failed to fetch collateral snapshots', e);
    }
  };

  const fetchRiskAlerts = async () => {
    try {
      const res = await fetch('/api/risk/collateral-ratio/alerts?limit=10');
      const data = await res.json();
      setRiskAlerts(data);
    } catch (e) {
      console.error('Failed to fetch risk alerts', e);
    }
  };

  const connectWebSocket = () => {
    const ws = new WebSocket(`ws://${window.location.host}/api/ws/collateral-ratios?alerts=true`);
    
    ws.onopen = () => {
      setWsConnected(true);
      console.log('Collateral ratio WebSocket connected');
    };

    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      
      switch (message.type) {
        case 'initial_snapshot':
          setCollateralSnapshots(message.snapshots);
          break;
        case 'ratio_update':
          setCollateralSnapshots((prev) => {
            const index = prev.findIndex((s) => s.asset === message.snapshot.asset);
            if (index >= 0) {
              const updated = [...prev];
              updated[index] = message.snapshot;
              return updated;
            }
            return [...prev, message.snapshot];
          });
          break;
        case 'alert':
          setRiskAlerts((prev) => [message.alert, ...prev].slice(0, 50));
          break;
      }
    };

    ws.onclose = () => {
      setWsConnected(false);
      console.log('Collateral ratio WebSocket disconnected');
      setTimeout(connectWebSocket, 5000);
    };

    ws.onerror = (error) => {
      console.error('WebSocket error', error);
    };

    return () => ws.close();
  };

  const disconnectWebSocket = () => {
    // Cleanup handled in useEffect return
  };

  const acknowledgeAlert = async (alertId: string) => {
    try {
      await fetch(`/api/risk/collateral-ratio/alerts/${alertId}/acknowledge`, { method: 'POST' });
      setRiskAlerts((prev) => prev.map((a) => (a.id === alertId ? { ...a, acknowledged: true } : a)));
    } catch (e) {
      console.error('Failed to acknowledge alert', e);
    }
  };

  const fetchScenarios = async () => {
    try {
      const res = await fetch('/api/risk/scenarios');
      const data = await res.json();
      setScenarios(data.scenarios);
    } catch (e) {
      console.error('Failed to fetch scenarios', e);
    }
  };

  const runSimulation = async () => {
    setIsLoading(true);
    try {
      const position = {
        collateral: collateral.map((c) => ({ asset: c.asset, amount: c.amount, price: c.price })),
        borrow: borrow.map((b) => ({ asset: b.asset, amount: b.amount, price: b.price })),
      };

      const scenario = {
        name: 'Custom Price Drop',
        description: 'Manual price simulation',
        priceChanges: [
          { asset: collateral[0]?.asset || 'XLM', changePercent: -20 },
        ],
      };

      const res = await fetch('/api/risk/simulate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ position, scenario }),
      });
      const result = await res.json();
      setResults((prev) => [...prev, result]);
    } catch (e) {
      console.error('Simulation failed', e);
    }
    setIsLoading(false);
  };

  const totalCollateral = collateral.reduce((s, c) => s + c.amount * c.price, 0);
  const totalDebt = borrow.reduce((s, b) => s + b.amount * b.price, 0);
  const healthFactor = totalDebt > 0 ? totalCollateral / totalDebt : Infinity;

  const addCollateral = () => {
    setCollateral((prev) => [...prev, { asset: 'XLM', amount: 1000, price: 0.12 }]);
  };

  const addBorrow = () => {
    setBorrow((prev) => [...prev, { asset: 'USDC', amount: 100, price: 1.0 }]);
  };

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <h2>Liquidation Risk Analyzer</h2>
        <div style={styles.wsStatus}>
          <span style={{ ...styles.statusDot, backgroundColor: wsConnected ? COLORS.safe : COLORS.critical }} />
          {wsConnected ? 'Live' : 'Disconnected'}
        </div>
      </div>

      {/* Real-time Collateral Ratio Monitoring Widget */}
      {showCollateralMonitoring && (
        <div style={styles.collateralMonitoring}>
          <div style={styles.sectionHeader}>
            <h3>Real-time Collateral Ratios</h3>
            <button onClick={() => setShowCollateralMonitoring(false)} style={styles.closeBtn}>×</button>
          </div>
          
          <div style={styles.snapshotsGrid}>
            {collateralSnapshots.map((snapshot) => (
              <div
                key={snapshot.asset}
                style={{
                  ...styles.snapshotCard,
                  borderLeft: `4px solid ${COLORS[snapshot.riskLevel]}`,
                }}
              >
                <div style={styles.cardHeader}>
                  <strong>{snapshot.asset}</strong>
                  <span style={{ ...styles.badge, backgroundColor: COLORS[snapshot.riskLevel] }}>
                    {snapshot.riskLevel.toUpperCase()}
                  </span>
                </div>
                <div style={styles.metricRow}>
                  <span style={styles.metricLabel}>Health Factor:</span>
                  <span style={{ ...styles.metricValue, color: COLORS[snapshot.riskLevel] }}>
                    {snapshot.healthFactor.toFixed(2)}
                  </span>
                </div>
                <div style={styles.metricRow}>
                  <span style={styles.metricLabel}>Current Ratio:</span>
                  <span style={styles.metricValue}>{(snapshot.currentRatio / 100).toFixed(2)}%</span>
                </div>
                <div style={styles.metricRow}>
                  <span style={styles.metricLabel}>Required Ratio:</span>
                  <span style={styles.metricValue}>{(snapshot.requiredRatio / 100).toFixed(2)}%</span>
                </div>
                <div style={styles.metricRow}>
                  <span style={styles.metricLabel}>Collateral:</span>
                  <span style={styles.metricValue}>${parseFloat(snapshot.collateralValue).toLocaleString()}</span>
                </div>
                <div style={styles.metricRow}>
                  <span style={styles.metricLabel}>Debt:</span>
                  <span style={styles.metricValue}>${parseFloat(snapshot.debtValue).toLocaleString()}</span>
                </div>
              </div>
            ))}
          </div>

          {/* Risk Alerts */}
          {riskAlerts.length > 0 && (
            <div style={styles.alertsSection}>
              <h4>Risk Alerts</h4>
              {riskAlerts.slice(0, 5).map((alert) => (
                <div
                  key={alert.id}
                  style={{
                    ...styles.alertCard,
                    borderLeft: `4px solid ${COLORS[alert.severity]}`,
                    opacity: alert.acknowledged ? 0.6 : 1,
                  }}
                >
                  <div style={styles.alertHeader}>
                    <span style={{ ...styles.badge, backgroundColor: COLORS[alert.severity] }}>
                      {alert.severity.toUpperCase()}
                    </span>
                    <span style={styles.alertTime}>
                      {new Date(alert.timestamp).toLocaleTimeString()}
                    </span>
                  </div>
                  <p style={styles.alertMessage}>{alert.message}</p>
                  <div style={styles.alertFooter}>
                    <span style={styles.alertAsset}>{alert.asset}</span>
                    {!alert.acknowledged && (
                      <button
                        onClick={() => acknowledgeAlert(alert.id)}
                        style={styles.ackBtn}
                      >
                        Acknowledge
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {!showCollateralMonitoring && (
        <button onClick={() => setShowCollateralMonitoring(true)} style={styles.showBtn}>
          Show Collateral Monitoring
        </button>
      )}

      <div style={styles.divider} />

      <div style={styles.panel}>
        <div style={styles.section}>
          <h3>Collateral Assets</h3>
          {collateral.map((c, i) => (
            <div key={i} style={styles.row}>
              <input
                style={styles.inputSmall}
                value={c.asset}
                onChange={(e) => {
                  const next = [...collateral];
                  next[i] = { ...next[i], asset: e.target.value };
                  setCollateral(next);
                }}
              />
              <input
                style={styles.inputSmall}
                type="number"
                value={c.amount}
                onChange={(e) => {
                  const next = [...collateral];
                  next[i] = { ...next[i], amount: parseFloat(e.target.value) || 0 };
                  setCollateral(next);
                }}
              />
              <input
                style={styles.inputSmall}
                type="number"
                step="0.01"
                value={c.price}
                onChange={(e) => {
                  const next = [...collateral];
                  next[i] = { ...next[i], price: parseFloat(e.target.value) || 0 };
                  setCollateral(next);
                }}
              />
            </div>
          ))}
          <button onClick={addCollateral} style={styles.smallBtn}>+ Add Collateral</button>
        </div>

        <div style={styles.section}>
          <h3>Borrow Assets</h3>
          {borrow.map((b, i) => (
            <div key={i} style={styles.row}>
              <input
                style={styles.inputSmall}
                value={b.asset}
                onChange={(e) => {
                  const next = [...borrow];
                  next[i] = { ...next[i], asset: e.target.value };
                  setBorrow(next);
                }}
              />
              <input
                style={styles.inputSmall}
                type="number"
                value={b.amount}
                onChange={(e) => {
                  const next = [...borrow];
                  next[i] = { ...next[i], amount: parseFloat(e.target.value) || 0 };
                  setBorrow(next);
                }}
              />
              <input
                style={styles.inputSmall}
                type="number"
                step="0.01"
                value={b.price}
                onChange={(e) => {
                  const next = [...borrow];
                  next[i] = { ...next[i], price: parseFloat(e.target.value) || 0 };
                  setBorrow(next);
                }}
              />
            </div>
          ))}
          <button onClick={addBorrow} style={styles.smallBtn}>+ Add Borrow</button>
        </div>
      </div>

      <div style={styles.healthBar}>
        <span style={{ ...styles.healthLabel, backgroundColor: getHealthColor(healthFactor) }}>
          Health Factor: {healthFactor === Infinity ? '∞' : healthFactor.toFixed(2)}
        </span>
        <span style={styles.healthLabel}>Total Collateral: ${totalCollateral.toFixed(2)}</span>
        <span style={styles.healthLabel}>Total Debt: ${totalDebt.toFixed(2)}</span>
      </div>

      <div style={styles.actions}>
        <button onClick={fetchScenarios} style={styles.btn}>Load Scenarios</button>
        <button onClick={runSimulation} disabled={isLoading} style={styles.btn}>
          {isLoading ? 'Running...' : 'Run -20% Simulation'}
        </button>
      </div>

      {scenarios.length > 0 && (
        <div style={styles.scenarioList}>
          <h3>Available Scenarios</h3>
          {scenarios.map((s: any, i: number) => (
            <div key={i} style={styles.scenarioCard}>
              <strong>{s.name}</strong> - {s.description}
            </div>
          ))}
        </div>
      )}

      {results.length > 0 && (
        <div style={styles.resultsSection}>
          <h3>Simulation Results</h3>
          {results.map((r, i) => (
            <div key={i} style={styles.resultCard}>
              <div style={styles.resultHeader}>
                <h4>{r.scenario.name}</h4>
                <span style={{
                  ...styles.badge,
                  backgroundColor: r.isLiquidatable ? COLORS.critical : COLORS.safe,
                }}>
                  {r.isLiquidatable ? 'LIQUIDATABLE' : 'SAFE'}
                </span>
              </div>
              <div style={styles.metrics}>
                <Metric label="Current HF" value={r.currentHealthFactor.toFixed(2)} />
                <Metric label="After HF" value={r.afterScenarioHealthFactor.toFixed(2)} color={getHealthColor(r.afterScenarioHealthFactor)} />
                <Metric label="Current LTV" value={`${(r.currentLtv * 100).toFixed(1)}%`} />
                <Metric label="After LTV" value={`${(r.afterScenarioLtv * 100).toFixed(1)}%`} />
                <Metric label="Safety Margin" value={`${r.safetyMargin.toFixed(1)}%`} />
                <Metric label="Value Change" value={`$${r.collateralValueChange.toFixed(2)}`} />
              </div>
              <p style={styles.recommendation}>{r.recommendation}</p>
              {Object.keys(r.liquidationPrice).length > 0 && (
                <div style={styles.liqPrices}>
                  <strong>Liquidation Prices:</strong>
                  {Object.entries(r.liquidationPrice).map(([asset, price]) => (
                    <span key={asset}> {asset}: ${price.toFixed(4)}</span>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

const Metric: React.FC<{ label: string; value: string; color?: string }> = ({ label, value, color }) => (
  <div style={styles.metric}>
    <span style={styles.metricLabel}>{label}</span>
    <span style={{ ...styles.metricValue, color: color || '#333' }}>{value}</span>
  </div>
);

const styles: Record<string, React.CSSProperties> = {
  container: { maxWidth: 960, margin: '0 auto', padding: 24 },
  header: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 20 },
  wsStatus: { display: 'flex', alignItems: 'center', gap: 8, fontSize: 14, color: '#666' },
  statusDot: { width: 10, height: 10, borderRadius: '50%' },
  collateralMonitoring: { marginBottom: 24, padding: 20, background: '#f8f9fa', borderRadius: 8, border: '1px solid #e0e0e0' },
  sectionHeader: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 },
  closeBtn: { background: 'none', border: 'none', fontSize: 24, cursor: 'pointer', color: '#666', padding: 0, lineHeight: 1 },
  snapshotsGrid: { display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))', gap: 16, marginBottom: 20 },
  snapshotCard: { padding: 16, background: 'white', borderRadius: 8, border: '1px solid #e0e0e0', boxShadow: '0 2px 4px rgba(0,0,0,0.05)' },
  cardHeader: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 },
  badge: { padding: '4px 12px', borderRadius: 4, color: 'white', fontSize: 12, fontWeight: 'bold' },
  metricRow: { display: 'flex', justifyContent: 'space-between', marginBottom: 8 },
  metricLabel: { fontSize: 13, color: '#666' },
  metricValue: { fontSize: 14, fontWeight: 'bold', color: '#333' },
  alertsSection: { marginTop: 16 },
  alertCard: { padding: 12, marginBottom: 8, background: 'white', borderRadius: 6, border: '1px solid #e0e0e0' },
  alertHeader: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 },
  alertTime: { fontSize: 12, color: '#999' },
  alertMessage: { margin: '8px 0', fontSize: 14, color: '#333' },
  alertFooter: { display: 'flex', justifyContent: 'space-between', alignItems: 'center' },
  alertAsset: { fontSize: 13, color: '#666', fontWeight: 'bold' },
  ackBtn: { padding: '6px 12px', background: '#6c757d', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer', fontSize: 12 },
  showBtn: { padding: '10px 20px', background: '#007bff', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer', marginBottom: 20 },
  divider: { height: 1, background: '#e0e0e0', margin: '24px 0' },
  panel: { display: 'flex', gap: 24, marginBottom: 20 },
  section: { flex: 1, padding: 16, background: '#f5f5f5', borderRadius: 8 },
  row: { display: 'flex', gap: 8, marginBottom: 8 },
  inputSmall: { width: '30%', padding: 6, borderRadius: 4, border: '1px solid #ccc' },
  healthBar: { display: 'flex', gap: 12, marginBottom: 20 },
  healthLabel: { padding: '8px 16px', borderRadius: 4, color: 'white', fontWeight: 'bold' },
  actions: { display: 'flex', gap: 12, marginBottom: 20 },
  btn: { padding: '10px 20px', background: '#007bff', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' },
  smallBtn: { padding: '6px 12px', marginTop: 8, background: '#6c757d', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' },
  scenarioList: { marginBottom: 20 },
  scenarioCard: { padding: 10, marginBottom: 6, background: '#e9ecef', borderRadius: 4 },
  resultsSection: { marginTop: 20 },
  resultCard: { padding: 16, marginBottom: 12, background: '#f9f9f9', borderRadius: 8, border: '1px solid #e0e0e0' },
  resultHeader: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 },
  metrics: { display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10, marginBottom: 12 },
  metric: { textAlign: 'center' },
  recommendation: { padding: 10, background: '#fff3cd', borderRadius: 4, fontSize: 14, margin: '10px 0' },
  liqPrices: { fontSize: 13, color: '#dc3545', padding: 8, background: '#f8d7da', borderRadius: 4 },
};
