import React, { useState, useEffect, useCallback } from 'react';

interface DebtTokenPosition {
  owner: string;
  principal: number;
  mintedTokens: number;
  accruedInterest: number;
  navPerToken: number;
  depositTimestamp: string;
}

interface DebtTokenConfig {
  name: string;
  symbol: string;
  totalSupply: number;
  totalPrincipal: number;
  interestIndex: number;
  navPerToken: number;
  isLocked: boolean;
}

interface DebtTokenPortfolioState {
  config: DebtTokenConfig | null;
  positions: DebtTokenPosition[];
  isLoading: boolean;
  action: 'deposit' | 'redeem' | 'transfer' | null;
  amount: number;
  recipient: string;
}

export const DebtTokenPortfolio: React.FC = () => {
  const [state, setState] = useState<DebtTokenPortfolioState>({
    config: null,
    positions: [],
    isLoading: true,
    action: null,
    amount: 0,
    recipient: '',
  });

  const loadPortfolio = useCallback(async () => {
    setState(prev => ({ ...prev, isLoading: true }));
    try {
      const [configRes, positionsRes] = await Promise.all([
        fetch('/api/debt-token/config'),
        fetch('/api/debt-token/positions'),
      ]);

      const config = await configRes.json();
      const positionsData = await positionsRes.json();

      setState(prev => ({
        ...prev,
        config,
        positions: positionsData.positions || [],
        isLoading: false,
      }));
    } catch (err) {
      setState(prev => ({ ...prev, isLoading: false }));
    }
  }, []);

  useEffect(() => {
    loadPortfolio();
  }, [loadPortfolio]);

  const handleDeposit = async () => {
    try {
      await fetch('/api/debt-token/deposit', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ amount: state.amount }),
      });
      setState(prev => ({ ...prev, action: null, amount: 0 }));
      loadPortfolio();
    } catch (err) {
      console.error('Deposit failed:', err);
    }
  };

  const handleRedeem = async () => {
    try {
      await fetch('/api/debt-token/redeem', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ amount: state.amount }),
      });
      setState(prev => ({ ...prev, action: null, amount: 0 }));
      loadPortfolio();
    } catch (err) {
      console.error('Redeem failed:', err);
    }
  };

  const handleTransfer = async () => {
    try {
      await fetch('/api/debt-token/transfer', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ to: state.recipient, amount: state.amount }),
      });
      setState(prev => ({ ...prev, action: null, amount: 0, recipient: '' }));
      loadPortfolio();
    } catch (err) {
      console.error('Transfer failed:', err);
    }
  };

  const totalValue = state.positions.reduce(
    (sum, p) => sum + p.mintedTokens * p.navPerToken,
    0
  );

  const totalInterest = state.positions.reduce(
    (sum, p) => sum + p.accruedInterest,
    0
  );

  return (
    <div style={styles.container}>
      <h2 style={styles.title}>Debt Token Portfolio</h2>

      {state.config && (
        <div style={styles.configBar}>
          <span style={styles.configItem}>
            <strong>{state.config.symbol}</strong>
          </span>
          <span style={styles.configItem}>
            Total Supply: {state.config.totalSupply.toLocaleString()}
          </span>
          <span style={styles.configItem}>
            NAV/Token: {state.config.navPerToken.toFixed(4)}
          </span>
          <span style={styles.configItem}>
            Interest Index: {(state.config.interestIndex / 10000).toFixed(4)}
          </span>
          {state.config.isLocked && (
            <span style={styles.lockedBadge}>Transfers Locked</span>
          )}
        </div>
      )}

      <div style={styles.summaryGrid}>
        <div style={styles.summaryCard}>
          <div style={styles.summaryLabel}>Total Value</div>
          <div style={styles.summaryValue}>{totalValue.toLocaleString(undefined, { maximumFractionDigits: 2 })}</div>
        </div>
        <div style={styles.summaryCard}>
          <div style={styles.summaryLabel}>Accrued Interest</div>
          <div style={styles.summaryValue} style={{ color: '#28a745' }}>
            +{totalInterest.toLocaleString(undefined, { maximumFractionDigits: 2 })}
          </div>
        </div>
        <div style={styles.summaryCard}>
          <div style={styles.summaryLabel}>Positions</div>
          <div style={styles.summaryValue}>{state.positions.length}</div>
        </div>
      </div>

      <div style={styles.actionBar}>
        <button
          onClick={() => setState(prev => ({ ...prev, action: 'deposit' }))}
          style={styles.actionButton}
        >
          Deposit & Mint
        </button>
        <button
          onClick={() => setState(prev => ({ ...prev, action: 'redeem' }))}
          style={styles.actionButton}
        >
          Redeem
        </button>
        <button
          onClick={() => setState(prev => ({ ...prev, action: 'transfer' }))}
          style={styles.actionButton}
        >
          Transfer
        </button>
      </div>

      {state.action && (
        <div style={styles.actionCard}>
          <h4 style={styles.actionTitle}>
            {state.action === 'deposit'
              ? 'Deposit & Mint Debt Tokens'
              : state.action === 'redeem'
              ? 'Redeem Debt Tokens'
              : 'Transfer Debt Tokens'}
          </h4>
          <div style={styles.formGroup}>
            <label style={styles.label}>Amount</label>
            <input
              type="number"
              value={state.amount || ''}
              onChange={e =>
                setState(prev => ({ ...prev, amount: parseFloat(e.target.value) || 0 }))
              }
              style={styles.input}
              placeholder="Enter amount"
            />
          </div>
          {state.action === 'transfer' && (
            <div style={styles.formGroup}>
              <label style={styles.label}>Recipient Address</label>
              <input
                type="text"
                value={state.recipient}
                onChange={e =>
                  setState(prev => ({ ...prev, recipient: e.target.value }))
                }
                style={styles.input}
                placeholder="Recipient address"
              />
            </div>
          )}
          <div style={styles.buttonGroup}>
            <button
              onClick={() => setState(prev => ({ ...prev, action: null, amount: 0 }))}
              style={styles.cancelButton}
            >
              Cancel
            </button>
            <button
              onClick={
                state.action === 'deposit'
                  ? handleDeposit
                  : state.action === 'redeem'
                  ? handleRedeem
                  : handleTransfer
              }
              style={styles.confirmButton}
              disabled={!state.amount || (state.action === 'transfer' && !state.recipient)}
            >
              Confirm
            </button>
          </div>
        </div>
      )}

      <div style={styles.tableContainer}>
        <h3 style={styles.tableTitle}>Your Positions</h3>
        <table style={styles.table}>
          <thead>
            <tr>
              <th style={styles.th}>Principal</th>
              <th style={styles.th}>Tokens</th>
              <th style={styles.th}>Interest Earned</th>
              <th style={styles.th}>Value (NAV)</th>
              <th style={styles.th}>Deposited</th>
            </tr>
          </thead>
          <tbody>
            {state.positions.length === 0 ? (
              <tr>
                <td colSpan={5} style={{ ...styles.td, textAlign: 'center', color: '#888' }}>
                  No positions found. Deposit to get started.
                </td>
              </tr>
            ) : (
              state.positions.map((pos, i) => (
                <tr key={i} style={styles.tr}>
                  <td style={styles.td}>{pos.principal.toLocaleString()}</td>
                  <td style={styles.td}>{pos.mintedTokens.toLocaleString()}</td>
                  <td style={{ ...styles.td, color: '#28a745' }}>
                    +{pos.accruedInterest.toLocaleString(undefined, { maximumFractionDigits: 2 })}
                  </td>
                  <td style={styles.td}>
                    {(pos.mintedTokens * pos.navPerToken).toLocaleString(undefined, {
                      maximumFractionDigits: 2,
                    })}
                  </td>
                  <td style={styles.td}>{new Date(pos.depositTimestamp).toLocaleDateString()}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  container: {
    padding: '24px',
    maxWidth: '900px',
    margin: '0 auto',
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
  },
  title: {
    fontSize: '24px',
    fontWeight: 600,
    marginBottom: '20px',
    color: '#1a1a2e',
  },
  configBar: {
    display: 'flex',
    gap: '20px',
    alignItems: 'center',
    padding: '12px 16px',
    backgroundColor: '#f8f9fa',
    borderRadius: '8px',
    marginBottom: '20px',
    fontSize: '13px',
  },
  configItem: {
    color: '#555',
  },
  lockedBadge: {
    padding: '2px 8px',
    backgroundColor: '#f8d7da',
    color: '#721c24',
    borderRadius: '4px',
    fontSize: '12px',
    fontWeight: 600,
  },
  summaryGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    gap: '16px',
    marginBottom: '20px',
  },
  summaryCard: {
    padding: '16px',
    backgroundColor: '#fff',
    borderRadius: '12px',
    border: '1px solid #e0e0e0',
    textAlign: 'center',
  },
  summaryLabel: {
    fontSize: '12px',
    color: '#666',
    marginBottom: '4px',
    textTransform: 'uppercase',
    fontWeight: 600,
  },
  summaryValue: {
    fontSize: '22px',
    fontWeight: 700,
    color: '#1a1a2e',
  },
  actionBar: {
    display: 'flex',
    gap: '12px',
    marginBottom: '20px',
  },
  actionButton: {
    padding: '10px 20px',
    backgroundColor: '#0066ff',
    color: '#fff',
    border: 'none',
    borderRadius: '8px',
    fontSize: '14px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  actionCard: {
    padding: '20px',
    backgroundColor: '#fff',
    borderRadius: '12px',
    border: '1px solid #e0e0e0',
    marginBottom: '20px',
  },
  actionTitle: {
    fontSize: '16px',
    fontWeight: 600,
    marginBottom: '16px',
  },
  formGroup: {
    marginBottom: '12px',
  },
  label: {
    display: 'block',
    fontSize: '13px',
    fontWeight: 500,
    marginBottom: '4px',
    color: '#555',
  },
  input: {
    width: '100%',
    padding: '10px 12px',
    borderRadius: '8px',
    border: '1px solid #ddd',
    fontSize: '14px',
    boxSizing: 'border-box',
  },
  buttonGroup: {
    display: 'flex',
    gap: '12px',
    justifyContent: 'flex-end',
  },
  cancelButton: {
    padding: '8px 20px',
    backgroundColor: '#f5f5f5',
    border: '1px solid #ddd',
    borderRadius: '6px',
    fontSize: '14px',
    cursor: 'pointer',
  },
  confirmButton: {
    padding: '8px 20px',
    backgroundColor: '#0066ff',
    color: '#fff',
    border: 'none',
    borderRadius: '6px',
    fontSize: '14px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  tableContainer: {
    backgroundColor: '#fff',
    borderRadius: '12px',
    border: '1px solid #e0e0e0',
    overflow: 'hidden',
  },
  tableTitle: {
    fontSize: '16px',
    fontWeight: 600,
    padding: '16px 20px 0',
    margin: 0,
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    fontSize: '13px',
  },
  th: {
    textAlign: 'left',
    padding: '12px 20px',
    borderBottom: '2px solid #e0e0e0',
    color: '#666',
    fontSize: '11px',
    fontWeight: 600,
    textTransform: 'uppercase',
  },
  tr: {
    borderBottom: '1px solid #f0f0f0',
  },
  td: {
    padding: '10px 20px',
  },
};
