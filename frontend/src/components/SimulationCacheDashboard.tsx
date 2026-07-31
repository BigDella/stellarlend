import React, { useState, useEffect } from 'react';

interface SimCacheStats {
  hits: number;
  misses: number;
  evictions: number;
  total_entries: number;
  current_block: number;
}

interface SimCacheConfig {
  max_entries: number;
  enabled: boolean;
}

interface SimulationResult {
  health_after: number;
  collateral_value_after: number;
  debt_value_after: number;
  would_succeed: boolean;
  cached: boolean;
}

export const SimulationCacheDashboard: React.FC = () => {
  const [stats, setStats] = useState<SimCacheStats | null>(null);
  const [config, setConfig] = useState<SimCacheConfig | null>(null);
  const [loading, setLoading] = useState(false);

  // Manual cache lookup state
  const [lookupOp, setLookupOp] = useState('0');
  const [lookupPool, setLookupPool] = useState('');
  const [lookupUser, setLookupUser] = useState('');
  const [lookupAsset, setLookupAsset] = useState('');
  const [lookupAmount, setLookupAmount] = useState('');
  const [lookupResult, setLookupResult] = useState<SimulationResult | null>(null);

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    setLoading(true);
    try {
      const [statsRes, configRes] = await Promise.all([
        fetch('/api/sim-cache/stats'),
        fetch('/api/sim-cache/config'),
      ]);
      if (statsRes.ok) setStats(await statsRes.json());
      if (configRes.ok) setConfig(await configRes.json());
    } catch (error) {
      console.error('Failed to load simulation cache data:', error);
    }
    setLoading(false);
  };

  const handleLookup = async () => {
    if (!lookupPool || !lookupUser || !lookupAsset || !lookupAmount) return;
    try {
      const res = await fetch(
        `/api/sim-cache/lookup?op=${lookupOp}&pool=${lookupPool}&user=${lookupUser}&asset=${lookupAsset}&amount=${lookupAmount}`
      );
      if (res.ok) {
        const result: SimulationResult | null = await res.json();
        setLookupResult(result);
      }
    } catch (error) {
      console.error('Cache lookup failed:', error);
    }
  };

  const handleClearCache = async () => {
    try {
      const res = await fetch('/api/sim-cache/clear', { method: 'POST' });
      if (res.ok) {
        loadData();
      }
    } catch (error) {
      console.error('Cache clear failed:', error);
    }
  };

  const getHitRate = (): string => {
    if (!stats || stats.hits + stats.misses === 0) return '0%';
    return ((stats.hits / (stats.hits + stats.misses)) * 100).toFixed(1) + '%';
  };

  const operationNames: Record<string, string> = {
    '0': 'Deposit',
    '1': 'Withdraw',
    '2': 'Borrow',
    '3': 'Repay',
    '4': 'Liquidate',
  };

  return (
    <div style={styles.container}>
      <h2>Simulation Cache Dashboard</h2>

      <div style={styles.statsRow}>
        <div style={styles.statCard}>
          <h3>Cache Hits</h3>
          <div style={{ ...styles.statValue, color: '#28a745' }}>
            {stats?.hits ?? '—'}
          </div>
        </div>
        <div style={styles.statCard}>
          <h3>Cache Misses</h3>
          <div style={{ ...styles.statValue, color: '#dc3545' }}>
            {stats?.misses ?? '—'}
          </div>
        </div>
        <div style={styles.statCard}>
          <h3>Hit Rate</h3>
          <div style={styles.statValue}>{getHitRate()}</div>
        </div>
        <div style={styles.statCard}>
          <h3>Entries</h3>
          <div style={styles.statValue}>
            {stats?.total_entries ?? 0} / {config?.max_entries ?? 64}
          </div>
        </div>
        <div style={styles.statCard}>
          <h3>Evictions</h3>
          <div style={{ ...styles.statValue, color: '#ffc107' }}>
            {stats?.evictions ?? '—'}
          </div>
        </div>
        <div style={styles.statCard}>
          <h3>Current Block</h3>
          <div style={styles.statValue}>{stats?.current_block ?? '—'}</div>
        </div>
      </div>

      <div style={styles.section}>
        <h3>Cache Status</h3>
        <div style={styles.progressBar}>
          <div
            style={{
              ...styles.progressFill,
              width: `${((stats?.total_entries ?? 0) / (config?.max_entries ?? 64)) * 100}%`,
              backgroundColor:
                (stats?.total_entries ?? 0) / (config?.max_entries ?? 64) > 0.8
                  ? '#dc3545'
                  : '#28a745',
            }}
          />
        </div>
        <p style={styles.progressLabel}>
          {stats?.total_entries ?? 0} / {config?.max_entries ?? 64} slots used
          {config?.enabled === false && ' (CACHING DISABLED)'}
        </p>
      </div>

      <div style={styles.section}>
        <h3>Manual Cache Lookup</h3>
        <div style={styles.lookupGrid}>
          <div style={styles.lookupField}>
            <label style={styles.lookupLabel}>Operation:</label>
            <select
              value={lookupOp}
              onChange={(e) => setLookupOp(e.target.value)}
              style={styles.select}
            >
              {Object.entries(operationNames).map(([val, name]) => (
                <option key={val} value={val}>
                  {name}
                </option>
              ))}
            </select>
          </div>
          <div style={styles.lookupField}>
            <label style={styles.lookupLabel}>Pool:</label>
            <input
              type="text"
              value={lookupPool}
              onChange={(e) => setLookupPool(e.target.value)}
              placeholder="Pool address"
              style={styles.input}
            />
          </div>
          <div style={styles.lookupField}>
            <label style={styles.lookupLabel}>User:</label>
            <input
              type="text"
              value={lookupUser}
              onChange={(e) => setLookupUser(e.target.value)}
              placeholder="User address"
              style={styles.input}
            />
          </div>
          <div style={styles.lookupField}>
            <label style={styles.lookupLabel}>Asset:</label>
            <input
              type="text"
              value={lookupAsset}
              onChange={(e) => setLookupAsset(e.target.value)}
              placeholder="Asset address"
              style={styles.input}
            />
          </div>
          <div style={styles.lookupField}>
            <label style={styles.lookupLabel}>Amount:</label>
            <input
              type="number"
              value={lookupAmount}
              onChange={(e) => setLookupAmount(e.target.value)}
              placeholder="Amount"
              style={styles.input}
            />
          </div>
          <div style={styles.lookupActions}>
            <button onClick={handleLookup} style={styles.lookupBtn}>
              Lookup
            </button>
            <button onClick={handleClearCache} style={styles.clearBtn}>
              Clear Cache
            </button>
          </div>
        </div>

        {lookupResult !== null && (
          <div style={styles.lookupResult}>
            <h4>Cache Lookup Result</h4>
            {lookupResult === null ? (
              <p style={styles.cacheMiss}>Cache MISS — no cached result found.</p>
            ) : (
              <div style={styles.resultGrid}>
                <div style={styles.resultItem}>
                  <span style={styles.resultLabel}>Status:</span>
                  <span style={lookupResult.cached ? styles.cacheHit : styles.cacheMiss}>
                    {lookupResult.cached ? 'HIT' : 'MISS'}
                  </span>
                </div>
                <div style={styles.resultItem}>
                  <span style={styles.resultLabel}>Would Succeed:</span>
                  <span style={{ color: lookupResult.would_succeed ? '#28a745' : '#dc3545' }}>
                    {lookupResult.would_succeed ? 'Yes' : 'No'}
                  </span>
                </div>
                <div style={styles.resultItem}>
                  <span style={styles.resultLabel}>Health After:</span>
                  <span>{(lookupResult.health_after / 10000).toFixed(4)}</span>
                </div>
                <div style={styles.resultItem}>
                  <span style={styles.resultLabel}>Collateral Value After:</span>
                  <span>{lookupResult.collateral_value_after.toLocaleString()}</span>
                </div>
                <div style={styles.resultItem}>
                  <span style={styles.resultLabel}>Debt Value After:</span>
                  <span>{lookupResult.debt_value_after.toLocaleString()}</span>
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      <div style={styles.section}>
        <h3>Gas Savings Indicator</h3>
        <p style={styles.description}>
          Identical simulations within the same block reuse cached results, saving
          compute gas. The cache is automatically invalidated on new blocks.
        </p>
        <div style={styles.gasIndicator}>
          <div style={styles.gasStat}>
            <span style={styles.gasLabel}>Cached simulations:</span>
            <span style={styles.gasValue}>{stats?.hits ?? 0}</span>
          </div>
          <div style={styles.gasStat}>
            <span style={styles.gasLabel}>Uncached (recomputed):</span>
            <span style={styles.gasValue}>{stats?.misses ?? 0}</span>
          </div>
          <div style={styles.gasStat}>
            <span style={styles.gasLabel}>Cache hit rate:</span>
            <span style={styles.gasValue}>{getHitRate()}</span>
          </div>
        </div>
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
  statsRow: {
    display: 'grid',
    gridTemplateColumns: 'repeat(6, 1fr)',
    gap: '12px',
    marginBottom: '20px',
  },
  statCard: {
    padding: '12px',
    backgroundColor: '#f8f9fa',
    borderRadius: '8px',
    border: '1px solid #e0e0e0',
    textAlign: 'center',
  },
  statValue: {
    fontSize: '20px',
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
  progressBar: {
    height: '20px',
    backgroundColor: '#e9ecef',
    borderRadius: '10px',
    overflow: 'hidden',
    marginTop: '10px',
  },
  progressFill: {
    height: '100%',
    borderRadius: '10px',
    transition: 'width 0.3s ease',
  },
  progressLabel: {
    fontSize: '13px',
    color: '#6c757d',
    marginTop: '5px',
    textAlign: 'center',
  },
  lookupGrid: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '10px',
    alignItems: 'flex-end',
  },
  lookupField: {
    display: 'flex',
    flexDirection: 'column',
    flex: '1 1 180px',
  },
  lookupLabel: {
    fontSize: '12px',
    color: '#6c757d',
    marginBottom: '4px',
  },
  input: {
    padding: '8px 12px',
    borderRadius: '4px',
    border: '1px solid #ddd',
    fontSize: '13px',
    fontFamily: 'monospace',
  },
  select: {
    padding: '8px 12px',
    borderRadius: '4px',
    border: '1px solid #ddd',
    fontSize: '13px',
  },
  lookupActions: {
    display: 'flex',
    gap: '10px',
    flex: '0 0 auto',
  },
  lookupBtn: {
    padding: '8px 16px',
    backgroundColor: '#007bff',
    color: 'white',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '14px',
    fontWeight: 'bold',
  },
  clearBtn: {
    padding: '8px 16px',
    backgroundColor: '#dc3545',
    color: 'white',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '14px',
  },
  lookupResult: {
    marginTop: '15px',
    padding: '12px',
    backgroundColor: 'white',
    borderRadius: '4px',
    border: '1px solid #e0e0e0',
  },
  resultGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
    gap: '10px',
    marginTop: '8px',
  },
  resultItem: {
    display: 'flex',
    flexDirection: 'column',
    padding: '8px',
    backgroundColor: '#f8f9fa',
    borderRadius: '4px',
  },
  resultLabel: {
    fontSize: '11px',
    color: '#6c757d',
    marginBottom: '4px',
  },
  cacheHit: {
    color: '#28a745',
    fontWeight: 'bold',
  },
  cacheMiss: {
    color: '#dc3545',
    fontStyle: 'italic',
  },
  description: {
    fontSize: '13px',
    color: '#6c757d',
    lineHeight: '1.5',
    marginBottom: '10px',
  },
  gasIndicator: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    gap: '10px',
  },
  gasStat: {
    display: 'flex',
    flexDirection: 'column',
    padding: '10px',
    backgroundColor: 'white',
    borderRadius: '4px',
    border: '1px solid #e0e0e0',
  },
  gasLabel: {
    fontSize: '12px',
    color: '#6c757d',
    marginBottom: '4px',
  },
  gasValue: {
    fontSize: '18px',
    fontWeight: 'bold',
    color: '#333',
  },
};

export default SimulationCacheDashboard;
