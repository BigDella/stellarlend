import React, { useState, useEffect } from 'react';

interface RateGuardConfig {
  alert_threshold_bps: number;
  pause_threshold_bps: number;
  twap_window_secs: number;
  max_log_entries: number;
}

interface RateManipulationAttempt {
  address: string;
  amount: number;
  rate_impact_bps: number;
  old_rate_bps: number;
  new_rate_bps: number;
  timestamp: number;
  was_paused: boolean;
}

interface RateTwap {
  weighted_sum: number;
  total_time: number;
  twap_bps: number;
  last_update: number;
}

interface RateCheckResult {
  deviation_bps: number;
  will_alert: boolean;
  will_pause: boolean;
}

export const RateManipulationDashboard: React.FC = () => {
  const [config, setConfig] = useState<RateGuardConfig | null>(null);
  const [attempts, setAttempts] = useState<RateManipulationAttempt[]>([]);
  const [twap, setTwap] = useState<RateTwap | null>(null);
  const [checkRate, setCheckRate] = useState<string>('');
  const [checkResult, setCheckResult] = useState<RateCheckResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [whitelistInput, setWhitelistInput] = useState('');
  const [whitelistStatus, setWhitelistStatus] = useState<'idle' | 'checking' | 'result'>('idle');
  const [isWhitelisted, setIsWhitelisted] = useState(false);

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    setLoading(true);
    try {
      const [configRes, attemptsRes, twapRes] = await Promise.all([
        fetch('/api/rate-guard/config'),
        fetch('/api/rate-guard/attempts'),
        fetch('/api/rate-guard/twap'),
      ]);
      if (configRes.ok) setConfig(await configRes.json());
      if (attemptsRes.ok) setAttempts(await attemptsRes.json());
      if (twapRes.ok) setTwap(await twapRes.json());
    } catch (error) {
      console.error('Failed to load rate guard data:', error);
    }
    setLoading(false);
  };

  const handleCheckRate = async () => {
    const rateBps = parseInt(checkRate, 10);
    if (isNaN(rateBps)) return;
    try {
      const res = await fetch(`/api/rate-guard/check?rate=${rateBps}`);
      if (res.ok) {
        const result: RateCheckResult = await res.json();
        setCheckResult(result);
      }
    } catch (error) {
      console.error('Rate check failed:', error);
    }
  };

  const handleCheckWhitelist = async () => {
    if (!whitelistInput) return;
    setWhitelistStatus('checking');
    try {
      const res = await fetch(`/api/rate-guard/whitelist?address=${whitelistInput}`);
      if (res.ok) {
        const data = await res.json();
        setIsWhitelisted(data.whitelisted);
        setWhitelistStatus('result');
      }
    } catch (error) {
      console.error('Whitelist check failed:', error);
    }
  };

  const formatBps = (bps: number): string => `${(bps / 100).toFixed(2)}%`;

  const formatTimestamp = (ts: number): string => new Date(ts * 1000).toLocaleString();

  return (
    <div style={styles.container}>
      <h2>Rate Manipulation Guard</h2>

      <div style={styles.statsRow}>
        <div style={styles.statCard}>
          <h3>Alert Threshold</h3>
          <div style={styles.statValue}>{config ? formatBps(config.alert_threshold_bps) : '—'}</div>
        </div>
        <div style={styles.statCard}>
          <h3>Pause Threshold</h3>
          <div style={styles.statValue}>{config ? formatBps(config.pause_threshold_bps) : '—'}</div>
        </div>
        <div style={styles.statCard}>
          <h3>TWAP Rate</h3>
          <div style={styles.statValue}>{twap ? formatBps(twap.twap_bps) : '—'}</div>
        </div>
        <div style={styles.statCard}>
          <h3>Attempts Logged</h3>
          <div style={styles.statValue}>{attempts.length}</div>
        </div>
      </div>

      <div style={styles.section}>
        <h3>Rate Check Simulator</h3>
        <div style={styles.inputRow}>
          <input
            type="number"
            value={checkRate}
            onChange={(e) => setCheckRate(e.target.value)}
            placeholder="Enter rate in bps"
            style={styles.input}
          />
          <button onClick={handleCheckRate} style={styles.button}>
            Check
          </button>
        </div>
        {checkResult && (
          <div style={styles.checkResult}>
            <span>Deviation: {formatBps(checkResult.deviation_bps)}</span>
            <span style={{ color: checkResult.will_alert ? '#ffc107' : '#28a745' }}>
              {checkResult.will_alert ? 'ALERT' : 'OK'}
            </span>
            <span style={{ color: checkResult.will_pause ? '#dc3545' : '#28a745' }}>
              {checkResult.will_pause ? 'WILL PAUSE' : 'SAFE'}
            </span>
          </div>
        )}
      </div>

      <div style={styles.section}>
        <h3>Whitelisted Aggregators</h3>
        <div style={styles.inputRow}>
          <input
            type="text"
            value={whitelistInput}
            onChange={(e) => setWhitelistInput(e.target.value)}
            placeholder="Contract address"
            style={styles.input}
          />
          <button onClick={handleCheckWhitelist} style={styles.button}>
            Check Status
          </button>
        </div>
        {whitelistStatus === 'result' && (
          <div style={styles.whitelistResult}>
            {isWhitelisted ? '✓ Whitelisted' : '✗ Not whitelisted'}
          </div>
        )}
      </div>

      <div style={styles.section}>
        <h3>Manipulation Attempt Log</h3>
        <table style={styles.table}>
          <thead>
            <tr>
              <th>Address</th>
              <th>Amount</th>
              <th>Old Rate</th>
              <th>New Rate</th>
              <th>Impact</th>
              <th>Paused</th>
              <th>Time</th>
            </tr>
          </thead>
          <tbody>
            {attempts.map((attempt, idx) => (
              <tr key={idx} style={attempt.was_paused ? styles.pausedRow : undefined}>
                <td style={styles.addressCell}>{attempt.address.slice(0, 10)}...</td>
                <td>{attempt.amount.toLocaleString()}</td>
                <td>{formatBps(attempt.old_rate_bps)}</td>
                <td>{formatBps(attempt.new_rate_bps)}</td>
                <td style={{ color: attempt.rate_impact_bps > 2500 ? '#dc3545' : '#ffc107' }}>
                  {formatBps(attempt.rate_impact_bps)}
                </td>
                <td>{attempt.was_paused ? '🛑 YES' : '⚠️ NO'}</td>
                <td>{formatTimestamp(attempt.timestamp)}</td>
              </tr>
            ))}
          </tbody>
        </table>
        {attempts.length === 0 && <p style={styles.emptyText}>No manipulation attempts recorded.</p>}
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
    gridTemplateColumns: 'repeat(4, 1fr)',
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
  inputRow: {
    display: 'flex',
    gap: '10px',
    marginBottom: '10px',
  },
  input: {
    flex: 1,
    padding: '8px 12px',
    borderRadius: '4px',
    border: '1px solid #ddd',
    fontSize: '14px',
  },
  button: {
    padding: '8px 16px',
    backgroundColor: '#007bff',
    color: 'white',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '14px',
  },
  checkResult: {
    display: 'flex',
    gap: '15px',
    padding: '10px',
    backgroundColor: '#e9ecef',
    borderRadius: '4px',
    fontSize: '14px',
    fontWeight: 'bold',
  },
  whitelistResult: {
    padding: '10px',
    borderRadius: '4px',
    fontSize: '14px',
    fontWeight: 'bold',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    fontSize: '13px',
  },
  pausedRow: {
    backgroundColor: '#fff3cd',
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

export default RateManipulationDashboard;
