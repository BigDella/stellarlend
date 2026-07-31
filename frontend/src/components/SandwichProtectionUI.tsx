import React, { useState, useEffect } from 'react';

type ProtectionLevel = 'None' | 'Basic' | 'Max';

interface SandwichConfig {
  min_delay_secs: number;
  max_delay_secs: number;
  commit_expiry_secs: number;
  large_tx_threshold: number;
  premium_fee_bps: number;
}

interface SandwichDetection {
  suspect_address: string;
  victim_address: string;
  asset: string;
  block_number: number;
  timestamp: number;
  estimated_profit: number;
  reversed: boolean;
}

interface PendingTx {
  index: number;
  owner: string;
  operation_type: number;
  asset: string;
  amount: number;
  protection_level: ProtectionLevel;
}

export const SandwichProtectionUI: React.FC = () => {
  const [config, setConfig] = useState<SandwichConfig | null>(null);
  const [userProtection, setUserProtection] = useState<ProtectionLevel>('Basic');
  const [detections, setDetections] = useState<SandwichDetection[]>([]);
  const [executionOrder, setExecutionOrder] = useState<number[]>([]);
  const [pendingTxCount, setPendingTxCount] = useState(0);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    setLoading(true);
    try {
      const [configRes, detectionRes, orderRes] = await Promise.all([
        fetch('/api/sandwich/config'),
        fetch('/api/sandwich/detections'),
        fetch('/api/sandwich/execution-order'),
      ]);
      if (configRes.ok) setConfig(await configRes.json());
      if (detectionRes.ok) setDetections(await detectionRes.json());
      if (orderRes.ok) {
        const data = await orderRes.json();
        setExecutionOrder(data.order || []);
        setPendingTxCount(data.count || 0);
      }
    } catch (error) {
      console.error('Failed to load sandwich protection data:', error);
    }
    setLoading(false);
  };

  const handleSetProtection = async (level: ProtectionLevel) => {
    try {
      const res = await fetch('/api/sandwich/protection', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ level }),
      });
      if (res.ok) {
        setUserProtection(level);
      }
    } catch (error) {
      console.error('Failed to set protection level:', error);
    }
  };

  const getProtectionColor = (level: ProtectionLevel): string => {
    switch (level) {
      case 'Max':
        return '#28a745';
      case 'Basic':
        return '#ffc107';
      case 'None':
        return '#dc3545';
      default:
        return '#6c757d';
    }
  };

  const getProtectionDescription = (level: ProtectionLevel): string => {
    switch (level) {
      case 'Max':
        return 'Commit-reveal + delay + batch ordering + detection';
      case 'Basic':
        return 'Delay + randomized batch ordering + detection';
      case 'None':
        return 'No protection (not recommended)';
      default:
        return '';
    }
  };

  return (
    <div style={styles.container}>
      <h2>Sandwich Attack Protection</h2>

      <div style={styles.configSection}>
        <h3>Protection Levels</h3>
        <div style={styles.levelCards}>
          {(['None', 'Basic', 'Max'] as ProtectionLevel[]).map((level) => (
            <div
              key={level}
              style={{
                ...styles.levelCard,
                borderColor: userProtection === level ? getProtectionColor(level) : '#e0e0e0',
                borderWidth: userProtection === level ? '2px' : '1px',
              }}
              onClick={() => handleSetProtection(level)}
            >
              <div
                style={{
                  ...styles.levelBadge,
                  backgroundColor: getProtectionColor(level),
                }}
              >
                {level}
              </div>
              <p style={styles.levelDescription}>{getProtectionDescription(level)}</p>
              {userProtection === level && (
                <div style={styles.activeLabel}>ACTIVE</div>
              )}
            </div>
          ))}
        </div>
      </div>

      {config && (
        <div style={styles.configSection}>
          <h3>Configuration</h3>
          <div style={styles.configGrid}>
            <div style={styles.configItem}>
              <span style={styles.configLabel}>Min Delay:</span>
              <span>{config.min_delay_secs}s</span>
            </div>
            <div style={styles.configItem}>
              <span style={styles.configLabel}>Max Delay:</span>
              <span>{config.max_delay_secs}s</span>
            </div>
            <div style={styles.configItem}>
              <span style={styles.configLabel}>Commit Expiry:</span>
              <span>{config.commit_expiry_secs}s</span>
            </div>
            <div style={styles.configItem}>
              <span style={styles.configLabel}>Large Tx Threshold:</span>
              <span>{config.large_tx_threshold.toLocaleString()}</span>
            </div>
            <div style={styles.configItem}>
              <span style={styles.configLabel}>Premium Fee:</span>
              <span>{(config.premium_fee_bps / 100).toFixed(2)}%</span>
            </div>
          </div>
        </div>
      )}

      <div style={styles.statsRow}>
        <div style={styles.statCard}>
          <h3>Pending Txs</h3>
          <div style={styles.statValue}>{pendingTxCount}</div>
        </div>
        <div style={styles.statCard}>
          <h3>Detections</h3>
          <div style={styles.statValue}>{detections.length}</div>
        </div>
        <div style={styles.statCard}>
          <h3>Reversed</h3>
          <div style={styles.statValue}>
            {detections.filter((d) => d.reversed).length}
          </div>
        </div>
      </div>

      {executionOrder.length > 0 && (
        <div style={styles.section}>
          <h3>Current Block Execution Order (Randomized)</h3>
          <div style={styles.orderList}>
            {executionOrder.map((idx, pos) => (
              <div key={idx} style={styles.orderItem}>
                <span style={styles.orderPosition}>#{pos + 1}</span>
                <span>Tx Index: {idx}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      <div style={styles.section}>
        <h3>Sandwich Detection Log</h3>
        <table style={styles.table}>
          <thead>
            <tr>
              <th>Suspect</th>
              <th>Victim</th>
              <th>Asset</th>
              <th>Est. Profit</th>
              <th>Reversed</th>
              <th>Block</th>
            </tr>
          </thead>
          <tbody>
            {detections.map((detection, idx) => (
              <tr key={idx}>
                <td style={styles.addressCell}>{detection.suspect_address.slice(0, 10)}...</td>
                <td style={styles.addressCell}>{detection.victim_address.slice(0, 10)}...</td>
                <td>{detection.asset.slice(0, 8)}...</td>
                <td>{detection.estimated_profit.toLocaleString()}</td>
                <td style={{ color: detection.reversed ? '#28a745' : '#dc3545' }}>
                  {detection.reversed ? '✓ Yes' : '✗ No'}
                </td>
                <td>{detection.block_number}</td>
              </tr>
            ))}
          </tbody>
        </table>
        {detections.length === 0 && (
          <p style={styles.emptyText}>No sandwich attacks detected.</p>
        )}
      </div>
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  container: {
    padding: '20px',
    maxWidth: '1200px',
    margin: '0 auto',
  },
  configSection: {
    marginBottom: '20px',
    padding: '15px',
    backgroundColor: '#f8f9fa',
    borderRadius: '8px',
    border: '1px solid #e0e0e0',
  },
  levelCards: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    gap: '15px',
    marginTop: '10px',
  },
  levelCard: {
    padding: '15px',
    backgroundColor: 'white',
    borderRadius: '8px',
    border: '1px solid #e0e0e0',
    cursor: 'pointer',
    textAlign: 'center',
    position: 'relative',
  },
  levelBadge: {
    display: 'inline-block',
    padding: '4px 12px',
    borderRadius: '4px',
    color: 'white',
    fontWeight: 'bold',
    fontSize: '14px',
    marginBottom: '8px',
  },
  levelDescription: {
    fontSize: '12px',
    color: '#6c757d',
    margin: '8px 0 0 0',
  },
  activeLabel: {
    position: 'absolute',
    top: '8px',
    right: '8px',
    fontSize: '10px',
    fontWeight: 'bold',
    color: '#28a745',
  },
  configGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(5, 1fr)',
    gap: '10px',
    marginTop: '10px',
  },
  configItem: {
    display: 'flex',
    flexDirection: 'column',
    padding: '8px',
    backgroundColor: 'white',
    borderRadius: '4px',
    border: '1px solid #e0e0e0',
  },
  configLabel: {
    fontSize: '11px',
    color: '#6c757d',
    marginBottom: '4px',
  },
  statsRow: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    gap: '15px',
    marginBottom: '20px',
  },
  statCard: {
    padding: '15px',
    backgroundColor: '#f8f9fa',
    borderRadius: '8px',
    border: '1px solid #e0e0e0',
    textAlign: 'center',
  },
  statValue: {
    fontSize: '24px',
    fontWeight: 'bold',
    color: '#333',
    marginTop: '5px',
  },
  section: {
    marginBottom: '20px',
    padding: '15px',
    backgroundColor: '#f8f9fa',
    borderRadius: '8px',
    border: '1px solid #e0e0e0',
  },
  orderList: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '8px',
    marginTop: '10px',
  },
  orderItem: {
    display: 'flex',
    gap: '6px',
    padding: '6px 10px',
    backgroundColor: 'white',
    borderRadius: '4px',
    border: '1px solid #e0e0e0',
    fontSize: '13px',
  },
  orderPosition: {
    fontWeight: 'bold',
    color: '#007bff',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    fontSize: '13px',
  },
  addressCell: {
    fontFamily: 'monospace',
    fontSize: '12px',
  },
  emptyText: {
    textAlign: 'center',
    color: '#6c757d',
    fontStyle: 'italic',
  },
};

export default SandwichProtectionUI;
