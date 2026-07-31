import React, { useState, useEffect } from 'react';

export enum CurveType {
  PIECEWISE_LINEAR = 'PIECEWISE_LINEAR',
  POLYNOMIAL = 'POLYNOMIAL',
  NELSON_SIEGEL = 'NELSON_SIEGEL',
}

export interface YieldCurveConfig {
  curveType: CurveType;
  baseRateBps: number;
  kinkUtilizationBps: number;
  slope1Bps: number;
  slope2Bps: number;
  polyCoeffABps: number;
  polyCoeffBBps: number;
  reserveFactorBps: number;
  rateFloorBps: number;
  rateCeilingBps: number;
}

export interface YieldPoint {
  utilizationBps: number;
  utilizationPercentage: number;
  borrowRateBps: number;
  borrowRatePercentage: number;
  supplyRateBps: number;
  supplyRatePercentage: number;
  protocolSpreadBps: number;
  protocolSpreadPercentage: number;
  projectedRevenueBps: number;
  liquidityRiskScore: number;
}

export interface YieldCurvePredictionResponse {
  config: YieldCurveConfig;
  points: YieldPoint[];
  optimalKinkBps: number;
  maxProjectedRevenueBps: number;
  summary: {
    baseBorrowRatePercentage: number;
    kinkBorrowRatePercentage: number;
    maxBorrowRatePercentage: number;
    optimalSupplyRatePercentage: number;
    riskCategory: 'LOW' | 'MODERATE' | 'HIGH' | 'CRITICAL';
  };
}

export const YieldCurveDashboard: React.FC = () => {
  const [config, setConfig] = useState<YieldCurveConfig>({
    curveType: CurveType.PIECEWISE_LINEAR,
    baseRateBps: 200,
    kinkUtilizationBps: 8000,
    slope1Bps: 1000,
    slope2Bps: 6000,
    polyCoeffABps: 500,
    polyCoeffBBps: 1500,
    reserveFactorBps: 1000,
    rateFloorBps: 100,
    rateCeilingBps: 10000,
  });

  const [prediction, setPrediction] = useState<YieldCurvePredictionResponse | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [targetUtilization, setTargetUtilization] = useState<number>(80);
  const [optimizedResult, setOptimizedResult] = useState<any>(null);

  useEffect(() => {
    fetchPrediction();
  }, [config]);

  const fetchPrediction = async () => {
    setLoading(true);
    try {
      const res = await fetch('/api/yield-curve/predict', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ config, stepBps: 500 }),
      });
      if (res.ok) {
        const data = await res.json();
        setPrediction(data);
      }
    } catch (err) {
      console.error('Failed to fetch yield curve prediction:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleOptimize = async () => {
    try {
      const res = await fetch('/api/yield-curve/optimize', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          currentConfig: config,
          targetUtilizationBps: targetUtilization * 100,
        }),
      });
      if (res.ok) {
        const data = await res.json();
        setOptimizedResult(data);
        if (data.recommendedConfig) {
          setConfig(data.recommendedConfig);
        }
      }
    } catch (err) {
      console.error('Optimization failed:', err);
    }
  };

  const getRiskBadgeColor = (category?: string) => {
    switch (category) {
      case 'LOW':
        return '#10B981';
      case 'MODERATE':
        return '#F59E0B';
      case 'HIGH':
        return '#EF4444';
      case 'CRITICAL':
        return '#DC2626';
      default:
        return '#6B7280';
    }
  };

  return (
    <div style={styles.container}>
      <header style={styles.header}>
        <h2>Yield Curve Prediction & Rate Optimization Model</h2>
        <p>Predict borrow/supply rate curves, optimize kink utilization, and maximize protocol capital efficiency.</p>
      </header>

      {prediction && (
        <div style={styles.summaryGrid}>
          <div style={styles.card}>
            <span style={styles.cardTitle}>Base Borrow APY</span>
            <span style={styles.cardValue}>{prediction.summary.baseBorrowRatePercentage.toFixed(2)}%</span>
          </div>

          <div style={styles.card}>
            <span style={styles.cardTitle}>Kink Borrow APY</span>
            <span style={styles.cardValue}>{prediction.summary.kinkBorrowRatePercentage.toFixed(2)}%</span>
          </div>

          <div style={styles.card}>
            <span style={styles.cardTitle}>Optimal Kink Utilization</span>
            <span style={styles.cardValue}>{(prediction.optimalKinkBps / 100).toFixed(1)}%</span>
          </div>

          <div style={styles.card}>
            <span style={styles.cardTitle}>Risk Category</span>
            <span
              style={{
                ...styles.badge,
                backgroundColor: getRiskBadgeColor(prediction.summary.riskCategory),
              }}
            >
              {prediction.summary.riskCategory}
            </span>
          </div>
        </div>
      )}

      {/* Controls & Model Parameters */}
      <div style={styles.section}>
        <h3>Curve Model Parameters</h3>
        <div style={styles.controlsGrid}>
          <div style={styles.controlGroup}>
            <label>Curve Model Type:</label>
            <select
              value={config.curveType}
              onChange={(e) => setConfig({ ...config, curveType: e.target.value as CurveType })}
              style={styles.select}
            >
              <option value={CurveType.PIECEWISE_LINEAR}>Piecewise Linear (Standard Kink)</option>
              <option value={CurveType.POLYNOMIAL}>Polynomial Quadratic Curve</option>
              <option value={CurveType.NELSON_SIEGEL}>Nelson-Siegel Model</option>
            </select>
          </div>

          <div style={styles.controlGroup}>
            <label>Base Rate (BPS): {config.baseRateBps} ({config.baseRateBps / 100}%)</label>
            <input
              type="range"
              min="0"
              max="2000"
              step="50"
              value={config.baseRateBps}
              onChange={(e) => setConfig({ ...config, baseRateBps: parseInt(e.target.value, 10) })}
            />
          </div>

          <div style={styles.controlGroup}>
            <label>Kink Utilization (BPS): {config.kinkUtilizationBps} ({config.kinkUtilizationBps / 100}%)</label>
            <input
              type="range"
              min="1000"
              max="9500"
              step="100"
              value={config.kinkUtilizationBps}
              onChange={(e) => setConfig({ ...config, kinkUtilizationBps: parseInt(e.target.value, 10) })}
            />
          </div>

          <div style={styles.controlGroup}>
            <label>Slope Below Kink (BPS): {config.slope1Bps}</label>
            <input
              type="range"
              min="100"
              max="5000"
              step="100"
              value={config.slope1Bps}
              onChange={(e) => setConfig({ ...config, slope1Bps: parseInt(e.target.value, 10) })}
            />
          </div>

          <div style={styles.controlGroup}>
            <label>Slope Above Kink (BPS): {config.slope2Bps}</label>
            <input
              type="range"
              min="1000"
              max="20000"
              step="500"
              value={config.slope2Bps}
              onChange={(e) => setConfig({ ...config, slope2Bps: parseInt(e.target.value, 10) })}
            />
          </div>
        </div>

        <div style={styles.optimizeSection}>
          <h4>Auto-Optimize Rate Model</h4>
          <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
            <label>Target Utilization (%):</label>
            <input
              type="number"
              min="10"
              max="95"
              value={targetUtilization}
              onChange={(e) => setTargetUtilization(parseInt(e.target.value, 10) || 80)}
              style={styles.inputNumber}
            />
            <button onClick={handleOptimize} style={styles.button}>
              Run Rate Optimizer
            </button>
          </div>
          {optimizedResult && (
            <p style={styles.optGainText}>
              ✅ Optimized! Projected Revenue Gain: <strong>+{optimizedResult.revenueGainPercentage}%</strong>
            </p>
          )}
        </div>
      </div>

      {/* Yield Points Table */}
      {prediction && (
        <div style={styles.section}>
          <h3>Predicted Yield & Rate Curve Matrix</h3>
          <div style={styles.tableWrapper}>
            <table style={styles.table}>
              <thead>
                <tr>
                  <th>Utilization</th>
                  <th>Borrow APY</th>
                  <th>Supply APY</th>
                  <th>Spread</th>
                  <th>Projected Protocol Yield</th>
                  <th>Liquidity Risk</th>
                </tr>
              </thead>
              <tbody>
                {prediction.points.map((pt) => (
                  <tr
                    key={pt.utilizationBps}
                    style={pt.utilizationBps === config.kinkUtilizationBps ? styles.kinkRow : {}}
                  >
                    <td>{pt.utilizationPercentage.toFixed(0)}%</td>
                    <td style={{ color: '#3B82F6', fontWeight: 600 }}>{pt.borrowRatePercentage.toFixed(2)}%</td>
                    <td style={{ color: '#10B981', fontWeight: 600 }}>{pt.supplyRatePercentage.toFixed(2)}%</td>
                    <td>{pt.protocolSpreadPercentage.toFixed(2)}%</td>
                    <td>{(pt.projectedRevenueBps / 100).toFixed(2)}% TVL</td>
                    <td>
                      <span
                        style={{
                          fontWeight: 600,
                          color: pt.liquidityRiskScore > 75 ? '#EF4444' : pt.liquidityRiskScore > 40 ? '#F59E0B' : '#10B981',
                        }}
                      >
                        {pt.liquidityRiskScore} / 100
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
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
    marginBottom: '24px',
  },
  summaryGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
    gap: '16px',
    marginBottom: '24px',
  },
  card: {
    backgroundColor: '#1E293B',
    padding: '16px',
    borderRadius: '8px',
    display: 'flex',
    flexDirection: 'column',
    gap: '8px',
  },
  cardTitle: {
    fontSize: '12px',
    color: '#94A3B8',
    textTransform: 'uppercase',
  },
  cardValue: {
    fontSize: '22px',
    fontWeight: 700,
    color: '#38BDF8',
  },
  badge: {
    alignSelf: 'flex-start',
    padding: '4px 10px',
    borderRadius: '12px',
    fontSize: '12px',
    fontWeight: 700,
    color: '#FFF',
  },
  section: {
    backgroundColor: '#1E293B',
    padding: '20px',
    borderRadius: '8px',
    marginBottom: '24px',
  },
  controlsGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))',
    gap: '16px',
    marginBottom: '16px',
  },
  controlGroup: {
    display: 'flex',
    flexDirection: 'column',
    gap: '6px',
    fontSize: '14px',
  },
  select: {
    padding: '8px',
    borderRadius: '6px',
    backgroundColor: '#0F172A',
    color: '#FFF',
    border: '1px solid #334155',
  },
  optimizeSection: {
    marginTop: '16px',
    paddingTop: '16px',
    borderTop: '1px solid #334155',
  },
  inputNumber: {
    width: '80px',
    padding: '6px',
    borderRadius: '6px',
    backgroundColor: '#0F172A',
    color: '#FFF',
    border: '1px solid #334155',
  },
  button: {
    padding: '8px 16px',
    backgroundColor: '#3B82F6',
    color: '#FFF',
    border: 'none',
    borderRadius: '6px',
    cursor: 'pointer',
    fontWeight: 600,
  },
  optGainText: {
    color: '#34D399',
    marginTop: '8px',
    fontWeight: 600,
  },
  tableWrapper: {
    overflowX: 'auto',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    textAlign: 'left',
    fontSize: '14px',
  },
  kinkRow: {
    backgroundColor: 'rgba(59, 130, 246, 0.15)',
    fontWeight: 'bold',
  },
};

export default YieldCurveDashboard;
