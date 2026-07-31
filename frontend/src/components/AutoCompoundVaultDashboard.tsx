import React, { useState, useEffect, useCallback } from 'react';

interface VaultConfig {
  performanceFeeBps: number;
  managementFeeBps: number;
  harvestIntervalSecs: number;
  slippageToleranceBps: number;
  depositPaused: boolean;
  withdrawPaused: boolean;
  active: boolean;
}

interface VaultSnapshot {
  totalAssets: string;
  totalShares: string;
  sharePrice: string;
  lastHarvestedAt: number;
  accruedManagementFees: string;
  accruedPerformanceFees: string;
}

interface ApyBoostResult {
  manualApy: number;
  autoApy: number;
  boostBps: number;
}

export const AutoCompoundVaultDashboard: React.FC = () => {
  const [config, setConfig] = useState<VaultConfig | null>(null);
  const [snapshot, setSnapshot] = useState<VaultSnapshot | null>(null);
  const [apyBoost, setApyBoost] = useState<ApyBoostResult | null>(null);
  const [selectedInterval, setSelectedInterval] = useState('daily');

  const loadData = useCallback(async () => {
    try {
      const [configRes, snapshotRes] = await Promise.all([
        fetch('/api/vault/config'),
        fetch('/api/vault/snapshot'),
      ]);
      const configData = await configRes.json();
      const snapshotData = await snapshotRes.json();
      if (configData.success) setConfig(configData.config);
      if (snapshotData.success) setSnapshot(snapshotData.snapshot);
    } catch (err) {
      console.error('Failed to load vault data', err);
    }
  }, []);

  const loadApyBoost = useCallback(async () => {
    try {
      const res = await fetch(`/api/vault/apy-boost?interval=${selectedInterval}`);
      const data = await res.json();
      if (data.success) setApyBoost(data);
    } catch (err) {
      console.error('Failed to load APY boost', err);
    }
  }, [selectedInterval]);

  useEffect(() => { loadData(); }, [loadData]);
  useEffect(() => { loadApyBoost(); }, [loadApyBoost]);

  const formatSecs = (secs: number): string => {
    if (secs < 3600) return `${Math.floor(secs / 60)}m`;
    if (secs < 86400) return `${Math.floor(secs / 3600)}h`;
    return `${Math.floor(secs / 86400)}d`;
  };

  const formatBps = (bps: number): string => `${(bps / 100).toFixed(2)}%`;

  return (
    <div style={styles.container}>
      <h2 style={styles.title}>Auto-Compounding Vault</h2>

      {config && (
        <div style={styles.configCard}>
          <h3 style={styles.sectionTitle}>Vault Configuration</h3>
          <div style={styles.configGrid}>
            <div style={styles.configItem}>
              <span style={styles.configLabel}>Status</span>
              <span style={{ ...styles.configValue, color: config.active ? '#2e7d32' : '#d32f2f' }}>
                {config.active ? 'Active' : 'Inactive'}
              </span>
            </div>
            <div style={styles.configItem}>
              <span style={styles.configLabel}>Performance Fee</span>
              <span style={styles.configValue}>{formatBps(config.performanceFeeBps)}</span>
            </div>
            <div style={styles.configItem}>
              <span style={styles.configLabel}>Management Fee</span>
              <span style={styles.configValue}>{formatBps(config.managementFeeBps)}</span>
            </div>
            <div style={styles.configItem}>
              <span style={styles.configLabel}>Harvest Interval</span>
              <span style={styles.configValue}>{formatSecs(config.harvestIntervalSecs)}</span>
            </div>
            <div style={styles.configItem}>
              <span style={styles.configLabel}>Slippage Tolerance</span>
              <span style={styles.configValue}>{formatBps(config.slippageToleranceBps)}</span>
            </div>
            <div style={styles.configItem}>
              <span style={styles.configLabel}>Deposits</span>
              <span style={{ ...styles.configValue, color: config.depositPaused ? '#d32f2f' : '#2e7d32' }}>
                {config.depositPaused ? 'Paused' : 'Open'}
              </span>
            </div>
          </div>
        </div>
      )}

      {snapshot && (
        <div style={styles.snapshotCard}>
          <h3 style={styles.sectionTitle}>Vault Snapshot</h3>
          <div style={styles.statsGrid}>
            <div style={styles.statCard}>
              <div style={styles.statLabel}>Total Assets</div>
              <div style={styles.statValue}>
                {parseInt(snapshot.totalAssets).toLocaleString()}
              </div>
            </div>
            <div style={styles.statCard}>
              <div style={styles.statLabel}>Total Shares</div>
              <div style={styles.statValue}>
                {parseInt(snapshot.totalShares).toLocaleString()}
              </div>
            </div>
            <div style={styles.statCard}>
              <div style={styles.statLabel}>Share Price</div>
              <div style={styles.statValue}>
                {(parseInt(snapshot.sharePrice) / 1000000).toFixed(6)}
              </div>
            </div>
            <div style={styles.statCard}>
              <div style={styles.statLabel}>Last Harvest</div>
              <div style={styles.statValue}>
                {snapshot.lastHarvestedAt
                  ? new Date(snapshot.lastHarvestedAt * 1000).toLocaleDateString()
                  : 'Never'}
              </div>
            </div>
          </div>
          <div style={styles.feesRow}>
            <div style={styles.feeItem}>
              <span style={styles.feeLabel}>Accrued Mgmt Fees</span>
              <span style={styles.feeValue}>{parseInt(snapshot.accruedManagementFees).toLocaleString()}</span>
            </div>
            <div style={styles.feeItem}>
              <span style={styles.feeLabel}>Accrued Perf Fees</span>
              <span style={styles.feeValue}>{parseInt(snapshot.accruedPerformanceFees).toLocaleString()}</span>
            </div>
          </div>
        </div>
      )}

      {apyBoost && (
        <div style={styles.apyCard}>
          <h3 style={styles.sectionTitle}>APY Comparison</h3>
          <div style={styles.intervalSelector}>
            {['hourly', 'daily', 'weekly'].map((interval) => (
              <button
                key={interval}
                onClick={() => setSelectedInterval(interval)}
                style={selectedInterval === interval ? styles.intervalActive : styles.intervalButton}
              >
                {interval.charAt(0).toUpperCase() + interval.slice(1)}
              </button>
            ))}
          </div>
          <div style={styles.apyGrid}>
            <div style={styles.apyItem}>
              <div style={styles.apyLabel}>Manual APY</div>
              <div style={styles.apyValue}>{apyBoost.manualApy.toFixed(2)}%</div>
            </div>
            <div style={styles.apyItem}>
              <div style={styles.apyLabel}>Auto-Compound APY</div>
              <div style={{ ...styles.apyValue, color: '#0066ff' }}>{apyBoost.autoApy.toFixed(2)}%</div>
            </div>
            <div style={styles.apyItem}>
              <div style={styles.apyLabel}>Boost</div>
              <div style={{ ...styles.apyValue, color: '#2e7d32' }}>+{apyBoost.boostBps} bps</div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  container: { padding: '24px', maxWidth: '1000px', margin: '0 auto', fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif' },
  title: { fontSize: '24px', fontWeight: 600, marginBottom: '24px', color: '#1a1a2e' },
  sectionTitle: { fontSize: '16px', fontWeight: 600, marginBottom: '16px', color: '#1a1a2e' },
  configCard: { backgroundColor: '#fff', borderRadius: '12px', border: '1px solid #e0e0e0', padding: '20px', marginBottom: '16px' },
  configGrid: { display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '12px' },
  configItem: { display: 'flex', flexDirection: 'column', gap: '4px' },
  configLabel: { fontSize: '11px', color: '#666', textTransform: 'uppercase', fontWeight: 600 },
  configValue: { fontSize: '16px', fontWeight: 600, color: '#1a1a2e' },
  snapshotCard: { backgroundColor: '#fff', borderRadius: '12px', border: '1px solid #e0e0e0', padding: '20px', marginBottom: '16px' },
  statsGrid: { display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '16px', marginBottom: '16px' },
  statCard: { padding: '16px', backgroundColor: '#f8f9fa', borderRadius: '8px', textAlign: 'center' },
  statLabel: { fontSize: '11px', color: '#666', marginBottom: '4px', textTransform: 'uppercase', fontWeight: 600 },
  statValue: { fontSize: '18px', fontWeight: 700, color: '#1a1a2e' },
  feesRow: { display: 'flex', gap: '16px' },
  feeItem: { display: 'flex', justifyContent: 'space-between', padding: '8px 12px', backgroundColor: '#fff3e0', borderRadius: '6px', flex: 1 },
  feeLabel: { fontSize: '12px', color: '#e65100', fontWeight: 500 },
  feeValue: { fontSize: '14px', fontWeight: 700, color: '#e65100' },
  apyCard: { backgroundColor: '#fff', borderRadius: '12px', border: '1px solid #e0e0e0', padding: '20px' },
  intervalSelector: { display: 'flex', gap: '8px', marginBottom: '16px' },
  intervalButton: { padding: '6px 16px', backgroundColor: '#f5f5f5', border: '1px solid #ddd', borderRadius: '6px', fontSize: '13px', cursor: 'pointer' },
  intervalActive: { padding: '6px 16px', backgroundColor: '#0066ff', color: '#fff', border: '1px solid #0066ff', borderRadius: '6px', fontSize: '13px', cursor: 'pointer' },
  apyGrid: { display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '16px' },
  apyItem: { textAlign: 'center', padding: '16px', backgroundColor: '#f8f9fa', borderRadius: '8px' },
  apyLabel: { fontSize: '12px', color: '#666', marginBottom: '8px', fontWeight: 600, textTransform: 'uppercase' },
  apyValue: { fontSize: '24px', fontWeight: 700, color: '#1a1a2e' },
};