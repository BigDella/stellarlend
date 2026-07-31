import React, { useState } from 'react';

interface PositionQuery {
  pool: string;
  user: string;
  asset: string;
}

interface HealthResult {
  pool: string;
  user: string;
  asset: string;
  collateral_balance: number;
  collateral_value: number;
  debt_balance: number;
  debt_value: number;
  health_factor: number;
  is_liquidatable: boolean;
  max_liquidatable: number;
  success: boolean;
}

interface HealthSummary {
  results: HealthResult[];
  total_positions: number;
  healthy_positions: number;
  liquidatable_positions: number;
  avg_health_factor: number;
}

export const BatchHealthCheck: React.FC = () => {
  const [queries, setQueries] = useState<PositionQuery[]>([
    { pool: '', user: '', asset: '' },
  ]);
  const [summary, setSummary] = useState<HealthSummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(0);
  const pageSize = 20;

  const addQuery = () => {
    setQueries([...queries, { pool: '', user: '', asset: '' }]);
  };

  const removeQuery = (index: number) => {
    setQueries(queries.filter((_, i) => i !== index));
  };

  const updateQuery = (index: number, field: keyof PositionQuery, value: string) => {
    const updated = [...queries];
    updated[index] = { ...updated[index], [field]: value };
    setQueries(updated);
  };

  const handleBatchCheck = async () => {
    const validQueries = queries.filter((q) => q.pool && q.user && q.asset);
    if (validQueries.length === 0) return;

    setLoading(true);
    try {
      const res = await fetch('/api/batch/health-check', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          queries: validQueries,
          offset: page * pageSize,
          limit: pageSize,
        }),
      });
      if (res.ok) {
        const data: HealthSummary = await res.json();
        setSummary(data);
      }
    } catch (error) {
      console.error('Batch health check failed:', error);
    }
    setLoading(false);
  };

  const getHealthColor = (hf: number): string => {
    if (hf >= 20000) return '#28a745';
    if (hf >= 15000) return '#17a2b8';
    if (hf >= 10000) return '#ffc107';
    if (hf > 0) return '#fd7e14';
    return '#dc3545';
  };

  const formatHealth = (hf: number): string => {
    if (hf === 100000000) return '∞ (no debt)';
    return (hf / 10000).toFixed(4);
  };

  return (
    <div style={styles.container}>
      <h2>Multi-Pool Batch Health Check</h2>

      <div style={styles.section}>
        <h3>Position Queries</h3>
        {queries.map((q, idx) => (
          <div key={idx} style={styles.queryRow}>
            <input
              type="text"
              value={q.pool}
              onChange={(e) => updateQuery(idx, 'pool', e.target.value)}
              placeholder="Pool address"
              style={{ ...styles.input, flex: 2 }}
            />
            <input
              type="text"
              value={q.user}
              onChange={(e) => updateQuery(idx, 'user', e.target.value)}
              placeholder="User address"
              style={{ ...styles.input, flex: 2 }}
            />
            <input
              type="text"
              value={q.asset}
              onChange={(e) => updateQuery(idx, 'asset', e.target.value)}
              placeholder="Asset address"
              style={{ ...styles.input, flex: 2 }}
            />
            {queries.length > 1 && (
              <button onClick={() => removeQuery(idx)} style={styles.removeBtn}>
                ✕
              </button>
            )}
          </div>
        ))}
        <div style={styles.queryActions}>
          <button onClick={addQuery} style={styles.addBtn}>
            + Add Position
          </button>
          <button
            onClick={handleBatchCheck}
            disabled={loading}
            style={styles.checkBtn}
          >
            {loading ? 'Checking...' : `Batch Health Check (${queries.filter((q) => q.pool && q.user && q.asset).length} positions)`}
          </button>
        </div>
      </div>

      {summary && (
        <>
          <div style={styles.statsRow}>
            <div style={styles.statCard}>
              <h3>Total Positions</h3>
              <div style={styles.statValue}>{summary.total_positions}</div>
            </div>
            <div style={styles.statCard}>
              <h3>Healthy</h3>
              <div style={{ ...styles.statValue, color: '#28a745' }}>
                {summary.healthy_positions}
              </div>
            </div>
            <div style={styles.statCard}>
              <h3>Liquidatable</h3>
              <div style={{ ...styles.statValue, color: '#dc3545' }}>
                {summary.liquidatable_positions}
              </div>
            </div>
            <div style={styles.statCard}>
              <h3>Avg Health Factor</h3>
              <div style={styles.statValue}>{formatHealth(summary.avg_health_factor)}</div>
            </div>
          </div>

          <div style={styles.section}>
            <h3>Results</h3>
            <table style={styles.table}>
              <thead>
                <tr>
                  <th>User</th>
                  <th>Collateral</th>
                  <th>Collateral Value</th>
                  <th>Debt</th>
                  <th>Debt Value</th>
                  <th>Health Factor</th>
                  <th>Status</th>
                  <th>Max Liquidatable</th>
                </tr>
              </thead>
              <tbody>
                {summary.results.map((result, idx) => (
                  <tr
                    key={idx}
                    style={result.is_liquidatable ? styles.liquidatableRow : undefined}
                  >
                    <td style={styles.addressCell}>{result.user.slice(0, 10)}...</td>
                    <td>{result.collateral_balance.toLocaleString()}</td>
                    <td>{result.collateral_value.toLocaleString()}</td>
                    <td>{result.debt_balance.toLocaleString()}</td>
                    <td>{result.debt_value.toLocaleString()}</td>
                    <td>
                      <span
                        style={{
                          ...styles.healthBadge,
                          backgroundColor: getHealthColor(result.health_factor),
                        }}
                      >
                        {formatHealth(result.health_factor)}
                      </span>
                    </td>
                    <td>
                      {result.is_liquidatable ? (
                        <span style={styles.liquidatableBadge}>LIQUIDATABLE</span>
                      ) : (
                        <span style={styles.healthyBadge}>HEALTHY</span>
                      )}
                    </td>
                    <td>{result.max_liquidatable.toLocaleString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {summary.total_positions > pageSize && (
            <div style={styles.pagination}>
              <button
                onClick={() => setPage(Math.max(0, page - 1))}
                disabled={page === 0}
                style={styles.pageBtn}
              >
                Previous
              </button>
              <span>
                Page {page + 1} of {Math.ceil(summary.total_positions / pageSize)}
              </span>
              <button
                onClick={() => setPage(page + 1)}
                disabled={(page + 1) * pageSize >= summary.total_positions}
                style={styles.pageBtn}
              >
                Next
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  container: {
    padding: '20px',
    maxWidth: '1200px',
    margin: '0 auto',
  },
  section: {
    marginBottom: '20px',
    padding: '15px',
    backgroundColor: '#f8f9fa',
    borderRadius: '8px',
    border: '1px solid #e0e0e0',
  },
  queryRow: {
    display: 'flex',
    gap: '10px',
    marginBottom: '8px',
    alignItems: 'center',
  },
  input: {
    padding: '8px 12px',
    borderRadius: '4px',
    border: '1px solid #ddd',
    fontSize: '13px',
    fontFamily: 'monospace',
  },
  removeBtn: {
    padding: '6px 10px',
    backgroundColor: '#dc3545',
    color: 'white',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '12px',
  },
  queryActions: {
    display: 'flex',
    gap: '10px',
    marginTop: '10px',
  },
  addBtn: {
    padding: '8px 16px',
    backgroundColor: '#6c757d',
    color: 'white',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '14px',
  },
  checkBtn: {
    padding: '8px 16px',
    backgroundColor: '#007bff',
    color: 'white',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '14px',
    fontWeight: 'bold',
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
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    fontSize: '13px',
  },
  liquidatableRow: {
    backgroundColor: '#fff3cd',
  },
  addressCell: {
    fontFamily: 'monospace',
    fontSize: '12px',
  },
  healthBadge: {
    display: 'inline-block',
    padding: '2px 8px',
    borderRadius: '4px',
    color: 'white',
    fontSize: '12px',
    fontWeight: 'bold',
  },
  liquidatableBadge: {
    display: 'inline-block',
    padding: '2px 8px',
    borderRadius: '4px',
    backgroundColor: '#dc3545',
    color: 'white',
    fontSize: '11px',
    fontWeight: 'bold',
  },
  healthyBadge: {
    display: 'inline-block',
    padding: '2px 8px',
    borderRadius: '4px',
    backgroundColor: '#28a745',
    color: 'white',
    fontSize: '11px',
    fontWeight: 'bold',
  },
  pagination: {
    display: 'flex',
    justifyContent: 'center',
    alignItems: 'center',
    gap: '15px',
    marginTop: '15px',
  },
  pageBtn: {
    padding: '6px 12px',
    backgroundColor: '#007bff',
    color: 'white',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '13px',
  },
};

export default BatchHealthCheck;
