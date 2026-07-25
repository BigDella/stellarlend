import React, { useState, useEffect, useCallback } from 'react';

interface MigrationPreview {
  estimatedGas: number;
  estimatedSlippageBps: number;
  interestImpact: number;
  expectedOutput: number;
  sourcePositionValue: number;
  destinationPoolApy: number;
  netBenefitBps: number;
}

interface MigrationHistoryEntry {
  id: number;
  sourcePool: string;
  destinationPool: string;
  asset: string;
  amount: number;
  status: string;
  createdAt: string;
  completedAt?: string;
}

interface Pool {
  address: string;
  name: string;
  apy: number;
  tvl: number;
}

interface MigrationState {
  sourcePool: string;
  destinationPool: string;
  amount: number;
  migrationPercentage: number;
  preview: MigrationPreview | null;
  history: MigrationHistoryEntry[];
  availablePools: Pool[];
  isLoading: boolean;
  isExecuting: boolean;
  error: string | null;
  step: 'configure' | 'preview' | 'confirm' | 'executing' | 'complete';
}

export const MigrationUI: React.FC = () => {
  const [state, setState] = useState<MigrationState>({
    sourcePool: '',
    destinationPool: '',
    amount: 0,
    migrationPercentage: 100,
    preview: null,
    history: [],
    availablePools: [],
    isLoading: false,
    isExecuting: false,
    error: null,
    step: 'configure',
  });

  const loadPools = useCallback(async () => {
    try {
      const res = await fetch('/api/pools');
      const data = await res.json();
      setState(prev => ({ ...prev, availablePools: data.pools || [] }));
    } catch (err) {
      console.error('Failed to load pools:', err);
    }
  }, []);

  const loadHistory = useCallback(async () => {
    try {
      const res = await fetch('/api/migration/history');
      const data = await res.json();
      setState(prev => ({ ...prev, history: data.migrations || [] }));
    } catch (err) {
      console.error('Failed to load migration history:', err);
    }
  }, []);

  useEffect(() => {
    loadPools();
    loadHistory();
  }, [loadPools, loadHistory]);

  const handlePreview = async () => {
    setState(prev => ({ ...prev, isLoading: true, error: null }));
    try {
      const response = await fetch('/api/migration/preview', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          sourcePool: state.sourcePool,
          destinationPool: state.destinationPool,
          amount: state.amount,
          percentage: state.migrationPercentage,
        }),
      });

      if (!response.ok) {
        throw new Error('Preview failed');
      }

      const data: MigrationPreview = await response.json();
      setState(prev => ({ ...prev, preview: data, isLoading: false, step: 'preview' }));
    } catch (error) {
      setState(prev => ({
        ...prev,
        isLoading: false,
        error: error instanceof Error ? error.message : 'Preview failed',
      }));
    }
  };

  const handleConfirm = () => {
    setState(prev => ({ ...prev, step: 'confirm' }));
  };

  const handleExecute = async () => {
    setState(prev => ({ ...prev, isExecuting: true, error: null, step: 'executing' }));
    try {
      const endpoint =
        state.migrationPercentage === 100 ? '/api/migration/full' : '/api/migration/partial';

      const response = await fetch(endpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          sourcePool: state.sourcePool,
          destinationPool: state.destinationPool,
          amount: state.amount,
          percentage: state.migrationPercentage,
        }),
      });

      if (!response.ok) {
        throw new Error('Migration failed');
      }

      setState(prev => ({
        ...prev,
        isExecuting: false,
        step: 'complete',
        amount: 0,
      }));
      loadHistory();
    } catch (error) {
      setState(prev => ({
        ...prev,
        isExecuting: false,
        error: error instanceof Error ? error.message : 'Migration failed',
        step: 'configure',
      }));
    }
  };

  const handleReset = () => {
    setState(prev => ({
      ...prev,
      sourcePool: '',
      destinationPool: '',
      amount: 0,
      migrationPercentage: 100,
      preview: null,
      error: null,
      step: 'configure',
    }));
  };

  const handleRollback = async (migrationId: number) => {
    try {
      await fetch(`/api/migration/rollback/${migrationId}`, { method: 'POST' });
      loadHistory();
    } catch (err) {
      console.error('Rollback failed:', err);
    }
  };

  const getSourcePoolName = () => {
    const pool = state.availablePools.find(p => p.address === state.sourcePool);
    return pool?.name || state.sourcePool.slice(0, 8) + '...';
  };

  const getDestPoolName = () => {
    const pool = state.availablePools.find(p => p.address === state.destinationPool);
    return pool?.name || state.destinationPool.slice(0, 8) + '...';
  };

  return (
    <div style={styles.container}>
      <h2 style={styles.title}>Pool Migration Tool</h2>

      {state.error && <div style={styles.errorBanner}>{state.error}</div>}

      {state.step === 'configure' && (
        <div style={styles.section}>
          <h3>Configure Migration</h3>

          <div style={styles.formGroup}>
            <label style={styles.label}>Source Pool</label>
            <select
              value={state.sourcePool}
              onChange={e => setState(prev => ({ ...prev, sourcePool: e.target.value }))}
              style={styles.select}
            >
              <option value="">Select source pool</option>
              {state.availablePools.map(pool => (
                <option key={pool.address} value={pool.address}>
                  {pool.name} (APY: {pool.apy.toFixed(2)}%)
                </option>
              ))}
            </select>
          </div>

          <div style={styles.formGroup}>
            <label style={styles.label}>Destination Pool</label>
            <select
              value={state.destinationPool}
              onChange={e => setState(prev => ({ ...prev, destinationPool: e.target.value }))}
              style={styles.select}
            >
              <option value="">Select destination pool</option>
              {state.availablePools
                .filter(p => p.address !== state.sourcePool)
                .map(pool => (
                  <option key={pool.address} value={pool.address}>
                    {pool.name} (APY: {pool.apy.toFixed(2)}%)
                  </option>
                ))}
            </select>
          </div>

          <div style={styles.formGroup}>
            <label style={styles.label}>Amount</label>
            <input
              type="number"
              placeholder="Amount to migrate"
              value={state.amount || ''}
              onChange={e =>
                setState(prev => ({ ...prev, amount: parseFloat(e.target.value) || 0 }))
              }
              style={styles.input}
            />
          </div>

          <div style={styles.formGroup}>
            <label style={styles.label}>
              Migration: {state.migrationPercentage}%
            </label>
            <input
              type="range"
              min="1"
              max="100"
              value={state.migrationPercentage}
              onChange={e =>
                setState(prev => ({
                  ...prev,
                  migrationPercentage: parseInt(e.target.value),
                }))
              }
              style={styles.slider}
            />
            <div style={styles.sliderLabels}>
              <span>1%</span>
              <span>100%</span>
            </div>
          </div>

          <button
            onClick={handlePreview}
            disabled={
              !state.sourcePool || !state.destinationPool || !state.amount || state.isLoading
            }
            style={styles.primaryButton}
          >
            {state.isLoading ? 'Loading...' : 'Preview Migration'}
          </button>
        </div>
      )}

      {state.step === 'preview' && state.preview && (
        <div style={styles.section}>
          <h3>Migration Preview</h3>

          <div style={styles.previewGrid}>
            <div style={styles.previewCard}>
              <div style={styles.previewLabel}>From</div>
              <div style={styles.previewValue}>{getSourcePoolName()}</div>
            </div>
            <div style={styles.previewCard}>
              <div style={styles.previewLabel}>To</div>
              <div style={styles.previewValue}>{getDestPoolName()}</div>
            </div>
            <div style={styles.previewCard}>
              <div style={styles.previewLabel}>Amount</div>
              <div style={styles.previewValue}>{state.amount.toLocaleString()}</div>
            </div>
            <div style={styles.previewCard}>
              <div style={styles.previewLabel}>Migration %</div>
              <div style={styles.previewValue}>{state.migrationPercentage}%</div>
            </div>
          </div>

          <div style={styles.detailsCard}>
            <h4>Cost Breakdown</h4>
            <div style={styles.detailRow}>
              <span>Estimated Gas</span>
              <span>{state.preview.estimatedGas.toLocaleString()} units</span>
            </div>
            <div style={styles.detailRow}>
              <span>Estimated Slippage</span>
              <span>{(state.preview.estimatedSlippageBps / 100).toFixed(2)}%</span>
            </div>
            <div style={styles.detailRow}>
              <span>Interest Impact</span>
              <span>{state.preview.interestImpact.toLocaleString()}</span>
            </div>
            <div style={styles.detailRow}>
              <span>Expected Output</span>
              <span>{state.preview.expectedOutput.toLocaleString()}</span>
            </div>
            <div style={styles.detailRow}>
              <span>Destination APY</span>
              <span>{state.preview.destinationPoolApy.toFixed(2)}%</span>
            </div>
            <div style={{ ...styles.detailRow, ...styles.detailRowHighlight }}>
              <span>Net Benefit</span>
              <span>+{state.preview.netBenefitBps} bps</span>
            </div>
          </div>

          <div style={styles.buttonGroup}>
            <button onClick={handleReset} style={styles.secondaryButton}>
              Back
            </button>
            <button onClick={handleConfirm} style={styles.primaryButton}>
              Confirm Migration
            </button>
          </div>
        </div>
      )}

      {state.step === 'confirm' && (
        <div style={styles.section}>
          <h3>Confirm Migration</h3>
          <div style={styles.confirmCard}>
            <p style={styles.confirmText}>
              You are about to migrate {state.migrationPercentage}% of your position from{' '}
              <strong>{getSourcePoolName()}</strong> to{' '}
              <strong>{getDestPoolName()}</strong>.
            </p>
            <p style={styles.confirmWarning}>
              This action cannot be undone. Proceed only if you are sure.
            </p>
          </div>
          <div style={styles.buttonGroup}>
            <button onClick={() => setState(prev => ({ ...prev, step: 'preview' }))} style={styles.secondaryButton}>
              Go Back
            </button>
            <button onClick={handleExecute} style={styles.dangerButton}>
              Execute Migration
            </button>
          </div>
        </div>
      )}

      {state.step === 'executing' && (
        <div style={styles.section}>
          <h3>Executing Migration...</h3>
          <div style={styles.loadingSpinner} />
          <p style={styles.loadingText}>Please wait while your migration is being processed.</p>
        </div>
      )}

      {state.step === 'complete' && (
        <div style={styles.section}>
          <h3>Migration Complete</h3>
          <div style={styles.successCard}>
            <p>Your migration has been submitted successfully.</p>
          </div>
          <button onClick={handleReset} style={styles.primaryButton}>
            New Migration
          </button>
        </div>
      )}

      {state.history.length > 0 && (
        <div style={styles.section}>
          <h3>Migration History</h3>
          <table style={styles.table}>
            <thead>
              <tr>
                <th style={styles.th}>Date</th>
                <th style={styles.th}>From</th>
                <th style={styles.th}>To</th>
                <th style={styles.th}>Amount</th>
                <th style={styles.th}>Status</th>
                <th style={styles.th}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {state.history.map(entry => (
                <tr key={entry.id} style={styles.tr}>
                  <td style={styles.td}>
                    {new Date(entry.createdAt).toLocaleDateString()}
                  </td>
                  <td style={styles.td}>{entry.sourcePool.slice(0, 8)}...</td>
                  <td style={styles.td}>{entry.destinationPool.slice(0, 8)}...</td>
                  <td style={styles.td}>{entry.amount.toLocaleString()}</td>
                  <td style={styles.td}>
                    <span
                      style={
                        entry.status === 'completed'
                          ? styles.statusSuccess
                          : entry.status === 'failed'
                          ? styles.statusError
                          : styles.statusPending
                      }
                    >
                      {entry.status}
                    </span>
                  </td>
                  <td style={styles.td}>
                    {entry.status === 'completed' && (
                      <button
                        onClick={() => handleRollback(entry.id)}
                        style={styles.rollbackButton}
                      >
                        Rollback
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  container: {
    padding: '24px',
    maxWidth: '800px',
    margin: '0 auto',
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
  },
  title: {
    fontSize: '24px',
    fontWeight: 600,
    marginBottom: '24px',
    color: '#1a1a2e',
  },
  section: {
    marginBottom: '24px',
    padding: '20px',
    backgroundColor: '#fff',
    borderRadius: '12px',
    border: '1px solid #e0e0e0',
    boxShadow: '0 1px 3px rgba(0,0,0,0.05)',
  },
  formGroup: {
    marginBottom: '16px',
  },
  label: {
    display: 'block',
    fontSize: '14px',
    fontWeight: 500,
    marginBottom: '6px',
    color: '#333',
  },
  input: {
    width: '100%',
    padding: '10px 12px',
    borderRadius: '8px',
    border: '1px solid #ddd',
    fontSize: '14px',
    boxSizing: 'border-box',
    outline: 'none',
  },
  select: {
    width: '100%',
    padding: '10px 12px',
    borderRadius: '8px',
    border: '1px solid #ddd',
    fontSize: '14px',
    backgroundColor: '#fff',
    boxSizing: 'border-box',
    outline: 'none',
  },
  slider: {
    width: '100%',
    marginTop: '8px',
  },
  sliderLabels: {
    display: 'flex',
    justifyContent: 'space-between',
    fontSize: '12px',
    color: '#888',
    marginTop: '4px',
  },
  primaryButton: {
    padding: '10px 24px',
    backgroundColor: '#0066ff',
    color: '#fff',
    border: 'none',
    borderRadius: '8px',
    fontSize: '14px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  secondaryButton: {
    padding: '10px 24px',
    backgroundColor: '#f5f5f5',
    color: '#333',
    border: '1px solid #ddd',
    borderRadius: '8px',
    fontSize: '14px',
    cursor: 'pointer',
  },
  dangerButton: {
    padding: '10px 24px',
    backgroundColor: '#dc3545',
    color: '#fff',
    border: 'none',
    borderRadius: '8px',
    fontSize: '14px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  rollbackButton: {
    padding: '4px 10px',
    backgroundColor: '#fff3cd',
    color: '#856404',
    border: '1px solid #ffc107',
    borderRadius: '4px',
    fontSize: '12px',
    cursor: 'pointer',
  },
  previewGrid: {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr',
    gap: '12px',
    marginBottom: '20px',
  },
  previewCard: {
    padding: '12px',
    backgroundColor: '#f8f9fa',
    borderRadius: '8px',
  },
  previewLabel: {
    fontSize: '12px',
    color: '#666',
    marginBottom: '4px',
  },
  previewValue: {
    fontSize: '16px',
    fontWeight: 600,
    color: '#1a1a2e',
  },
  detailsCard: {
    padding: '16px',
    backgroundColor: '#f8f9fa',
    borderRadius: '8px',
    marginBottom: '20px',
  },
  detailRow: {
    display: 'flex',
    justifyContent: 'space-between',
    padding: '8px 0',
    borderBottom: '1px solid #e9ecef',
    fontSize: '14px',
  },
  detailRowHighlight: {
    borderBottom: 'none',
    fontWeight: 600,
    color: '#28a745',
  },
  buttonGroup: {
    display: 'flex',
    gap: '12px',
    justifyContent: 'flex-end',
  },
  confirmCard: {
    padding: '20px',
    backgroundColor: '#fff3cd',
    borderRadius: '8px',
    marginBottom: '20px',
  },
  confirmText: {
    fontSize: '14px',
    marginBottom: '8px',
  },
  confirmWarning: {
    fontSize: '13px',
    color: '#856404',
    fontWeight: 500,
  },
  successCard: {
    padding: '20px',
    backgroundColor: '#d4edda',
    borderRadius: '8px',
    marginBottom: '20px',
    textAlign: 'center',
  },
  errorBanner: {
    padding: '12px 16px',
    backgroundColor: '#f8d7da',
    color: '#721c24',
    borderRadius: '8px',
    marginBottom: '16px',
    fontSize: '14px',
  },
  loadingSpinner: {
    width: '40px',
    height: '40px',
    border: '4px solid #f3f3f3',
    borderTop: '4px solid #0066ff',
    borderRadius: '50%',
    margin: '20px auto',
    animation: 'spin 1s linear infinite',
  },
  loadingText: {
    textAlign: 'center',
    color: '#666',
    fontSize: '14px',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    fontSize: '14px',
  },
  th: {
    textAlign: 'left',
    padding: '10px 12px',
    borderBottom: '2px solid #e0e0e0',
    color: '#666',
    fontSize: '12px',
    fontWeight: 600,
    textTransform: 'uppercase',
  },
  tr: {
    borderBottom: '1px solid #f0f0f0',
  },
  td: {
    padding: '10px 12px',
  },
  statusSuccess: {
    padding: '2px 8px',
    backgroundColor: '#d4edda',
    color: '#155724',
    borderRadius: '4px',
    fontSize: '12px',
  },
  statusError: {
    padding: '2px 8px',
    backgroundColor: '#f8d7da',
    color: '#721c24',
    borderRadius: '4px',
    fontSize: '12px',
  },
  statusPending: {
    padding: '2px 8px',
    backgroundColor: '#fff3cd',
    color: '#856404',
    borderRadius: '4px',
    fontSize: '12px',
  },
};
