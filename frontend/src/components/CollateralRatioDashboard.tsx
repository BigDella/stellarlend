import React, { useState, useEffect } from 'react';

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

interface HistoricalTrend {
  asset: string;
  timestamp: number;
  avgHealthFactor: number;
  minHealthFactor: number;
  maxHealthFactor: number;
  positionCount: number;
  dangerCount: number;
  criticalCount: number;
}

interface AssetMetrics {
  asset: string;
  totalCollateralValue: string;
  totalDebtValue: string;
  avgHealthFactor: number;
  minHealthFactor: number;
  maxHealthFactor: number;
  positionCount: number;
  riskDistribution: {
    safe: number;
    warning: number;
    danger: number;
    critical: number;
  };
  timestamp: number;
}

const COLORS = {
  safe: '#28a745',
  warning: '#ffc107',
  danger: '#fd7e14',
  critical: '#dc3545',
  background: '#f8f9fa',
  card: '#ffffff',
  text: '#333333',
  border: '#dee2e6',
};

const getRiskColor = (level: string): string => {
  return COLORS[level as keyof typeof COLORS] || COLORS.safe;
};

const getSeverityColor = (severity: string): string => {
  const map: Record<string, string> = {
    low: '#17a2b8',
    medium: '#ffc107',
    high: '#fd7e14',
    critical: '#dc3545',
  };
  return map[severity] || '#6c757d';
};

export const CollateralRatioDashboard: React.FC = () => {
  const [snapshots, setSnapshots] = useState<CollateralRatioSnapshot[]>([]);
  const [alerts, setAlerts] = useState<RiskAlert[]>([]);
  const [selectedAsset, setSelectedAsset] = useState<string>('XLM');
  const [historicalTrends, setHistoricalTrends] = useState<HistoricalTrend[]>([]);
  const [assetMetrics, setAssetMetrics] = useState<AssetMetrics[]>([]);
  const [wsConnected, setWsConnected] = useState(false);
  const [activeWidgets, setActiveWidgets] = useState({
    ratios: true,
    alerts: true,
    trends: true,
    metrics: true,
  });

  useEffect(() => {
    fetchInitialData();
    connectWebSocket();
    
    return () => {
      disconnectWebSocket();
    };
  }, []);

  const fetchInitialData = async () => {
    try {
      const [snapshotsRes, alertsRes, metricsRes] = await Promise.all([
        fetch('/api/collateral-ratio/snapshots'),
        fetch('/api/collateral-ratio/alerts'),
        fetch('/api/collateral-ratio/metrics'),
      ]);

      const snapshotsData = await snapshotsRes.json();
      const alertsData = await alertsRes.json();
      const metricsData = await metricsRes.json();

      setSnapshots(snapshotsData);
      setAlerts(alertsData);
      setAssetMetrics(metricsData);

      if (snapshotsData.length > 0) {
        fetchHistoricalTrends(snapshotsData[0].asset);
      }
    } catch (error) {
      console.error('Failed to fetch initial data', error);
    }
  };

  const fetchHistoricalTrends = async (asset: string) => {
    try {
      const res = await fetch(`/api/collateral-ratio/trends/${asset}?hours=24`);
      const data = await res.json();
      setHistoricalTrends(data);
    } catch (error) {
      console.error('Failed to fetch historical trends', error);
    }
  };

  const connectWebSocket = () => {
    const ws = new WebSocket(`ws://${window.location.host}/api/ws/collateral-ratios?alerts=true&positions=true`);

    ws.onopen = () => {
      setWsConnected(true);
      console.log('WebSocket connected');
    };

    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      
      switch (message.type) {
        case 'initial_snapshot':
          setSnapshots(message.snapshots);
          break;
        case 'ratio_update':
          setSnapshots((prev) => {
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
          setAlerts((prev) => [message.alert, ...prev].slice(0, 50));
          break;
        case 'position_update':
          // Handle position updates if needed
          break;
      }
    };

    ws.onclose = () => {
      setWsConnected(false);
      console.log('WebSocket disconnected');
      // Reconnect after 5 seconds
      setTimeout(connectWebSocket, 5000);
    };

    ws.onerror = (error) => {
      console.error('WebSocket error', error);
    };

    return () => ws.close();
  };

  const disconnectWebSocket = () => {
    // WebSocket cleanup is handled in the return callback
  };

  const handleAssetChange = (asset: string) => {
    setSelectedAsset(asset);
    fetchHistoricalTrends(asset);
  };

  const acknowledgeAlert = async (alertId: string) => {
    try {
      await fetch(`/api/collateral-ratio/alerts/${alertId}/acknowledge`, { method: 'POST' });
      setAlerts((prev) => prev.map((a) => (a.id === alertId ? { ...a, acknowledged: true } : a)));
    } catch (error) {
      console.error('Failed to acknowledge alert', error);
    }
  };

  const toggleWidget = (widget: keyof typeof activeWidgets) => {
    setActiveWidgets((prev) => ({ ...prev, [widget]: !prev[widget] }));
  };

  const formatNumber = (num: number): string => {
    return new Intl.NumberFormat('en-US', { maximumFractionDigits: 2 }).format(num);
  };

  const formatCurrency = (value: string): string => {
    const num = parseFloat(value);
    return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(num);
  };

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <h1>Real-time Collateral Ratio Monitoring</h1>
        <div style={styles.status}>
          <span style={{ ...styles.statusDot, backgroundColor: wsConnected ? COLORS.safe : COLORS.critical }} />
          {wsConnected ? 'Connected' : 'Disconnected'}
        </div>
      </div>

      <div style={styles.widgetControls}>
        <button
          onClick={() => toggleWidget('ratios')}
          style={{ ...styles.widgetBtn, opacity: activeWidgets.ratios ? 1 : 0.5 }}
        >
          Collateral Ratios
        </button>
        <button
          onClick={() => toggleWidget('alerts')}
          style={{ ...styles.widgetBtn, opacity: activeWidgets.alerts ? 1 : 0.5 }}
        >
          Alerts
        </button>
        <button
          onClick={() => toggleWidget('trends')}
          style={{ ...styles.widgetBtn, opacity: activeWidgets.trends ? 1 : 0.5 }}
        >
          Historical Trends
        </button>
        <button
          onClick={() => toggleWidget('metrics')}
          style={{ ...styles.widgetBtn, opacity: activeWidgets.metrics ? 1 : 0.5 }}
        >
          Asset Metrics
        </button>
      </div>

      {activeWidgets.ratios && (
        <div style={styles.section}>
          <h2>Current Collateral Ratios</h2>
          <div style={styles.grid}>
            {snapshots.map((snapshot) => (
              <div
                key={snapshot.asset}
                style={{
                  ...styles.card,
                  borderLeft: `4px solid ${getRiskColor(snapshot.riskLevel)}`,
                }}
                onClick={() => handleAssetChange(snapshot.asset)}
              >
                <div style={styles.cardHeader}>
                  <h3>{snapshot.asset}</h3>
                  <span style={{ ...styles.badge, backgroundColor: getRiskColor(snapshot.riskLevel) }}>
                    {snapshot.riskLevel.toUpperCase()}
                  </span>
                </div>
                <div style={styles.metricRow}>
                  <span style={styles.metricLabel}>Health Factor:</span>
                  <span style={{ ...styles.metricValue, color: getRiskColor(snapshot.riskLevel) }}>
                    {formatNumber(snapshot.healthFactor)}
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
                  <span style={styles.metricValue}>{formatCurrency(snapshot.collateralValue)}</span>
                </div>
                <div style={styles.metricRow}>
                  <span style={styles.metricLabel}>Debt:</span>
                  <span style={styles.metricValue}>{formatCurrency(snapshot.debtValue)}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {activeWidgets.alerts && (
        <div style={styles.section}>
          <h2>Risk Alerts</h2>
          <div style={styles.alertList}>
            {alerts.slice(0, 10).map((alert) => (
              <div
                key={alert.id}
                style={{
                  ...styles.alertCard,
                  borderLeft: `4px solid ${getSeverityColor(alert.severity)}`,
                  opacity: alert.acknowledged ? 0.6 : 1,
                }}
              >
                <div style={styles.alertHeader}>
                  <span style={{ ...styles.badge, backgroundColor: getSeverityColor(alert.severity) }}>
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
        </div>
      )}

      {activeWidgets.trends && (
        <div style={styles.section}>
          <div style={styles.sectionHeader}>
            <h2>Historical Risk Trends</h2>
            <select
              value={selectedAsset}
              onChange={(e) => handleAssetChange(e.target.value)}
              style={styles.select}
            >
              {snapshots.map((s) => (
                <option key={s.asset} value={s.asset}>
                  {s.asset}
                </option>
              ))}
            </select>
          </div>
          <div style={styles.chartContainer}>
            <div style={styles.trendChart}>
              {historicalTrends.map((trend, index) => (
                <div key={index} style={styles.trendBar}>
                  <div
                    style={{
                      ...styles.trendBarFill,
                      height: `${(trend.avgHealthFactor / 3) * 100}%`,
                      backgroundColor: getRiskColor(
                        trend.avgHealthFactor >= 2 ? 'safe' : trend.avgHealthFactor >= 1.5 ? 'warning' : trend.avgHealthFactor >= 1.1 ? 'danger' : 'critical'
                      ),
                    }}
                  />
                  <div style={styles.trendLabel}>
                    {new Date(trend.timestamp).toLocaleTimeString()}
                  </div>
                </div>
              ))}
            </div>
            <div style={styles.trendLegend}>
              <div style={styles.legendItem}>
                <span style={{ ...styles.legendDot, backgroundColor: COLORS.safe }} />
                Safe (≥2.0)
              </div>
              <div style={styles.legendItem}>
                <span style={{ ...styles.legendDot, backgroundColor: COLORS.warning }} />
                Warning (≥1.5)
              </div>
              <div style={styles.legendItem}>
                <span style={{ ...styles.legendDot, backgroundColor: COLORS.danger }} />
                Danger (≥1.1)
              </div>
              <div style={styles.legendItem}>
                <span style={{ ...styles.legendDot, backgroundColor: COLORS.critical }} />
                Critical (&lt;1.1)
              </div>
            </div>
          </div>
        </div>
      )}

      {activeWidgets.metrics && (
        <div style={styles.section}>
          <h2>Asset Risk Metrics</h2>
          <div style={styles.metricsTable}>
            <div style={styles.tableHeader}>
              <span>Asset</span>
              <span>Avg HF</span>
              <span>Min HF</span>
              <span>Max HF</span>
              <span>Positions</span>
              <span>Risk Dist</span>
            </div>
            {assetMetrics.map((metrics) => (
              <div key={metrics.asset} style={styles.tableRow}>
                <span style={styles.tableCell}>{metrics.asset}</span>
                <span style={styles.tableCell}>{formatNumber(metrics.avgHealthFactor)}</span>
                <span style={styles.tableCell}>{formatNumber(metrics.minHealthFactor)}</span>
                <span style={styles.tableCell}>{formatNumber(metrics.maxHealthFactor)}</span>
                <span style={styles.tableCell}>{metrics.positionCount}</span>
                <span style={styles.tableCell}>
                  <span style={{ color: COLORS.safe }}>S:{metrics.riskDistribution.safe}</span>
                  <span style={{ color: COLORS.warning }}>W:{metrics.riskDistribution.warning}</span>
                  <span style={{ color: COLORS.danger }}>D:{metrics.riskDistribution.danger}</span>
                  <span style={{ color: COLORS.critical }}>C:{metrics.riskDistribution.critical}</span>
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  container: {
    maxWidth: 1400,
    margin: '0 auto',
    padding: 24,
    backgroundColor: COLORS.background,
    minHeight: '100vh',
  },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 24,
  },
  status: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    padding: '8px 16px',
    backgroundColor: COLORS.card,
    borderRadius: 4,
    border: `1px solid ${COLORS.border}`,
  },
  statusDot: {
    width: 10,
    height: 10,
    borderRadius: '50%',
  },
  widgetControls: {
    display: 'flex',
    gap: 12,
    marginBottom: 24,
    flexWrap: 'wrap',
  },
  widgetBtn: {
    padding: '8px 16px',
    backgroundColor: COLORS.card,
    border: `1px solid ${COLORS.border}`,
    borderRadius: 4,
    cursor: 'pointer',
    transition: 'opacity 0.2s',
  },
  section: {
    marginBottom: 32,
  },
  sectionHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 16,
  },
  grid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
    gap: 16,
  },
  card: {
    backgroundColor: COLORS.card,
    borderRadius: 8,
    padding: 16,
    boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
  },
  cardHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  badge: {
    padding: '4px 8px',
    borderRadius: 4,
    color: 'white',
    fontSize: 12,
    fontWeight: 'bold',
  },
  metricRow: {
    display: 'flex',
    justifyContent: 'space-between',
    marginBottom: 8,
  },
  metricLabel: {
    color: '#666',
    fontSize: 14,
  },
  metricValue: {
    fontWeight: 'bold',
    fontSize: 14,
  },
  alertList: {
    display: 'flex',
    flexDirection: 'column',
    gap: 12,
  },
  alertCard: {
    backgroundColor: COLORS.card,
    borderRadius: 8,
    padding: 16,
    boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
  },
  alertHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 8,
  },
  alertTime: {
    fontSize: 12,
    color: '#666',
  },
  alertMessage: {
    margin: '8px 0',
    color: COLORS.text,
  },
  alertFooter: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  alertAsset: {
    fontWeight: 'bold',
    color: COLORS.text,
  },
  ackBtn: {
    padding: '6px 12px',
    backgroundColor: COLORS.safe,
    color: 'white',
    border: 'none',
    borderRadius: 4,
    cursor: 'pointer',
  },
  select: {
    padding: '8px 12px',
    borderRadius: 4,
    border: `1px solid ${COLORS.border}`,
  },
  chartContainer: {
    backgroundColor: COLORS.card,
    borderRadius: 8,
    padding: 20,
    boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
  },
  trendChart: {
    display: 'flex',
    gap: 4,
    height: 200,
    alignItems: 'flex-end',
    marginBottom: 16,
  },
  trendBar: {
    flex: 1,
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
  },
  trendBarFill: {
    width: '100%',
    borderRadius: '4px 4px 0 0',
    transition: 'height 0.3s',
  },
  trendLabel: {
    fontSize: 10,
    color: '#666',
    marginTop: 4,
    transform: 'rotate(-45deg)',
  },
  trendLegend: {
    display: 'flex',
    gap: 16,
    justifyContent: 'center',
  },
  legendItem: {
    display: 'flex',
    alignItems: 'center',
    gap: 6,
    fontSize: 12,
  },
  legendDot: {
    width: 10,
    height: 10,
    borderRadius: '50%',
  },
  metricsTable: {
    backgroundColor: COLORS.card,
    borderRadius: 8,
    padding: 16,
    boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
  },
  tableHeader: {
    display: 'grid',
    gridTemplateColumns: 'repeat(6, 1fr)',
    padding: '12px 8px',
    borderBottom: `2px solid ${COLORS.border}`,
    fontWeight: 'bold',
  },
  tableRow: {
    display: 'grid',
    gridTemplateColumns: 'repeat(6, 1fr)',
    padding: '12px 8px',
    borderBottom: `1px solid ${COLORS.border}`,
  },
  tableCell: {
    display: 'flex',
    alignItems: 'center',
    gap: 4,
  },
};
