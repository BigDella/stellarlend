import React, { useState } from 'react';

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
      <h2>Liquidation Risk Analyzer</h2>

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
  badge: { padding: '4px 12px', borderRadius: 4, color: 'white', fontSize: 12, fontWeight: 'bold' },
  metrics: { display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10, marginBottom: 12 },
  metric: { textAlign: 'center' },
  metricLabel: { fontSize: 12, color: '#666', display: 'block' },
  metricValue: { fontSize: 16, fontWeight: 'bold', display: 'block' },
  recommendation: { padding: 10, background: '#fff3cd', borderRadius: 4, fontSize: 14, margin: '10px 0' },
  liqPrices: { fontSize: 13, color: '#dc3545', padding: 8, background: '#f8d7da', borderRadius: 4 },
};
